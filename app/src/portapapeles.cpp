#include "portapapeles.h"

#include "original.h"

#include <QClipboard>
#include <QDrag>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImageReader>
#include <QMimeData>
#include <QPixmap>
#include <QQuickItem>

namespace {

/// Lo mismo que pone en el portapapeles un gestor de archivos.
///
/// `text/uri-list` es lo que entiende todo el mundo para soltar o pegar
/// archivos. `x-special/gnome-copied-files` es lo que mira Nautilus para pegar:
/// sin él, Ctrl+V en una carpeta no hace nada. Y el texto, para que pegar en
/// un editor o una terminal dé las rutas y no nada.
QMimeData *paquete(const QList<QUrl> &urls)
{
    auto *m = new QMimeData;
    m->setUrls(urls);
    QByteArray gnome = "copy";
    QStringList rutas;
    for (const QUrl &u : urls) {
        gnome += '\n' + u.toEncoded();
        rutas << (u.isLocalFile() ? u.toLocalFile() : u.toString());
    }
    m->setData(QStringLiteral("x-special/gnome-copied-files"), gnome);
    m->setText(rutas.join(QLatin1Char('\n')));
    return m;
}

/// Si el archivo es una imagen que vale la pena poner entera en el
/// portapapeles. Un tope, porque decodificar una foto enorme aquí para un
/// Ctrl+C que quizá solo quería copiar el archivo es esperar sin motivo.
bool merecePonerImagen(const QString &ruta)
{
    const QFileInfo f(ruta);
    if (!f.isFile() || f.size() > 40 * 1024 * 1024) return false;
    static const QStringList imagenes = { "jpg", "jpeg", "png", "webp", "gif", "bmp" };
    return imagenes.contains(f.suffix().toLower());
}

} // namespace

void Portapapeles::copiarTexto(const QString &texto)
{
    QGuiApplication::clipboard()->setText(texto);
}

int Portapapeles::copiarArchivos(const QList<QUrl> &urls)
{
    if (urls.isEmpty()) return 0;
    QMimeData *m = paquete(urls);
    if (urls.size() == 1 && urls.first().isLocalFile()
            && merecePonerImagen(urls.first().toLocalFile())) {
        const QImage img = leerOriginal(urls.first().toLocalFile());
        if (!img.isNull()) m->setImageData(img);
    }
    QGuiApplication::clipboard()->setMimeData(m);
    return int(urls.size());
}

bool Portapapeles::arrastrarFuera(QQuickItem *agarre, const QList<QUrl> &urls,
                                  const QString &previa)
{
    if (!agarre || urls.isEmpty()) return false;
    agarre->ungrabMouse();

    auto *drag = new QDrag(agarre);
    drag->setMimeData(paquete(urls));
    // Lo que va pegado al cursor: la previa pequeña, que es lo que se está
    // arrastrando. Sin nada, el gestor de archivos pone un icono genérico y
    // no se sabe qué se lleva.
    if (!previa.isEmpty()) {
        QImageReader lector(previa);
        const QSize s = lector.size();
        if (s.isValid()) lector.setScaledSize(s.scaled(160, 160, Qt::KeepAspectRatio));
        const QImage img = lector.read();
        if (!img.isNull()) {
            drag->setPixmap(QPixmap::fromImage(img));
            drag->setHotSpot(QPoint(img.width() / 2, img.height() / 2));
        }
    }
    drag->exec(Qt::CopyAction, Qt::CopyAction);
    return true;
}
