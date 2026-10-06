#include "pixeles.h"

#include "original.h"

#include <QPointer>
#include <QThreadPool>

void Pixeles::cargar(const QUrl &url)
{
    if (url == m_fuente) return;
    m_fuente = url;
    m_imagen = QImage();
    const quint64 vuelta = ++m_vuelta;
    emit cambio();
    if (!url.isLocalFile()) return;

    QPointer<Pixeles> yo(this);
    const QString ruta = url.toLocalFile();
    QThreadPool::globalInstance()->start([yo, ruta, vuelta] {
        // Igual que la imagen del visor —la misma lectura—: si una foto viene
        // girada en su EXIF, las coordenadas tienen que ser las de lo que se ve.
        QImage img = leerOriginal(ruta);
        if (!img.isNull()) img = img.convertToFormat(QImage::Format_ARGB32);
        QMetaObject::invokeMethod(
            yo.data(),
            [yo, img, vuelta] {
                if (!yo || vuelta != yo->m_vuelta) return;
                yo->m_imagen = img;
                emit yo->cambio();
            },
            Qt::QueuedConnection);
    });
}

void Pixeles::soltar()
{
    ++m_vuelta;
    m_fuente = QUrl();
    m_imagen = QImage();
    emit cambio();
}

QString Pixeles::hex(int x, int y) const
{
    if (m_imagen.isNull() || x < 0 || y < 0 || x >= m_imagen.width() || y >= m_imagen.height())
        return {};
    const QColor c = QColor::fromRgba(m_imagen.pixel(x, y));
    QString s = c.name(QColor::HexRgb);
    // Como se escribe en la web y en los editores: el alfa al final, no al
    // principio como lo da Qt.
    if (c.alpha() < 255) s += QStringLiteral("%1").arg(c.alpha(), 2, 16, QLatin1Char('0'));
    return s;
}
