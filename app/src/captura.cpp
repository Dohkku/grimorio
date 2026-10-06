#include "captura.h"

#include "nucleo.h"
#include "portal.h"

#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QProcess>
#include <QStandardPaths>
#include <QUrl>

#ifdef Q_OS_WIN
#include <QCursor>
#include <QGuiApplication>
#include <QPixmap>
#include <QScreen>
#endif

Captura::Captura(Nucleo *nucleo, QObject *padre) : QObject(padre), m_nucleo(nucleo) {}

void Captura::ponerEnMarcha(bool si)
{
    if (m_enMarcha == si) return;
    m_enMarcha = si;
    emit enMarchaCambio();
}

void Captura::capturar(const QString &carpeta)
{
    if (m_enMarcha) return;
    m_carpeta = carpeta;
    ponerEnMarcha(true);
#ifdef Q_OS_WIN
    porPantallaEntera();
#else
    if (!porPortal()) porImageMagick();
#endif
}

QString Captura::rutaNueva() const
{
    const QString dir = m_nucleo->raiz() + QStringLiteral("/cache/entrada/")
                        + QString::number(QDateTime::currentMSecsSinceEpoch());
    QDir().mkpath(dir);
    return dir + QStringLiteral("/captura %1.png")
                     .arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyy-MM-dd HH.mm.ss")));
}

#ifdef Q_OS_WIN
void Captura::porPantallaEntera()
{
    // La pantalla en la que está el ratón, entera. Cuando esto se llama la
    // ventana ya se ha apartado (QML espera a que acabe de minimizarse).
    QScreen *pantalla = QGuiApplication::screenAt(QCursor::pos());
    if (!pantalla) pantalla = QGuiApplication::primaryScreen();
    const QPixmap foto = pantalla ? pantalla->grabWindow(0) : QPixmap();
    const QString ruta = rutaNueva();
    if (foto.isNull() || !foto.save(ruta, "PNG")) {
        terminar(QString(), tr("no pude capturar la pantalla"));
        return;
    }
    terminar(ruta, QString());
}
#endif

bool Captura::porPortal()
{
    auto *p = PeticionPortal::pedir(QStringLiteral("Screenshot"), QStringLiteral("Screenshot"),
                                    { QString() }, { { QStringLiteral("interactive"), true } },
                                    this);
    if (!p) return false;
    connect(p, &PeticionPortal::respuesta, this, &Captura::respuestaPortal);
    return true;
}

void Captura::respuestaPortal(uint codigo, const QVariantMap &resultados)
{
    // 0 es hecho, 1 es que se canceló —no es un error, es cambiar de idea—, y
    // lo demás sí lo es.
    if (codigo == 1) {
        terminar(QString(), QString());
        return;
    }
    const QUrl uri(resultados.value(QStringLiteral("uri")).toString());
    if (codigo != 0 || !uri.isLocalFile()) {
        terminar(QString(), tr("el escritorio no dejó hacer la captura"));
        return;
    }
    terminar(uri.toLocalFile(), QString());
}

void Captura::porImageMagick()
{
    if (QStandardPaths::findExecutable(QStringLiteral("import")).isEmpty()) {
        terminar(QString(), tr("no hay cómo capturar: ni el portal del escritorio ni ImageMagick"));
        return;
    }
    const QString ruta = rutaNueva();
    auto *p = new QProcess(this);
    connect(p, &QProcess::finished, this, [this, p, ruta](int salida, QProcess::ExitStatus) {
        p->deleteLater();
        if (salida == 0 && QFileInfo(ruta).size() > 0) terminar(ruta, QString());
        else terminar(QString(), QString());
    });
    p->start(QStringLiteral("import"), { ruta });
}

void Captura::terminar(const QString &ruta, const QString &error)
{
    ponerEnMarcha(false);
    emit acabada();
    if (!error.isEmpty()) {
        emit aviso(error, true);
        return;
    }
    if (ruta.isEmpty()) return; // cancelada: nada que decir
    m_nucleo->importarDeFuera({ ruta }, m_carpeta, QString(), true);
}
