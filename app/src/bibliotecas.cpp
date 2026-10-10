#include "bibliotecas.h"

#include "grimorio.h"
#include "portal.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLockFile>
#include <QProcess>
#include <QStandardPaths>
#include <QTimer>
#include <QUrl>

namespace {

const char *CLAVE = "bibliotecas/recientes";
constexpr int CUANTAS = 10;

bool esBiblioteca(const QString &ruta)
{
    const QByteArray b = ruta.toUtf8();
    return grim_es_biblioteca(reinterpret_cast<const uint8_t *>(b.constData()), uint32_t(b.size())) != 0;
}

bool crearBiblioteca(const QString &ruta)
{
    const QByteArray b = ruta.toUtf8();
    return grim_crear_biblioteca(reinterpret_cast<const uint8_t *>(b.constData()), uint32_t(b.size())) != 0;
}

} // namespace

Bibliotecas::Bibliotecas(const QString &actual, const QStringList &extra, QObject *padre)
    : QObject(padre), m_actual(QDir(actual).absolutePath()), m_extra(extra)
{
    // La de ahora primero; las que ya no existen, fuera: un disco que no está
    // conectado no tiene que llenar el menú de entradas que no abren.
    QStringList antes = m_disco.value(CLAVE).toStringList();
    m_recientes << m_actual;
    for (const QString &r : antes) {
        if (r != m_actual && QFileInfo::exists(r) && m_recientes.size() < CUANTAS) m_recientes << r;
    }
    guardar();
}

void Bibliotecas::guardar()
{
    m_disco.setValue(CLAVE, m_recientes);
    emit cambio();
}

QString Bibliotecas::nombreDe(const QString &ruta) const
{
    // El nombre que se le puso (se puede renombrar sin mover la carpeta); si
    // no se puede leer, el de la carpeta.
    QFile f(QDir(ruta).filePath(QStringLiteral("library.json")));
    if (f.open(QIODevice::ReadOnly)) {
        const QString puesto = QJsonDocument::fromJson(f.read(64 * 1024)).object()
                                   .value(QStringLiteral("name")).toString().trimmed();
        if (!puesto.isEmpty()) return puesto;
    }
    QString n = QFileInfo(ruta).fileName();
    if (n.endsWith(QLatin1String(".grimorio"))) n.chop(9);
    return n;
}

void Bibliotecas::olvidar(const QString &ruta)
{
    if (ruta == m_actual) return;
    m_recientes.removeAll(ruta);
    guardar();
}

void Bibliotecas::abrir(const QString &ruta)
{
    const QString r = QDir(ruta).absolutePath();
    if (r == m_actual) return;
    if (!esBiblioteca(r)) {
        emit aviso(tr("«%1» no es una biblioteca de Grimorio (o ya no está)").arg(r), true);
        olvidar(r);
        return;
    }
    {
        // Mirar el cerrojo sin quedárselo: si otro proceso lo tiene, esa
        // biblioteca ya está abierta en otra ventana.
        QLockFile cerrojo(QDir(r).filePath(QStringLiteral("abierta.lock")));
        if (!cerrojo.tryLock(0)) {
            emit aviso(tr("«%1» ya está abierta en otra ventana").arg(nombreDe(r)), true);
            return;
        }
        cerrojo.unlock();
    }
    QStringList args = m_extra;
    args << QStringLiteral("--lib") << r;
    if (!QProcess::startDetached(QCoreApplication::applicationFilePath(), args)) {
        emit aviso(tr("no pude arrancar Grimorio con «%1»").arg(nombreDe(r)), true);
        return;
    }
    // Irse en la vuelta siguiente: quien ha llamado está en mitad de un clic.
    QTimer::singleShot(0, qApp, &QCoreApplication::quit);
}

void Bibliotecas::elegir(bool nueva)
{
    const QString titulo = nueva ? tr("Carpeta para la biblioteca nueva")
                                 : tr("Abrir una biblioteca de Grimorio");
    if (!elegirCarpeta(this, titulo, [this, nueva](const QString &r) { elegida(r, nueva); }))
        emit aviso(tr("no hay diálogo de carpetas: ni el portal del escritorio ni zenity"), true);
}

void Bibliotecas::elegida(const QString &ruta, bool nueva)
{
    if (ruta.isEmpty()) return;
    if (esBiblioteca(ruta)) {
        abrir(ruta);
        return;
    }
    if (!nueva) {
        emit aviso(tr("esa carpeta no es una biblioteca de Grimorio; para hacer una, «nueva…»"), true);
        return;
    }
    // Una carpeta con cosas dentro no se convierte en biblioteca: quedarían
    // mezclados los archivos de alguien con los de Grimorio. Se crea dentro.
    QString destino = ruta;
    const QDir d(ruta);
    if (!d.isEmpty(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden)) {
        destino = d.filePath(QStringLiteral("Grimorio.grimorio"));
        for (int n = 2; QFileInfo::exists(destino); ++n)
            destino = d.filePath(QStringLiteral("Grimorio %1.grimorio").arg(n));
    }
    if (!crearBiblioteca(destino)) {
        emit aviso(tr("no pude crear la biblioteca en «%1»").arg(destino), true);
        return;
    }
    abrir(destino);
}
