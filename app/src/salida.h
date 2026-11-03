// La salida de sonido que el sistema tiene ahora mismo.
//
// En Linux, Qt 6.4 (el de Ubuntu 24.04) dice cuál es la salida por defecto al
// arrancar y luego ya no se entera: si se cambia de auriculares a altavoz en
// la barra del escritorio, `MediaDevices.defaultAudioOutput` sigue nombrando
// los auriculares, y no hay señal que avise. Probado en una sesión de
// PipeWire aparte, con dos salidas de mentira: el valor no cambia nunca.
//
// Y eso no se queda en una etiqueta vieja. Cada vídeo nuevo abre su sonido
// pidiendo esa salida por su nombre, y PipeWire lo trata como una elección
// de quien escucha: lo deja allí aunque todo el escritorio suene ya por otro
// lado. Pedido con el nombre de la salida que de verdad está por defecto,
// en cambio, PipeWire lo trata como «la de por defecto» y lo mueve él solo
// en el siguiente cambio.
//
// Así que se pregunta al sistema, que sí lo sabe: `pactl subscribe` avisa de
// cada cambio del servidor de sonido sin tener que preguntar cada poco, y
// `pactl get-default-sink` dice el nombre, que es el mismo `id` que Qt da a
// sus dispositivos. Va igual con PipeWire que con PulseAudio. Sin `pactl`
// (o en Windows, donde Qt sí sigue al sistema) `predeterminada` se queda
// vacía y el reproductor se queda con lo que diga Qt.
#pragma once
#include <QObject>
#include <QString>

class QProcess;

class SalidaSonido : public QObject {
    Q_OBJECT
    /// El nombre de la salida por defecto del sistema, o vacío si no se sabe.
    Q_PROPERTY(QString predeterminada READ predeterminada NOTIFY predeterminadaCambiada)
public:
    explicit SalidaSonido(QObject *parent = nullptr);
    ~SalidaSonido() override;

    QString predeterminada() const { return m_predeterminada; }

signals:
    void predeterminadaCambiada();

private:
    void preguntar();
    void escuchar();
    void moverFlujos();

    QString m_predeterminada;
    QProcess *m_escucha = nullptr;
    QProcess *m_pregunta = nullptr;
    // Un aviso que llega mientras aún se espera la respuesta anterior: se
    // vuelve a preguntar al acabar, que la respuesta en camino puede ser vieja.
    bool m_otraVez = false;
    int m_reintentos = 0;
    QProcess *m_flujos = nullptr;
};
