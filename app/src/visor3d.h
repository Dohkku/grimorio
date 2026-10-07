// Un modelo 3D que se puede girar, acercar y mover.
//
// OpenGL a mano y no Qt Quick 3D: Qt Quick 3D va con licencia GPL o comercial,
// y Grimorio se distribuye como binario cerrado (ver docs/DEPENDENCIAS.md).
// Para enseñar una malla sin texturas hacen falta un búfer de vértices y dos
// sombreadores, y eso está en Qt Gui, que es LGPL.
//
// La malla llega ya preparada por el núcleo (`.malla`: centrada, a escala
// unidad, Y hacia arriba) y sin normales: el sombreador las saca de la
// pendiente de la posición, que da caras planas, que es lo que se quiere ver
// en una pieza de impresión.
#pragma once
#include <QColor>
#include <QQuickFramebufferObject>
#include <QVector3D>

class Visor3D : public QQuickFramebufferObject {
    Q_OBJECT
    Q_PROPERTY(QString archivo READ archivo WRITE setArchivo NOTIFY archivoCambio)
    /// Grados alrededor del eje vertical y de inclinación hacia abajo.
    Q_PROPERTY(qreal giro MEMBER m_giro NOTIFY cambio)
    Q_PROPERTY(qreal inclinacion MEMBER m_inclinacion NOTIFY cambio)
    /// 1 encaja la pieza; más es más cerca.
    Q_PROPERTY(qreal acercar MEMBER m_acercar NOTIFY cambio)
    /// Desplazamiento en pantalla, en unidades del modelo.
    Q_PROPERTY(qreal panX MEMBER m_panX NOTIFY cambio)
    Q_PROPERTY(qreal panY MEMBER m_panY NOTIFY cambio)
    Q_PROPERTY(bool alambre MEMBER m_alambre NOTIFY cambio)
    /// Corrección de la pieza en cuartos de vuelta, antes de mirarla: para el
    /// modelo que llega tumbado o boca abajo (Z arriba, ejes de otro
    /// programa). Alrededor de X y luego de Z, de 0 a 3 cada uno.
    Q_PROPERTY(int vueltasX MEMBER m_vueltasX NOTIFY cambio)
    Q_PROPERTY(int vueltasZ MEMBER m_vueltasZ NOTIFY cambio)
    Q_PROPERTY(QColor colorFondo MEMBER m_fondo NOTIFY cambio)
    Q_PROPERTY(QColor colorPieza MEMBER m_pieza NOTIFY cambio)
    Q_PROPERTY(QColor colorAlambre MEMBER m_colorAlambre NOTIFY cambio)
    /// Triángulos cargados; 0 mientras no hay nada.
    Q_PROPERTY(int triangulos READ triangulos NOTIFY cargado)

public:
    explicit Visor3D(QQuickItem *padre = nullptr);
    Renderer *createRenderer() const override;

    QString archivo() const { return m_archivo; }
    void setArchivo(const QString &a);
    int triangulos() const { return m_triangulos; }

    // Lo que el pintor lee al sincronizar, ya en su hilo.
    qreal giro() const { return m_giro; }
    qreal inclinacion() const { return m_inclinacion; }
    qreal acercar() const { return m_acercar; }
    QVector3D pan() const { return {float(m_panX), float(m_panY), 0}; }
    bool alambre() const { return m_alambre; }
    int vueltasX() const { return m_vueltasX; }
    int vueltasZ() const { return m_vueltasZ; }
    QColor fondo() const { return m_fondo; }
    QColor pieza() const { return m_pieza; }
    QColor colorAlambre() const { return m_colorAlambre; }
    /// Lo avisa el pintor cuando sube la malla.
    void anotarCargado(int n);

signals:
    void archivoCambio();
    void cambio();
    void cargado();

private:
    QString m_archivo;
    qreal m_giro = -35, m_inclinacion = 25, m_acercar = 1, m_panX = 0, m_panY = 0;
    bool m_alambre = false;
    int m_vueltasX = 0, m_vueltasZ = 0;
    QColor m_fondo, m_pieza, m_colorAlambre;
    int m_triangulos = 0;
};
