// Pruebas del arrastre en el árbol de carpetas.
//
// Esta parte no la caza `qmllint` ni se ve en una captura: el árbol se
// iluminaba al pasar algo por encima y al soltar no pasaba nada, porque la
// carga viajaba en el `Drag.mimeData` y en un arrastre interno de Qt Quick ese
// mapa no llega al otro lado. Y cuando la carga sí llegó, las bandas salían
// corridas media fila, porque el bulto se quedaba atrás el umbral de arrastre.
//
// Las dos cosas se ven solo moviendo el ratón de verdad, así que eso es lo que
// hace esta prueba: carga las `FilaCarpeta` de verdad y arrastra.

#include "tema.h"

#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickView>
#include <QTest>

/// Apunta lo que le piden en vez de tocar la biblioteca.
class NucleoFalso : public QObject {
    Q_OBJECT
public:
    QStringList apuntes;

    Q_INVOKABLE void moverCarpeta(const QString &id, const QString &padre,
                                  const QString &antesDe = QString())
    {
        apuntes << QStringLiteral("moverCarpeta(%1,%2,%3)").arg(id, padre, antesDe);
    }
    Q_INVOKABLE void moverEntreCarpetas(const QStringList &ids, const QString &de,
                                        const QString &a)
    {
        apuntes << QStringLiteral("moverEntreCarpetas(%1,%2,%3)").arg(ids.join(u'+'), de, a);
    }
    Q_INVOKABLE void sacarDeCarpeta(const QStringList &ids, const QString &carpeta)
    {
        apuntes << QStringLiteral("sacarDeCarpeta(%1,%2)").arg(ids.join(u'+'), carpeta);
    }
    /// En la escena no hay jerarquía: ninguna cuelga de ninguna.
    Q_INVOKABLE bool cuelgaDe(const QString &, const QString &) const { return false; }
};

/// Lo que la ventana de verdad guarda mientras dura un arrastre.
class VentanaFalsa : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString arrastreCarpeta MEMBER m_carpeta NOTIFY cambio)
    Q_PROPERTY(QStringList arrastreIds MEMBER m_ids NOTIFY cambio)
    Q_PROPERTY(QString arrastreOrigen MEMBER m_origen NOTIFY cambio)
public:
    Q_INVOKABLE void arrastrarCarpeta(const QString &id)
    {
        m_carpeta = id;
        m_ids.clear();
        m_origen.clear();
        emit cambio();
    }
    Q_INVOKABLE void arrastrarElementos(const QStringList &ids, const QString &origen)
    {
        m_carpeta.clear();
        m_ids = ids;
        m_origen = origen;
        emit cambio();
    }
    Q_INVOKABLE void acabarArrastre()
    {
        m_carpeta.clear();
        m_ids.clear();
        m_origen.clear();
        emit cambio();
    }
    /// En la escena solo hay «a» y «b»; detrás de «a» se inventa una tercera
    /// para que se distinga de soltar al final.
    Q_INVOKABLE QString hermanaSiguiente(const QString &id) const
    {
        return id == QLatin1String("a") ? QStringLiteral("c") : QString();
    }
    /// Dónde va el fantasma que se pega al cursor.
    Q_INVOKABLE void moverArrastre(const QPointF &punto) { ultimoPunto = punto; }
    Q_INVOKABLE void abrirMenuCarpeta(qreal, qreal, const QString &, const QString &) {}

    QPointF ultimoPunto;

signals:
    void cambio();

private:
    QString m_carpeta;
    QStringList m_ids;
    QString m_origen;
};

/// Los avisos de QML no paran nada por su cuenta: una llamada a algo que no
/// existe deja un `TypeError` por la salida de errores y el programa sigue. Aquí
/// se recogen para que tumben la prueba, que es justo lo que no pasó cuando el
/// fantasma del arrastre llamaba a un método que esta ventana falsa no tenía.
static QStringList *avisos = nullptr;

static void recoger(QtMsgType tipo, const QMessageLogContext &, const QString &texto)
{
    if (avisos && (tipo == QtWarningMsg || tipo == QtCriticalMsg || tipo == QtFatalMsg))
        *avisos << texto;
}

class PruebaArrastre : public QObject {
    Q_OBJECT

    QStringList m_avisos;

    Tema m_tema;
    NucleoFalso m_nucleo;
    VentanaFalsa m_ventana;
    QQuickView *m_vista = nullptr;

    /// El alto de una fila sale del tamaño de letra, igual que en el programa.
    int filaAlta() const { return qRound(m_tema.property("fuente").toReal() * 2.6); }

    /// Arrastra de un punto a otro, con paradas por el camino: sin ellas no se
    /// pasa el umbral y `MouseArea` no llega a arrancar el arrastre.
    void arrastrar(int desdeY, int hastaY)
    {
        const int x = 120;
        QTest::mousePress(m_vista, Qt::LeftButton, Qt::NoModifier, QPoint(x, desdeY));
        const int pasos = 8;
        for (int i = 1; i <= pasos; ++i) {
            const int y = desdeY + (hastaY - desdeY) * i / pasos;
            QTest::mouseMove(m_vista, QPoint(x, y));
        }
        QTest::mouseRelease(m_vista, Qt::LeftButton, Qt::NoModifier, QPoint(x, hastaY));
        QTest::qWait(20);
    }

private slots:
    void initTestCase()
    {
        avisos = &m_avisos;
        qInstallMessageHandler(recoger);
        m_vista = new QQuickView;
        m_vista->rootContext()->setContextProperty(QStringLiteral("tema"), &m_tema);
        m_vista->rootContext()->setContextProperty(QStringLiteral("nucleo"), &m_nucleo);
        m_vista->rootContext()->setContextProperty(QStringLiteral("ventana"), &m_ventana);
        m_vista->setSource(QUrl::fromLocalFile(QStringLiteral(RUTA_ESCENA)));
        QCOMPARE(m_vista->status(), QQuickView::Ready);
        m_vista->show();
        QVERIFY(QTest::qWaitForWindowExposed(m_vista));
    }

    void cleanupTestCase()
    {
        qInstallMessageHandler(nullptr);
        avisos = nullptr;
        delete m_vista;
    }

    void init()
    {
        m_nucleo.apuntes.clear();
        m_avisos.clear();
        m_ventana.ultimoPunto = QPointF();
    }

    /// Después de cada caso: ni un aviso de QML.
    void cleanup() { QVERIFY2(m_avisos.isEmpty(), qPrintable(m_avisos.join(u'\n'))); }

    /// Lo primero que estaba roto: la carga no llegaba y no se movía nada.
    void soltar_una_carpeta_encima_de_otra_la_mete_dentro()
    {
        arrastrar(filaAlta() + filaAlta() / 2, filaAlta() / 2);
        QCOMPARE(m_nucleo.apuntes, QStringList{ QStringLiteral("moverCarpeta(b,a,)") });
    }

    /// El borde de arriba la coloca delante, como hermana. Esta y la siguiente
    /// son las que cazan que el bulto se quede atrás: con el arrastre corrido el
    /// umbral, un soltado en el borde cae en la banda de al lado.
    void soltar_en_el_borde_de_arriba_la_coloca_delante()
    {
        arrastrar(filaAlta() + filaAlta() / 2, 3);
        QCOMPARE(m_nucleo.apuntes, QStringList{ QStringLiteral("moverCarpeta(b,,a)") });
    }

    void soltar_en_el_borde_de_abajo_la_coloca_detras()
    {
        arrastrar(filaAlta() + filaAlta() / 2, filaAlta() - 3);
        QCOMPARE(m_nucleo.apuntes, QStringList{ QStringLiteral("moverCarpeta(b,,c)") });
    }

    /// Una carpeta no cabe dentro de sí misma, pero sus dos bordes siguen
    /// valiendo. Antes la fila rechazaba la entrada al verse llegar encima, y
    /// Qt no vuelve a avisar a una zona que ha rechazado: se quedaba sorda el
    /// resto del arrastre y perdía también las bandas.
    void una_carpeta_sobre_si_misma_conserva_sus_bordes()
    {
        // Del borde de abajo al de arriba de la misma fila: por el camino se
        // pasa por su centro, que es donde no cabe.
        arrastrar(filaAlta() * 2 - 4, filaAlta() + 3);
        QCOMPARE(m_nucleo.apuntes, QStringList{ QStringLiteral("moverCarpeta(b,,b)") });
    }

    void soltar_una_carpeta_en_el_centro_de_si_misma_no_hace_nada()
    {
        arrastrar(filaAlta() * 2 - 2, filaAlta() + filaAlta() / 2);
        QVERIFY(m_nucleo.apuntes.isEmpty());
    }

    /// Los elementos no tienen bandas: caigan donde caigan en la fila, entran.
    void soltar_elementos_en_una_carpeta_los_mueve_a_ella()
    {
        arrastrar(filaAlta() * 2 + 10, 3);
        QCOMPARE(m_nucleo.apuntes,
                 QStringList{ QStringLiteral("moverEntreCarpetas(uno+dos,vieja,a)") });
    }

    void soltar_elementos_en_el_centro_de_una_carpeta_los_mueve_igual()
    {
        arrastrar(filaAlta() * 2 + 10, filaAlta() / 2);
        QCOMPARE(m_nucleo.apuntes,
                 QStringList{ QStringLiteral("moverEntreCarpetas(uno+dos,vieja,a)") });
    }

    /// El fantasma que se pega al cursor: la ventana tiene que enterarse de
    /// dónde está, y en el sitio donde está de verdad. Sin esto, arrastrar es
    /// un acto de fe hasta llegar al final del camino.
    void la_ventana_sabe_donde_esta_lo_que_se_arrastra()
    {
        const int hasta = filaAlta() / 2;
        arrastrar(filaAlta() * 2 + 10, hasta);
        QCOMPARE(m_ventana.ultimoPunto.x(), 120.0);
        QVERIFY2(qAbs(m_ventana.ultimoPunto.y() - hasta) < 2.0,
                 qPrintable(QString::number(m_ventana.ultimoPunto.y())));
    }
};

QTEST_MAIN(PruebaArrastre)
#include "arrastre_test.moc"
