#include "derivados.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QDeadlineTimer>
#include <QDir>
#include <QFile>
#include <QMetaObject>
#include <QMutex>
#include <QMutexLocker>

#ifdef Q_OS_LINUX
#include <csignal>
#include <sys/types.h>
#include <unistd.h>
#endif

extern "C" {
#include "grimorio.h"
}

/// El puente entre una tarea en marcha y el `Derivados` que la pidió.
///
/// Antes cada tarea llevaba un `QPointer`, pero un `QPointer` no se puede
/// mirar desde otro hilo mientras el principal destruye el objeto: es una
/// carrera. Con el cerrojo, el destructor y la tarea se turnan; quien llega
/// después del destructor encuentra `d` a nulo y se calla.
struct EnlaceDerivados {
    QMutex cerrojo;
    Derivados *d = nullptr;
};

namespace {

/// Cuánto se espera al cerrar a que termine lo que está en marcha. Lo bastante
/// para que una página de PDF o una onda acaben; muy poco para un proxy de
/// vídeo, que es justo lo que no se quiere esperar.
constexpr int ESPERA_CIERRE_MS = 1500;

/// Para a los procesos hijos —ffmpeg, Blender, pdftoppm— que sigan vivos.
///
/// El núcleo los vigila cada diez milisegundos: en cuanto uno muere, la tarea
/// que lo esperaba vuelve con un error y el reparto se vacía. Sin esto, cerrar
/// con un vídeo convirtiéndose dejaba un ffmpeg huérfano trabajando para nadie.
///
/// Solo existe en Linux, donde `/proc` dice quiénes son los hijos. En otros
/// sistemas la tarea se abandona igual y el cierre no se cuelga, pero el hijo
/// termina por su cuenta; hacerlo bien en todas partes pide que el núcleo
/// sepa cancelar sus procesos, y eso es trabajo del núcleo.
void pararHijos(bool aLaFuerza)
{
#ifdef Q_OS_LINUX
    const QDir tareas(QStringLiteral("/proc/self/task"));
    for (const QString &t : tareas.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        QFile f(tareas.filePath(t) + QStringLiteral("/children"));
        if (!f.open(QIODevice::ReadOnly))
            continue;
        const QList<QByteArray> pids = f.readAll().simplified().split(' ');
        for (const QByteArray &p : pids) {
            bool ok = false;
            const pid_t pid = pid_t(p.toLongLong(&ok));
            if (ok && pid > 0)
                ::kill(pid, aLaFuerza ? SIGKILL : SIGTERM);
        }
    }
#else
    Q_UNUSED(aLaFuerza);
#endif
}

QVariantMap derivar(const QJsonObject &peticion)
{
    const QByteArray p = QJsonDocument(peticion).toJson(QJsonDocument::Compact);
    QByteArray salida(4096, Qt::Uninitialized);
    uint32_t n = grim_derivar(reinterpret_cast<const uint8_t *>(p.constData()), uint32_t(p.size()),
                              reinterpret_cast<uint8_t *>(salida.data()), uint32_t(salida.size()));
    if (n > uint32_t(salida.size())) {
        // No cabía. El derivado ya está en disco, así que repetir es leerlo.
        salida.resize(int(n));
        n = grim_derivar(reinterpret_cast<const uint8_t *>(p.constData()), uint32_t(p.size()),
                         reinterpret_cast<uint8_t *>(salida.data()), uint32_t(salida.size()));
    }
    salida.truncate(int(n));
    QVariantMap r = QJsonDocument::fromJson(salida).object().toVariantMap();
    if (r.isEmpty()) {
        r.insert(QStringLiteral("ok"), false);
        r.insert(QStringLiteral("error"), QStringLiteral("respuesta ilegible del núcleo"));
    }
    return r;
}

} // namespace

Derivados::Derivados(const QString &raiz, QObject *padre)
    : QObject(padre), m_raiz(raiz), m_hilos(new QThreadPool), m_lentos(new QThreadPool),
      m_enlace(std::make_shared<EnlaceDerivados>())
{
    m_enlace->d = this;
    m_hilos->setMaxThreadCount(3);
    m_lentos->setMaxThreadCount(1);
}

Derivados::~Derivados()
{
    // Primero, que nadie vuelva aquí: una tarea que termine a partir de ahora
    // encuentra el enlace vacío y tira su respuesta.
    {
        QMutexLocker l(&m_enlace->cerrojo);
        m_enlace->d = nullptr;
    }
    // Lo que no ha empezado no empieza.
    m_hilos->clear();
    m_lentos->clear();

    // Antes esto era un `waitForDone()` sin tope, y con un proxy de vídeo o una
    // malla de Blender en marcha la ventana se quedaba cerrada a medias durante
    // minutos. Ahora: un rato para lo que esté a punto de acabar; después, se
    // paran los procesos hijos, que es lo que de verdad tarda, y se vuelve a
    // esperar un poco. Si ni así, se abandona.
    QDeadlineTimer hasta(ESPERA_CIERRE_MS);
    bool listo = m_hilos->waitForDone(int(hasta.remainingTime()))
        && m_lentos->waitForDone(int(qMax<qint64>(0, hasta.remainingTime())));
    for (int vuelta = 0; !listo && vuelta < 3; ++vuelta) {
        pararHijos(vuelta == 2);
        listo = m_hilos->waitForDone(300) && m_lentos->waitForDone(300);
    }
    if (listo) {
        delete m_hilos;
        delete m_lentos;
    }
    // Si no, se dejan sin borrar a propósito: borrarlos sería volver a esperar
    // sin tope. El proceso está cerrando y el sistema recoge el resto; las
    // tareas que queden no tocan este objeto gracias al enlace.
}

QString Derivados::pedir(const QString &que, const QString &id, const QUrl &original,
                         const QVariantMap &extra)
{
    QString clave = que + QLatin1Char(':') + id;
    for (auto it = extra.cbegin(); it != extra.cend(); ++it)
        clave += QLatin1Char(':') + it.value().toString();

    if (m_hechos.contains(clave)) {
        const QVariantMap r = m_hechos.value(clave);
        QMetaObject::invokeMethod(
            this, [this, clave, r] { emit listo(clave, r); }, Qt::QueuedConnection);
        return clave;
    }
    if (m_enCamino.contains(clave))
        return clave;
    m_enCamino.insert(clave);

    QJsonObject p = QJsonObject::fromVariantMap(extra);
    p.insert(QStringLiteral("que"), que);
    p.insert(QStringLiteral("raiz"), m_raiz);
    p.insert(QStringLiteral("id"), id);
    p.insert(QStringLiteral("ruta"), original.isLocalFile() ? original.toLocalFile()
                                                            : original.toString());

    const std::shared_ptr<EnlaceDerivados> enlace = m_enlace;
    const bool lento = que == QLatin1String("proxy") || que == QLatin1String("tira");
    (lento ? m_lentos : m_hilos)->start([enlace, clave, p] {
        const QVariantMap r = derivar(p);
        // Con el cerrojo tomado el destructor no puede ir por delante: o el
        // objeto sigue vivo mientras se encola la respuesta, o ya no está y no
        // se encola nada. Lo encolado lo descarta Qt si el objeto muere antes
        // de que llegue su turno.
        QMutexLocker l(&enlace->cerrojo);
        Derivados *yo = enlace->d;
        if (!yo)
            return;
        QMetaObject::invokeMethod(
            yo,
            [yo, clave, r] {
                yo->m_enCamino.remove(clave);
                // Los fallos no se recuerdan: si faltaba ffmpeg y se instala,
                // volver a abrir el elemento tiene que volver a intentarlo.
                if (r.value(QStringLiteral("ok")).toBool())
                    yo->m_hechos.insert(clave, r);
                emit yo->listo(clave, r);
            },
            Qt::QueuedConnection);
    });
    return clave;
}

QVariantMap Derivados::hecho(const QString &clave) const
{
    return m_hechos.value(clave);
}
