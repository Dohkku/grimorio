#include "recorte.h"
#include "nucleo.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QProcess>
#include <QStandardPaths>
#include <QUuid>

namespace {

const char *const NOMBRE_MODELO = "isnet-general-use";
/// ISNet con los pesos en f16: la mitad de grande y el mismo recorte. Ver
/// herramientas/modelo/preparar.py, que lo rehace desde el original.
const char *const ARCHIVO_MODELO = "isnet-general-use-pesos16.onnx";
/// Viene con el programa. Esta descarga es solo para lo que se compila desde
/// el código sin el modelo al lado, y sale de las releases de Grimorio, no de
/// las de otro proyecto que pueda moverlo o borrarlo.
const char *const URL_MODELO =
    "https://github.com/Dohkku/grimorio/releases/download/modelo-isnet-1/isnet-general-use-pesos16.onnx";
/// La huella del archivo que se probó. Uno bajado que no la tenga no se usa:
/// lo que se ejecuta sobre las fotos de alguien tiene que ser exactamente eso.
const char *const HUELLA_MODELO =
    "ed44f83dfa784f3c7a1a8cf7928e9ff348eaad4d8a4a2ac4421eeb013637b91e";
/// Una foto tarda segundos; diez minutos es que algo se ha quedado colgado.
constexpr int TOPE_MS = 10 * 60 * 1000;

QString nombreDelPrograma()
{
#ifdef Q_OS_WIN
    return QStringLiteral("grim-recorte.exe");
#else
    return QStringLiteral("grim-recorte");
#endif
}

/// Junto al programa, que es donde lo dejan los paquetes; en la salida de
/// cargo, que es donde está mientras se desarrolla; o en el PATH.
QString buscarPrograma()
{
    const QString nombre = nombreDelPrograma();
    const QStringList sitios {
        QCoreApplication::applicationDirPath() + QLatin1Char('/') + nombre,
#ifdef GRIMORIO_CARGO_SALIDA
        QStringLiteral(GRIMORIO_CARGO_SALIDA) + QLatin1Char('/') + nombre,
#endif
    };
    for (const QString &s : sitios)
        if (QFileInfo(s).isExecutable()) return s;
    return QStandardPaths::findExecutable(QStringLiteral("grim-recorte"));
}

} // namespace

Recorte::Recorte(Nucleo *nucleo, bool conRed, QObject *padre)
    : QObject(padre), m_nucleo(nucleo), m_conRed(conRed), m_programa(buscarPrograma())
{
    m_tope.setSingleShot(true);
    m_tope.setInterval(TOPE_MS);
    connect(&m_tope, &QTimer::timeout, this, [this] {
        if (m_proceso) m_proceso->kill();
    });
}

/// Donde se deja el modelo si hay que bajarlo.
QString Recorte::rutaBajada() const
{
    return QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation)
           + QStringLiteral("/modelos/") + QLatin1String(ARCHIVO_MODELO);
}

/// El modelo que se usa: el que viene junto al programa, el de la carpeta
/// `modelos/` del código mientras se desarrolla, o el que se bajó.
void Recorte::refrescar()
{
    m_programa = buscarPrograma();
    emit cambio();
    siguiente();
}

QString Recorte::rutaModelo() const
{
    const QString archivo = QLatin1String(ARCHIVO_MODELO);
    const QStringList sitios {
        QCoreApplication::applicationDirPath() + QStringLiteral("/modelos/") + archivo,
#ifdef GRIMORIO_MODELOS
        QStringLiteral(GRIMORIO_MODELOS) + QLatin1Char('/') + archivo,
#endif
        rutaBajada(),
    };
    for (const QString &s : sitios)
        if (QFileInfo::exists(s)) return s;
    return {};
}

bool Recorte::hayModelo() const
{
    return !rutaModelo().isEmpty();
}

void Recorte::bajarModelo()
{
    if (m_bajada || hayModelo()) return;
    if (!m_conRed) {
        emit aviso(tr("sin red: el modelo para quitar fondos no se puede bajar ahora"), true);
        return;
    }
    QDir().mkpath(QFileInfo(rutaBajada()).absolutePath());
    m_parte.setFileName(rutaBajada() + QStringLiteral(".parte"));
    if (!m_parte.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        emit aviso(tr("no pude escribir en %1").arg(m_parte.fileName()), true);
        return;
    }
    m_huella.reset();
    m_progreso = 0;
    if (!m_red) m_red = new QNetworkAccessManager(this);

    QNetworkRequest pedido { QUrl(QString::fromLatin1(URL_MODELO)) };
    pedido.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                        QNetworkRequest::NoLessSafeRedirectPolicy);
    m_bajada = m_red->get(pedido);
    // Al disco según llega y no al final: son 170 MB, y en memoria serían 170
    // MB que el programa no necesita para nada.
    connect(m_bajada, &QNetworkReply::readyRead, this, [this] {
        const QByteArray trozo = m_bajada->readAll();
        m_huella.addData(trozo);
        m_parte.write(trozo);
    });
    connect(m_bajada, &QNetworkReply::downloadProgress, this, [this](qint64 hecho, qint64 total) {
        m_progreso = total > 0 ? qreal(hecho) / qreal(total) : 0;
        emit cambio();
    });
    connect(m_bajada, &QNetworkReply::finished, this, [this] {
        QNetworkReply *r = m_bajada;
        m_bajada = nullptr;
        r->deleteLater();
        const QByteArray resto = r->readAll();
        m_huella.addData(resto);
        m_parte.write(resto);
        m_parte.close();

        if (r->error() != QNetworkReply::NoError) {
            m_parte.remove();
            if (r->error() != QNetworkReply::OperationCanceledError)
                emit aviso(tr("no se pudo bajar el modelo: %1").arg(r->errorString()), true);
            m_cola.clear();
        } else if (m_huella.result().toHex() != QByteArray(HUELLA_MODELO)) {
            m_parte.remove();
            m_cola.clear();
            emit aviso(tr("el modelo bajado no es el que debería (la huella no coincide); no se usa"), true);
        } else {
            QFile::remove(rutaBajada());
            m_parte.rename(rutaBajada());
            emit aviso(tr("modelo listo: ya se pueden quitar fondos sin conexión"), false);
        }
        emit cambio();
        siguiente();
    });
    emit cambio();
}

void Recorte::cancelarBajada()
{
    if (m_bajada) m_bajada->abort();
}

void Recorte::quitarFondo(const QStringList &ids, const QList<QUrl> &originales)
{
    if (!hayPrograma()) {
        emit aviso(tr("falta grim-recorte junto al programa: no se pueden quitar fondos"), true);
        return;
    }
    for (int i = 0; i < ids.size() && i < originales.size(); ++i)
        if (originales[i].isLocalFile()) m_cola.enqueue({ ids[i], originales[i].toLocalFile() });
    emit cambio();
    if (!hayModelo()) {
        // Lo encolado espera a que el modelo llegue; la ventana es la que
        // pregunta si bajarlo, aquí solo se guarda lo pedido.
        return;
    }
    siguiente();
}

void Recorte::siguiente()
{
    if (m_proceso || m_cola.isEmpty() || !hayModelo()) return;
    const auto [id, original] = m_cola.dequeue();
    const QString raiz = m_nucleo->raiz();
    if (raiz.isEmpty()) return;
    // Dentro de la biblioteca y en esa carpeta en concreto: el núcleo mete lo
    // que hay ahí moviéndolo, y cualquier otra cosa copiándola.
    const QString dir = raiz + QStringLiteral("/cache/versiones");
    QDir().mkpath(dir);
    m_idActual = id;
    m_salida = dir + QLatin1Char('/') + QUuid::createUuid().toString(QUuid::WithoutBraces)
               + QStringLiteral(".png");

    m_proceso = new QProcess(this);
    m_proceso->setProcessChannelMode(QProcess::SeparateChannels);
    connect(m_proceso, &QProcess::finished, this, [this](int codigo, QProcess::ExitStatus) {
        terminado(codigo);
    });
    connect(m_proceso, &QProcess::errorOccurred, this, [this](QProcess::ProcessError e) {
        if (e == QProcess::FailedToStart) terminado(-1);
    });
    m_tope.start();
    m_proceso->start(m_programa, { QStringLiteral("--modelo"), rutaModelo(), original, m_salida });
    emit aviso(tr("quitando el fondo…"), false);
    emit cambio();
}

void Recorte::terminado(int codigo)
{
    if (!m_proceso) return;
    m_tope.stop();
    const QString error = QString::fromUtf8(m_proceso->readAllStandardError()).trimmed();
    m_proceso->deleteLater();
    m_proceso = nullptr;

    if (codigo == 0 && QFileInfo::exists(m_salida)) {
        m_nucleo->anadirVersion(m_idActual, m_salida, QString::fromLatin1(NOMBRE_MODELO));
    } else {
        QFile::remove(m_salida);
        const QString ultima = error.section(QLatin1Char('\n'), -1);
        emit aviso(tr("no se pudo quitar el fondo: %1")
                       .arg(ultima.isEmpty() ? tr("el recorte se paró") : ultima),
                   true);
    }
    emit cambio();
    siguiente();
}
