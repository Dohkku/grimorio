// Nombres de archivo que valgan en cualquier disco.
//
// El nombre de un elemento lo escribe una persona («Plano 2/3: alzado») o
// viene de una web, y acaba siendo nombre de archivo al arrastrarlo fuera o
// al guardar una descarga. En Linux basta con quitar la barra. En Windows hay
// muchas más trampas: `\ : * ? " < > |` no pueden ir en un nombre, un punto o
// un espacio al final se los come el sistema sin avisar, y `CON`, `NUL`,
// `COM1`… son dispositivos, no archivos, aunque lleven extensión.
//
// Se limpia igual en todos los sistemas y no solo en Windows: una biblioteca
// se copia a un disco externo en NTFS o exFAT, o se comparte por red, y un
// nombre que solo vale en Linux se rompería allí.
#pragma once
#include <QString>
#include <QStringList>

/// `n` hecho nombre de archivo válido. Si no queda nada que valga, `siVacio`.
inline QString nombreDeArchivo(QString n, const QString &siVacio)
{
    for (QChar &c : n) {
        if (c.unicode() < 0x20 || QStringLiteral("/\\:*?\"<>|").contains(c)) c = QLatin1Char('-');
    }
    n = n.trimmed();
    while (n.endsWith(QLatin1Char('.')) || n.endsWith(QLatin1Char(' '))) n.chop(1);
    if (n.isEmpty()) return siVacio;

    // Lo reservado es el nombre sin extensión: `nul.txt` tampoco se puede.
    static const QStringList reservados = [] {
        QStringList r { QStringLiteral("CON"), QStringLiteral("PRN"), QStringLiteral("AUX"),
                        QStringLiteral("NUL") };
        for (int i = 1; i <= 9; ++i)
            r << QStringLiteral("COM%1").arg(i) << QStringLiteral("LPT%1").arg(i);
        return r;
    }();
    const QString base = n.section(QLatin1Char('.'), 0, 0).trimmed();
    if (reservados.contains(base, Qt::CaseInsensitive)) n.prepend(QLatin1Char('_'));
    return n;
}
