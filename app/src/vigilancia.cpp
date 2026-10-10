#include "vigilancia.h"

#include "nucleo.h"
#include "portal.h"

#include <QFileInfo>

void Vigilancia::vigilar(const QString &carpeta)
{
    if (!elegirCarpeta(this, tr("Carpeta del disco que vincular"),
                       [this, carpeta](const QString &ruta) { m_nucleo->vigilar(ruta, carpeta); }))
        emit aviso(tr("no hay diálogo de carpetas: ni el portal del escritorio ni zenity"), true);
}

bool Vigilancia::existe(const QString &ruta) const
{
    return QFileInfo(ruta).isDir();
}
