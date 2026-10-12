#include "paquete.h"

#include <QDirIterator>
#include <QFile>
#include <QFileInfo>

namespace paquete {

QStringList piezas()
{
    return { QStringLiteral("grimorio"), QStringLiteral("grim"), QStringLiteral("grim-recorte"),
             QStringLiteral("modelos/isnet-general-use-pesos16.onnx"),
             QStringLiteral("navegador/manifest.json") };
}

QStringList faltan(const QDir &dir)
{
    QStringList v;
    for (const QString &p : piezas())
        if (!QFileInfo::exists(dir.filePath(p))) v << p;
    return v;
}

QString raizDe(const QString &suelto)
{
    for (const QFileInfo &f : QDir(suelto).entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot))
        if (QFileInfo::exists(QDir(f.absoluteFilePath()).filePath(QStringLiteral("grimorio"))))
            return f.absoluteFilePath();
    return {};
}

namespace {

/// Copia un archivo conservando que sea ejecutable si lo era.
bool copiar(const QString &de, const QString &a)
{
    QFile::remove(a);
    if (!QFile::copy(de, a)) return false;
    QFile::setPermissions(a, QFile(de).permissions() | QFileDevice::ReadOwner | QFileDevice::WriteOwner);
    return true;
}

/// Copia una carpeta entera, archivo a archivo.
bool copiarCarpeta(const QString &de, const QString &a)
{
    QDir(a).removeRecursively();
    if (!QDir().mkpath(a)) return false;
    QDirIterator it(de, QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot | QDir::Hidden,
                    QDirIterator::Subdirectories);
    while (it.hasNext()) {
        const QString origen = it.next();
        const QString destino = a + QLatin1Char('/') + QDir(de).relativeFilePath(origen);
        if (it.fileInfo().isDir()) {
            if (!QDir().mkpath(destino)) return false;
        } else if (!copiar(origen, destino)) {
            return false;
        }
    }
    return true;
}

void borrar(const QString &ruta)
{
    if (QFileInfo(ruta).isDir()) QDir(ruta).removeRecursively();
    else QFile::remove(ruta);
}

/// Lo que falta, sin tocar nada de lo que hay.
bool completar(const QString &raiz, const QDir &dir, QString *error, QStringList *puestos)
{
    QDirIterator it(raiz, QDir::Files | QDir::Hidden, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        const QString origen = it.next();
        const QString rel = QDir(raiz).relativeFilePath(origen);
        const QString destino = dir.filePath(rel);
        if (QFileInfo::exists(destino)) continue;
        // Al lado y renombrado: un corte a mitad no deja un modelo a medias
        // que parezca entero.
        const QString nuevo = destino + QStringLiteral(".nuevo");
        if (!QDir().mkpath(QFileInfo(destino).absolutePath()) || !copiar(origen, nuevo)
            || !QFile::rename(nuevo, destino)) {
            QFile::remove(nuevo);
            if (error) *error = QStringLiteral("no he podido poner %1").arg(rel);
            return false;
        }
        if (puestos) *puestos << rel;
    }
    return true;
}

} // namespace

bool instalar(const QString &raiz, const QDir &dir, bool soloLoQueFalta, QString *error,
              QStringList *puestos)
{
    if (soloLoQueFalta) return completar(raiz, dir, error, puestos);

    // Primero todo copiado al lado como «.nuevo»; solo entonces se cambia.
    const QFileInfoList entradas =
        QDir(raiz).entryInfoList(QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot | QDir::Hidden);
    for (const QFileInfo &e : entradas) {
        const QString nuevo = dir.filePath(e.fileName() + QStringLiteral(".nuevo"));
        const bool bien = e.isDir() ? copiarCarpeta(e.absoluteFilePath(), nuevo)
                                    : copiar(e.absoluteFilePath(), nuevo);
        if (!bien) {
            for (const QFileInfo &x : entradas) borrar(dir.filePath(x.fileName() + QStringLiteral(".nuevo")));
            if (error) *error = QStringLiteral("no he podido copiar %1 en %2").arg(e.fileName(), dir.path());
            return false;
        }
    }

    QStringList hechos;
    auto deshacer = [&] {
        for (const QString &h : hechos) {
            const QString puesto = dir.filePath(h);
            borrar(puesto);
            QFile::rename(puesto + QStringLiteral(".anterior"), puesto);
        }
    };
    for (const QFileInfo &e : entradas) {
        const QString puesto = dir.filePath(e.fileName());
        const QString anterior = puesto + QStringLiteral(".anterior");
        borrar(anterior);
        const bool habia = QFileInfo::exists(puesto);
        if ((habia && !QFile::rename(puesto, anterior))
            || !QFile::rename(puesto + QStringLiteral(".nuevo"), puesto)) {
            if (habia && !QFileInfo::exists(puesto)) QFile::rename(anterior, puesto);
            deshacer();
            for (const QFileInfo &x : entradas) borrar(dir.filePath(x.fileName() + QStringLiteral(".nuevo")));
            if (error) *error = QStringLiteral("no he podido cambiar %1 en %2").arg(e.fileName(), dir.path());
            return false;
        }
        hechos << e.fileName();
    }
    // Las carpetas viejas no se guardan: el modelo son 91 MB, y volver atrás ya
    // lo hace el programa de antes, que sí se guarda como «.anterior».
    for (const QString &h : hechos) {
        const QString anterior = dir.filePath(h) + QStringLiteral(".anterior");
        if (QFileInfo(anterior).isDir()) QDir(anterior).removeRecursively();
    }
    if (puestos) *puestos = hechos;
    return true;
}

} // namespace paquete
