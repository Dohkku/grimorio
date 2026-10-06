// Pruebas de la geometría de la malla.
//
// Es la parte de la interfaz donde un error no da un fallo ruidoso sino una
// malla ligeramente torcida, y esos son los que se quedan meses.

#include "filas.h"

#include <QElapsedTimer>
#include <QTest>

class PruebaFilas : public QObject {
    Q_OBJECT

    /// Alterna apaisadas y verticales, como una biblioteca de verdad.
    static Filas::Proporcion mezcla()
    {
        return [](int i) { return i % 3 == 0 ? 16.0 / 9.0 : 2.0 / 3.0; };
    }
    static Filas::Proporcion cuadradas()
    {
        return [](int) { return 1.0; };
    }

private slots:
    void la_cuadricula_llena_el_ancho()
    {
        Filas f;
        f.calcular(Filas::Cuadricula, 100, 1000, 200, 20, 10, cuadradas());
        QCOMPARE(f.columnas(), 4);
        const qreal lado = f.filas().first().alto;
        // Cuatro celdas y tres huecos tienen que sumar exactamente el ancho útil.
        QVERIFY(qAbs(lado * 4 + 10 * 3 - (1000 - 40)) < 0.01);
        QCOMPARE(f.filas().size(), 25);
    }

    void justificado_respeta_la_proporcion_de_cada_imagen()
    {
        Filas f;
        auto prop = mezcla();
        f.calcular(Filas::Justificado, 200, 1400, 220, 20, 10, prop);
        for (const Fila &fila : f.filas()) {
            for (int k = 0; k < fila.n; ++k) {
                const int i = fila.inicio + k;
                const qreal w = f.anchoDe(i, prop);
                QVERIFY2(qAbs(w / fila.alto - prop(i)) < 0.02,
                         qPrintable(QStringLiteral("celda %1").arg(i)));
            }
        }
    }

    void cada_fila_justificada_llega_al_borde()
    {
        Filas f;
        auto prop = mezcla();
        f.calcular(Filas::Justificado, 500, 1400, 220, 20, 10, prop);
        // Todas menos la última, que se deja corta a propósito en vez de
        // estirarla: estirarla es lo que hace que las galerías se vean rotas.
        for (int r = 0; r < f.filas().size() - 1; ++r) {
            const Fila &fila = f.filas().at(r);
            const int ultimo = fila.inicio + fila.n - 1;
            const qreal derecha = f.xDe(ultimo, prop) + f.anchoDe(ultimo, prop);
            QVERIFY2(qAbs(derecha - (1400 - 20)) < 0.5,
                     qPrintable(QStringLiteral("fila %1 acaba en %2").arg(r).arg(derecha)));
        }
    }

    void ningun_elemento_se_pierde_ni_se_repite()
    {
        for (auto modo : { Filas::Cuadricula, Filas::Justificado }) {
            Filas f;
            f.calcular(modo, 777, 1300, 180, 24, 8, mezcla());
            int esperado = 0;
            for (const Fila &fila : f.filas()) {
                QCOMPARE(fila.inicio, esperado);
                QVERIFY(fila.n > 0);
                esperado += fila.n;
            }
            QCOMPARE(esperado, 777);
        }
    }

    void saber_que_se_ve_no_depende_del_tamano_de_la_biblioteca()
    {
        Filas f;
        f.calcular(Filas::Cuadricula, 100000, 1600, 160, 20, 10, cuadradas());
        const int desde = f.filaEn(500000);
        QVERIFY(desde > 0 && desde < f.filas().size());
        const Fila &fila = f.filas().at(desde);
        QVERIFY(fila.y + fila.alto >= 500000);
        if (desde > 0) {
            const Fila &anterior = f.filas().at(desde - 1);
            QVERIFY(anterior.y + anterior.alto < 500000);
        }
    }

    void bajar_por_una_galeria_justificada_no_da_saltos_laterales()
    {
        Filas f;
        auto prop = mezcla();
        f.calcular(Filas::Justificado, 300, 1400, 200, 20, 10, prop);
        for (int i = 0; i < 60; ++i) {
            const int abajo = f.vecinoVertical(i, 1, prop);
            if (abajo == i) continue; // última fila
            QVERIFY2(f.filaDe(abajo) == f.filaDe(i) + 1, "tiene que bajar una sola fila");
            const qreal centroI = f.xDe(i, prop) + f.anchoDe(i, prop) / 2;
            const qreal centroAbajo = f.xDe(abajo, prop) + f.anchoDe(abajo, prop) / 2;
            // No puede irse más de una celda ancha de donde estaba.
            QVERIFY2(qAbs(centroI - centroAbajo) < 400,
                     qPrintable(QStringLiteral("de %1 a %2").arg(centroI).arg(centroAbajo)));
        }
    }

    void subir_desde_la_primera_fila_se_queda_donde_esta()
    {
        Filas f;
        auto prop = cuadradas();
        f.calcular(Filas::Cuadricula, 50, 1000, 200, 20, 10, prop);
        QCOMPARE(f.vecinoVertical(0, -1, prop), 0);
        QCOMPARE(f.vecinoVertical(49, 1, prop), 49);
        // Un índice imposible no debe tumbar nada.
        QCOMPARE(f.vecinoVertical(9999, 1, prop), 9999);
        QCOMPARE(f.filaDe(-1), -1);
    }

    void rehacer_la_disposicion_de_cien_mil_cabe_en_un_fotograma()
    {
        // No es un capricho: la disposición se rehace en cada píxel al
        // redimensionar la ventana y al abrir el panel de detalle. Si esto se
        // sale, arrastrar el borde de la ventana se vuelve pegajoso.
        auto prop = mezcla();
        Filas f;
        QElapsedTimer reloj;
        qint64 peor = 0;
        for (int intento = 0; intento < 5; ++intento) {
            reloj.start();
            f.calcular(Filas::Justificado, 100000, 1600 + intento, 200, 20, 10, prop);
            peor = qMax(peor, reloj.nsecsElapsed());
        }
        const qreal ms = peor / 1e6;
        QVERIFY2(ms < 16.0, qPrintable(QStringLiteral("tardó %1 ms").arg(ms)));
        QVERIFY(f.filas().size() > 1000);
    }

    void en_lista_cada_elemento_es_una_fila_a_todo_el_ancho()
    {
        Filas f;
        f.calcular(Filas::Lista, 50, 1000, 44, 20, 0, mezcla());
        QCOMPARE(f.filas().size(), 50);
        QCOMPARE(f.columnas(), 1);
        // La proporción no cuenta: la fila es del ancho útil, sea la foto como sea.
        QCOMPARE(f.anchoDe(0, mezcla()), 960.0);
        QCOMPARE(f.anchoDe(1, mezcla()), 960.0);
        QCOMPARE(f.yDe(3), 20.0 + 3 * 44.0);
        // Bajar es pasar a la siguiente, sin buscar columnas.
        QCOMPARE(f.vecinoVertical(7, 1, mezcla()), 8);
        QCOMPARE(f.vecinoVertical(7, -1, mezcla()), 6);
    }

    void el_pie_de_cada_fila_aparta_a_la_de_debajo()
    {
        Filas sin, con;
        sin.calcular(Filas::Cuadricula, 40, 1000, 200, 20, 10, cuadradas());
        con.calcular(Filas::Cuadricula, 40, 1000, 200, 20, 10, cuadradas(), 24);
        QCOMPARE(con.filas().size(), sin.filas().size());
        // La celda no encoge por el nombre: el sitio sale de entre las filas.
        QCOMPARE(con.filas().at(0).alto, sin.filas().at(0).alto);
        QCOMPARE(con.filas().at(1).y - sin.filas().at(1).y, 24.0);
        QCOMPARE(con.alturaTotal() - sin.alturaTotal(), 24.0 * con.filas().size());
    }

    void una_biblioteca_vacia_no_ocupa_alto()
    {
        Filas f;
        f.calcular(Filas::Justificado, 0, 1000, 200, 20, 10, cuadradas());
        QVERIFY(f.vacio());
        QCOMPARE(f.filaEn(0), 0);
        QCOMPARE(f.filaDe(0), -1);
    }

    void una_ventana_absurdamente_estrecha_no_rompe_nada()
    {
        Filas f;
        // Más estrecha que el margen: el ancho útil tiene un suelo justo para
        // que no salgan celdas de ancho negativo.
        f.calcular(Filas::Cuadricula, 10, 10, 200, 20, 10, cuadradas());
        QVERIFY(f.columnas() >= 1);
        for (const Fila &fila : f.filas()) QVERIFY(fila.alto > 0);
    }

    void un_panorama_no_aplasta_a_sus_vecinas()
    {
        Filas f;
        // 10:1 se recorta a 3.2; si no, la fila entera se quedaría en un hilo.
        auto prop = [](int i) { return i == 5 ? 10.0 : 1.0; };
        f.calcular(Filas::Justificado, 40, 1200, 200, 20, 10, prop);
        for (const Fila &fila : f.filas()) QVERIFY(fila.alto > 60);
        QVERIFY(f.anchoDe(5, prop) <= f.altoDe(5) * Filas::PROP_MAX + 0.01);
    }
};

QTEST_MAIN(PruebaFilas)
#include "filas_test.moc"
