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

/// De dónde puede bajarse una actualización: las releases del repositorio, y
/// nada más. Lo que se baja se ejecuta, así que un `version.json` cambiado
/// por quien no debe no tiene que poder mandar a otro sitio.
inline const char *RELEASES = "/Dohkku/grimorio/releases/download/";

/// El paquete de este sistema, si el `version.json` lo trae: con él la
/// actualización se baja y se instala desde dentro del programa.
struct Paquete {
    QUrl url;       // vacía si no hay paquete para este sistema
    QString sha256; // en minúsculas, 64 cifras
};

/// La clave de este sistema en `paquetes`.
inline QString sistema()
{
#if defined(Q_OS_WIN)
    return QStringLiteral("windows");
#elif defined(Q_OS_LINUX)
    return QStringLiteral("linux");
#else
    return {};
#endif
}

/// Un paquete solo vale si viene de las releases por https y trae su huella:
/// sin huella no hay forma de saber que lo bajado es lo publicado.
inline Paquete leerPaquete(const QJsonObject &o)
{
    static const QRegularExpression hex(QStringLiteral("^[0-9a-f]{64}$"));
    const QUrl u(o.value(QStringLiteral("url")).toString());
    const QString h = o.value(QStringLiteral("sha256")).toString().trimmed().toLower();
    if (!u.isValid() || u.scheme() != QLatin1String("https") || u.host() != QLatin1String("github.com")
        || !u.path().startsWith(QLatin1String(RELEASES)) || !u.query().isEmpty()
        || u.path().contains(QLatin1String("..")) || !hex.match(h).hasMatch())
        return {};
    return { u, h };
}

struct Publicada {
    QString version; // vacía si el archivo no sirve
    QUrl enlace;
    Paquete paquete; // el de este sistema
};

/// Lo que dice el `version.json`:
///
///     { "version": "0.2.0", "descargas": "https://…",
///       "paquetes": { "linux":   { "url": "https://github.com/…", "sha256": "…" },
///                     "windows": { "url": "https://github.com/…", "sha256": "…" } } }
///
/// Un enlace que no sea https se cambia por la portada: el aviso abre el
/// navegador con él, y no tiene que poder abrir cualquier cosa. `paquetes` es
/// de la 0.1.4 en adelante; las versiones de antes no lo miran.
inline Publicada leer(const QByteArray &json, const QString &so = sistema())
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
    if (!so.isEmpty())
        p.paquete = leerPaquete(o.value(QStringLiteral("paquetes")).toObject().value(so).toObject());
    return p;
}

} // namespace version
