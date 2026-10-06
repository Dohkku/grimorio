#include "proveedor.h"

#include "modelo.h"
#include "original.h"

#include <QBuffer>
#include <QFile>
#include <QImage>
#include <QImageReader>
#include <QPainter>
#include <QPainterPath>
#include <QQuickImageResponse>
#include <QRunnable>
#include <QThreadPool>
#include <QtGlobal>

#include <atomic>

namespace {

/// Máscara de esquinas en nueve trozos: todo es del color de fondo salvo el
/// interior redondeado, que es transparente. Se superpone a cada celda con un
/// BorderImage, así que una sola textura redondea las esquinas de la malla
/// entera sin necesidad de shaders propios.
///
/// El Qt 6.4 de Ubuntu viene sin qsb ni QtShaderTools, así que un ShaderEffect
/// con una SDF no se puede compilar sin añadir otro paquete de desarrollo. Este
/// truco da el mismo resultado mientras el fondo sea plano, que es el caso.
QImage mascaraEsquinas(qreal radio, const QColor &fondo)
{
    const int r = qMax(1, qRound(radio));
    const int lado = r * 2 + 2;
    QImage img(lado, lado, QImage::Format_ARGB32_Premultiplied);
    img.fill(fondo);
    QPainter p(&img);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setCompositionMode(QPainter::CompositionMode_Source);
    QPainterPath camino;
    camino.addRoundedRect(QRectF(0, 0, lado, lado), r, r);
    p.fillPath(camino, Qt::transparent);
    return img;
}

/// Decodifica ya al tamaño pedido, en vez de entera y encogida después.
///
/// `QImageReader::setScaledSize` llega hasta el decodificador de JPEG, que sabe
/// reconstruir la imagen a la mitad, a un cuarto o a un octavo directamente
/// desde los coeficientes, sin calcular los píxeles que luego tiraría. Antes se
/// decodificaba la miniatura entera de 320 px y se encogía con
/// `SmoothTransformation`: dos pasadas completas por cada celda pequeña.
///
/// Mismo contrato que tenía: se respeta el alto pedido y, si no hay, el ancho;
/// y solo se encoge, nunca se agranda.
QImage leerAlTamano(QImageReader &lector, const QSize &pedido)
{
    const QSize entera = lector.size();
    if (entera.isValid() && !entera.isEmpty()) {
        QSize destino;
        if (pedido.height() > 0 && pedido.height() < entera.height())
            destino = QSize(qMax(1, qRound(qreal(entera.width()) * pedido.height() / entera.height())),
                            pedido.height());
        else if (pedido.width() > 0 && pedido.width() < entera.width())
            destino = QSize(pedido.width(),
                            qMax(1, qRound(qreal(entera.height()) * pedido.width() / entera.width())));
        if (destino.isValid())
            lector.setScaledSize(destino);
    }
    return lector.read();
}

class Respuesta : public QQuickImageResponse {
public:
    /// Qt lo llama cuando la celda que pidió la imagen ya no la quiere: se ha
    /// salido de pantalla con un desplazamiento rápido, o ha cambiado de
    /// elemento. Llega desde el hilo de carga de Qt, por eso es atómico.
    void cancel() override { m_cancelada.store(true, std::memory_order_relaxed); }
    bool cancelada() const { return m_cancelada.load(std::memory_order_relaxed); }

    void terminar(const QImage &img)
    {
        m_img = img;
        emit finished();
    }
    void fallar(const QString &motivo)
    {
        m_error = motivo;
        emit finished();
    }
    QQuickTextureFactory *textureFactory() const override
    {
        return QQuickTextureFactory::textureFactoryForImage(m_img);
    }
    QString errorString() const override { return m_error; }

private:
    QImage m_img;
    QString m_error;
    std::atomic<bool> m_cancelada { false };
};

/// Si la respuesta ya no hace falta, la cierra sin trabajar y dice que sí.
///
/// Lo que Qt todavía no nos ha pedido ya lo descarta él solo. Esto cubre lo que
/// sí llegó a la cola del reparto: sin mirarlo, una celda que ya ha pasado de
/// largo se decodificaba igual y las que de verdad están en pantalla esperaban
/// detrás. Con miniaturas pequeñas y una máquina holgada casi nunca ocurre —en
/// el banco de cien mil salen de cero a una decena por pasada—; con
/// previsualizaciones u originales, que tardan, o con el procesador ocupado,
/// es donde ahorra. Se mira justo antes de decodificar, que es lo que cuesta.
/// Aun cancelada hay que emitir `finished`: es la señal con la que Qt suelta la
/// respuesta.
bool descartarSiCancelada(Proveedor *prov, Respuesta *resp)
{
    if (!resp->cancelada())
        return false;
    if (prov) prov->contarCancelada();
    QMetaObject::invokeMethod(
        resp, [r = resp] { r->fallar(QStringLiteral("cancelada")); }, Qt::QueuedConnection);
    return true;
}

} // namespace

/// Decodifica una miniatura de la malla desde el pack mapeado en memoria.
class TareaMalla : public QRunnable {
public:
    TareaMalla(Proveedor *prov, Respuesta *resp, const GrimVista *vista, int indice,
               const QSize &pedido)
        : m_prov(prov), m_resp(resp), m_vista(vista), m_indice(indice), m_pedido(pedido)
    {
        setAutoDelete(true);
    }

    ~TareaMalla() override
    {
        // La referencia a la vista se toma al encolar y se suelta aquí, pase lo
        // que pase: mientras esta tarea exista, los bytes del pack no se van.
        if (m_vista) grim_vista_soltar(m_vista);
    }

    void run() override
    {
        if (descartarSiCancelada(m_prov, m_resp))
            return;
        uint32_t len = 0;
        const uint8_t *bytes = grim_vista_thumb(m_vista, size_t(m_indice), &len);
        if (!bytes || len == 0) {
            m_prov->contarFallo();
            // Sin miniatura no es un error de verdad: la celda se queda con su
            // color dominante.
            QMetaObject::invokeMethod(
                m_resp, [r = m_resp] { r->fallar(QStringLiteral("sin miniatura")); },
                Qt::QueuedConnection);
            return;
        }
        // `fromRawData` evita copiar: los bytes viven en el pack mapeado y no se
        // mueven mientras esta tarea tenga tomada la vista.
        QByteArray crudo =
            QByteArray::fromRawData(reinterpret_cast<const char *>(bytes), int(len));
        QBuffer buf(&crudo);
        buf.open(QIODevice::ReadOnly);
        QImageReader lector(&buf, "JPEG");
        // El tamaño que pide QML se respeta, que es el contrato de un proveedor
        // de imágenes. Devolver siempre la miniatura entera funcionaba de vista
        // —Qt la encoge al pintar— pero deja sin efecto `sourceSize`, y de ahí
        // salía que pedir una imagen diminuta para difuminarla no difuminara
        // nada. Solo se encoge: agrandar aquí sería gastar memoria de textura
        // para no enseñar un píxel más.
        const QImage img = leerAlTamano(lector, m_pedido);
        if (img.isNull()) {
            m_prov->contarFallo();
            QMetaObject::invokeMethod(
                m_resp, [r = m_resp] { r->fallar(QStringLiteral("JPEG ilegible")); },
                Qt::QueuedConnection);
            return;
        }
        m_prov->contarDecodificada();
        QMetaObject::invokeMethod(
            m_resp, [r = m_resp, img] { r->terminar(img); }, Qt::QueuedConnection);
    }

private:
    Proveedor *m_prov;
    Respuesta *m_resp;
    const GrimVista *m_vista;
    int m_indice;
    QSize m_pedido;
};

/// Carga la previsualización de 1024 px desde disco, para el visor.
class TareaVisor : public QRunnable {
public:
    TareaVisor(Proveedor *prov, Respuesta *resp, QString ruta, const QSize &pedido)
        : m_prov(prov), m_resp(resp), m_ruta(std::move(ruta)), m_pedido(pedido)
    {
        setAutoDelete(true);
    }

    void run() override
    {
        if (descartarSiCancelada(m_prov, m_resp))
            return;
        // Desde el archivo y al tamaño pedido, si se pide alguno: el visor no
        // lo pide hoy, pero quien lo haga no paga una previa entera de 1024 px.
        QImageReader lector(m_ruta);
        const QImage img = leerAlTamano(lector, m_pedido);
        if (img.isNull()) {
            m_prov->contarFallo();
            QMetaObject::invokeMethod(
                m_resp,
                [r = m_resp, ruta = m_ruta] {
                    r->fallar(QStringLiteral("no pude leer %1").arg(ruta));
                },
                Qt::QueuedConnection);
            return;
        }
        m_prov->contarDecodificada();
        QMetaObject::invokeMethod(
            m_resp, [r = m_resp, img] { r->terminar(img); }, Qt::QueuedConnection);
    }

private:
    Proveedor *m_prov;
    Respuesta *m_resp;
    QString m_ruta;
    QSize m_pedido;
};

/// Carga el original entero, para acercarse en el visor hasta el píxel.
class TareaOriginal : public QRunnable {
public:
    TareaOriginal(Respuesta *resp, QString ruta) : m_resp(resp), m_ruta(std::move(ruta))
    {
        setAutoDelete(true);
    }

    void run() override
    {
        if (descartarSiCancelada(nullptr, m_resp))
            return;
        const QImage img = leerOriginal(m_ruta);
        if (img.isNull()) {
            QMetaObject::invokeMethod(
                m_resp,
                [r = m_resp, ruta = m_ruta] {
                    r->fallar(QStringLiteral("no pude abrir %1").arg(ruta));
                },
                Qt::QueuedConnection);
            return;
        }
        QMetaObject::invokeMethod(
            m_resp, [r = m_resp, img] { r->terminar(img); }, Qt::QueuedConnection);
    }

private:
    Respuesta *m_resp;
    QString m_ruta;
};

Proveedor::Proveedor(Modelo *modelo) : m_modelo(modelo) { }

QString Proveedor::rutaPrevia(const GrimVista *v, int indice)
{
    if (!v) return {};
    // La ruta la da el núcleo. Reconstruirla aquí sería tener el reparto de
    // carpetas escrito en dos sitios, y un día dejarían de coincidir.
    uint8_t buf[4096];
    const uint32_t n = grim_vista_previa(v, size_t(indice), buf, sizeof(buf));
    if (n == 0) return {};
    return QString::fromUtf8(reinterpret_cast<const char *>(buf), int(n));
}

int Proveedor::indiceDe(const GrimVista *v, const QString &id)
{
    if (!v || id.isEmpty()) return -1;
    const QByteArray b = id.toUtf8();
    const size_t i = grim_vista_indice_de(v, reinterpret_cast<const uint8_t *>(b.constData()),
                                          uint32_t(b.size()));
    return i == SIZE_MAX ? -1 : int(i);
}

/// La URL lleva el **id del elemento**, no su posición en la malla.
///
/// Costó encontrarlo: el caché de imágenes de Qt tiene por clave la URL, así
/// que con la posición dentro, filtrar la biblioteca hacía que la celda 4 de la
/// vista nueva sirviera la miniatura de la celda 4 de la vista vieja. Se veía
/// como cajas con la foto de otro dentro, y solo al filtrar.
///
/// Con el id, además, el caché sobrevive a un cambio de filtro: volver a una
/// búsqueda anterior ya no vuelve a decodificar nada.
QQuickImageResponse *Proveedor::requestImageResponse(const QString &id, const QSize &pedido)
{
    auto *resp = new Respuesta;

    if (id.startsWith(QLatin1String("esquinas/"))) {
        // Síncrona: son unas decenas de bytes compartidos por toda la malla, y
        // hacerla esperar en la cola provocaría un parpadeo de esquinas
        // cuadradas en el primer fotograma. `esquinas/<radio>/<fondo sin #>`.
        const QStringList partes = id.split(QLatin1Char('/'));
        const qreal radio = partes.value(1).toDouble();
        const QColor fondo(QLatin1Char('#') + partes.value(2));
        resp->terminar(mascaraEsquinas(radio, fondo.isValid() ? fondo : QColor(Qt::black)));
        return resp;
    }

    // Una sola vista para todo lo que viene: buscar la posición y decodificar.
    //
    // Antes se buscaba la posición en la vista que hubiera publicada y luego
    // se tomaba otra referencia para decodificar. Este método corre en el
    // hilo de carga de imágenes; si entre las dos cosas el hilo de la
    // interfaz cambiaba de vista —cambiar rápido de carpeta—, la posición de
    // la vieja se leía en la nueva y salía la miniatura de otro elemento. Y
    // como el caché de Qt la guarda con la URL del id pedido, se quedaba
    // pegada: test2 enseñaba la foto de un elemento de TEST.
    const GrimVista *mia = m_modelo->tomarVista();

    if (id.startsWith(QLatin1String("original/"))) {
        const int i = indiceDe(mia, id.mid(9));
        QString ruta;
        if (i >= 0) {
            uint8_t buf[4096];
            const uint32_t n = grim_vista_ruta(mia, size_t(i), buf, sizeof(buf));
            ruta = QString::fromUtf8(reinterpret_cast<const char *>(buf), int(n));
        }
        if (mia) grim_vista_soltar(mia);
        if (ruta.isEmpty()) {
            resp->fallar(QStringLiteral("no está en la vista: %1").arg(id));
            return resp;
        }
        QThreadPool::globalInstance()->start(new TareaOriginal(resp, ruta));
        return resp;
    }

    if (id.startsWith(QLatin1String("previa/"))) {
        const int i = indiceDe(mia, id.mid(7));
        const QString ruta = i >= 0 ? rutaPrevia(mia, i) : QString();
        if (ruta.isEmpty() || !QFile::exists(ruta)) {
            // Una biblioteca importada por una versión antigua puede no tener
            // previsualizaciones. El visor no se queda en blanco por eso: se
            // sirve la miniatura, que se ve peor pero se ve. El aviso sale una
            // sola vez, no una por imagen.
            if (!m_avisadoSinPrevias.loadRelaxed()) {
                m_avisadoSinPrevias.storeRelaxed(1);
                qWarning("esta biblioteca no tiene previsualizaciones de 1024 px; "
                         "el visor usará las miniaturas hasta que se regeneren");
            }
            if (i < 0) {
                if (mia) grim_vista_soltar(mia);
                resp->fallar(QStringLiteral("no está en la vista: %1").arg(id));
                return resp;
            }
            QThreadPool::globalInstance()->start(new TareaMalla(this, resp, mia, i, pedido));
            return resp;
        }
        if (mia) grim_vista_soltar(mia);
        QThreadPool::globalInstance()->start(new TareaVisor(this, resp, ruta, pedido));
        return resp;
    }

    const int indice = indiceDe(mia, id);
    if (indice < 0) {
        // No estar en la vista no se recuerda: Qt no guarda en caché un fallo,
        // así que cuando la celda vuelva a pedirlo con la vista buena, saldrá.
        if (mia) grim_vista_soltar(mia);
        resp->fallar(QStringLiteral("no está en la vista: %1").arg(id));
        return resp;
    }
    // La tarea se queda con la referencia y la suelta al terminar: mientras
    // decodifica, los bytes del pack de esa vista no se van.
    QThreadPool::globalInstance()->start(new TareaMalla(this, resp, mia, indice, pedido));
    return resp;
}
