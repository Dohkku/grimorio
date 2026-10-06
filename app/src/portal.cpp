#include "portal.h"

#include <QProcess>
#include <QRandomGenerator>
#include <QStandardPaths>
#include <QUrl>

// Los portales van por D-Bus, que solo hay en Linux (y en algún BSD). Sin
// D-Bus —Windows— `pedir` contesta siempre «no hay portal», y quien lo llama
// sigue por su otro camino, igual que en un Linux sin portal.
#ifdef GRIMORIO_DBUS
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusObjectPath>
#include <QDBusReply>
#endif

// En Windows los diálogos son los del propio sistema, a través de
// QFileDialog. Ver `elegirCarpeta`.
#ifdef Q_OS_WIN
#include <QFileDialog>
#include <QPointer>
#include <QTimer>
#endif

#ifdef GRIMORIO_DBUS
namespace {
const QString SERVICIO = QStringLiteral("org.freedesktop.portal.Desktop");
const QString RUTA = QStringLiteral("/org/freedesktop/portal/desktop");
const QString PETICION = QStringLiteral("org.freedesktop.portal.Request");
}
#endif

PeticionPortal *PeticionPortal::pedir(const QString &interfaz, const QString &metodo,
                                      const QVariantList &args, QVariantMap opciones,
                                      QObject *padre)
{
#ifndef GRIMORIO_DBUS
    Q_UNUSED(interfaz)
    Q_UNUSED(metodo)
    Q_UNUSED(args)
    Q_UNUSED(opciones)
    Q_UNUSED(padre)
    return nullptr;
#else
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) return nullptr;

    auto *p = new PeticionPortal(padre);
    const QString ficha = QStringLiteral("grimorio%1").arg(QRandomGenerator::global()->generate());
    QString yo = bus.baseService();
    yo.remove(0, 1).replace(QLatin1Char('.'), QLatin1Char('_'));
    p->m_ruta = RUTA + QStringLiteral("/request/") + yo + QLatin1Char('/') + ficha;
    bus.connect(SERVICIO, p->m_ruta, PETICION, QStringLiteral("Response"), p,
                SLOT(recibir(uint, QVariantMap)));

    QDBusInterface portal(SERVICIO, RUTA, QStringLiteral("org.freedesktop.portal.") + interfaz, bus);
    bool ok = portal.isValid();
    if (ok) {
        opciones.insert(QStringLiteral("handle_token"), ficha);
        QVariantList todo = args;
        todo << opciones;
        const QDBusReply<QDBusObjectPath> r = portal.callWithArgumentList(QDBus::Block, metodo, todo);
        ok = r.isValid();
    }
    if (!ok) {
        bus.disconnect(SERVICIO, p->m_ruta, PETICION, QStringLiteral("Response"), p,
                       SLOT(recibir(uint, QVariantMap)));
        delete p;
        return nullptr;
    }
    return p;
#endif
}

void PeticionPortal::recibir(uint codigo, const QVariantMap &resultados)
{
#ifdef GRIMORIO_DBUS
    QDBusConnection::sessionBus().disconnect(SERVICIO, m_ruta, PETICION, QStringLiteral("Response"),
                                             this, SLOT(recibir(uint, QVariantMap)));
#endif
    emit respuesta(codigo, resultados);
    deleteLater();
}

#ifdef Q_OS_WIN
namespace {
/// El diálogo de Windows (el mismo `IFileDialog` del Explorador, que es lo que
/// abre QFileDialog por debajo), en la vuelta siguiente del bucle de eventos.
///
/// QFileDialog es de QtWidgets y pide QApplication: por eso en Windows
/// `main.cpp` arranca con QApplication en vez de QGuiApplication. Se eligió
/// esto y no `QtQuick.Dialogs` porque los que piden diálogos son objetos de C++
/// con una función de vuelta, y desde QML habría que darle la vuelta a todos.
///
/// El diálogo es modal y no vuelve hasta que se cierra. Se lanza en diferido
/// para que, como con el portal, `elegir…` vuelva enseguida y la respuesta
/// llegue después: quien llama está en mitad de un clic de QML. Y si `dueno`
/// desaparece mientras el diálogo está abierto, la respuesta se tira.
void enDiferido(QObject *dueno, std::function<void(QPointer<QObject>)> f)
{
    QPointer<QObject> vivo(dueno);
    QTimer::singleShot(0, dueno, [vivo, f] { f(vivo); });
}
}

bool elegirCarpeta(QObject *dueno, const QString &titulo, std::function<void(const QString &)> listo)
{
    enDiferido(dueno, [titulo, listo](QPointer<QObject> vivo) {
        const QString ruta = QFileDialog::getExistingDirectory(nullptr, titulo);
        if (vivo && !ruta.isEmpty()) listo(ruta);
    });
    return true;
}

bool elegirArchivos(QObject *dueno, const QString &titulo, std::function<void(const QStringList &)> listo)
{
    enDiferido(dueno, [titulo, listo](QPointer<QObject> vivo) {
        const QStringList rutas = QFileDialog::getOpenFileNames(nullptr, titulo);
        if (vivo && !rutas.isEmpty()) listo(rutas);
    });
    return true;
}
#else

bool elegirCarpeta(QObject *dueno, const QString &titulo, std::function<void(const QString &)> listo)
{
    auto *p = PeticionPortal::pedir(QStringLiteral("FileChooser"), QStringLiteral("OpenFile"),
                                    { QString(), titulo },
                                    { { QStringLiteral("directory"), true },
                                      { QStringLiteral("modal"), true } },
                                    dueno);
    if (p) {
        QObject::connect(p, &PeticionPortal::respuesta, dueno,
                         [listo](uint codigo, const QVariantMap &r) {
                             if (codigo != 0) return; // cancelado
                             const QStringList uris = r.value(QStringLiteral("uris")).toStringList();
                             if (!uris.isEmpty()) listo(QUrl(uris.first()).toLocalFile());
                         });
        return true;
    }
    if (QStandardPaths::findExecutable(QStringLiteral("zenity")).isEmpty()) return false;
    auto *z = new QProcess(dueno);
    QObject::connect(z, &QProcess::finished, dueno, [z, listo](int salida, QProcess::ExitStatus) {
        z->deleteLater();
        const QString ruta = QString::fromUtf8(z->readAllStandardOutput()).trimmed();
        if (salida == 0 && !ruta.isEmpty()) listo(ruta);
    });
    z->start(QStringLiteral("zenity"), { QStringLiteral("--file-selection"), QStringLiteral("--directory"),
                                          QStringLiteral("--title=%1").arg(titulo) });
    return true;
}

bool elegirArchivos(QObject *dueno, const QString &titulo, std::function<void(const QStringList &)> listo)
{
    auto *p = PeticionPortal::pedir(QStringLiteral("FileChooser"), QStringLiteral("OpenFile"),
                                    { QString(), titulo },
                                    { { QStringLiteral("multiple"), true },
                                      { QStringLiteral("modal"), true } },
                                    dueno);
    if (p) {
        QObject::connect(p, &PeticionPortal::respuesta, dueno,
                         [listo](uint codigo, const QVariantMap &r) {
                             if (codigo != 0) return;
                             QStringList rutas;
                             for (const QString &u : r.value(QStringLiteral("uris")).toStringList())
                                 rutas << QUrl(u).toLocalFile();
                             if (!rutas.isEmpty()) listo(rutas);
                         });
        return true;
    }
    if (QStandardPaths::findExecutable(QStringLiteral("zenity")).isEmpty()) return false;
    auto *z = new QProcess(dueno);
    QObject::connect(z, &QProcess::finished, dueno, [z, listo](int salida, QProcess::ExitStatus) {
        z->deleteLater();
        const QString todo = QString::fromUtf8(z->readAllStandardOutput()).trimmed();
        if (salida == 0 && !todo.isEmpty()) listo(todo.split(QLatin1Char('\n')));
    });
    z->start(QStringLiteral("zenity"), { QStringLiteral("--file-selection"), QStringLiteral("--multiple"),
                                          QStringLiteral("--separator=\n"),
                                          QStringLiteral("--title=%1").arg(titulo) });
    return true;
}
#endif
