// Quitar el fondo: lanzar `grim-recorte` y meter lo que salga en la pila.
//
// El recorte no pasa por el núcleo. Lo hace un programa aparte, con su modelo,
// en su propio proceso: el núcleo no sabe de modelos, y si el modelo se atasca
// o se queda sin memoria se cae ese proceso y no la ventana. Aquí solo se
// encola, se lanza con tope de tiempo y, si sale bien, se le pide al núcleo
// que meta el PNG como versión (`anadir_version`). El original no se toca.
//
// El modelo (ISNet con los pesos en f16, 91 MB, Apache-2.0) viene con el
// programa, en `modelos/` junto al ejecutable. Solo lo que se compila desde el
// código sin él lo baja, preguntando antes, de las releases de Grimorio, y
// comprueba su huella antes de usarlo. Las fotos no salen nunca del equipo.
#pragma once
#include <QCryptographicHash>
#include <QFile>
#include <QList>
#include <QObject>
#include <QPair>
#include <QQueue>
#include <QString>
#include <QTimer>
#include <QUrl>

class Nucleo;
class QNetworkAccessManager;
class QNetworkReply;
class QProcess;

class Recorte : public QObject {
    Q_OBJECT
    /// Si el modelo está en el equipo. Sin él, «quitar fondo» primero pregunta
    /// si bajarlo.
    Q_PROPERTY(bool hayModelo READ hayModelo NOTIFY cambio)
    /// Si se encuentra `grim-recorte`. Sin él no hay nada que ofrecer.
    Q_PROPERTY(bool hayPrograma READ hayPrograma NOTIFY cambio)
    Q_PROPERTY(bool bajando READ bajando NOTIFY cambio)
    /// De 0 a 1, mientras se baja el modelo.
    Q_PROPERTY(qreal progreso READ progreso NOTIFY cambio)
    /// Cuántos recortes quedan, contando el que va en marcha.
    Q_PROPERTY(int pendientes READ pendientes NOTIFY cambio)
    /// Lo que ocupa el modelo, para decirlo antes de bajarlo.
    Q_PROPERTY(int megasModelo READ megasModelo CONSTANT)

public:
    Recorte(Nucleo *nucleo, bool conRed, QObject *padre = nullptr);

    bool hayModelo() const;
    bool hayPrograma() const { return !m_programa.isEmpty(); }
    bool bajando() const { return m_bajada != nullptr; }
    qreal progreso() const { return m_progreso; }
    int pendientes() const { return m_cola.size() + (m_proceso ? 1 : 0); }
    int megasModelo() const { return 91; }

    /// Baja el modelo. Al terminar, si había recortes esperando, empiezan.
    Q_INVOKABLE void bajarModelo();
    Q_INVOKABLE void cancelarBajada();
    /// Vuelve a buscar el programa: lo llama quien acaba de ponerlo junto al
    /// de Grimorio, al completar una instalación (ver novedades.h).
    void refrescar();
    /// Encola el recorte de cada `ids[i]`, cuyo archivo es `originales[i]`.
    Q_INVOKABLE void quitarFondo(const QStringList &ids, const QList<QUrl> &originales);

signals:
    void cambio();
    /// Para la barra de estado.
    void aviso(const QString &mensaje, bool error);

private:
    QString rutaModelo() const;
    QString rutaBajada() const;
    void siguiente();
    void terminado(int codigo);

    Nucleo *m_nucleo;
    bool m_conRed;
    QString m_programa;
    QNetworkAccessManager *m_red = nullptr;
    QNetworkReply *m_bajada = nullptr;
    QFile m_parte;
    QCryptographicHash m_huella { QCryptographicHash::Sha256 };
    qreal m_progreso = 0;

    QQueue<QPair<QString, QString>> m_cola; // (id, ruta del original)
    QProcess *m_proceso = nullptr;
    QString m_idActual;
    QString m_salida;
    QTimer m_tope;
    int m_hechos = 0;
};
