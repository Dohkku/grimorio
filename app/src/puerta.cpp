#include "puerta.h"
#include "nucleo.h"
#include "traer.h"

#include <QCoreApplication>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QProcess>
#include <QRegularExpression>
#include <QTimer>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocalServer>
#include <QLocalSocket>
#include <QUrl>

#ifndef Q_OS_WIN
#include <unistd.h>
#endif

namespace {

/// Lo más que se espera de una petición. Una foto en base64 son unos pocos
/// megas; un vídeo largo no pasa por aquí, se baja por su dirección.
constexpr qint64 TOPE_BYTES = 96ll * 1024 * 1024;

QByteArray error(const QString &mensaje)
{
    return QJsonDocument(QJsonObject { { QStringLiteral("ok"), false },
                                       { QStringLiteral("error"), mensaje } })
        .toJson(QJsonDocument::Compact);
}

} // namespace

QString Puerta::nombre()
{
#ifdef Q_OS_WIN
    return QStringLiteral("grimorio-puerta-") + qEnvironmentVariable("USERNAME");
#else
    // En el directorio de ejecución del usuario: es suyo (0700) y se vacía al
    // cerrar la sesión. Sin él, en /tmp con el uid en el nombre.
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (!runtime.isEmpty()) return runtime + QStringLiteral("/grimorio.sock");
    return QDir::tempPath() + QStringLiteral("/grimorio-") + QString::number(getuid())
           + QStringLiteral(".sock");
#endif
}

Puerta::Puerta(Nucleo *nucleo, Traer *traer, const QString &version, bool escuchar,
               QObject *padre)
    : QObject(padre), m_nucleo(nucleo), m_traer(traer), m_version(version),
      m_servidor(new QLocalServer(this))
{
    if (!escuchar) return;
    m_servidor->setSocketOptions(QLocalServer::UserAccessOption);
    connect(m_servidor, &QLocalServer::newConnection, this, [this] {
        while (QLocalSocket *s = m_servidor->nextPendingConnection()) atender(s);
    });
    if (!intentarAbrir()) {
        // Otra ventana la tiene. Si un día la suelta —se cierra—, esta la
        // hereda, y lo que llegue del navegador sigue teniendo dónde entrar.
        m_reintento = new QTimer(this);
        m_reintento->setInterval(5000);
        connect(m_reintento, &QTimer::timeout, this, [this] {
            if (intentarAbrir()) m_reintento->stop();
        });
        m_reintento->start();
    }
}

bool Puerta::intentarAbrir()
{
    // Antes de escuchar y no después: con permisos de usuario, Qt crea el
    // socket aparte y lo renombra encima de la ruta, así que `listen` no falla
    // aunque otra ventana esté escuchando ahí; se la quitaba sin decir nada y
    // la dejaba sorda. Si alguien contesta, la puerta es suya.
    QLocalSocket prueba;
    prueba.connectToServer(nombre());
    if (prueba.waitForConnected(300)) {
        prueba.disconnectFromServer();
        return false;
    }
    // Nadie contesta: no hay nada, o es el resto de una que se cerró a lo bruto.
    QLocalServer::removeServer(nombre());
    return m_servidor->listen(nombre());
}

bool Puerta::abierta() const
{
    return m_servidor->isListening();
}

QString Puerta::carpetaExtension() const
{
    // Junto al programa en los paquetes; en el código mientras se desarrolla.
    const QString junto = QCoreApplication::applicationDirPath() + QStringLiteral("/navegador");
    if (QFileInfo::exists(junto + QStringLiteral("/manifest.json"))) return junto;
#ifdef GRIMORIO_NAVEGADOR
    return QStringLiteral(GRIMORIO_NAVEGADOR);
#else
    return junto;
#endif
}

QString Puerta::conectarNavegadores()
{
#ifdef Q_OS_WIN
    const QString nombre = QStringLiteral("grim.exe");
#else
    const QString nombre = QStringLiteral("grim");
#endif
    QString grim = QCoreApplication::applicationDirPath() + QLatin1Char('/') + nombre;
#ifdef GRIMORIO_CARGO_SALIDA
    if (!QFileInfo(grim).isExecutable())
        grim = QStringLiteral(GRIMORIO_CARGO_SALIDA) + QLatin1Char('/') + nombre;
#endif
    if (!QFileInfo(grim).isExecutable()) return tr("no encuentro grim junto al programa");
    QProcess p;
    p.start(grim, { QStringLiteral("navegador"), QStringLiteral("instalar") });
    if (!p.waitForFinished(10000)) return tr("grim no contestó");
    const QString dicho = QString::fromUtf8(p.readAllStandardOutput() + p.readAllStandardError())
                              .remove(QRegularExpression(QStringLiteral("\x1b\[[0-9;]*m")))
                              .trimmed();
    return dicho.isEmpty() ? tr("hecho") : dicho;
}

void Puerta::abrirCarpetaExtension()
{
    QDesktopServices::openUrl(QUrl::fromLocalFile(carpetaExtension()));
}

void Puerta::atender(QLocalSocket *s)
{
    connect(s, &QLocalSocket::disconnected, s, &QObject::deleteLater);
    connect(s, &QLocalSocket::readyRead, this, [this, s] {
        if (s->bytesAvailable() > TOPE_BYTES) {
            s->write(error(tr("demasiado grande")) + '\n');
            s->disconnectFromServer();
            return;
        }
        if (!s->canReadLine()) return;
        const QByteArray respuesta = responder(s->readLine().trimmed());
        s->write(respuesta + '\n');
        s->flush();
        s->disconnectFromServer();
    });
}

QByteArray Puerta::responder(const QByteArray &linea)
{
    QJsonParseError e;
    const QJsonObject p = QJsonDocument::fromJson(linea, &e).object();
    if (e.error != QJsonParseError::NoError) return error(tr("no es JSON: %1").arg(e.errorString()));
    const QString accion = p.value(QStringLiteral("accion")).toString();

    if (accion == QLatin1String("hola")) {
        return QJsonDocument(QJsonObject { { QStringLiteral("ok"), true },
                                           { QStringLiteral("biblioteca"), m_nucleo->nombre() },
                                           { QStringLiteral("version"), m_version } })
            .toJson(QJsonDocument::Compact);
    }

    if (accion == QLatin1String("carpetas")) {
        QJsonArray lista;
        for (const QVariant &v : m_nucleo->carpetas()) {
            const QVariantMap c = v.toMap();
            lista.append(QJsonObject { { QStringLiteral("id"), c.value(QStringLiteral("id")).toString() },
                                       { QStringLiteral("nombre"), c.value(QStringLiteral("nombre")).toString() },
                                       { QStringLiteral("nivel"), c.value(QStringLiteral("nivel")).toInt() } });
        }
        return QJsonDocument(QJsonObject { { QStringLiteral("ok"), true },
                                           { QStringLiteral("carpetas"), lista } })
            .toJson(QJsonDocument::Compact);
    }

    if (accion == QLatin1String("traer")) {
        const QString carpeta = p.value(QStringLiteral("carpeta")).toString();
        // Una carpeta que ya no existe —borrada desde que la extensión la
        // apuntó— no impide guardar: entra suelto, como al importar.
        bool existe = carpeta.isEmpty();
        for (const QVariant &v : m_nucleo->carpetas())
            if (v.toMap().value(QStringLiteral("id")).toString() == carpeta) existe = true;
        const QString destino = existe ? carpeta : QString();
        const QString origen = p.value(QStringLiteral("origen")).toString();
        const QString nombre = p.value(QStringLiteral("nombre")).toString();

        const QString datos = p.value(QStringLiteral("datos")).toString();
        if (!datos.isEmpty()) {
            const QByteArray bytes = QByteArray::fromBase64(datos.toLatin1());
            if (!m_traer->guardarDatos(bytes, p.value(QStringLiteral("tipo")).toString(), nombre,
                                       destino, origen))
                return error(tr("no se pudo guardar"));
        } else {
            const QUrl url(p.value(QStringLiteral("url")).toString());
            if (!url.isValid() || url.scheme().isEmpty()) return error(tr("falta la dirección"));
            m_traer->traerDesde(url, origen, destino);
        }
        return QJsonDocument(QJsonObject { { QStringLiteral("ok"), true } })
            .toJson(QJsonDocument::Compact);
    }

    return error(tr("no sé hacer «%1»").arg(accion));
}
