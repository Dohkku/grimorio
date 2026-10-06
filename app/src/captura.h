// Capturar una zona de la pantalla y meterla en la biblioteca.
//
// Por el portal del escritorio (`org.freedesktop.portal.Screenshot`) en modo
// interactivo: en GNOME es su propia pantalla de captura, con la que ya se sabe
// elegir una zona, una ventana o todo. Funciona igual en X11 y en Wayland, que
// es lo que no hace ninguna herramienta de captura por su cuenta.
//
// Sin portal —otro escritorio, una sesión rara— se cae a `import` de
// ImageMagick, que en X11 deja arrastrar un rectángulo con el ratón.
//
// En Windows no hay ni lo uno ni lo otro, y se captura la pantalla entera en
// la que está el ratón con `QScreen::grabWindow`. Elegir una zona allí sería
// dibujar una ventana propia de selección; de momento no está, y una captura
// entera ya sirve de referencia (se recorta luego si hace falta).
//
// Lo capturado entra **movido**: es un archivo hecho para esto, y dejar una
// copia en ~/Imágenes/Capturas por cada referencia sería ensuciar.
#pragma once
#include <QObject>
#include <QString>
#include <QVariantMap>

class Nucleo;

class Captura : public QObject {
    Q_OBJECT
    /// Si hay una captura en marcha: mientras, el botón no hace nada.
    Q_PROPERTY(bool enMarcha READ enMarcha NOTIFY enMarchaCambio)

public:
    Captura(Nucleo *nucleo, QObject *padre = nullptr);

    bool enMarcha() const { return m_enMarcha; }

    /// Empieza. La ventana ya tiene que estar fuera de en medio: eso lo hace
    /// QML, que es quien la tiene.
    Q_INVOKABLE void capturar(const QString &carpeta);

signals:
    void enMarchaCambio();
    /// Ha terminado, con captura o sin ella: es cuando la ventana vuelve.
    void acabada();
    void aviso(const QString &mensaje, bool error);

private:
    void respuestaPortal(uint codigo, const QVariantMap &resultados);
    bool porPortal();
    void porImageMagick();
    /// Solo en Windows: ver arriba.
    void porPantallaEntera();
    /// Dónde dejar la captura: dentro de la biblioteca, para moverla y no copiarla.
    QString rutaNueva() const;
    void terminar(const QString &ruta, const QString &error);
    void ponerEnMarcha(bool si);

    Nucleo *m_nucleo;
    bool m_enMarcha = false;
    QString m_carpeta;
};
