// Pruebas de lo que decide si sale el aviso de versión nueva.
//
// Un fallo aquí no se ve hasta que se publica una versión, y entonces ya está
// en el ordenador de todo el mundo: o el aviso no sale nunca, o sale siempre.

#include "version.h"

#include <QTest>

class PruebaVersion : public QObject {
    Q_OBJECT

private slots:
    void compara_numero_a_numero()
    {
        QVERIFY(version::comparar("0.1.0", "0.1.1") < 0);
        QVERIFY(version::comparar("0.2.0", "0.1.9") > 0);
        QVERIFY(version::comparar("1.0.0", "0.99.99") > 0);
        QCOMPARE(version::comparar("0.1.0", "0.1.0"), 0);
    }

    void diez_va_detras_de_nueve()
    {
        // Comparadas como texto, «0.10.0» saldría más vieja que «0.9.0».
        QVERIFY(version::comparar("0.10.0", "0.9.0") > 0);
    }

    void la_v_y_los_sufijos_no_cuentan()
    {
        QCOMPARE(version::comparar("v0.2.0", "0.2.0"), 0);
        QCOMPARE(version::comparar("0.2.0-rc1", "0.2.0"), 0);
    }

    void lo_que_no_es_version_nunca_es_nueva()
    {
        QVERIFY(version::comparar("", "0.1.0") < 0);
        QVERIFY(version::comparar("hola", "0.1.0") < 0);
        QVERIFY(version::comparar("0.2", "0.1.0") < 0);
    }

    void lee_el_archivo_de_la_web()
    {
        const auto p = version::leer(
            "{\"version\": \"0.2.0\", \"descargas\": \"https://grimorio.frederickandrade.com/#descargar\"}");
        QCOMPARE(p.version, QStringLiteral("0.2.0"));
        QCOMPARE(p.enlace, QUrl("https://grimorio.frederickandrade.com/#descargar"));
    }

    void sin_enlace_manda_a_la_portada()
    {
        const auto p = version::leer("{\"version\": \"0.2.0\"}");
        QCOMPARE(p.enlace, QUrl(version::PAGINA));
    }

    void un_enlace_que_no_es_https_no_se_abre()
    {
        for (const char *malo : { "http://ejemplo.com/", "file:///etc/passwd", "javascript:alert(1)", "https://" }) {
            const auto p = version::leer(QByteArray("{\"version\":\"0.2.0\",\"descargas\":\"") + malo + "\"}");
            QCOMPARE(p.enlace, QUrl(version::PAGINA));
        }
    }

    void un_archivo_roto_no_dice_nada()
    {
        QVERIFY(version::leer("").version.isEmpty());
        QVERIFY(version::leer("<html>404</html>").version.isEmpty());
        QVERIFY(version::leer("{\"version\": 2}").version.isEmpty());
        QVERIFY(version::leer("{\"version\": \"0.0.0\"}").version.isEmpty());
    }
};

QTEST_MAIN(PruebaVersion)
#include "version_test.moc"
