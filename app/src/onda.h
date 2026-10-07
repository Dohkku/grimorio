// La onda y el espectro de un sonido, pintados a partir de un `.onda`.
//
// El análisis lo hace el núcleo una vez y lo guarda; aquí solo se lee y se
// pinta, así que acercar la vista o mover el tramo no cuesta más que repintar.
// Los colores llegan del tema como propiedades: ni uno escrito aquí.
#pragma once
#include <QColor>
#include <QImage>
#include <QQuickPaintedItem>
#include <QVector>

class Onda : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(QString archivo READ archivo WRITE setArchivo NOTIFY archivoCambio)
    /// 0: la onda en el tiempo; 1: el espectro (tiempo y frecuencia).
    Q_PROPERTY(int modo MEMBER m_modo NOTIFY pintar)
    /// Tramo visible, de 0 a 1 sobre la duración entera.
    Q_PROPERTY(qreal desde MEMBER m_desde NOTIFY pintar)
    Q_PROPERTY(qreal hasta MEMBER m_hasta NOTIFY pintar)
    Q_PROPERTY(QColor colorFondo MEMBER m_fondo NOTIFY colores)
    Q_PROPERTY(QColor colorOnda MEMBER m_onda NOTIFY colores)
    Q_PROPERTY(QColor colorMedio MEMBER m_medio NOTIFY colores)
    Q_PROPERTY(QColor colorAlto MEMBER m_alto NOTIFY colores)
    Q_PROPERTY(bool listo READ listo NOTIFY archivoCambio)
    Q_PROPERTY(qreal duracionS READ duracionS NOTIFY archivoCambio)
    Q_PROPERTY(qreal fMin READ fMin NOTIFY archivoCambio)
    Q_PROPERTY(qreal fMax READ fMax NOTIFY archivoCambio)

public:
    explicit Onda(QQuickItem *padre = nullptr);

    QString archivo() const { return m_archivo; }
    void setArchivo(const QString &a);
    bool listo() const { return !m_picos.isEmpty(); }
    qreal duracionS() const { return m_tasa ? qreal(m_muestras) / m_tasa : 0; }
    qreal fMin() const { return m_fMin; }
    qreal fMax() const { return m_fMax; }

    /// La frecuencia que hay a una altura del espectro, de 0 (abajo) a 1.
    Q_INVOKABLE qreal frecuenciaEn(qreal alturaRelativa) const;

    void paint(QPainter *p) override;

signals:
    void archivoCambio();
    void pintar();
    void colores();

private:
    void colorear();

    QString m_archivo;
    int m_modo = 0;
    qreal m_desde = 0, m_hasta = 1;
    QColor m_fondo, m_onda, m_medio, m_alto;

    quint32 m_tasa = 0;
    quint64 m_muestras = 0;
    quint32 m_hopPicos = 256;
    QVector<float> m_picos; // mínimo y máximo, alternos
    quint32 m_columnas = 0, m_filas = 0;
    QByteArray m_espectro;
    float m_fMin = 0, m_fMax = 0;
    QImage m_imagen; // el espectro ya coloreado
};
