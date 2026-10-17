#include "visor3d.h"

#include <QFile>
#include <QImage>
#include <QMatrix4x4>
#include <QMetaObject>
#include <QOpenGLBuffer>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QOpenGLFunctions>
#include <QOpenGLShaderProgram>
#include <QPointer>
#include <QQuickOpenGLUtils>
#include <QVector4D>
#include <QtEndian>
#include <cmath>

namespace {

// GLSL 1.20 sin `#version`: lo entiende el contexto de compatibilidad que
// abre Qt Quick tanto con la tarjeta como con llvmpipe en una pantalla virtual.
const char *VERTICES = R"(
attribute vec3 posicion;
attribute vec2 coord;
uniform mat4 mvp;
uniform mat4 mv;
varying vec3 enVista;
varying vec2 enTextura;
void main() {
    enVista = (mv * vec4(posicion, 1.0)).xyz;
    enTextura = coord;
    gl_Position = mvp * vec4(posicion, 1.0);
}
)";

// La normal sale de cómo cambia la posición entre píxeles vecinos: cara plana
// y sin tener que guardarla en el archivo. Luz de dos lados, porque muchos
// modelos traen caras con el sentido cambiado y no por eso son negros. Las
// mismas luces que la miniatura del núcleo, para que abrir no sea un salto.
const char *FRAGMENTOS = R"(
varying vec3 enVista;
varying vec2 enTextura;
uniform vec4 color;
uniform float plano;
uniform sampler2D textura;
uniform float conTextura;
uniform float corte;
void main() {
    if (plano > 0.5) { gl_FragColor = vec4(color.rgb, plano - 0.5); return; }
    vec4 base = color;
    if (conTextura > 0.5) base *= texture2D(textura, enTextura);
    if (corte > 0.0 && base.a < corte) discard;
    vec3 n = normalize(cross(dFdx(enVista), dFdy(enVista)));
    if (n.z < 0.0) n = -n;
    vec3 clave = normalize(vec3(-0.45, 0.65, 0.62));
    vec3 relleno = normalize(vec3(0.7, 0.1, 0.5));
    float i = 0.16 + 0.70 * max(dot(n, clave), 0.0)
                   + 0.22 * max(dot(n, relleno), 0.0)
                   + 0.18 * pow(1.0 - n.z, 2.0);
    gl_FragColor = vec4(base.rgb * min(i, 1.25), 1.0);
}
)";

// Una parte del modelo con el mismo material: se pinta de una vez.
struct Grupo {
    int primero = 0, cuantos = 0;
    QVector4D color{1, 1, 1, 1};
    int textura = -1;
    float corte = 0;
    bool dosCaras = true;
    // Sin color propio: va del color de la pieza, como un STL.
    bool sinColor = true;
};

class Pintor : public QQuickFramebufferObject::Renderer, protected QOpenGLFunctions {
public:
    Pintor() { initializeOpenGLFunctions(); }
    ~Pintor() override { soltarTexturas(); }

    QOpenGLFramebufferObject *createFramebufferObject(const QSize &tam) override
    {
        QOpenGLFramebufferObjectFormat f;
        f.setAttachment(QOpenGLFramebufferObject::CombinedDepthStencil);
        f.setSamples(4);
        return new QOpenGLFramebufferObject(tam, f);
    }

    void synchronize(QQuickFramebufferObject *item) override
    {
        auto *v = static_cast<Visor3D *>(item);
        m_item = v;
        m_giro = float(v->giro());
        m_incl = float(v->inclinacion());
        m_acercar = float(v->acercar());
        m_pan = v->pan();
        m_alambre = v->alambre();
        m_vueltasX = v->vueltasX();
        m_vueltasZ = v->vueltasZ();
        m_fondo = v->fondo();
        m_pieza = v->pieza();
        m_colorAlambre = v->colorAlambre();
        if (v->archivo() != m_archivo) {
            m_archivo = v->archivo();
            cargar();
        }
    }

    void soltarTexturas()
    {
        if (!m_texturas.isEmpty())
            glDeleteTextures(GLsizei(m_texturas.size()), m_texturas.constData());
        m_texturas.clear();
    }

    // Sube una imagen tal cual venía en el modelo. Fila a fila como está, sin
    // darle la vuelta: la primera fila es v = 0, como en glTF.
    GLuint subirTextura(const QByteArray &bytes)
    {
        QImage img = QImage::fromData(bytes);
        if (img.isNull())
            return 0;
        GLint tope = 2048;
        glGetIntegerv(GL_MAX_TEXTURE_SIZE, &tope);
        tope = qMin(tope, 4096);
        if (img.width() > tope || img.height() > tope)
            img = img.scaled(tope, tope, Qt::KeepAspectRatio, Qt::SmoothTransformation);
        img = img.convertToFormat(QImage::Format_RGBA8888);
        GLuint t = 0;
        glGenTextures(1, &t);
        glBindTexture(GL_TEXTURE_2D, t);
        glPixelStorei(GL_UNPACK_ALIGNMENT, 4);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, img.width(), img.height(), 0, GL_RGBA,
                     GL_UNSIGNED_BYTE, img.constBits());
        glGenerateMipmap(GL_TEXTURE_2D);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT);
        glBindTexture(GL_TEXTURE_2D, 0);
        return t;
    }

    void cargar()
    {
        m_n = 0;
        m_grupos.clear();
        m_conUv = false;
        soltarTexturas();
        QFile f(m_archivo);
        QByteArray b;
        if (!m_archivo.isEmpty() && f.open(QIODevice::ReadOnly))
            b = f.readAll();
        // Formato en grimorio-core/src/modelo3d.rs, `escribir_malla`.
        if (b.size() >= 32 && b.startsWith("GRIMALLA")) {
            const quint32 version = qFromLittleEndian<quint32>(b.constData() + 8);
            const quint32 n = qFromLittleEndian<quint32>(b.constData() + 12);
            if (qint64(b.size()) >= 32 + qint64(n) * 36) {
                if (!m_vbo.isCreated())
                    m_vbo.create();
                m_vbo.bind();
                m_vbo.allocate(b.constData() + 32, int(n * 36));
                m_vbo.release();
                m_n = int(n);
                // Encajar por la esfera que la contiene y no por la caja: la
                // caja girada asoma por las esquinas, y se cortaba una punta.
                float r2 = 0;
                const char *v = b.constData() + 32;
                for (quint32 k = 0; k < n * 3; ++k) {
                    const float x = qFromLittleEndian<float>(v + k * 12);
                    const float y = qFromLittleEndian<float>(v + k * 12 + 4);
                    const float z = qFromLittleEndian<float>(v + k * 12 + 8);
                    r2 = qMax(r2, x * x + y * y + z * z);
                }
                m_radio = r2 > 0 ? std::sqrt(r2) : 1;
                if (version >= 2)
                    leerMateriales(b, 32 + qint64(n) * 36, n);
                if (m_grupos.isEmpty()) {
                    Grupo todo;
                    todo.cuantos = m_n;
                    m_grupos.append(todo);
                }
            }
        }
        QPointer<Visor3D> it(m_item);
        const int n = m_n;
        QMetaObject::invokeMethod(
            m_item, [it, n] { if (it) it->anotarCargado(n); }, Qt::QueuedConnection);
    }

    // Lo que va detrás de los vértices desde la versión 2. Si algo no cuadra
    // se queda sin materiales y se pinta del color de la pieza, como antes.
    void leerMateriales(const QByteArray &b, qint64 o, quint32 n)
    {
        const char *d = b.constData();
        const auto hay = [&](qint64 bytes) { return bytes >= 0 && o + bytes <= b.size(); };
        const auto u32 = [&]() {
            const quint32 x = qFromLittleEndian<quint32>(d + o);
            o += 4;
            return x;
        };
        const auto f32 = [&]() {
            const float x = qFromLittleEndian<float>(d + o);
            o += 4;
            return x;
        };
        if (!hay(4))
            return;
        const quint32 ng = u32();
        if (!hay(qint64(ng) * 36))
            return;
        QVector<Grupo> grupos;
        for (quint32 k = 0; k < ng; ++k) {
            Grupo g;
            g.primero = int(u32());
            g.cuantos = int(u32());
            const float r = f32(), v = f32(), a = f32(), al = f32();
            g.color = QVector4D(r, v, a, al);
            g.textura = qint32(u32());
            g.corte = f32();
            const quint32 banderas = u32();
            g.dosCaras = banderas & 1;
            g.sinColor = banderas & 2;
            if (g.primero < 0 || g.cuantos < 0 || qint64(g.primero) + g.cuantos > qint64(n))
                return;
            grupos.append(g);
        }
        if (!hay(4))
            return;
        if (u32() == 1) {
            if (!hay(qint64(n) * 24))
                return;
            if (!m_vboUv.isCreated())
                m_vboUv.create();
            m_vboUv.bind();
            m_vboUv.allocate(d + o, int(n * 24));
            m_vboUv.release();
            o += qint64(n) * 24;
            m_conUv = true;
        }
        m_grupos = grupos;
        if (!hay(4))
            return;
        const quint32 nt = u32();
        for (quint32 k = 0; k < nt && hay(4); ++k) {
            const quint32 largo = u32();
            if (!hay(largo))
                break;
            m_texturas.append(subirTextura(QByteArray::fromRawData(d + o, int(largo))));
            o += largo;
        }
    }

    void render() override
    {
        if (!m_programa.isLinked()) {
            m_programa.addShaderFromSourceCode(QOpenGLShader::Vertex, VERTICES);
            m_programa.addShaderFromSourceCode(QOpenGLShader::Fragment, FRAGMENTOS);
            m_programa.bindAttributeLocation("posicion", 0);
            m_programa.bindAttributeLocation("coord", 1);
            m_programa.link();
        }
        const QColor &fo = m_fondo;
        glClearColor(fo.redF(), fo.greenF(), fo.blueF(), 1);
        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        if (m_n == 0 || !m_programa.isLinked())
            return;

        const QSize tam = framebufferObject()->size();
        const float aspecto = tam.height() > 0 ? float(tam.width()) / tam.height() : 1;
        // Ortográfica: en una pieza de impresión las paralelas se quieren
        // paralelas, y medir a ojo con perspectiva engaña.
        const float lado = m_radio * 1.06f / qMax(0.05f, m_acercar);
        QMatrix4x4 p;
        if (aspecto >= 1)
            p.ortho(-lado * aspecto, lado * aspecto, -lado, lado, -10, 10);
        else
            p.ortho(-lado, lado, -lado / aspecto, lado / aspecto, -10, 10);
        QMatrix4x4 mv;
        mv.translate(m_pan);
        mv.rotate(m_incl, 1, 0, 0);
        mv.rotate(m_giro, 0, 1, 0);
        // La corrección va la última: se aplica a la pieza antes que la vista,
        // así «arriba» sigue siendo arriba después de enderezarla.
        mv.rotate(90.0f * m_vueltasZ, 0, 0, 1);
        mv.rotate(90.0f * m_vueltasX, 1, 0, 0);

        glEnable(GL_DEPTH_TEST);
        glDepthFunc(GL_LESS);
        glDisable(GL_CULL_FACE);
        glFrontFace(GL_CCW);
        m_programa.bind();
        m_programa.setUniformValue("mvp", p * mv);
        m_programa.setUniformValue("mv", mv);
        m_programa.setUniformValue("textura", 0);
        m_vbo.bind();
        m_programa.enableAttributeArray(0);
        m_programa.setAttributeBuffer(0, GL_FLOAT, 0, 3, 12);
        m_vbo.release();
        if (m_conUv) {
            m_vboUv.bind();
            m_programa.enableAttributeArray(1);
            m_programa.setAttributeBuffer(1, GL_FLOAT, 0, 2, 8);
            m_vboUv.release();
        } else {
            m_programa.disableAttributeArray(1);
            m_programa.setAttributeValue(1, 0.0f, 0.0f);
        }

        const auto rgb = [](const QColor &c) { return QVector4D(c.redF(), c.greenF(), c.blueF(), 1); };
        m_programa.setUniformValue("plano", 0.0f);
        if (m_alambre) {
            // Primero las caras un poco hacia atrás y luego las aristas encima:
            // así se ven solo las aristas de delante, no la madeja entera.
            glEnable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(1, 1);
        }
        glActiveTexture(GL_TEXTURE0);
        for (const Grupo &g : std::as_const(m_grupos)) {
            const GLuint t =
                g.textura >= 0 && g.textura < m_texturas.size() ? m_texturas[g.textura] : 0;
            m_programa.setUniformValue("color", g.sinColor ? rgb(m_pieza) : g.color);
            m_programa.setUniformValue("conTextura", t && m_conUv ? 1.0f : 0.0f);
            m_programa.setUniformValue("corte", t ? g.corte : 0.0f);
            glBindTexture(GL_TEXTURE_2D, t);
            // Lo que solo tiene cara por delante no se pinta por detrás: así
            // desaparece la cáscara del contorno de un personaje de dibujo.
            if (g.dosCaras) {
                glDisable(GL_CULL_FACE);
            } else {
                glEnable(GL_CULL_FACE);
                glCullFace(GL_BACK);
            }
            glDrawArrays(GL_TRIANGLES, g.primero * 3, g.cuantos * 3);
        }
        glBindTexture(GL_TEXTURE_2D, 0);
        glDisable(GL_CULL_FACE);
        m_programa.setUniformValue("conTextura", 0.0f);
        m_programa.setUniformValue("corte", 0.0f);
        if (m_alambre) {
            glDisable(GL_POLYGON_OFFSET_FILL);
            // glPolygonMode no está en el subconjunto ES que trae
            // QOpenGLFunctions; en un contexto de escritorio sí existe.
            using Modo = void (*)(GLenum, GLenum);
            static Modo modo = reinterpret_cast<Modo>(
                QOpenGLContext::currentContext()->getProcAddress("glPolygonMode"));
            if (modo) {
                // Aristas a media luz y no opacas: en una malla de impresión de
                // un millón de triángulos, opacas tapan la pieza entera y lo
                // que se ve es un borrón de color. Así lo denso se lee denso.
                modo(GL_FRONT_AND_BACK, 0x1B01 /* GL_LINE */);
                glEnable(GL_BLEND);
                glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                glDepthMask(GL_FALSE);
                // `plano` lleva la opacidad más medio: por encima de 0,5 es línea.
                m_programa.setUniformValue("plano", 0.5f + 0.45f);
                m_programa.setUniformValue("color", rgb(m_colorAlambre));
                glDrawArrays(GL_TRIANGLES, 0, m_n * 3);
                glDepthMask(GL_TRUE);
                glDisable(GL_BLEND);
                modo(GL_FRONT_AND_BACK, 0x1B02 /* GL_FILL */);
            }
        }
        m_programa.disableAttributeArray(0);
        m_programa.disableAttributeArray(1);
        m_programa.release();
        glDisable(GL_DEPTH_TEST);
        // Qt Quick comparte el contexto: lo que se toca se deja como estaba.
        QQuickOpenGLUtils::resetOpenGLState();
    }

private:
    QPointer<Visor3D> m_item;
    QOpenGLShaderProgram m_programa;
    QOpenGLBuffer m_vbo{QOpenGLBuffer::VertexBuffer};
    QOpenGLBuffer m_vboUv{QOpenGLBuffer::VertexBuffer};
    QVector<Grupo> m_grupos;
    QVector<GLuint> m_texturas;
    bool m_conUv = false;
    QString m_archivo;
    int m_n = 0;
    float m_radio = 1;
    float m_giro = 0, m_incl = 0, m_acercar = 1;
    QVector3D m_pan;
    bool m_alambre = false;
    int m_vueltasX = 0, m_vueltasZ = 0;
    QColor m_fondo, m_pieza, m_colorAlambre;
};

} // namespace

Visor3D::Visor3D(QQuickItem *padre) : QQuickFramebufferObject(padre)
{
    setMirrorVertically(true);
    connect(this, &Visor3D::cambio, this, &QQuickItem::update);
}

QQuickFramebufferObject::Renderer *Visor3D::createRenderer() const
{
    return new Pintor;
}

void Visor3D::setArchivo(const QString &a)
{
    if (a == m_archivo)
        return;
    m_archivo = a;
    emit archivoCambio();
    update();
}

void Visor3D::anotarCargado(int n)
{
    m_triangulos = n;
    emit cargado();
}
