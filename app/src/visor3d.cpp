#include "visor3d.h"

#include <QFile>
#include <QMatrix4x4>
#include <QMetaObject>
#include <QOpenGLBuffer>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QOpenGLFunctions>
#include <QOpenGLShaderProgram>
#include <QPointer>
#include <QQuickOpenGLUtils>
#include <QtEndian>
#include <cmath>

namespace {

// GLSL 1.20 sin `#version`: lo entiende el contexto de compatibilidad que
// abre Qt Quick tanto con la tarjeta como con llvmpipe en una pantalla virtual.
const char *VERTICES = R"(
attribute vec3 posicion;
uniform mat4 mvp;
uniform mat4 mv;
varying vec3 enVista;
void main() {
    enVista = (mv * vec4(posicion, 1.0)).xyz;
    gl_Position = mvp * vec4(posicion, 1.0);
}
)";

// La normal sale de cómo cambia la posición entre píxeles vecinos: cara plana
// y sin tener que guardarla en el archivo. Luz de dos lados, porque muchos
// modelos traen caras con el sentido cambiado y no por eso son negros. Las
// mismas luces que la miniatura del núcleo, para que abrir no sea un salto.
const char *FRAGMENTOS = R"(
varying vec3 enVista;
uniform vec3 color;
uniform float plano;
void main() {
    if (plano > 0.5) { gl_FragColor = vec4(color, plano - 0.5); return; }
    vec3 n = normalize(cross(dFdx(enVista), dFdy(enVista)));
    if (n.z < 0.0) n = -n;
    vec3 clave = normalize(vec3(-0.45, 0.65, 0.62));
    vec3 relleno = normalize(vec3(0.7, 0.1, 0.5));
    float i = 0.16 + 0.70 * max(dot(n, clave), 0.0)
                   + 0.22 * max(dot(n, relleno), 0.0)
                   + 0.18 * pow(1.0 - n.z, 2.0);
    gl_FragColor = vec4(color * min(i, 1.25), 1.0);
}
)";

class Pintor : public QQuickFramebufferObject::Renderer, protected QOpenGLFunctions {
public:
    Pintor() { initializeOpenGLFunctions(); }

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

    void cargar()
    {
        m_n = 0;
        QFile f(m_archivo);
        QByteArray b;
        if (!m_archivo.isEmpty() && f.open(QIODevice::ReadOnly))
            b = f.readAll();
        // Formato en grimorio-core/src/modelo3d.rs, `escribir_malla`.
        if (b.size() >= 32 && b.startsWith("GRIMALLA")) {
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
            }
        }
        QPointer<Visor3D> it(m_item);
        const int n = m_n;
        QMetaObject::invokeMethod(
            m_item, [it, n] { if (it) it->anotarCargado(n); }, Qt::QueuedConnection);
    }

    void render() override
    {
        if (!m_programa.isLinked()) {
            m_programa.addShaderFromSourceCode(QOpenGLShader::Vertex, VERTICES);
            m_programa.addShaderFromSourceCode(QOpenGLShader::Fragment, FRAGMENTOS);
            m_programa.bindAttributeLocation("posicion", 0);
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
        m_programa.bind();
        m_programa.setUniformValue("mvp", p * mv);
        m_programa.setUniformValue("mv", mv);
        m_vbo.bind();
        m_programa.enableAttributeArray(0);
        m_programa.setAttributeBuffer(0, GL_FLOAT, 0, 3, 12);

        const auto rgb = [](const QColor &c) { return QVector3D(c.redF(), c.greenF(), c.blueF()); };
        m_programa.setUniformValue("plano", 0.0f);
        m_programa.setUniformValue("color", rgb(m_pieza));
        if (m_alambre) {
            // Primero las caras un poco hacia atrás y luego las aristas encima:
            // así se ven solo las aristas de delante, no la madeja entera.
            glEnable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(1, 1);
        }
        glDrawArrays(GL_TRIANGLES, 0, m_n * 3);
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
        m_vbo.release();
        m_programa.release();
        glDisable(GL_DEPTH_TEST);
        // Qt Quick comparte el contexto: lo que se toca se deja como estaba.
        QQuickOpenGLUtils::resetOpenGLState();
    }

private:
    QPointer<Visor3D> m_item;
    QOpenGLShaderProgram m_programa;
    QOpenGLBuffer m_vbo{QOpenGLBuffer::VertexBuffer};
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
