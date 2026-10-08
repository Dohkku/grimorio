// El aviso de que hay una versión nueva de Grimorio.
//
// Grimorio no se actualiza solo. Lo que hace es preguntar a la web, como mucho
// una vez al día, qué versión es la última:
//
//     https://grimorio.frederickandrade.com/version.json
//
// Si es más nueva que la que está corriendo, la barra de estado lo dice con un
// enlace a la página de descargas, y ya. No baja nada ni instala nada, y no
// manda nada de quien pregunta: es un GET sin cookies a un archivo estático.
// La web lo dice en su página de privacidad, y se apaga en Ajustes.
//
// Por qué la web y no la API de GitHub: la web es nuestra. Si las descargas
// cambian de sitio, se cambia el enlace del archivo y los programas que ya
// están instalados siguen funcionando; GitHub además pone tope a las consultas
// sin cuenta.
//
// Lo último que se supo se guarda en los ajustes, así que el aviso sale al
// arrancar aunque ese día no se pregunte o no haya red.
#pragma once
#include <QDateTime>
#include <QObject>
#include <QSettings>
#include <QString>
#include <QUrl>

class Ajustes;
class QNetworkAccessManager;

class Novedades : public QObject {
    Q_OBJECT
    /// La versión nueva que hay que avisar, o vacía si no hay que avisar de
    /// nada (no la hay, se ignoró o el aviso está apagado).
    Q_PROPERTY(QString nueva READ nueva NOTIFY cambio)
    /// Dónde se descarga.
    Q_PROPERTY(QUrl enlace READ enlace NOTIFY cambio)
    /// Una pregunta en marcha, para que el botón de Ajustes no se pulse dos
    /// veces seguidas.
    Q_PROPERTY(bool preguntando READ preguntando NOTIFY cambio)
    /// Cuándo se preguntó con éxito por última vez; inválida si nunca.
    Q_PROPERTY(QDateTime ultimaVez READ ultimaVez NOTIFY cambio)

public:
    /// `activa` en falso para las pasadas automáticas (--bench, --captura,
    /// --guion…): una prueba no tiene que depender de la red ni enseñar un
    /// aviso que cambie lo que se mide o se captura.
    Novedades(Ajustes *ajustes, const QString &actual, bool activa, QObject *padre = nullptr);

    QString nueva() const;
    QUrl enlace() const { return m_enlace; }
    bool preguntando() const { return m_preguntando; }
    QDateTime ultimaVez() const { return m_ultimaVez; }

    /// Pregunta ya, haga lo que haga el reloj. El botón de Ajustes.
    Q_INVOKABLE void comprobar();
    /// No volver a avisar de esta versión. De la siguiente sí.
    Q_INVOKABLE void ignorar();
    /// Abre la página de descargas en el navegador.
    Q_INVOKABLE void abrir();

signals:
    void cambio();
    /// Para la barra de estado, solo cuando se pregunta a mano: la pregunta
    /// de cada día no tiene que contar nada si no hay nada nuevo.
    void aviso(const QString &mensaje, bool error);

private:
    void alArrancar();
    void preguntar(bool aMano);

    Ajustes *m_ajustes;
    QString m_actual;
    bool m_activa;
    QSettings m_disco;
    QNetworkAccessManager *m_red = nullptr;
    QString m_ultima;   // la última publicada que se conoce
    QString m_ignorada; // la que se pidió no avisar
    QUrl m_enlace;
    QDateTime m_ultimaVez;
    bool m_preguntando = false;
    bool m_buscaba = true; // el ajuste, para saber cuándo cambia
};
