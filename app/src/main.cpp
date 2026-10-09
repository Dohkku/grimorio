//! Grimorio.
//!
//!   grimorio RUTA.grimorio              abre una biblioteca
//!   grimorio RUTA --bench 12            recorrido automático y métricas
//!   grimorio RUTA --captura a.png       una imagen y salir
//!
//! El modo banco no es un resto del spike: se queda dentro a propósito, para
//! poder medir el programa de verdad y no una maqueta suya.

#include "ajustes.h"
#include "captura.h"
#include "banco.h"
#include "bibliotecas.h"
#include "derivados.h"
#include "disposicion.h"
#include "exportar.h"
#include "fotogramas.h"
#include "modelo.h"
#include "novedades.h"
#include "nucleo.h"
#include "onda.h"
#include "pixeles.h"
#include "portapapeles.h"
#include "proveedor.h"
#include "tema.h"
#include "traer.h"
#include "vigilancia.h"
#include "visor3d.h"
#include "grimorio.h"

#include <QDir>
#include <QFileInfo>
#include <QLockFile>
#include <QSettings>
#include <QStandardPaths>

#include <QGuiApplication>
#include <QIcon>

// En Windows la aplicación es QApplication y no QGuiApplication: los diálogos
// de abrir archivos y carpetas son QFileDialog (ver `portal.cpp`), que es de
// QtWidgets, y QtWidgets la necesita. Para la ventana de QML no cambia nada.
#ifdef Q_OS_WIN
#include <QApplication>
#include <QMessageBox>
using Aplicacion = QApplication;
#else
using Aplicacion = QGuiApplication;
#endif
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlEngine>
#include <QSGRendererInterface>
#include <QQuickWindow>
#include <QSurfaceFormat>
#include <QTextStream>
#include <QThreadPool>
#include <QTimer>

namespace {

struct Args {
    QString lib;
    QString tema;
    QString captura;
    double bench = -1;
    int ancho = 1600;
    int alto = 1000;
    double celda = -1;
    int modo = -1; // -1 = el del tema
    int visor = 0;
    int lote = 0;
    QString guion;
    QString filtro;
    bool vsync = true;
    bool ver = false;
};

const char *AYUDA =
    "grimorio — biblioteca visual de referencias\n\n"
    "  grimorio [RUTA.grimorio] [opciones]\n"
    "  (sin ruta: la última que se abrió, o una nueva en Documentos)\n\n"
    "  --tema ARCHIVO      tokens de estilo en JSON (se recarga al guardar)\n"
    "  --celda N           lado objetivo de la celda en píxeles\n"
    "  --cuadricula        celdas cuadradas recortadas\n"
    "  --justificado       filas justificadas conservando proporción\n"
    "  --ancho/--alto N    tamaño de la ventana (por defecto 1600x1000)\n"
    "  --bench SEGUNDOS    recorrido automático y estadísticas de fotograma\n"
    "  --captura ARCHIVO   guarda un fotograma en PNG y sale\n"
    "  --visor N           abre el visor N veces y mide cuánto tarda en verse\n"
    "  --lote N            etiqueta N elementos mientras la galería se desplaza\n"
    "  --guion PREFIJO     recorre la interfaz sola y captura cada paso\n"
    "  --filtro TEXTO      arranca con el buscador puesto\n"
    "  --ver               abre el visor con el primer elemento (con --filtro\n"
    "                      y --captura, para revisar un visor sin ratón)\n"
    "  --sin-vsync         no esperar al monitor: mide el techo real\n";

bool parsear(const QStringList &argv, Args *a, QString *error)
{
    a->lib = qEnvironmentVariable("GRIMORIO_LIB");
    for (int i = 1; i < argv.size(); ++i) {
        const QString &s = argv.at(i);
        const auto valor = [&](double *destino) {
            if (i + 1 >= argv.size()) return false;
            bool ok = false;
            const double v = argv.at(++i).toDouble(&ok);
            if (ok) *destino = v;
            return ok;
        };
        const auto texto = [&](QString *destino) {
            if (i + 1 >= argv.size()) return false;
            *destino = argv.at(++i);
            return true;
        };
        if (s == QLatin1String("--lib") || s == QLatin1String("-L")) {
            if (!texto(&a->lib)) { *error = QStringLiteral("--lib sin ruta"); return false; }
        } else if (s == QLatin1String("--tema")) {
            if (!texto(&a->tema)) { *error = QStringLiteral("--tema sin archivo"); return false; }
        } else if (s == QLatin1String("--captura")) {
            if (!texto(&a->captura)) { *error = QStringLiteral("--captura sin archivo"); return false; }
        } else if (s == QLatin1String("--bench")) {
            if (!valor(&a->bench)) { *error = QStringLiteral("segundos inválidos"); return false; }
        } else if (s == QLatin1String("--celda")) {
            if (!valor(&a->celda)) { *error = QStringLiteral("celda inválida"); return false; }
        } else if (s == QLatin1String("--ancho")) {
            double v = 0;
            if (!valor(&v)) { *error = QStringLiteral("ancho inválido"); return false; }
            a->ancho = int(v);
        } else if (s == QLatin1String("--alto")) {
            double v = 0;
            if (!valor(&v)) { *error = QStringLiteral("alto inválido"); return false; }
            a->alto = int(v);
        } else if (s == QLatin1String("--filtro")) {
            if (!texto(&a->filtro)) { *error = QStringLiteral("--filtro sin texto"); return false; }
        } else if (s == QLatin1String("--guion")) {
            if (!texto(&a->guion)) { *error = QStringLiteral("--guion sin prefijo"); return false; }
        } else if (s == QLatin1String("--lote")) {
            double v = 0;
            if (!valor(&v)) { *error = QStringLiteral("cuántos elementos"); return false; }
            a->lote = int(v);
        } else if (s == QLatin1String("--visor")) {
            double v = 0;
            if (!valor(&v)) { *error = QStringLiteral("cuántas aperturas"); return false; }
            a->visor = int(v);
        } else if (s == QLatin1String("--cuadricula")) {
            a->modo = 0;
        } else if (s == QLatin1String("--justificado")) {
            a->modo = 1;
        } else if (s == QLatin1String("--ver")) {
            a->ver = true;
        } else if (s == QLatin1String("--sin-vsync")) {
            a->vsync = false;
        } else if (s == QLatin1String("-h") || s == QLatin1String("--help")
                   || s == QLatin1String("--ayuda")) {
            QTextStream(stdout) << AYUDA;
            ::exit(0);
        } else if (!s.startsWith(QLatin1Char('-'))) {
            // La forma natural: `grimorio ~/Referencias.grimorio`.
            a->lib = s;
        } else {
            *error = QStringLiteral("opción desconocida: %1").arg(s);
            return false;
        }
    }
    return true;
}

/// Un error de arranque. Por la salida de error, como siempre; y en Windows
/// además en un cuadro, porque allí el programa no tiene consola y el mensaje
/// se perdería: se haría doble clic en el icono y no pasaría nada.
void avisarDeError(const QString &mensaje)
{
    QTextStream(stderr) << "error: " << mensaje << "\n";
#ifdef Q_OS_WIN
    QMessageBox::critical(nullptr, QStringLiteral("Grimorio"), mensaje);
#endif
}

/// La biblioteca cuando no se dice cuál, que es lo que pasa al abrir Grimorio
/// desde el menú de inicio o el lanzador: la última que se abrió (la lista de
/// recientes de `Bibliotecas`) y, si no hay ninguna, una nueva en Documentos.
/// Sin esto, arrancar sin argumentos solo sabía decir «falta la ruta» por una
/// consola que fuera de la terminal nadie ve.
bool esBiblioteca(const QString &ruta)
{
    const QByteArray b = ruta.toUtf8();
    return grim_es_biblioteca(reinterpret_cast<const uint8_t *>(b.constData()), uint32_t(b.size())) != 0;
}

/// La biblioteca que contiene `ruta`, subiendo de carpeta en carpeta: así se
/// puede soltar sobre el programa un archivo de dentro (`items/…/original.jpg`)
/// y abre la suya. Vacío si no hay ninguna por encima.
QString bibliotecaQueContiene(const QString &ruta)
{
    QFileInfo info(ruta);
    QDir d = info.isDir() ? QDir(info.absoluteFilePath()) : info.absoluteDir();
    if (!d.exists()) return {};
    do {
        if (esBiblioteca(d.absolutePath())) return d.absolutePath();
    } while (d.cdUp());
    return {};
}

QString bibliotecaPorDefecto(QString *error, const QString &menos = QString())
{
    const QStringList recientes = QSettings().value(QStringLiteral("bibliotecas/recientes")).toStringList();
    for (const QString &r : recientes) {
        if (r != menos && esBiblioteca(r)) return r;
    }
    const QString docs = QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation);
    if (docs.isEmpty()) {
        *error = QStringLiteral("falta la ruta de la biblioteca (o la variable GRIMORIO_LIB)");
        return {};
    }
    const QString nueva = QDir(docs).filePath(QStringLiteral("Grimorio.grimorio"));
    const QByteArray b = nueva.toUtf8();
    const auto *p = reinterpret_cast<const uint8_t *>(b.constData());
    if (!esBiblioteca(nueva) && !grim_crear_biblioteca(p, uint32_t(b.size()))) {
        *error = QStringLiteral("no pude crear una biblioteca en «%1»").arg(nueva);
        return {};
    }
    return nueva;
}

} // namespace

/// Lo poco que QML necesita llamar hacia fuera que no es ni el núcleo ni el
/// modelo: cerrar el banco de pruebas con su informe.
class App : public QObject {
    Q_OBJECT
public:
    App(Banco *banco, Proveedor *prov, Modelo *modelo) : m_banco(banco), m_prov(prov), m_modelo(modelo) { }

    void setVentana(QQuickWindow *v) { m_ventana = v; }

    /// Guarda un fotograma. Lo usa el modo guion para dejar constancia de cada
    /// paso, que es la única forma de revisar la interfaz sin una persona
    /// delante.
    Q_INVOKABLE bool capturar(const QString &ruta)
    {
        if (!m_ventana) return false;
        const bool ok = m_ventana->grabWindow().save(ruta);
        QTextStream(stdout) << (ok ? "  captura " : "  no pude guardar ") << ruta << "\n";
        return ok;
    }

    Q_INVOKABLE void terminar(const QString &titulo)
    {
        double propia = 0, archivos = 0;
        memoriaMB(&propia, &archivos);
        const QString extra =
            QStringLiteral("  imágenes decodif.   %1  (%2 fallos, %7 canceladas)\n"
                           "  hilos de decodif.   %3\n"
                           "  memoria propia      %4 MB  (+ %5 MB de archivos mapeados)\n"
                           "  elementos en vista  %6\n")
                .arg(m_prov->decodificadas())
                .arg(m_prov->fallidas())
                .arg(QThreadPool::globalInstance()->maxThreadCount())
                .arg(propia, 0, 'f', 0)
                .arg(archivos, 0, 'f', 0)
                .arg(m_modelo->total())
                .arg(m_prov->canceladas());
        m_banco->informe(titulo, extra);
        QGuiApplication::quit();
    }

private:
    Banco *m_banco;
    Proveedor *m_prov;
    Modelo *m_modelo;
    QQuickWindow *m_ventana = nullptr;
};

int main(int argc, char **argv)
{
    Aplicacion app(argc, argv);
    app.setApplicationName(QStringLiteral("Grimorio"));
    app.setOrganizationName(QStringLiteral("Grimorio"));
#ifdef Q_OS_WIN
    // Los ajustes en un archivo y no en el registro, que es lo que haría Qt
    // por defecto en Windows: `%APPDATA%\Grimorio\Grimorio.ini` se puede
    // mirar, copiar a otro equipo y borrar a mano, igual que el
    // `~/.config/Grimorio/Grimorio.conf` de Linux. Tiene que ir antes del
    // primer QSettings.
    QSettings::setDefaultFormat(QSettings::IniFormat);
#endif
    // En Wayland la ventana se empareja con su .desktop por este nombre, y de
    // ahí saca el icono del panel; sin él sale el genérico.
    app.setDesktopFileName(QStringLiteral("grimorio"));
    {
        QIcon icono;
        for (int s : { 16, 24, 32, 48, 64, 128, 256 })
            icono.addFile(QStringLiteral(":/icono/grimorio-%1.png").arg(s), QSize(s, s));
        app.setWindowIcon(icono);
    }

    Args args;
    QString error;
    if (!parsear(app.arguments(), &args, &error)) {
        QTextStream(stderr) << "error: " << error << "\n\n" << AYUDA;
        return 2;
    }
    const bool automatica = args.bench >= 0 || !args.captura.isEmpty() || args.visor > 0
                            || args.lote > 0 || !args.guion.isEmpty();
    // Lo que se dice en la ventana nada más abrirla, si la biblioteca no es la
    // que se pidió.
    QString avisoInicial;
    if (!args.lib.isEmpty()) {
        const QString pedida = args.lib;
        args.lib = bibliotecaQueContiene(pedida);
        if (args.lib.isEmpty()) {
            // Una biblioteca que no está —un disco sin montar, una carpeta
            // movida, un GRIMORIO_LIB viejo— no puede dejar a nadie sin
            // ventana: se abre la de siempre y se dice. Las pasadas
            // automáticas sí fallan, porque medir otra biblioteca sin
            // avisar daría números que no son.
            const QString motivo = QFileInfo::exists(pedida)
                                       ? QStringLiteral("«%1» no es una biblioteca de Grimorio").arg(pedida)
                                       : QStringLiteral("no encuentro la biblioteca «%1»").arg(pedida);
            if (automatica) {
                avisarDeError(motivo);
                return 1;
            }
            QTextStream(stderr) << "aviso: " << motivo << "; abro otra\n";
            avisoInicial = motivo + QStringLiteral("; se ha abierto otra");
            args.lib = bibliotecaPorDefecto(&error, QDir(pedida).absolutePath());
        }
    } else {
        args.lib = bibliotecaPorDefecto(&error);
    }
    if (args.lib.isEmpty()) {
        avisarDeError(error);
        return 2;
    }

    if (!args.vsync) {
        QSurfaceFormat f = QSurfaceFormat::defaultFormat();
        f.setSwapInterval(0);
        QSurfaceFormat::setDefaultFormat(f);
    }

    // Los ajustes van antes que el tema: de ellos sale cuál se carga y a qué
    // tamaño.
    Ajustes ajustes;
    Tema tema;
    tema.setEscala(ajustes.escala());
    // `--tema` manda al arrancar; si no, el elegido en los ajustes, de los que
    // vienen dentro.
    const auto temaDeDentro = [&tema](const QString &nombre) {
        QString e;
        if (!tema.cargar(QStringLiteral(":/temas/%1.json").arg(nombre), &e))
            tema.cargar(QStringLiteral(":/temas/oscuro.json"), &e);
    };
    if (!args.tema.isEmpty()) {
        QString e;
        if (tema.cargar(args.tema, &e)) {
            tema.vigilar();
        } else {
            QTextStream(stderr) << "aviso: no pude leer el tema " << args.tema << " (" << e
                                << "); sigo con el de los ajustes\n";
            temaDeDentro(ajustes.tema());
        }
    } else {
        temaDeDentro(ajustes.tema());
    }
    // Elegir otro en los ajustes lo pone al momento, y mover el tamaño también.
    QString temaPuesto = ajustes.tema();
    qreal escalaPuesta = ajustes.escala();
    QObject::connect(&ajustes, &Ajustes::cambio, &tema, [&] {
        if (ajustes.tema() != temaPuesto) {
            temaPuesto = ajustes.tema();
            temaDeDentro(temaPuesto);
        }
        if (!qFuzzyCompare(ajustes.escala(), escalaPuesta)) {
            escalaPuesta = ajustes.escala();
            tema.setEscala(escalaPuesta);
        }
    });

    // Una ventana por biblioteca, y no dos.
    //
    // Dos procesos sobre la misma biblioteca escriben los dos en `folders.json`
    // y en el índice, y el segundo en guardar pisa al primero sin decir nada. Y
    // con el vídeo automático puesto se pelean además por el decodificador de
    // la tarjeta: cada uno abre su tubería por vídeo, la tarjeta se queda sin
    // sesiones y la reproducción se cae con «No decoder available».
    //
    // El cerrojo guarda el pid de quien lo tiene, así que un cierre a lo bruto
    // no deja la biblioteca condenada: el siguiente ve que ese proceso ya no
    // está y se lo queda.
    QLockFile cerrojo(QDir(args.lib).filePath(QStringLiteral("abierta.lock")));
    if (!cerrojo.tryLock()) {
        qint64 pid = 0;
        QString maquina, programa;
        cerrojo.getLockInfo(&pid, &maquina, &programa);
        avisarDeError(QStringLiteral("«%1» ya está abierta%2")
                          .arg(args.lib,
                               pid > 0 ? QStringLiteral(" (proceso %1)").arg(pid) : QString()));
        return 1;
    }

    Nucleo nucleo;
    if (!nucleo.abrir(args.lib)) {
        avisarDeError(QStringLiteral("«%1» no es una biblioteca de Grimorio").arg(args.lib));
        return 1;
    }

    Modelo modelo(&nucleo);
    Disposicion disposicion(&modelo);
    disposicion.setObjetivo(args.celda > 0 ? args.celda : tema.celda());
    if (args.modo >= 0)
        disposicion.setModo(args.modo == 0 ? Disposicion::Cuadricula : Disposicion::Justificado);
    disposicion.setMargen(tema.margen());
    disposicion.setHueco(tema.hueco());
    auto *proveedor = new Proveedor(&modelo);
    Banco banco(args.bench);
    App puente(&banco, proveedor, &modelo);

    // La vista que se dejó puesta, salvo que se pida otra al arrancar.
    if (args.modo < 0 && ajustes.vista() >= 0 && ajustes.vista() <= Disposicion::Lista)
        disposicion.setModo(Disposicion::Modo(ajustes.vista()));
    QObject::connect(&disposicion, &Disposicion::modoCambio, &ajustes,
                     [&] { ajustes.setVista(int(disposicion.modo())); });
    Derivados derivados(QDir(args.lib).absolutePath());
    Fotogramas fotogramas;
    Pixeles pixeles;
    Portapapeles portapapeles;
    Traer traer(&nucleo);
    Captura captura(&nucleo);
    // Lo que el proceso nuevo hereda al cambiar de biblioteca: el tema.
    QStringList heredados;
    if (!args.tema.isEmpty()) heredados << QStringLiteral("--tema") << args.tema;
    Bibliotecas bibliotecas(args.lib, heredados);
    Exportar exportar;
    Vigilancia vigilancia(&nucleo);
    // Las pasadas automáticas no preguntan por versiones: no tienen que
    // depender de la red ni sacar un aviso en lo que capturan.
    // GRIMORIO_SIN_RED es para lo que arranca el programa normal desde fuera,
    // como el recorrido de pruebas.
    Novedades novedades(&ajustes, QStringLiteral(GRIMORIO_VERSION),
                        !automatica && qEnvironmentVariableIsEmpty("GRIMORIO_SIN_RED"));

    // El visor 3D pinta con OpenGL dentro de la escena de Qt Quick, y eso solo
    // funciona si la escena entera va por OpenGL. En Linux ya es lo que elige
    // Qt por su cuenta; se dice para que no dependa de eso. En Windows Qt
    // elegiría Direct3D, así que aquí sí cambia algo: se usa el OpenGL del
    // controlador de la tarjeta, y en un equipo sin él (una máquina virtual,
    // un escritorio remoto) Qt cae solo a `opengl32sw.dll`, el OpenGL por
    // software que el instalador lleva al lado del programa.
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
    qmlRegisterType<Onda>("Grimorio", 1, 0, "Onda");
    qmlRegisterType<Visor3D>("Grimorio", 1, 0, "Visor3D");

    QQmlApplicationEngine motor;
    motor.addImageProvider(QStringLiteral("grim"), proveedor); // el motor lo adopta
    auto *ctx = motor.rootContext();
    ctx->setContextProperty(QStringLiteral("nucleo"), &nucleo);
    ctx->setContextProperty(QStringLiteral("modelo"), &modelo);
    ctx->setContextProperty(QStringLiteral("disposicion"), &disposicion);
    ctx->setContextProperty(QStringLiteral("tema"), &tema);
    ctx->setContextProperty(QStringLiteral("ajustes"), &ajustes);
    ctx->setContextProperty(QStringLiteral("derivados"), &derivados);
    ctx->setContextProperty(QStringLiteral("fotogramas"), &fotogramas);
    ctx->setContextProperty(QStringLiteral("pixeles"), &pixeles);
    ctx->setContextProperty(QStringLiteral("portapapeles"), &portapapeles);
    ctx->setContextProperty(QStringLiteral("traer"), &traer);
    ctx->setContextProperty(QStringLiteral("captura"), &captura);
    ctx->setContextProperty(QStringLiteral("bibliotecas"), &bibliotecas);
    ctx->setContextProperty(QStringLiteral("exportar"), &exportar);
    ctx->setContextProperty(QStringLiteral("vigilancia"), &vigilancia);
    ctx->setContextProperty(QStringLiteral("novedades"), &novedades);
    ctx->setContextProperty(QStringLiteral("banco"), &banco);
    ctx->setContextProperty(QStringLiteral("app"), &puente);
    ctx->setContextProperty(QStringLiteral("versionGrimorio"), QStringLiteral(GRIMORIO_VERSION));
    ctx->setContextProperty(QStringLiteral("versionQt"), QString::fromLatin1(qVersion()));
    ctx->setContextProperty(QStringLiteral("anchoInicial"), args.ancho);
    ctx->setContextProperty(QStringLiteral("altoInicial"), args.alto);
    ctx->setContextProperty(QStringLiteral("pruebaVisor"), args.visor);
    ctx->setContextProperty(QStringLiteral("pruebaLote"), args.lote);
    ctx->setContextProperty(QStringLiteral("guion"), args.guion);
    ctx->setContextProperty(QStringLiteral("filtroInicial"), args.filtro);
    ctx->setContextProperty(QStringLiteral("verAlArrancar"), args.ver);
    ctx->setContextProperty(QStringLiteral("celdaInicial"),
                            args.celda > 0 ? args.celda : tema.celda());

    motor.load(QUrl(QStringLiteral("qrc:/qml/Ventana.qml")));
    if (motor.rootObjects().isEmpty()) {
        QTextStream(stderr) << "error: no pude cargar la interfaz\n";
        return 1;
    }
    auto *ventana = qobject_cast<QQuickWindow *>(motor.rootObjects().first());
    if (!ventana) {
        QTextStream(stderr) << "error: la raíz de QML no es una ventana\n";
        return 1;
    }
    banco.vigilar(ventana);
    if (!avisoInicial.isEmpty()) {
        // Con la ventana ya a la vista; antes, el aviso se iría con nadie
        // mirando.
        QTimer::singleShot(600, &bibliotecas, [&bibliotecas, avisoInicial] {
            emit bibliotecas.aviso(avisoInicial, true);
        });
    }
    puente.setVentana(ventana);

    if (!args.captura.isEmpty()) {
        // Un par de segundos para que la cola de decodificación llene la
        // pantalla; capturar antes daría una imagen de marcadores de color.
        QTimer::singleShot(2500, ventana, [&] {
            const QImage img = ventana->grabWindow();
            if (img.save(args.captura))
                QTextStream(stdout)
                    << "  captura guardada en " << args.captura
                    << QStringLiteral(" (%1 imágenes decodificadas)\n").arg(proveedor->decodificadas());
            else
                QTextStream(stderr) << "error: no pude guardar " << args.captura << "\n";
            QGuiApplication::quit();
        });
    }

    return app.exec();
}

#include "main.moc"
