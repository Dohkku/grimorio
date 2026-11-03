#include "salida.h"

#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QTimer>

#ifdef Q_OS_LINUX
#include <csignal>
#include <sys/prctl.h>
#endif

SalidaSonido::SalidaSonido(QObject *parent) : QObject(parent) {
#ifdef Q_OS_LINUX
    escuchar();
    preguntar();
#endif
}

SalidaSonido::~SalidaSonido() {
    // `pactl subscribe` no acaba nunca por su cuenta: se corta aquí para no
    // dejarlo huérfano ni esperar a que el destructor de QProcess avise.
    if (m_escucha) {
        m_escucha->disconnect(this);
        m_escucha->kill();
        m_escucha->waitForFinished(500);
    }
}

void SalidaSonido::escuchar() {
    m_escucha = new QProcess(this);
    m_escucha->setProcessChannelMode(QProcess::SeparateChannels);
    m_escucha->setStandardErrorFile(QProcess::nullDevice());
#ifdef Q_OS_LINUX
    // Y si Grimorio se cierra a la fuerza, sin pasar por el destructor, que se
    // vaya con él: si no, se queda escuchando huérfano hasta cerrar la sesión.
    m_escucha->setChildProcessModifier([] { ::prctl(PR_SET_PDEATHSIG, SIGTERM); });
#endif
    connect(m_escucha, &QProcess::readyReadStandardOutput, this, [this] {
        bool toca = false;
        while (m_escucha->canReadLine()) {
            const QByteArray linea = m_escucha->readLine();
            // «Event 'change' on server #-1» es el cambio de salida por
            // defecto. Una salida que aparece o se va (unos auriculares
            // Bluetooth) también puede cambiarla, y el servidor no siempre lo
            // dice por separado. Los «change» de una salida son de volumen:
            // esos no.
            if (linea.contains(" on server")
                || (linea.contains(" on sink #") && !linea.contains("'change'")))
                toca = true;
        }
        if (toca) preguntar();
    });
    // Si el servidor de sonido se reinicia, `pactl subscribe` se cae con él.
    // Se vuelve a escuchar al rato, unas pocas veces: sin `pactl` instalado
    // no tiene sentido insistir para siempre.
    connect(m_escucha, &QProcess::finished, this, [this] {
        m_escucha->deleteLater();
        m_escucha = nullptr;
        if (++m_reintentos > 5) return;
        QTimer::singleShot(3000, this, [this] { escuchar(); preguntar(); });
    });
    connect(m_escucha, &QProcess::errorOccurred, this, [this](QProcess::ProcessError e) {
        if (e != QProcess::FailedToStart) return;
        m_escucha->deleteLater();
        m_escucha = nullptr;
    });
    m_escucha->start(QStringLiteral("pactl"), {QStringLiteral("subscribe")});
}

void SalidaSonido::preguntar() {
    if (m_pregunta) {
        m_otraVez = true;
        return;
    }
    m_pregunta = new QProcess(this);
    m_pregunta->setStandardErrorFile(QProcess::nullDevice());
    auto acabar = [this] {
        const bool bien = m_pregunta->exitStatus() == QProcess::NormalExit
                          && m_pregunta->exitCode() == 0;
        const QString nombre = bien ? QString::fromUtf8(m_pregunta->readAllStandardOutput()).trimmed()
                                    : QString();
        m_pregunta->deleteLater();
        m_pregunta = nullptr;
        if (!nombre.isEmpty() && nombre != m_predeterminada) {
            m_predeterminada = nombre;
            emit predeterminadaCambiada();
            moverFlujos();
        }
        if (m_otraVez) {
            m_otraVez = false;
            preguntar();
        }
    };
    connect(m_pregunta, &QProcess::finished, this, acabar);
    connect(m_pregunta, &QProcess::errorOccurred, this, [this](QProcess::ProcessError e) {
        if (e != QProcess::FailedToStart) return;
        m_pregunta->deleteLater();
        m_pregunta = nullptr;
        m_otraVez = false;
    });
    m_pregunta->start(QStringLiteral("pactl"), {QStringLiteral("get-default-sink")});
}

// Lo que ya suena, a la salida nueva.
//
// El reproductor no puede cambiar de dispositivo con un vídeo cargado (ver
// Reproductor.qml: Qt 6.4 se queda esperando para siempre), y casi siempre
// no hace falta: PipeWire lleva solo a la de por defecto lo que se abrió en
// ella. Pero no todo. Un flujo abierto en unos auriculares que luego se
// apagan y vuelven se queda atado a ellos: si después se elige el altavoz,
// ese vídeo seguía sonando en los auriculares hasta abrir otro.
//
// Así que se le pide al servidor de sonido que mueva los flujos de este
// programa, como haría el control de volumen del escritorio. Va por fuera de
// Qt y de su tubería, sin esperar a nada: no puede congelar la ventana.
// Mover uno que ya está donde toca no hace nada.
void SalidaSonido::moverFlujos() {
    if (m_flujos || m_predeterminada.isEmpty()) return;
    m_flujos = new QProcess(this);
    m_flujos->setStandardErrorFile(QProcess::nullDevice());
    connect(m_flujos, &QProcess::finished, this, [this] {
        const QJsonArray lista = QJsonDocument::fromJson(m_flujos->readAllStandardOutput()).array();
        m_flujos->deleteLater();
        m_flujos = nullptr;
        const QString mio = QString::number(QCoreApplication::applicationPid());
        for (const QJsonValue &v : lista) {
            const QJsonObject flujo = v.toObject();
            const QJsonObject props = flujo.value(QStringLiteral("properties")).toObject();
            if (props.value(QStringLiteral("application.process.id")).toString() != mio) continue;
            QProcess::startDetached(QStringLiteral("pactl"),
                                    {QStringLiteral("move-sink-input"),
                                     QString::number(flujo.value(QStringLiteral("index")).toInt()),
                                     m_predeterminada});
        }
    });
    connect(m_flujos, &QProcess::errorOccurred, this, [this](QProcess::ProcessError e) {
        if (e != QProcess::FailedToStart) return;
        m_flujos->deleteLater();
        m_flujos = nullptr;
    });
    m_flujos->start(QStringLiteral("pactl"),
                    {QStringLiteral("-f"), QStringLiteral("json"), QStringLiteral("list"),
                     QStringLiteral("sink-inputs")});
}
