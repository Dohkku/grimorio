// Pruebas de lo que pone un paquete de Linux en su sitio.
//
// Como las del aviso de versión, un fallo aquí se ve tarde: en el ordenador de
// quien acaba de actualizar. Y lo que se prueba es justo el caso que pasó de
// verdad: una 0.1.5 que actualiza y se queda sin lo que la 0.1.6 trae nuevo.

#include "paquete.h"

#include <QFile>
#include <QTemporaryDir>
#include <QTest>

namespace {

void escribir(const QString &ruta, const QByteArray &datos)
{
    QDir().mkpath(QFileInfo(ruta).absolutePath());
    QFile f(ruta);
    QVERIFY(f.open(QIODevice::WriteOnly));
    f.write(datos);
}

QByteArray leer(const QString &ruta)
{
    QFile f(ruta);
    return f.open(QIODevice::ReadOnly) ? f.readAll() : QByteArray();
}

/// Un paquete 0.1.6 descomprimido en `suelto`, con su carpeta de siempre.
QString paquete016(const QString &suelto)
{
    const QString r = suelto + QStringLiteral("/Grimorio-0.1.6-linux-x64");
    escribir(r + "/grimorio", "grimorio 0.1.6");
    escribir(r + "/grim", "grim 0.1.6");
    escribir(r + "/grim-recorte", "recorte 0.1.6");
    escribir(r + "/LICENSE.md", "licencia");
    escribir(r + "/modelos/isnet-general-use-pesos16.onnx", "modelo");
    escribir(r + "/navegador/manifest.json", "{}");
    escribir(r + "/navegador/iconos/16.png", "png");
    QFile::setPermissions(r + "/grim-recorte", QFile::permissions(r + "/grim-recorte") | QFileDevice::ExeOwner);
    return r;
}

/// Lo que deja instalado una 0.1.5 que acaba de actualizarse a la 0.1.6 con
/// su actualizador de entonces: los dos programas nuevos y nada más.
void instalacion015Actualizada(const QString &dir)
{
    escribir(dir + "/grimorio", "grimorio 0.1.6");
    escribir(dir + "/grim", "grim 0.1.6");
    escribir(dir + "/grimorio.anterior", "grimorio 0.1.5");
    escribir(dir + "/LICENSE.md", "licencia de antes");
}

} // namespace

class PruebaPaquete : public QObject {
    Q_OBJECT

private slots:
    void encuentra_la_carpeta_del_programa()
    {
        QTemporaryDir t;
        const QString r = paquete016(t.path());
        QCOMPARE(paquete::raizDe(t.path()), r);
        QTemporaryDir vacio;
        QVERIFY(paquete::raizDe(vacio.path()).isEmpty());
    }

    void sabe_lo_que_le_falta_a_una_015_actualizada()
    {
        QTemporaryDir t;
        instalacion015Actualizada(t.path());
        QCOMPARE(paquete::faltan(QDir(t.path())),
                 QStringList({ "grim-recorte", "modelos/isnet-general-use-pesos16.onnx",
                               "navegador/manifest.json" }));
    }

    void completar_pone_lo_que_falta_y_no_toca_nada_mas()
    {
        QTemporaryDir suelto, inst;
        const QString r = paquete016(suelto.path());
        instalacion015Actualizada(inst.path());

        QString error;
        QStringList puestos;
        QVERIFY2(paquete::instalar(r, QDir(inst.path()), true, &error, &puestos), qPrintable(error));
        QVERIFY(paquete::faltan(QDir(inst.path())).isEmpty());
        QVERIFY(puestos.contains("grim-recorte"));
        QVERIFY(puestos.contains("navegador/iconos/16.png"));
        QVERIFY(!puestos.contains("grimorio"));
        // Lo que había sigue como estaba, también lo que el paquete trae distinto.
        QCOMPARE(leer(inst.path() + "/LICENSE.md"), QByteArray("licencia de antes"));
        QCOMPARE(leer(inst.path() + "/grimorio.anterior"), QByteArray("grimorio 0.1.5"));
        QVERIFY(QFileInfo(inst.path() + "/grim-recorte").isExecutable());
        QVERIFY(!QFileInfo::exists(inst.path() + "/grim-recorte.nuevo"));
    }

    void actualizar_lo_pone_todo_y_guarda_el_programa_de_antes()
    {
        QTemporaryDir suelto, inst;
        const QString r = paquete016(suelto.path());
        escribir(inst.path() + "/grimorio", "grimorio 0.1.5");
        escribir(inst.path() + "/grim", "grim 0.1.5");
        escribir(inst.path() + "/navegador/manifest.json", "viejo");
        escribir(inst.path() + "/navegador/sobra.js", "de una versión anterior");

        QString error;
        QVERIFY2(paquete::instalar(r, QDir(inst.path()), false, &error), qPrintable(error));
        QVERIFY(paquete::faltan(QDir(inst.path())).isEmpty());
        QCOMPARE(leer(inst.path() + "/grimorio"), QByteArray("grimorio 0.1.6"));
        QCOMPARE(leer(inst.path() + "/grimorio.anterior"), QByteArray("grimorio 0.1.5"));
        QCOMPARE(leer(inst.path() + "/navegador/manifest.json"), QByteArray("{}"));
        // Una carpeta se cambia entera: lo que ya no viene no se queda.
        QVERIFY(!QFileInfo::exists(inst.path() + "/navegador/sobra.js"));
        QVERIFY(!QFileInfo::exists(inst.path() + "/navegador.anterior"));
        QVERIFY(!QFileInfo::exists(inst.path() + "/grimorio.nuevo"));
    }

    void si_no_puede_cambiar_algo_deja_la_version_de_antes_entera()
    {
        QTemporaryDir suelto, inst;
        const QString r = paquete016(suelto.path());
        escribir(inst.path() + "/grimorio", "grimorio 0.1.5");
        escribir(inst.path() + "/grim", "grim 0.1.5");
        // Una carpeta de solo lectura donde va el modelo: la copia se puede
        // hacer al lado, pero el cambio de la carpeta no.
        QDir().mkpath(inst.path() + "/modelos");
        QFile::setPermissions(inst.path(), QFileDevice::ReadOwner | QFileDevice::ExeOwner);

        QString error;
        const bool bien = paquete::instalar(r, QDir(inst.path()), false, &error);
        QFile::setPermissions(inst.path(), QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner);
        if (bien) QSKIP("como root los permisos no cuentan");
        QVERIFY(!error.isEmpty());
        QCOMPARE(leer(inst.path() + "/grimorio"), QByteArray("grimorio 0.1.5"));
        QCOMPARE(leer(inst.path() + "/grim"), QByteArray("grim 0.1.5"));
    }
};

QTEST_GUILESS_MAIN(PruebaPaquete)
#include "paquete_test.moc"
