#include "exportar.h"

#include "portal.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QPointer>
#include <QThreadPool>

namespace {

/// Un nombre libre en `dir`: el mismo si no está, y si no «nombre (2).ext».
QString libre(const QDir &dir, const QString &nombre)
{
    if (!dir.exists(nombre)) return dir.filePath(nombre);
    const QFileInfo f(nombre);
    const QString base = f.completeBaseName();
    const QString ext = f.suffix().isEmpty() ? QString() : QLatin1Char('.') + f.suffix();
    for (int n = 2;; ++n) {
        const QString otro = QStringLiteral("%1 (%2)%3").arg(base).arg(n).arg(ext);
        if (!dir.exists(otro)) return dir.filePath(otro);
    }
}

} // namespace

void Exportar::exportar(const QList<QUrl> &urls)
{
    if (urls.isEmpty() || m_enMarcha) return;
    const QString titulo = urls.size() == 1 ? tr("Exportar a…") : tr("Exportar %1 a…").arg(urls.size());
    if (!elegirCarpeta(this, titulo, [this, urls](const QString &d) { copiar(urls, d); }))
        emit aviso(tr("no hay diálogo de carpetas: ni el portal del escritorio ni zenity"), true);
}

void Exportar::copiar(const QList<QUrl> &urls, const QString &destino)
{
    m_enMarcha = true;
    emit enMarchaCambio();
    emit aviso(tr("exportando %1…").arg(urls.size()), false);
    QPointer<Exportar> yo(this);
    QThreadPool::globalInstance()->start([yo, urls, destino] {
        const QDir dir(destino);
        int bien = 0, mal = 0;
        for (const QUrl &u : urls) {
            const QString de = u.toLocalFile();
            if (QFile::copy(de, libre(dir, QFileInfo(de).fileName()))) ++bien;
            else ++mal;
        }
        QMetaObject::invokeMethod(
            yo.data(),
            [yo, bien, mal, destino] {
                if (!yo) return;
                yo->m_enMarcha = false;
                emit yo->enMarchaCambio();
                if (mal == 0)
                    emit yo->aviso(tr("%1 exportado%2 a %3").arg(bien).arg(bien == 1 ? "" : "s", destino), false);
                else
                    emit yo->aviso(tr("%1 exportados, %2 no se pudieron copiar a %3").arg(bien).arg(mal).arg(destino),
                                   true);
            },
            Qt::QueuedConnection);
    });
}
