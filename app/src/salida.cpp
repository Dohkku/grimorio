#include "salida.h"

#include <QProcess>
#include <QTimer>

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
