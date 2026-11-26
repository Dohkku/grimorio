// Pruebas de guardar una página entera desde su dirección (src/pagina.h), sin
// red: lo que se baja se da hecho.

#include "pagina.h"

#include <QTest>

using namespace pagina;

class PruebaPagina : public QObject {
    Q_OBJECT

    const QUrl base { QStringLiteral("https://ejemplo.org/blog/entrada.html") };

private slots:
    void quita_scripts_y_manejadores()
    {
        const QString html = QStringLiteral(
            "<html><head><script src=\"a.js\"></script><script>if (a < b) document.write('<p>x</p>')</script>"
            "</head><body onload=\"x()\"><a href=\"javascript:void(0)\" onclick='y()'>a</a></body></html>");
        const QString r = montar(html, base, {}, QStringLiteral("https://ejemplo.org"), QStringLiteral("2026"), {});
        QVERIFY(!r.contains(QLatin1String("<script")));
        QVERIFY(!r.contains(QLatin1String("document.write")));
        QVERIFY(!r.contains(QLatin1String("onload")));
        QVERIFY(!r.contains(QLatin1String("onclick")));
        QVERIFY(!r.contains(QLatin1String("javascript:")));
        QVERIFY(r.contains(QLatin1String("<a>a</a>")));
    }

    void recoge_hojas_imagenes_y_portada()
    {
        const QString html = QStringLiteral(
            "<head><title>Mi &amp; entrada</title>"
            "<meta property=\"og:image\" content=\"/portada.jpg\">"
            "<link rel=\"stylesheet\" href=\"../estilo.css\"><link rel=\"preload\" href=\"x.js\">"
            "<style>@import 'otra.css'; body{background:url(fondo.png)}</style></head>"
            "<body><img src=\"foto.jpg\"><div style=\"background-image:url('/b.webp')\"></div></body>");
        const Recursos r = recoger(html, base);
        QCOMPARE(r.titulo, QStringLiteral("Mi & entrada"));
        QCOMPARE(r.portada, QStringLiteral("https://ejemplo.org/portada.jpg"));
        QCOMPARE(r.hojas, QStringList { QStringLiteral("https://ejemplo.org/estilo.css") });
        QVERIFY(r.datos.contains(QStringLiteral("https://ejemplo.org/blog/foto.jpg")));
        QCOMPARE(r.cssDentro.size(), 2);
        QCOMPARE(importsDeCss(r.cssDentro[0], base), QStringList { QStringLiteral("https://ejemplo.org/blog/otra.css") });
        QCOMPARE(urlsDeCss(r.cssDentro[0], base), QStringList { QStringLiteral("https://ejemplo.org/blog/fondo.png") });
        QCOMPARE(urlsDeCss(r.cssDentro[1], base), QStringList { QStringLiteral("https://ejemplo.org/b.webp") });
    }

    void monta_todo_dentro()
    {
        const QString html = QStringLiteral(
            "<!doctype html><html><head><meta charset=\"iso-8859-1\">"
            "<link rel=\"stylesheet\" media=\"screen\" href=\"/e.css\"></head>"
            "<body><img src=\"f.jpg\" srcset=\"f-400.jpg 400w, f-1200.jpg 1200w, f-4000.jpg 4000w\" loading=\"lazy\">"
            "<a href=\"/otra\">otra</a></body></html>");
        Bajado b;
        b.textos.insert(QStringLiteral("https://ejemplo.org/e.css"),
                        QStringLiteral("@import url(\"sub.css\") print; h1{background:url(t.png)}"));
        b.textos.insert(QStringLiteral("https://ejemplo.org/sub.css"), QStringLiteral("p{color:red}"));
        b.datos.insert(QStringLiteral("https://ejemplo.org/t.png"), QStringLiteral("data:image/png;base64,AAA"));
        b.datos.insert(QStringLiteral("https://ejemplo.org/blog/f-1200.jpg"), QStringLiteral("data:image/jpeg;base64,BBB"));
        const QString r = montar(html, base, b, QStringLiteral("https://ejemplo.org/blog/entrada.html"),
                                 QStringLiteral("2026-11-26T10:00:00Z"), QStringLiteral("data:image/jpeg;base64,CCC"));
        QVERIFY(r.startsWith(QLatin1String("<!doctype html><!-- Guardada con Grimorio desde https://ejemplo.org/blog/entrada.html")));
        QVERIFY(r.contains(QLatin1String("<meta name=\"grimorio:captura\" content=\"data:image/jpeg;base64,CCC\">")));
        QVERIFY(r.contains(QLatin1String("<meta charset=\"utf-8\">")));
        QVERIFY(!r.contains(QLatin1String("iso-8859-1")));
        QVERIFY(r.contains(QLatin1String("<style media=\"screen\">")));
        QVERIFY(r.contains(QLatin1String("@media print {\np{color:red}\n}")));
        QVERIFY(r.contains(QLatin1String("url(\"data:image/png;base64,AAA\")")));
        QVERIFY(r.contains(QLatin1String("<img src=\"data:image/jpeg;base64,BBB\">")));
        QVERIFY(r.contains(QLatin1String("<a href=\"https://ejemplo.org/otra\">")));
        QVERIFY(!r.contains(QLatin1String("<link")));
        // La captura, lo primero del <head>, que es donde la busca el núcleo.
        QVERIFY(r.indexOf(QLatin1String("grimorio:captura")) < r.indexOf(QLatin1String("<style")));
    }

    void lo_no_bajado_queda_apuntando_a_la_web()
    {
        const QString html = QStringLiteral("<link rel=stylesheet href=e.css><img src=\"x.png\">");
        const QString r = montar(html, base, {}, QStringLiteral("o"), QStringLiteral("f"), {});
        QVERIFY(r.contains(QLatin1String("href=\"https://ejemplo.org/blog/e.css\"")));
        QVERIFY(r.contains(QLatin1String("src=\"https://ejemplo.org/blog/x.png\"")));
    }

    void rescata_las_perezosas()
    {
        const QString html = QStringLiteral(
            "<img src=\"data:image/gif;base64,R0lGOD\" data-src=\"/de-verdad.jpg\" class=\"lazy\">"
            "<noscript><img src=\"/sin-scripts.jpg\"></noscript>"
            "<noscript>Activa JavaScript</noscript>");
        const Recursos r = recoger(html, base);
        QVERIFY(r.datos.contains(QStringLiteral("https://ejemplo.org/de-verdad.jpg")));
        QVERIFY(r.datos.contains(QStringLiteral("https://ejemplo.org/sin-scripts.jpg")));
        const QString m = montar(html, base, {}, QStringLiteral("o"), QStringLiteral("f"), {});
        QVERIFY(!m.contains(QLatin1String("noscript")));
        QVERIFY(!m.contains(QLatin1String("Activa JavaScript")));
        QVERIFY(!m.contains(QLatin1String("data-src")));
    }

    void base_y_picture()
    {
        const QString html = QStringLiteral(
            "<base href=\"https://cdn.ejemplo.org/x/\"><picture><source srcset=\"a.avif\" type=\"image/avif\">"
            "<img src=\"a.jpg\"></picture>");
        const QUrl b = baseDe(html, base);
        QCOMPARE(b, QUrl(QStringLiteral("https://cdn.ejemplo.org/x/")));
        const QString m = montar(html, b, {}, QStringLiteral("o"), QStringLiteral("f"), {});
        QVERIFY(!m.contains(QLatin1String("<base")));
        QVERIFY(!m.contains(QLatin1String("<source")));
        QVERIFY(m.contains(QLatin1String("src=\"https://cdn.ejemplo.org/x/a.jpg\"")));
    }

    void redireccion_y_doctype_sin_head()
    {
        const QString html = QStringLiteral("<!doctype html>\n<meta http-equiv=\"refresh\" content=\"0; url=/nueva/\">"
                                            "<title>Redirect</title>");
        QCOMPARE(redireccion(html, base), QUrl(QStringLiteral("https://ejemplo.org/nueva/")));
        QCOMPARE(redireccion(QStringLiteral("<meta http-equiv=refresh content=\"60; url=/x\">"), base), QUrl());
        const QString m = montar(html, base, {}, QStringLiteral("o"), QStringLiteral("f"), {});
        QVERIFY(m.startsWith(QLatin1String("<!doctype html>\n<!-- Guardada")));
        QVERIFY(m.indexOf(QLatin1String("<head>")) > m.indexOf(QLatin1String("<!doctype")));
    }

    void srcset_elige_la_buena()
    {
        QCOMPARE(mayorDeSrcset(QStringLiteral("a.jpg 400w, b.jpg 1600w, c.jpg 3000w")), QStringLiteral("b.jpg"));
        QCOMPARE(mayorDeSrcset(QStringLiteral("a.jpg 3000w, b.jpg 5000w")), QStringLiteral("a.jpg"));
        QCOMPARE(mayorDeSrcset(QStringLiteral("a.jpg, b.jpg 2x")), QStringLiteral("b.jpg"));
        QCOMPARE(mayorDeSrcset(QString()), QString());
    }

    void codificaciones()
    {
        const QByteArray latin = "<p>espa\xf1ol</p>";
        QCOMPARE(decodificar(latin, QStringLiteral("text/html; charset=ISO-8859-1")), QStringLiteral("<p>español</p>"));
        QCOMPARE(decodificar("<meta charset=\"windows-1252\"><p>a\xf1</p>", {}),
                 QStringLiteral("<meta charset=\"windows-1252\"><p>añ</p>"));
        QCOMPARE(decodificar("<p>a\xc3\xb1</p>", QStringLiteral("text/html")), QStringLiteral("<p>añ</p>"));
    }

    void atributos_raros_no_rompen()
    {
        // Comillas con `>` dentro, sin comillas, y una etiqueta sin cerrar.
        const QString html = QStringLiteral("<div title=\"a > b\" data-x=1><img alt='x>y' src=i.png><p");
        const QString m = montar(html, base, {}, QStringLiteral("o"), QStringLiteral("f"), {});
        QVERIFY(m.contains(QLatin1String("<div title=\"a > b\" data-x=1>")));
        QVERIFY(m.contains(QLatin1String("alt='x>y'")));
        QVERIFY(m.endsWith(QLatin1String("<p")));
    }
};

QTEST_GUILESS_MAIN(PruebaPagina)
#include "pagina_test.moc"
