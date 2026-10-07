// La disposición de la malla: dónde va cada celda.
//
// Sustituye a `GridView` por dos razones. La primera es que GridView solo hace
// celdas iguales, y las filas justificadas —cada imagen con su proporción, la
// fila entera al ancho— son la mitad de por qué una galería de referencias se
// mira a gusto. La segunda es que teniendo la disposición en casa hay un solo
// camino para los dos modos en vez de uno nativo y otro a mano.
//
// La virtualización es un anillo de ranuras: se calcula qué celdas se ven, y
// cada una se asigna a la ranura `indice % ranuras`. Como las celdas visibles
// son siempre un tramo contiguo, dos no caen nunca en la misma ranura mientras
// haya ranuras de sobra. El delegado no se crea ni se destruye al desplazarse:
// cambian sus propiedades, que es lo barato.
#pragma once
#include "filas.h"

#include <QAbstractListModel>
#include <QVector>

class Modelo;

class Disposicion : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(Modo modo READ modo WRITE setModo NOTIFY modoCambio)
    Q_PROPERTY(qreal objetivo READ objetivo WRITE setObjetivo NOTIFY geometriaCambio)
    Q_PROPERTY(qreal ancho READ ancho WRITE setAncho NOTIFY geometriaCambio)
    Q_PROPERTY(qreal margen READ margen WRITE setMargen NOTIFY geometriaCambio)
    Q_PROPERTY(qreal hueco READ hueco WRITE setHueco NOTIFY geometriaCambio)
    /// Sitio bajo cada fila para el nombre de sus celdas; 0, sin nombres.
    Q_PROPERTY(qreal pie READ pie WRITE setPie NOTIFY geometriaCambio)
    /// Alto de cada fila en la vista de lista.
    Q_PROPERTY(qreal altoLista READ altoLista WRITE setAltoLista NOTIFY geometriaCambio)
    Q_PROPERTY(qreal alturaTotal READ alturaTotal NOTIFY geometriaCambio)
    Q_PROPERTY(int columnas READ columnas NOTIFY geometriaCambio)
    Q_PROPERTY(int filas READ filas NOTIFY geometriaCambio)
    /// Se está reordenando arrastrando: la malla se pinta como si lo arrastrado
    /// ya estuviera donde caería.
    Q_PROPERTY(bool reordenando READ reordenando NOTIFY reordenCambio)
    /// Las celdas se deslizan a su sitio en vez de saltar. Dura un poco más
    /// que el arrastre, lo que tardan en asentarse.
    Q_PROPERTY(bool animando READ animando NOTIFY reordenCambio)
    /// El elemento que se arrastra (índice del modelo), o -1.
    Q_PROPERTY(int arrastrado READ arrastrado NOTIFY reordenCambio)

public:
    enum Modo { Cuadricula, Justificado, Lista };
    Q_ENUM(Modo)

    /// Los papeles de la celda **y** los del elemento que le toca.
    ///
    /// Están juntos a propósito: si el delegado tuviera que preguntar la
    /// posición a un modelo y el color a otro, las dos respuestas llegarían en
    /// momentos distintos y la celda parpadearía al desplazarse.
    enum Papel {
        Indice = Qt::UserRole + 1,
        X,
        Y,
        Ancho_,
        Alto_,
        Activo,
        Dominante,
        TieneMiniatura,
        Elegido,
        Estrellas,
        IdItem,
        Familia,
        Duracion,
        Adulto,
        SinDibujo,
        Nombre,
    };

    explicit Disposicion(Modelo *modelo, QObject *padre = nullptr);

    int rowCount(const QModelIndex &padre = QModelIndex()) const override;
    QVariant data(const QModelIndex &idx, int papel) const override;
    QHash<int, QByteArray> roleNames() const override;

    Modo modo() const { return m_modo; }
    void setModo(Modo m);
    qreal objetivo() const { return m_objetivo; }
    void setObjetivo(qreal v);
    qreal ancho() const { return m_ancho; }
    void setAncho(qreal v);
    qreal margen() const { return m_margen; }
    void setMargen(qreal v);
    qreal hueco() const { return m_hueco; }
    void setHueco(qreal v);
    qreal pie() const { return m_pie; }
    void setPie(qreal v);
    qreal altoLista() const { return m_altoLista; }
    void setAltoLista(qreal v);
    qreal alturaTotal() const { return m_filas.alturaTotal(); }
    int columnas() const { return m_filas.columnas(); }
    int filas() const { return int(m_filas.filas().size()); }

    /// Qué se ve entre `y0` e `y1`, en coordenadas de contenido. Reparte las
    /// celdas por las ranuras y avisa solo de las que cambian.
    Q_INVOKABLE void mirar(qreal y0, qreal y1);
    /// Coordenada Y donde empieza una celda, para llevar la vista hasta ella.
    Q_INVOKABLE qreal yDe(int indice) const;
    Q_INVOKABLE qreal altoDe(int indice) const;
    /// Dónde cae una celda entera: `{x, y, ancho, alto}`, o vacío si ese
    /// elemento no está en ninguna fila. Vale para cualquier índice, esté o no
    /// dentro de la ventana que se está mirando, porque quien lo pregunta es el
    /// anillo del foco y el foco puede estar en cualquier sitio.
    Q_INVOKABLE QVariantMap sitioDe(int indice) const;
    /// La celda que queda encima o debajo de otra, en la misma columna visual.
    Q_INVOKABLE int vecinoVertical(int indice, int direccion) const;

    // ------------------------------------------------------------ reordenar
    //
    // Mientras se arrastra, la disposición se calcula sobre una permutación:
    // la lista sin lo arrastrado, con lo arrastrado metido en el hueco. Y cada
    // ranura va ligada al **elemento**, no al sitio: así una celda conserva su
    // imagen y solo cambia de x/y, que la vista anima. Ligada al sitio, cada
    // celda cambiaría de imagen de golpe y no se vería nada moverse.
    bool reordenando() const { return m_arrastrado >= 0 && !m_esperando; }
    bool animando() const { return m_animando; }
    int arrastrado() const { return m_arrastrado; }
    Q_INVOKABLE void empezarReorden(int indice);
    /// El cursor está en (x, y) de contenido: el hueco va donde caería.
    Q_INVOKABLE void moverReorden(qreal x, qreal y);
    /// Vuelve lo arrastrado a su sitio (el cursor se fue de la malla).
    Q_INVOKABLE void devolverReorden();
    /// Si al soltar ha cambiado de sitio.
    Q_INVOKABLE bool reordenMovido() const;
    /// Los vecinos donde ha caído: `{antes, despues}`, ids o "".
    Q_INVOKABLE QVariantMap vecinosReorden() const;
    /// Se suelta. Guardado, la vista previa se queda hasta que llegue la vista
    /// nueva del núcleo —que ya viene en ese orden— y así no salta nada; sin
    /// guardar, se deshace al momento.
    Q_INVOKABLE void acabarReorden(bool guardado);

public slots:
    void recalcular();
    /// Algo del elemento cambió (selección, estrellas): las posiciones siguen
    /// valiendo, solo hay que repintar. Son 192 ranuras, no 100.000 filas.
    void refrescarContenido();
    /// Lo mismo, pero solo para las ranuras que sirven a los elementos de los
    /// que se avisa. El modelo avisa por elemento y aquí se sirve por ranura:
    /// sin traducir, un aviso de un elemento repintaba las cuatrocientas.
    void refrescarFilas(const QModelIndex &desde, const QModelIndex &hasta,
                        const QList<int> &papeles);

signals:
    void modoCambio();
    void geometriaCambio();
    void reordenCambio();

private:
    /// El reparto de verdad. `mirar` es esto más recordar la ventana, y
    /// `recalcular` es esto sobre la ventana que ya se estaba mirando.
    void repartir(qreal y0, qreal y1);
    /// Deja todas las ranuras sin elemento, avisando solo de las que lo tenían.
    void vaciar();

    struct Ranura {
        int indice = -1;
        qreal x = 0, y = 0, w = 0, h = 0;
    };

    void asegurarRanuras(int hacenFalta);
    void colocarFila(const Fila &f, QVector<Ranura> &destino) const;
    Filas::Proporcion proporcion() const;
    /// Qué elemento va en la posición `p` (la identidad si no se reordena).
    int enPosicion(int p) const;
    void soltarReorden();

    Modelo *m_modelo = nullptr;
    Modo m_modo = Justificado;
    qreal m_objetivo = 200;
    qreal m_ancho = 0;
    qreal m_margen = 20;
    qreal m_hueco = 10;
    qreal m_pie = 0;
    qreal m_altoLista = 40;
    Filas m_filas;
    QVector<Ranura> m_ranuras;
    /// La última ventana que se pidió mirar. Sin recordarla, rehacer la
    /// disposición tenía que vaciar las ranuras y esperar a que la vista
    /// volviera a pedir: eso hacía parpadear la malla entera.
    qreal m_y0 = 0;
    qreal m_y1 = 0;

    int m_arrastrado = -1;
    int m_origen = -1;
    int m_destino = -1;
    bool m_esperando = false;
    bool m_animando = false;
};
