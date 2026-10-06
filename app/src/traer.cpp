#include "traer.h"

#include "nombres.h"
#include "nucleo.h"
#include "portal.h"

#include <QClipboard>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QHash>
#include <QMimeData>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QRegularExpression>

namespace {

bool esWeb(const QUrl &u)
{
    return u.isValid() && (u.scheme() == QLatin1String("http") || u.scheme() == QLatin1String("https"));
}

/// La extensión que corresponde a un tipo, para lo que llega sin ella.
QString extensionDe(const QString &tipo)
{
    static const QHash<QString, QString> tabla = {
        { "image/jpeg", "jpg" },       { "image/png", "png" },       { "image/webp", "webp" },
        { "image/gif", "gif" },        { "image/avif", "avif" },     { "image/svg+xml", "svg" },
        { "image/bmp", "bmp" },        { "image/tiff", "tiff" },     { "video/mp4", "mp4" },
        { "video/webm", "webm" },      { "video/quicktime", "mov" }, { "video/x-matroska", "mkv" },
        { "audio/mpeg", "mp3" },       { "audio/ogg", "ogg" },       { "audio/wav", "wav" },
        { "audio/x-wav", "wav" },      { "audio/flac", "flac" },     { "application/pdf", "pdf" },
        { "model/stl", "stl" },        { "model/gltf-binary", "glb" },
    };
    return tabla.value(tipo);
}

/// Si lo que ha llegado se puede importar tal cual.
bool tipoImportable(const QString &tipo)
{
    return tipo.startsWith(QLatin1String("image/")) || tipo.startsWith(QLatin1String("video/"))
           || tipo.startsWith(QLatin1String("audio/")) || tipo.startsWith(QLatin1String("model/"))
           || tipo == QLatin1String("application/pdf");
}

QString desentidades(QString s)
{
    s.replace(QLatin1String("&amp;"), QLatin1String("&"));
    s.replace(QLatin1String("&quot;"), QLatin1String("\""));
    s.replace(QLatin1String("&#39;"), QLatin1String("'"));
    return s;
}

/// La imagen que la página dice que la representa: la de las tarjetas de
/// redes (og:image, twitter:image). Es la que eligió quien hizo la página, y
/// casi siempre es la buena: la foto de un artículo, la obra de un portafolio.
QUrl imagenPrincipal(const QString &html, const QUrl &base)
{
    static const QRegularExpression meta(QStringLiteral("<meta\\b[^>]*>"),
                                         QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression clave(
        QStringLiteral("(?:property|name)\\s*=\\s*[\"'](og:image(?::secure_url|:url)?|twitter:image(?::src)?)[\"']"),
        QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression contenido(QStringLiteral("content\\s*=\\s*[\"']([^\"']+)[\"']"),
                                              QRegularExpression::CaseInsensitiveOption);
    auto it = meta.globalMatch(html);
    while (it.hasNext()) {
        const QString etiqueta = it.next().captured(0);
        if (!clave.match(etiqueta).hasMatch()) continue;
        const auto c = contenido.match(etiqueta);
        if (!c.hasMatch()) continue;
        const QUrl u = base.resolved(QUrl(desentidades(c.captured(1))));
        if (esWeb(u)) return u;
    }
    return {};
}

/// La imagen que un navegador deja en el HTML al copiar una imagen: de ahí
/// sale de dónde venía.
QUrl srcDeHtml(const QString &html)
{
    static const QRegularExpression img(QStringLiteral("<img\\b[^>]*\\bsrc\\s*=\\s*[\"']([^\"']+)[\"']"),
                                        QRegularExpression::CaseInsensitiveOption);
    const auto m = img.match(html);
    if (!m.hasMatch()) return {};
    const QUrl u(desentidades(m.captured(1)));
    return esWeb(u) ? u : QUrl();
}

QString limpiarNombre(const QString &n)
{
    // Se recorta antes de limpiar, para que el corte no deje un punto o un
    // espacio al final, que Windows no admite.
    return nombreDeArchivo(n.left(150), QStringLiteral("descarga"));
}

} // namespace

Traer::Traer(Nucleo *nucleo, QObject *padre)
    : QObject(padre), m_nucleo(nucleo), m_red(new QNetworkAccessManager(this))
{
}

QString Traer::dirTemporal() const
{
    // Dentro de la biblioteca y no en /tmp: así importar es mover —renombrar—
    // en vez de copiar, porque está en el mismo disco.
    const QString d = m_nucleo->raiz() + QStringLiteral("/cache/entrada/")
                      + QString::number(QDateTime::currentMSecsSinceEpoch());
    QDir().mkpath(d);
    return d;
}

QString Traer::idPropio(const QString &ruta) const
{
    const QString raiz = QDir(m_nucleo->raiz()).absolutePath();
    if (raiz.isEmpty()) return {};
    const QString r = QFileInfo(ruta).absoluteFilePath();
    // En Windows `C:\Biblioteca` y `c:\biblioteca` son la misma carpeta, y el
    // portapapeles o el Explorador pueden traerla escrita de cualquiera de
    // las dos formas.
#ifdef Q_OS_WIN
    const Qt::CaseSensitivity mayusculas = Qt::CaseInsensitive;
#else
    const Qt::CaseSensitivity mayusculas = Qt::CaseSensitive;
#endif
    const QString salida = raiz + QStringLiteral("/cache/salida/");
    if (r.startsWith(salida, mayusculas)) return r.mid(salida.size()).section(QLatin1Char('/'), 0, 0);
    if (r.startsWith(raiz + QStringLiteral("/items/"), mayusculas))
        return QFileInfo(r).dir().dirName();
    return {};
}

void Traer::pegar(const QString &carpeta)
{
    const QMimeData *m = QGuiApplication::clipboard()->mimeData();
    if (!m || m->formats().isEmpty()) {
        emit aviso(tr("no hay nada que pegar"), false);
        return;
    }
    if (m->hasUrls() && !m->urls().isEmpty()) {
        repartir(m->urls(), carpeta);
        return;
    }
    if (m->hasImage()) {
        const QImage img = qvariant_cast<QImage>(m->imageData());
        // Si el navegador dice de dónde era, se baja el archivo de verdad
        // —el JPEG o el WebP, no un PNG rehecho del mapa de bits— y la
        // imagen del portapapeles queda de reserva por si la descarga falla.
        const QUrl de = m->hasHtml() ? srcDeHtml(m->html()) : QUrl();
        if (esWeb(de)) bajar(de, carpeta, de.toString(), 1, img);
        else guardarImagen(img, carpeta, QString());
        return;
    }
    if (m->hasText()) {
        const QString t = m->text().trimmed();
        const QUrl u(t);
        if (!t.contains(QLatin1Char('\n')) && esWeb(u)) {
            descargar(u, carpeta);
            return;
        }
        if (QFileInfo::exists(t)) {
            repartir({ QUrl::fromLocalFile(t) }, carpeta);
            return;
        }
        emit aviso(tr("lo que hay en el portapapeles es texto, no un archivo ni una dirección web"),
                   true);
        return;
    }
    emit aviso(tr("no sé pegar eso"), true);
}

void Traer::soltar(const QList<QUrl> &urls, const QString &carpeta)
{
    repartir(urls, carpeta);
}

void Traer::repartir(const QList<QUrl> &urls, const QString &carpeta)
{
    QStringList locales;
    QStringList ids;
    QList<QUrl> web;
    for (const QUrl &u : urls) {
        if (u.isLocalFile()) {
            const QString id = idPropio(u.toLocalFile());
            if (!id.isEmpty()) ids << id;
            else locales << u.toLocalFile();
        } else if (esWeb(u)) {
            web << u;
        }
    }
    if (!ids.isEmpty()) {
        if (!carpeta.isEmpty()) {
            m_nucleo->moverEntreCarpetas(ids, QString(), carpeta);
        } else {
            emit aviso(ids.size() == 1 ? tr("eso ya está en la biblioteca")
                                       : tr("esos %1 ya están en la biblioteca").arg(ids.size()),
                       false);
        }
    }
    if (!locales.isEmpty()) m_nucleo->importar(locales, carpeta);
    for (const QUrl &u : web) descargar(u, carpeta);
    if (ids.isEmpty() && locales.isEmpty() && web.isEmpty())
        emit aviso(tr("eso no son archivos ni direcciones web"), true);
}

void Traer::descargar(const QUrl &url, const QString &carpeta)
{
    if (!esWeb(url)) {
        emit aviso(tr("no es una dirección web: %1").arg(url.toString()), true);
        return;
    }
    bajar(url, carpeta, QString(), 0, QImage());
}

void Traer::guardarImagen(const QImage &img, const QString &carpeta, const QString &origen)
{
    if (img.isNull()) {
        emit aviso(tr("la imagen del portapapeles está vacía"), true);
        return;
    }
    const QString nombre = QStringLiteral("pegado %1.png")
                               .arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyy-MM-dd HH.mm.ss")));
    const QString ruta = dirTemporal() + QLatin1Char('/') + nombre;
    if (!img.save(ruta, "PNG")) {
        emit aviso(tr("no pude guardar la imagen pegada"), true);
        return;
    }
    m_nucleo->importarDeFuera({ ruta }, carpeta, origen, true);
}

void Traer::bajar(const QUrl &url, const QString &carpeta, const QString &origen, int saltos,
                  const QImage &reserva)
{
    QNetworkRequest pide(url);
    // Hay sitios que a un cliente sin nombre le devuelven una página de error
    // en vez de la imagen.
    pide.setHeader(QNetworkRequest::UserAgentHeader,
                   QStringLiteral("Mozilla/5.0 (X11; Linux x86_64) Grimorio"));
    pide.setTransferTimeout(30000);
    QNetworkReply *r = m_red->get(pide);
    emit aviso(tr("descargando %1…").arg(url.host()), false);

    connect(r, &QNetworkReply::finished, this, [=] {
        r->deleteLater();
        if (r->error() != QNetworkReply::NoError) {
            if (!reserva.isNull()) {
                guardarImagen(reserva, carpeta, origen);
                return;
            }
            emit aviso(tr("no pude descargar %1: %2").arg(url.toString(), r->errorString()), true);
            return;
        }
        const QString tipo = r->header(QNetworkRequest::ContentTypeHeader)
                                 .toString().section(QLatin1Char(';'), 0, 0).trimmed().toLower();
        const QByteArray datos = r->readAll();
        const QUrl final_ = r->url();

        // Una página: se va a por su imagen principal, una vez. El origen es
        // la página, que es lo que se querrá volver a abrir.
        if (tipo == QLatin1String("text/html") || tipo == QLatin1String("application/xhtml+xml")) {
            const QUrl img = saltos == 0 ? imagenPrincipal(QString::fromUtf8(datos), final_) : QUrl();
            if (img.isValid()) {
                bajar(img, carpeta, origen.isEmpty() ? url.toString() : origen, saltos + 1, reserva);
                return;
            }
            if (!reserva.isNull()) {
                guardarImagen(reserva, carpeta, origen);
                return;
            }
            emit aviso(tr("esa página no dice cuál es su imagen; abre la imagen y copia su dirección"),
                       true);
            return;
        }

        QString nombre = limpiarNombre(QUrl::fromPercentEncoding(final_.path().section(QLatin1Char('/'), -1).toUtf8()));
        const QString ext = extensionDe(tipo);
        if (!ext.isEmpty() && QFileInfo(nombre).suffix().toLower() != ext
                && !(ext == QLatin1String("jpg") && QFileInfo(nombre).suffix().toLower() == QLatin1String("jpeg")))
            nombre += QLatin1Char('.') + ext;
        if (!tipoImportable(tipo) && QFileInfo(nombre).suffix().isEmpty()) {
            if (!reserva.isNull()) {
                guardarImagen(reserva, carpeta, origen);
                return;
            }
            emit aviso(tr("eso no es una imagen ni un vídeo (%1)").arg(tipo.isEmpty() ? tr("sin tipo") : tipo),
                       true);
            return;
        }

        const QString ruta = dirTemporal() + QLatin1Char('/') + nombre;
        QFile f(ruta);
        if (!f.open(QIODevice::WriteOnly) || f.write(datos) != datos.size()) {
            emit aviso(tr("no pude guardar la descarga"), true);
            return;
        }
        f.close();
        m_nucleo->importarDeFuera({ ruta }, carpeta, origen.isEmpty() ? url.toString() : origen, true);
    });
}

void Traer::elegirEImportar(const QString &carpeta)
{
    if (!elegirArchivos(this, tr("Importar a Grimorio"),
                        [this, carpeta](const QStringList &rutas) { m_nucleo->importar(rutas, carpeta); }))
        emit aviso(tr("no hay diálogo de archivos: ni el portal del escritorio ni zenity"), true);
}
