#include "novedades.h"

#include "ajustes.h"
#include "paquete.h"
#include "version.h"

#include <QCoreApplication>
#include <QCryptographicHash>
#include <QDesktopServices>
#include <QDir>
#include <QFile>
#include <QRegularExpression>
#include <QVariantMap>
#include <QFileInfo>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QProcess>
#include <QStandardPaths>
#include <QTimer>

#include <memory>

namespace {

const char *DIRECCION = "https://grimorio.frederickandrade.com/version.json";

/// Cada cuánto se pregunta sola. Veinte horas y no veinticuatro: quien abre
/// Grimorio cada mañana a una hora parecida pregunta cada día, y no cada dos.
constexpr qint64 CADA_SEGUNDOS = 20 * 3600;

/// Lo que se espera después de arrancar antes de preguntar. Al abrir, la red y
/// el disco están ocupados llenando la primera pantalla de miniaturas, y esto
/// puede esperar.
constexpr int ESPERA_MS = 8000;

/// Un version.json que pase de esto no es el nuestro.
constexpr qint64 TOPE_BYTES = 16 * 1024;

/// Ni el instalador de Windows (unos 55 MB) ni el paquete de Linux se acercan
/// a esto; una descarga que pasa de aquí no es la nuestra.
constexpr qint64 TOPE_PAQUETE = 400ll * 1024 * 1024;

/// Para probar el aviso y la actualización contra un version.json propio.
QString direccion()
{
    const QString otra = qEnvironmentVariable("GRIMORIO_NOVEDADES_URL");
    return otra.isEmpty() ? QString::fromLatin1(DIRECCION) : otra;
}

QNetworkRequest peticion(const QUrl &url, int esperaMs)
{
    QNetworkRequest req{url};
    // Sin versión en el agente: no hace falta para contestar, y así la
    // pregunta es la misma para todos.
    req.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("Grimorio"));
    req.setAttribute(QNetworkRequest::CookieSaveControlAttribute, QNetworkRequest::Manual);
    req.setAttribute(QNetworkRequest::CookieLoadControlAttribute, QNetworkRequest::Manual);
    req.setAttribute(QNetworkRequest::CacheLoadControlAttribute, QNetworkRequest::AlwaysNetwork);
    // GitHub manda los archivos de las releases a su almacén con una
    // redirección; de https a http no se sigue.
    req.setAttribute(QNetworkRequest::RedirectPolicyAttribute, QNetworkRequest::NoLessSafeRedirectPolicy);
    req.setTransferTimeout(esperaMs);
    return req;
}

#if defined(Q_OS_LINUX)
/// Descomprime el paquete de Linux al lado del archivo y devuelve la carpeta
/// del programa de dentro, o vacía con el motivo en `error`.
QString descomprimir(const QString &archivo, QString *error)
{
    const QString suelto = QFileInfo(archivo).absolutePath() + QStringLiteral("/paquete");
    QDir(suelto).removeRecursively();
    QDir().mkpath(suelto);
    QProcess tar;
    tar.start(QStringLiteral("tar"), { QStringLiteral("-xzf"), archivo, QStringLiteral("-C"), suelto });
    if (!tar.waitForFinished(300000) || tar.exitStatus() != QProcess::NormalExit || tar.exitCode() != 0) {
        *error = QObject::tr("no he podido abrir el paquete");
        return {};
    }
    return paquete::raizDe(suelto);
}
#endif

} // namespace

Novedades::Novedades(Ajustes *ajustes, const QString &actual, bool activa, QObject *padre)
    : QObject(padre), m_ajustes(ajustes), m_actual(actual), m_activa(activa)
{
    m_ultima = m_disco.value(QStringLiteral("novedades/ultima")).toString();
    m_ignorada = m_disco.value(QStringLiteral("novedades/ignorada")).toString();
    m_enlace = QUrl(m_disco.value(QStringLiteral("novedades/enlace"),
                                  QString::fromLatin1(version::PAGINA)).toString());
    m_ultimaVez = m_disco.value(QStringLiteral("novedades/cuando")).toDateTime();
    {
        // Pasa otra vez por el mismo filtro que lo leído de la web.
        const version::Paquete q = version::leerPaquete(
            QJsonObject{ { QStringLiteral("url"), m_disco.value(QStringLiteral("novedades/paquete")).toString() },
                         { QStringLiteral("sha256"), m_disco.value(QStringLiteral("novedades/huella")).toString() } });
        m_paquete = q.url;
        m_huella = q.sha256;
    }
    m_buscaba = m_ajustes->buscarVersiones();

    // Encender el aviso en Ajustes pregunta en el acto si toca; apagarlo
    // quita el que hubiera en la barra.
    connect(m_ajustes, &Ajustes::cambio, this, [this] {
        const bool ahora = m_ajustes->buscarVersiones();
        if (ahora == m_buscaba) return;
        m_buscaba = ahora;
        emit cambio();
        if (ahora) alArrancar();
    });

    if (m_activa) QTimer::singleShot(ESPERA_MS, this, &Novedades::alArrancar);
}

QString Novedades::nueva() const
{
    if (!m_activa || !m_ajustes->buscarVersiones()) return {};
    if (version::comparar(m_ultima, m_actual) <= 0) return {};
    if (!m_ignorada.isEmpty() && version::comparar(m_ultima, m_ignorada) <= 0) return {};
    return m_ultima;
}

void Novedades::alArrancar()
{
    // Antes que nada, y aunque no se quiera el aviso de versiones: completar
    // lo que falta es terminar una actualización que ya se pidió.
    if (m_activa) completarSiHaceFalta();
    if (!m_activa || !m_ajustes->buscarVersiones()) return;
    const QDateTime ahora = QDateTime::currentDateTimeUtc();
    // Un reloj que se fue al futuro y volvió no puede dejar de preguntar para
    // siempre: una fecha guardada por delante de ahora cuenta como vieja.
    if (m_ultimaVez.isValid() && m_ultimaVez <= ahora
        && m_ultimaVez.secsTo(ahora) < CADA_SEGUNDOS)
        return;
    preguntar(false);
}

void Novedades::comprobar()
{
    preguntar(true);
}

void Novedades::preguntar(bool aMano)
{
    if (m_preguntando) return;
    if (!m_red) m_red = new QNetworkAccessManager(this);
    m_preguntando = true;
    emit cambio();

    QNetworkReply *r = m_red->get(peticion(QUrl(direccion()), 10000));
    connect(r, &QNetworkReply::finished, this, [this, r, aMano] {
        r->deleteLater();
        m_preguntando = false;

        const bool bien = r->error() == QNetworkReply::NoError
                          && r->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt() == 200;
        const version::Publicada p = bien ? version::leer(r->read(TOPE_BYTES)) : version::Publicada{};
        if (p.version.isEmpty()) {
            emit cambio();
            if (aMano)
                emit aviso(tr("no he podido saber cuál es la última versión: sin conexión o la web no responde"),
                           true);
            return;
        }

        m_ultima = p.version;
        m_enlace = p.enlace;
        m_paquete = p.paquete.url;
        m_huella = p.paquete.sha256;
        m_ultimaVez = QDateTime::currentDateTimeUtc();
        m_disco.setValue(QStringLiteral("novedades/ultima"), m_ultima);
        m_disco.setValue(QStringLiteral("novedades/paquete"), m_paquete.toString());
        m_disco.setValue(QStringLiteral("novedades/huella"), m_huella);
        m_disco.setValue(QStringLiteral("novedades/enlace"), m_enlace.toString());
        m_disco.setValue(QStringLiteral("novedades/cuando"), m_ultimaVez);
        // Preguntar a mano es pedir el aviso otra vez, aunque se hubiera
        // ignorado esta versión.
        if (aMano) {
            m_ignorada.clear();
            m_disco.remove(QStringLiteral("novedades/ignorada"));
        }
        m_disco.sync();
        emit cambio();
        completarSiHaceFalta();

        if (aMano) {
            if (version::comparar(m_ultima, m_actual) > 0)
                emit aviso(tr("hay una versión nueva: %1").arg(m_ultima), false);
            else
                emit aviso(tr("tienes la última versión (%1)").arg(m_actual), false);
        }
    });
}

void Novedades::completarSiHaceFalta()
{
#if defined(GRIMORIO_PAQUETE) && defined(Q_OS_LINUX)
    if (m_completando || !m_fase.isEmpty()) return;
    const QDir dir(QCoreApplication::applicationDirPath());
    if (paquete::faltan(dir).isEmpty()) return;
    // Solo con el paquete de esta misma versión: el de otra traería otro
    // programa. Quien viene de actualizar lo tiene guardado, con su huella.
    if (version::comparar(m_ultima, m_actual) != 0 || m_paquete.isEmpty() || m_huella.isEmpty()) return;
    if (!QFileInfo(dir.path()).isWritable()) return;

    m_completando = true;
    emit aviso(tr("terminando de instalar la versión %1: falta quitar fondos y la extensión del navegador")
                   .arg(m_actual), false);
    bajar(QStringLiteral("completar"), false,
          [this, dir](const QString &archivo) {
              QString error;
              const QString raiz = descomprimir(archivo, &error);
              QStringList puestos;
              const bool bien = !raiz.isEmpty() && paquete::instalar(raiz, dir, true, &error, &puestos);
              QDir(QFileInfo(archivo).absolutePath()).removeRecursively();
              m_completando = false;
              if (!bien) {
                  emit aviso(tr("no he podido terminar de instalar: %1").arg(error), true);
                  return;
              }
              emit completado();
              emit aviso(tr("instalación completa: ya se pueden quitar fondos"), false);
          },
          [this](const QString &mensaje) {
              m_completando = false;
              emit aviso(tr("no he podido terminar de instalar: %1").arg(mensaje), true);
          });
#endif
}

void Novedades::ignorar()
{
    if (m_ultima.isEmpty()) return;
    m_ignorada = m_ultima;
    m_disco.setValue(QStringLiteral("novedades/ignorada"), m_ignorada);
    m_disco.sync();
    emit cambio();
}

void Novedades::abrir()
{
    QDesktopServices::openUrl(m_enlace);
}

bool Novedades::instalable() const
{
#ifndef GRIMORIO_PAQUETE
    // Un build de desarrollo no se pisa con lo publicado.
    return false;
#else
    if (m_paquete.isEmpty() || m_huella.isEmpty()) return false;
    if (version::comparar(m_ultima, m_actual) <= 0) return false;
    const QString dir = QCoreApplication::applicationDirPath();
#if defined(Q_OS_WIN)
    // Instalado con el instalador, que deja su desinstalador al lado. El zip
    // portable no: ahí el instalador pondría otra copia en otro sitio.
    return QFileInfo::exists(QDir(dir).filePath(QStringLiteral("unins000.exe")));
#elif defined(Q_OS_LINUX)
    // Los binarios se cambian en su sitio: la carpeta tiene que ser de quien
    // lo usa (~/.local/lib/grimorio, por ejemplo, y no /opt).
    return QFileInfo(dir).isWritable() && QFileInfo(QCoreApplication::applicationFilePath()).isWritable();
#else
    return false;
#endif
#endif
}

void Novedades::actualizar()
{
    if (!m_fase.isEmpty()) return;
    if (!instalable()) {
        abrir();
        return;
    }
    m_fase = QStringLiteral("bajando");
    m_progreso = -1;
    m_error.clear();
    emit cambio();
    emit progresoCambio();
    bajar(QStringLiteral("actualizacion"), true,
          [this](const QString &archivo) { bajado(archivo); },
          [this](const QString &mensaje) { fallo(mensaje); });
}

void Novedades::bajar(const QString &sub, bool conProgreso, std::function<void(const QString &)> listo,
                      std::function<void(const QString &)> mal)
{
    if (!m_red) m_red = new QNetworkAccessManager(this);

    QDir cache(QStandardPaths::writableLocation(QStandardPaths::CacheLocation));
    QDir(cache.filePath(sub)).removeRecursively();
    if (!cache.mkpath(sub)) {
        mal(tr("no puedo escribir en %1").arg(cache.filePath(sub)));
        return;
    }
    const QString destino = QDir(cache.filePath(sub)).filePath(QFileInfo(m_paquete.path()).fileName());
    auto archivo = std::make_shared<QFile>(destino);
    if (!archivo->open(QIODevice::WriteOnly)) {
        mal(tr("no puedo escribir %1").arg(destino));
        return;
    }
    auto huella = std::make_shared<QCryptographicHash>(QCryptographicHash::Sha256);
    auto bytes = std::make_shared<qint64>(0);
    const QString esperada = m_huella;
    const QString cual = m_ultima;

    QNetworkReply *r = m_red->get(peticion(m_paquete, 30000));
    connect(r, &QNetworkReply::readyRead, this, [r, archivo, huella, bytes] {
        const QByteArray trozo = r->readAll();
        *bytes += trozo.size();
        if (*bytes > TOPE_PAQUETE) {
            r->abort();
            return;
        }
        huella->addData(trozo);
        archivo->write(trozo);
    });
    if (conProgreso) {
        connect(r, &QNetworkReply::downloadProgress, this, [this](qint64 llevo, qint64 total) {
            m_progreso = total > 0 ? qreal(llevo) / qreal(total) : -1;
            emit progresoCambio();
        });
    }
    connect(r, &QNetworkReply::finished, this,
            [this, r, archivo, huella, bytes, destino, esperada, cual, listo, mal] {
        r->deleteLater();
        // Lo que quede por leer, que `readyRead` no siempre llega a avisar.
        const QByteArray resto = r->readAll();
        *bytes += resto.size();
        if (*bytes <= TOPE_PAQUETE) {
            huella->addData(resto);
            archivo->write(resto);
        }
        archivo->close();
        if (r->error() != QNetworkReply::NoError || *bytes > TOPE_PAQUETE) {
            QFile::remove(destino);
            mal(tr("no he podido bajar la versión %1: %2").arg(cual, r->errorString()));
            return;
        }
        // Lo que se va a ejecutar tiene que ser lo publicado, bit a bit.
        if (QString::fromLatin1(huella->result().toHex()) != esperada) {
            QFile::remove(destino);
            mal(tr("lo bajado no coincide con lo publicado; no se instala"));
            return;
        }
        listo(destino);
    });
}

void Novedades::bajado(const QString &archivo)
{
    m_fase = QStringLiteral("instalando");
    m_progreso = 1;
    emit cambio();
    emit progresoCambio();
    // En la vuelta siguiente, para que la ventana llegue a decir «instalando».
    QTimer::singleShot(50, this, [this, archivo] { instalar(archivo); });
}

void Novedades::fallo(const QString &mensaje)
{
    m_fase.clear();
    m_progreso = 0;
    m_error = mensaje;
    emit cambio();
    emit progresoCambio();
    emit aviso(mensaje, true);
}

void Novedades::instalar(const QString &archivo)
{
#if defined(Q_OS_WIN)
    // El instalador, sin preguntas, en el mismo modo en que se instaló (para
    // todos o solo para quien lo usa): si no, quedarían dos copias. Se cierra
    // este programa para que pueda cambiar sus archivos, y el propio
    // instalador lo vuelve a abrir al acabar (`/actualizar=1`, ver el .iss).
    const QString dir = QDir::cleanPath(QCoreApplication::applicationDirPath()).toLower();
    bool deTodos = false;
    for (const char *v : { "ProgramFiles", "ProgramW6432" }) {
        const QString pf = QDir::cleanPath(QDir::fromNativeSeparators(qEnvironmentVariable(v))).toLower();
        if (!pf.isEmpty() && dir.startsWith(pf + QLatin1Char('/'))) deTodos = true;
    }
    const QStringList args{ QStringLiteral("/SILENT"), QStringLiteral("/SUPPRESSMSGBOXES"),
                            QStringLiteral("/NORESTART"), QStringLiteral("/CLOSEAPPLICATIONS"),
                            deTodos ? QStringLiteral("/ALLUSERS") : QStringLiteral("/CURRENTUSER"),
                            QStringLiteral("/actualizar=1") };
    if (!QProcess::startDetached(archivo, args)) {
        fallo(tr("no he podido abrir el instalador"));
        return;
    }
    QTimer::singleShot(0, qApp, &QCoreApplication::quit);
#elif defined(Q_OS_LINUX)
    // La ruta se coge antes de mover nada: en Linux sale de /proc/self/exe, y
    // después de renombrar apuntaría a «grimorio.anterior».
    const QString exe = QCoreApplication::applicationFilePath();
    const QDir dir(QCoreApplication::applicationDirPath());

    QString error;
    const QString raiz = descomprimir(archivo, &error);
    if (raiz.isEmpty()) {
        fallo(error.isEmpty() ? tr("el paquete de la versión %1 no trae el programa").arg(m_ultima) : error);
        return;
    }
    // El paquete entero y no solo los dos programas: la 0.1.5 cambiaba
    // `grimorio` y `grim` y nada más, y lo que la 0.1.6 trajo nuevo (quitar
    // fondos, la extensión) se quedaba fuera.
    if (!paquete::instalar(raiz, dir, false, &error)) {
        fallo(error);
        return;
    }
    QDir(QFileInfo(archivo).absolutePath()).removeRecursively();

    // El nuevo no puede arrancar hasta que este suelte el cerrojo de la
    // biblioteca, o diría «ya está abierta». Lo espera un `sh`, que no
    // depende de lo que sepa hacer la versión nueva.
    QStringList args{ QStringLiteral("-c"),
                      QStringLiteral("while kill -0 \"$1\" 2>/dev/null; do sleep 0.1; done; shift; exec \"$@\""),
                      QStringLiteral("sh"), QString::number(QCoreApplication::applicationPid()), exe };
    args += m_relanzar;
    if (!QProcess::startDetached(QStringLiteral("/bin/sh"), args)) {
        fallo(tr("la versión %1 está instalada; ábrela otra vez a mano").arg(m_ultima));
        return;
    }
    QTimer::singleShot(0, qApp, &QCoreApplication::quit);
#else
    Q_UNUSED(archivo);
    abrir();
#endif
}

void Novedades::prepararCambios(bool yaSeUsaba, bool mostrar)
{
    m_mostrarCambios = mostrar;
    m_vistos = m_disco.value(QStringLiteral("novedades/vistos")).toString();
    if (m_vistos.isEmpty() && !yaSeUsaba) {
        // La primera vez: nada es nuevo todavía.
        m_vistos = m_actual;
        m_disco.setValue(QStringLiteral("novedades/vistos"), m_vistos);
    } else if (m_vistos.isEmpty()) {
        // Viene de una versión de antes de que esto existiera: no se sabe
        // cuál, así que se enseña solo lo de esta.
        m_vistos = QStringLiteral("0.0.0");
    }
    emit cambio();
}

bool Novedades::cambiosPendientes() const
{
    return m_mostrarCambios && !m_vistos.isEmpty() && version::comparar(m_actual, m_vistos) > 0
           && !cambios(false).isEmpty();
}

QVariantList Novedades::cambios(bool todos) const
{
    QFile f(QStringLiteral(":/NOVEDADES.md"));
    if (!f.open(QIODevice::ReadOnly)) return {};
    const QString md = QString::fromUtf8(f.readAll());

    // Cada versión es un `## X.Y.Z`; lo de encima del primero es la
    // explicación del archivo y no se enseña.
    static const QRegularExpression cabecera(QStringLiteral("^## +v?(\\d+\\.\\d+\\.\\d+)[^\\n]*$"),
                                             QRegularExpression::MultilineOption);
    QVariantList fuera;
    auto it = cabecera.globalMatch(md);
    QList<QRegularExpressionMatch> trozos;
    while (it.hasNext()) trozos << it.next();
    // Si se viene de antes de que esto existiera, solo lo de esta versión.
    const bool soloEsta = !todos && m_vistos == QLatin1String("0.0.0");
    for (int i = 0; i < trozos.size(); ++i) {
        const QString v = trozos[i].captured(1);
        if (version::comparar(v, m_actual) > 0) continue; // escrita para una que aún no ha salido
        if (!todos) {
            if (soloEsta ? version::comparar(v, m_actual) != 0 : version::comparar(v, m_vistos) <= 0)
                continue;
        }
        const int ini = trozos[i].capturedEnd();
        const int fin = i + 1 < trozos.size() ? trozos[i + 1].capturedStart() : md.size();
        fuera << QVariantMap{ { QStringLiteral("version"), v },
                              { QStringLiteral("texto"), md.mid(ini, fin - ini).trimmed() } };
    }
    return fuera;
}

void Novedades::cambiosVistos()
{
    if (version::comparar(m_actual, m_vistos) <= 0) return;
    m_vistos = m_actual;
    m_disco.setValue(QStringLiteral("novedades/vistos"), m_vistos);
    m_disco.sync();
    emit cambio();
}
