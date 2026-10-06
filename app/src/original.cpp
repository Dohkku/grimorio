#include "original.h"

#include "grimorio.h"

#include <QImageReader>

namespace {

void soltarPixeles(void *info)
{
    auto *p = static_cast<std::pair<uint8_t *, size_t> *>(info);
    grim_pixeles_soltar(p->first, p->second);
    delete p;
}

} // namespace

QImage leerOriginal(const QString &ruta)
{
    QImageReader lector(ruta);
    lector.setAutoTransform(true);
    // Sin tope: el de Qt (256 MB) deja fuera fotos grandes de verdad, y son
    // justo las que más se quieren acercar.
    lector.setAllocationLimit(0);
    QImage img = lector.read();
    if (!img.isNull()) return img;

    const QByteArray b = ruta.toUtf8();
    uint32_t ancho = 0, alto = 0;
    uint8_t *px = grim_decodificar(reinterpret_cast<const uint8_t *>(b.constData()),
                                   uint32_t(b.size()), &ancho, &alto);
    if (!px || ancho == 0 || alto == 0) return {};
    const size_t bytes = size_t(ancho) * alto * 4;
    // Los píxeles siguen siendo del núcleo: la imagen los suelta al morir.
    return QImage(px, int(ancho), int(alto), int(ancho) * 4, QImage::Format_RGBA8888,
                  soltarPixeles, new std::pair<uint8_t *, size_t>(px, bytes));
}
