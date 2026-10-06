// Los tokens de estilo, leídos de un JSON y recargados en caliente.
//
// La regla del proyecto: en la capa visual no hay ni un color ni un número
// mágico escrito a mano. Todo sale de aquí, y el archivo se puede tocar con el
// programa abierto: guardar cambia la ventana sin reiniciar. Eso no es un
// lujo, es lo que hace que alguien pueda hacerse su tema sin compilar nada.
#pragma once
#include <QColor>
#include <QObject>
#include <QString>

class QFileSystemWatcher;

class Tema : public QObject {
    Q_OBJECT
    Q_PROPERTY(QColor fondo MEMBER m_fondo NOTIFY cambio)
    Q_PROPERTY(QColor panel MEMBER m_panel NOTIFY cambio)
    Q_PROPERTY(QColor texto MEMBER m_texto NOTIFY cambio)
    Q_PROPERTY(QColor textoTenue MEMBER m_textoTenue NOTIFY cambio)
    Q_PROPERTY(QColor seleccion MEMBER m_seleccion NOTIFY cambio)
    Q_PROPERTY(QColor marcador MEMBER m_marcador NOTIFY cambio)
    Q_PROPERTY(QColor borde MEMBER m_borde NOTIFY cambio)
    /// Los ejes del visor 3D. Rojo, verde y azul para X, Y y Z es lo que
    /// enseña cualquier programa 3D; se cambian, pero mejor no se inventan.
    Q_PROPERTY(QColor ejeX MEMBER m_ejeX NOTIFY cambio)
    Q_PROPERTY(QColor ejeY MEMBER m_ejeY NOTIFY cambio)
    Q_PROPERTY(QColor ejeZ MEMBER m_ejeZ NOTIFY cambio)
    /// La pieza del visor 3D. No es `texto`: en un tema claro el texto es casi
    /// negro y la pieza salía como una silueta sin volumen. Un tono medio deja
    /// sitio a las luces para subir y bajar.
    Q_PROPERTY(QColor pieza3d MEMBER m_pieza3d NOTIFY cambio)
    Q_PROPERTY(qreal radio READ radio NOTIFY cambio)
    Q_PROPERTY(qreal margen READ margen NOTIFY cambio)
    Q_PROPERTY(qreal hueco READ hueco NOTIFY cambio)
    Q_PROPERTY(qreal celda MEMBER m_celda NOTIFY cambio)
    Q_PROPERTY(qreal realce MEMBER m_realce NOTIFY cambio)
    Q_PROPERTY(qreal realcePx MEMBER m_realcePx NOTIFY cambio)
    Q_PROPERTY(qreal aparicionS MEMBER m_aparicionS NOTIFY cambio)
    Q_PROPERTY(qreal scrollTau MEMBER m_scrollTau NOTIFY cambio)
    Q_PROPERTY(int subidasPorFotograma MEMBER m_subidas NOTIFY cambio)
    Q_PROPERTY(qreal lateral READ lateral NOTIFY cambio)
    Q_PROPERTY(qreal inspector READ inspector NOTIFY cambio)
    Q_PROPERTY(qreal fuente READ fuente NOTIFY cambio)
    Q_PROPERTY(QString archivo READ archivo NOTIFY cambio)
    /// El factor de `Ajustes::escala`. Lo que mide —letra, márgenes, huecos,
    /// esquinas y paneles— sale ya multiplicado, así que el QML no se entera:
    /// sigue pidiendo `tema.fuente` y le llega la de ese tamaño.
    Q_PROPERTY(qreal escala READ escala WRITE setEscala NOTIFY cambio)

public:
    explicit Tema(QObject *padre = nullptr);

    /// Aplica lo que haya en el archivo; lo que falte conserva su valor de
    /// fábrica. Devuelve false solo si no se pudo leer o parsear.
    bool cargar(const QString &ruta, QString *error);
    /// Vuelve a leer el archivo cada vez que cambie en disco.
    void vigilar();

    QString archivo() const { return m_archivo; }
    qreal radio() const { return m_radio * m_escala; }
    qreal celda() const { return m_celda; }
    qreal margen() const { return m_margen * m_escala; }
    qreal hueco() const { return m_hueco * m_escala; }
    qreal lateral() const { return m_lateral * m_escala; }
    qreal inspector() const { return m_inspector * m_escala; }
    qreal fuente() const { return m_fuente * m_escala; }
    qreal escala() const { return m_escala; }
    void setEscala(qreal e);
    QColor fondo() const { return m_fondo; }

signals:
    void cambio();

private:
    void aplicarDefectos();

    QString m_archivo;
    QFileSystemWatcher *m_vigia = nullptr;
    // No es del archivo: aplicarDefectos() no lo toca.
    qreal m_escala = 1.0;

    QColor m_fondo { QStringLiteral("#12100f") };
    QColor m_panel { QStringLiteral("#1a1817") };
    QColor m_texto { QStringLiteral("#e8e3dc") };
    QColor m_textoTenue { QStringLiteral("#8a8179") };
    QColor m_seleccion { QStringLiteral("#63b6bb") };
    QColor m_marcador { QStringLiteral("#282c30") };
    QColor m_borde { QStringLiteral("#2a2624") };
    QColor m_ejeX { QStringLiteral("#e0545c") };
    QColor m_ejeY { QStringLiteral("#86b84a") };
    QColor m_ejeZ { QStringLiteral("#4c8ce0") };
    QColor m_pieza3d { QStringLiteral("#d9d2c7") };
    qreal m_radio = 6.0;
    qreal m_margen = 20.0;
    qreal m_hueco = 10.0;
    qreal m_celda = 200.0;
    qreal m_realce = 0.06;
    qreal m_realcePx = 2.0;
    qreal m_aparicionS = 0.12;
    qreal m_scrollTau = 0.055;
    int m_subidas = 24;
    qreal m_lateral = 240.0;
    qreal m_inspector = 300.0;
    qreal m_fuente = 13.0;
};
