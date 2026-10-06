// Lo que cada uno mueve para sí: anchos de panel, modo seguro, reproducción.
//
// Va aparte del tema a propósito. El tema es dato del proyecto —un JSON que se
// edita, se comparte y se recarga solo— y esto es lo contrario: preferencias de
// quien está delante, que tienen que seguir ahí mañana. Mezclarlos significaría
// que arrastrar un panel reescribe el archivo de estilo de otro.
//
// Los anchos valen cero mientras nadie los toque, y cero quiere decir «el que
// diga el tema». Así el tema sigue mandando por defecto y el ajuste solo pesa
// cuando alguien ha decidido algo.
#pragma once
#include <QObject>
#include <QSettings>

class Ajustes : public QObject {
    Q_OBJECT
    Q_PROPERTY(qreal anchoLateral READ anchoLateral WRITE setAnchoLateral NOTIFY cambio)
    Q_PROPERTY(qreal anchoInspector READ anchoInspector WRITE setAnchoInspector NOTIFY cambio)
    /// Pasar por encima de un vídeo en la malla lo pone en marcha.
    ///
    /// Solo el de debajo del ratón, nunca varios. Moverlos todos a la vez se
    /// probó y se quitó: cada uno es una tubería de decodificación entera, y
    /// cinco costaban 0,88 GB y un tercio de un núcleo contra los 0,19 GB del
    /// programa sin vídeo. Ver app/README.md.
    Q_PROPERTY(bool videoAlPasar READ videoAlPasar WRITE setVideoAlPasar NOTIFY cambio)
    /// Con el modo seguro puesto, lo marcado como +18 sale difuminado.
    Q_PROPERTY(bool modoSeguro READ modoSeguro WRITE setModoSeguro NOTIFY cambio)
    /// El nombre de cada elemento debajo de su celda. Apagado de fábrica:
    /// la malla es de imágenes, y el nombre ya está en el panel.
    Q_PROPERTY(bool verNombres READ verNombres WRITE setVerNombres NOTIFY cambio)
    /// El panel de detalle a la derecha, siempre puesto aunque no haya nada
    /// elegido: así la malla no cambia de ancho cada vez que se elige algo.
    Q_PROPERTY(bool verInspector READ verInspector WRITE setVerInspector NOTIFY cambio)
    /// Cuadrícula, justificado o lista (los números de `Disposicion::Modo`);
    /// -1 mientras nadie lo haya elegido.
    Q_PROPERTY(int vista READ vista WRITE setVista NOTIFY cambio)
    /// El tamaño de todo lo que no es imagen —letra, iconos, márgenes,
    /// paneles—, como un factor sobre lo que diga el tema. 1 es el tema tal
    /// cual. Las miniaturas no: esas ya tienen su tamaño con Ctrl+±.
    Q_PROPERTY(qreal escala READ escala WRITE setEscala NOTIFY cambio)
    /// El tema de los que vienen dentro: "oscuro" o "papel".
    Q_PROPERTY(QString tema READ tema WRITE setTema NOTIFY cambio)

public:
    explicit Ajustes(QObject *padre = nullptr);

    qreal anchoLateral() const { return m_anchoLateral; }
    qreal anchoInspector() const { return m_anchoInspector; }
    bool videoAlPasar() const { return m_videoAlPasar; }
    bool modoSeguro() const { return m_modoSeguro; }
    bool verNombres() const { return m_verNombres; }
    bool verInspector() const { return m_verInspector; }
    int vista() const { return m_vista; }
    qreal escala() const { return m_escala; }
    QString tema() const { return m_tema; }

    void setAnchoLateral(qreal v);
    void setAnchoInspector(qreal v);
    void setVideoAlPasar(bool v);
    void setModoSeguro(bool v);
    void setVerNombres(bool v);
    void setVerInspector(bool v);
    void setVista(int v);
    void setEscala(qreal v);
    void setTema(const QString &v);

    /// Cómo se endereza un modelo 3D concreto: `vueltasX + 4 * vueltasZ`, 0
    /// si nunca se tocó. Es de quien lo mira, como el resto de esto: no cambia
    /// el archivo ni la biblioteca.
    Q_INVOKABLE int orientacion3d(const QString &id) const;
    Q_INVOKABLE void ponerOrientacion3d(const QString &id, int v);

signals:
    void cambio();

private:
    void guardar(const char *clave, const QVariant &valor);

    QSettings m_disco;
    qreal m_anchoLateral = 0;
    qreal m_anchoInspector = 0;
    bool m_videoAlPasar = true;
    bool m_modoSeguro = true;
    bool m_verNombres = false;
    bool m_verInspector = true;
    int m_vista = -1;
    qreal m_escala = 1.0;
    QString m_tema;
};
