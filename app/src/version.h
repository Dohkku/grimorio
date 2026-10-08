// Comparar versiones y leer el `version.json` de la web, sin red ni ventana.
//
// Va aparte de `novedades.cpp` para poder probarlo solo: lo que decide si
// alguien ve el aviso de versión nueva tiene que estar bien siempre, y la red
// no se puede meter en una prueba.
#pragma once
#include <QByteArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>
#include <QString>
#include <QUrl>

namespace version {

/// La página a la que se manda a quien quiere la versión nueva si el
/// `version.json` no dice otra. La portada ya elige la descarga de cada
/// sistema, así que no hace falta más.
inline const char *PAGINA = "https://grimorio.frederickandrade.com/";

/// Negativo si `a` es más vieja que `b`, cero si son la misma, positivo si es
/// más nueva. Solo cuentan los tres números de delante: «0.2.0-rc1» se lee
/// como 0.2.0. Lo que no empieza por X.Y.Z vale como 0.0.0, así que nunca
/// pasa por nueva.
inline int comparar(const QString &a, const QString &b)
{
    static const QRegularExpression patron(QStringLiteral("^\\s*v?(\\d+)\\.(\\d+)\\.(\\d+)"));
    const auto ma = patron.match(a);
    const auto mb = patron.match(b);
    for (int i = 1; i <= 3; ++i) {
        const qlonglong x = ma.hasMatch() ? ma.captured(i).toLongLong() : 0;
        const qlonglong y = mb.hasMatch() ? mb.captured(i).toLongLong() : 0;
        if (x != y) return x < y ? -1 : 1;
    }
    return 0;
}

struct Publicada {
    QString version; // vacía si el archivo no sirve
    QUrl enlace;
};

/// Lo que dice el `version.json`:
///
///     { "version": "0.2.0", "descargas": "https://…" }
///
/// Un enlace que no sea https se cambia por la portada: el aviso abre el
/// navegador con él, y no tiene que poder abrir cualquier cosa.
inline Publicada leer(const QByteArray &json)
{
    Publicada p;
    const QJsonObject o = QJsonDocument::fromJson(json).object();
    const QString v = o.value(QStringLiteral("version")).toString().trimmed();
    if (comparar(v, QStringLiteral("0.0.0")) <= 0) return p;
    p.version = v;
    const QUrl u(o.value(QStringLiteral("descargas")).toString());
    p.enlace = u.isValid() && u.scheme() == QLatin1String("https") && !u.host().isEmpty()
                   ? u
                   : QUrl(QString::fromLatin1(PAGINA));
    return p;
}

} // namespace version
