#include "novedades.h"

#include "ajustes.h"
#include "version.h"

#include <QDesktopServices>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QTimer>

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

} // namespace

Novedades::Novedades(Ajustes *ajustes, const QString &actual, bool activa, QObject *padre)
    : QObject(padre), m_ajustes(ajustes), m_actual(actual), m_activa(activa)
{
    m_ultima = m_disco.value(QStringLiteral("novedades/ultima")).toString();
    m_ignorada = m_disco.value(QStringLiteral("novedades/ignorada")).toString();
    m_enlace = QUrl(m_disco.value(QStringLiteral("novedades/enlace"),
                                  QString::fromLatin1(version::PAGINA)).toString());
    m_ultimaVez = m_disco.value(QStringLiteral("novedades/cuando")).toDateTime();
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

    QNetworkRequest req{QUrl(QString::fromLatin1(DIRECCION))};
    // Sin versión en el agente: no hace falta para contestar, y así la
    // pregunta es la misma para todos.
    req.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("Grimorio"));
    req.setAttribute(QNetworkRequest::CookieSaveControlAttribute, QNetworkRequest::Manual);
    req.setAttribute(QNetworkRequest::CookieLoadControlAttribute, QNetworkRequest::Manual);
    req.setAttribute(QNetworkRequest::CacheLoadControlAttribute, QNetworkRequest::AlwaysNetwork);
    req.setTransferTimeout(10000);

    QNetworkReply *r = m_red->get(req);
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
        m_ultimaVez = QDateTime::currentDateTimeUtc();
        m_disco.setValue(QStringLiteral("novedades/ultima"), m_ultima);
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

        if (aMano) {
            if (version::comparar(m_ultima, m_actual) > 0)
                emit aviso(tr("hay una versión nueva: %1").arg(m_ultima), false);
            else
                emit aviso(tr("tienes la última versión (%1)").arg(m_actual), false);
        }
    });
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
