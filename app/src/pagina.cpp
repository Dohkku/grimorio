#include "pagina.h"

#include <QDateTime>
#include <QMimeDatabase>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QRegularExpression>
#include <QStringDecoder>

namespace {

/// Lo que se mete dentro, en total (ya en base64) y por archivo. Sin el tope
/// de los mensajes de la extensión, pero una página no debería pesar más que
/// un vídeo corto.
constexpr qint64 PRESUPUESTO = 30ll << 20;
constexpr qint64 TOPE_RECURSO = 15ll << 20;
/// Hojas de estilo que importan otras, que importan otras…
constexpr int PROFUNDIDAD_IMPORT = 4;
/// De un `srcset`, la mayor hasta este ancho: la de 4000 px pesa seis veces
/// más y en pantalla no se nota.
constexpr int ANCHO_BUENO = 1600;

QString desentidades(const QString &s)
{
    if (!s.contains(QLatin1Char('&'))) return s;
    static const QRegularExpression ent(QStringLiteral("&(#[xX][0-9a-fA-F]+|#[0-9]+|amp|quot|apos|lt|gt|nbsp);"));
    QString r;
    qsizetype ultimo = 0;
    auto it = ent.globalMatch(s);
    while (it.hasNext()) {
        const auto m = it.next();
        r += s.mid(ultimo, m.capturedStart() - ultimo);
        const QString e = m.captured(1);
        if (e.startsWith(QLatin1String("#x"), Qt::CaseInsensitive)) r += QChar(char32_t(e.mid(2).toUInt(nullptr, 16)));
        else if (e.startsWith(QLatin1Char('#'))) r += QChar(char32_t(e.mid(1).toUInt()));
        else if (e == QLatin1String("amp")) r += QLatin1Char('&');
        else if (e == QLatin1String("quot")) r += QLatin1Char('"');
        else if (e == QLatin1String("apos")) r += QLatin1Char('\'');
        else if (e == QLatin1String("lt")) r += QLatin1Char('<');
        else if (e == QLatin1String("gt")) r += QLatin1Char('>');
        else r += QChar(0xa0);
        ultimo = m.capturedEnd();
    }
    r += s.mid(ultimo);
    return r;
}

QString comillas(QString s)
{
    s.replace(QLatin1Char('&'), QLatin1String("&amp;"));
    s.replace(QLatin1Char('"'), QLatin1String("&quot;"));
    return s;
}

/// La dirección entera, o "" si no es algo que se pueda bajar.
QString absoluta(const QString &u, const QUrl &base)
{
    const QString t = u.trimmed();
    if (t.isEmpty() || t.startsWith(QLatin1String("data:"), Qt::CaseInsensitive)) return {};
    const QUrl r = base.resolved(QUrl(t));
    if (!r.isValid() || (r.scheme() != QLatin1String("http") && r.scheme() != QLatin1String("https")))
        return {};
    return r.toString(QUrl::FullyEncoded);
}

/// Lo mismo para lo que se queda apuntando a la web (enlaces, vídeos): lo que
/// no sea una dirección relativa se deja como estaba.
QString entera(const QString &u, const QUrl &base)
{
    const QString t = u.trimmed();
    if (t.isEmpty() || t.startsWith(QLatin1Char('#'))
            || QRegularExpression(QStringLiteral("^(data|mailto|tel|javascript):"),
                                  QRegularExpression::CaseInsensitiveOption).match(t).hasMatch())
        return u;
    const QUrl r = base.resolved(QUrl(t));
    return r.isValid() ? r.toString(QUrl::FullyEncoded) : u;
}

struct Atributo {
    QString nombre;  // en minúsculas
    QString crudo;   // tal cual venía: `nombre="valor"`
    QString valor;   // ya sin entidades
    bool cambiado = false;
};

struct Etiqueta {
    QString nombre;      // en minúsculas
    QString nombreCrudo;
    QList<Atributo> atributos;
    bool cerrada = false; // `<… />`
    bool quitar = false;
    QString reemplazo;    // si no está vacío, va en su lugar

    int buscar(const QString &n) const
    {
        for (int i = 0; i < atributos.size(); ++i)
            if (atributos[i].nombre == n) return i;
        return -1;
    }
    bool tiene(const QString &n) const { return buscar(n) >= 0; }
    QString de(const QString &n) const
    {
        const int i = buscar(n);
        return i < 0 ? QString() : atributos[i].valor;
    }
    void poner(const QString &n, const QString &v)
    {
        const int i = buscar(n);
        if (i < 0) {
            atributos.append({ n, QString(), v, true });
        } else if (atributos[i].valor != v || !atributos[i].cambiado) {
            atributos[i].valor = v;
            atributos[i].cambiado = true;
        }
    }
    void quitarAtributo(const QString &n)
    {
        for (int i = atributos.size() - 1; i >= 0; --i)
            if (atributos[i].nombre == n) atributos.removeAt(i);
    }
    QString texto() const
    {
        QString s = QLatin1Char('<') + nombreCrudo;
        for (const Atributo &a : atributos) {
            s += QLatin1Char(' ');
            s += a.cambiado ? a.nombre + QStringLiteral("=\"") + comillas(a.valor) + QLatin1Char('"') : a.crudo;
        }
        if (cerrada) s += QStringLiteral(" /");
        return s + QLatin1Char('>');
    }
};

bool esEspacio(QChar c)
{
    return c == QLatin1Char(' ') || c == QLatin1Char('\n') || c == QLatin1Char('\t')
           || c == QLatin1Char('\r') || c == QLatin1Char('\f');
}

/// Lee la etiqueta que empieza en `html[i] == '<'`. Devuelve dónde acaba
/// (justo después de `>`), o -1 si no acaba.
qsizetype leerEtiqueta(const QString &html, qsizetype i, Etiqueta &t)
{
    const qsizetype n = html.size();
    qsizetype p = i + 1;
    const qsizetype ini = p;
    while (p < n && !esEspacio(html[p]) && html[p] != QLatin1Char('>') && html[p] != QLatin1Char('/')) ++p;
    t.nombreCrudo = html.mid(ini, p - ini);
    t.nombre = t.nombreCrudo.toLower();
    while (p < n) {
        while (p < n && (esEspacio(html[p]) || html[p] == QLatin1Char('/'))) {
            if (html[p] == QLatin1Char('/') && p + 1 < n && html[p + 1] == QLatin1Char('>')) t.cerrada = true;
            ++p;
        }
        if (p >= n) return -1;
        if (html[p] == QLatin1Char('>')) return p + 1;
        const qsizetype a0 = p;
        while (p < n && !esEspacio(html[p]) && html[p] != QLatin1Char('=') && html[p] != QLatin1Char('>')
               && !(html[p] == QLatin1Char('/') && p + 1 < n && html[p + 1] == QLatin1Char('>')))
            ++p;
        Atributo a;
        a.nombre = html.mid(a0, p - a0).toLower();
        qsizetype q = p;
        while (q < n && esEspacio(html[q])) ++q;
        if (q < n && html[q] == QLatin1Char('=')) {
            ++q;
            while (q < n && esEspacio(html[q])) ++q;
            if (q < n && (html[q] == QLatin1Char('"') || html[q] == QLatin1Char('\''))) {
                const QChar c = html[q];
                const qsizetype fin = html.indexOf(c, q + 1);
                if (fin < 0) return -1;
                a.valor = desentidades(html.mid(q + 1, fin - q - 1));
                p = fin + 1;
            } else {
                const qsizetype v0 = q;
                while (q < n && !esEspacio(html[q]) && html[q] != QLatin1Char('>')) ++q;
                a.valor = desentidades(html.mid(v0, q - v0));
                p = q;
            }
        }
        a.crudo = html.mid(a0, p - a0);
        if (!a.nombre.isEmpty()) t.atributos.append(a);
    }
    return -1;
}

/// Dónde acaba el bloque de texto crudo de `<nombre>` (script, style…) que
/// empieza en `desde`: el principio de `</nombre`, o el final del texto.
qsizetype finDeBloque(const QString &html, qsizetype desde, const QString &nombre)
{
    const qsizetype f = html.indexOf(QStringLiteral("</") + nombre, desde, Qt::CaseInsensitive);
    return f < 0 ? html.size() : f;
}

/// Justo después del `>` de la etiqueta de cierre que empieza en `f`.
qsizetype trasCierre(const QString &html, qsizetype f)
{
    if (f >= html.size()) return html.size();
    const qsizetype g = html.indexOf(QLatin1Char('>'), f);
    return g < 0 ? html.size() : g + 1;
}

QStringList tokensRel(const Etiqueta &t)
{
    return t.de(QStringLiteral("rel")).toLower().split(QRegularExpression(QStringLiteral("\\s+")),
                                                       Qt::SkipEmptyParts);
}

/// Una etiqueta de la página, en las dos vueltas: con `rec` apunta lo que hay
/// que bajar; con `b` la cambia por lo bajado.
void tratar(Etiqueta &t, const QUrl &base, const pagina::Bajado *b, pagina::Recursos *rec)
{
    // Nada que se ejecute: la copia tiene que verse igual hoy que dentro de
    // diez años.
    for (int i = t.atributos.size() - 1; i >= 0; --i) {
        if (t.atributos[i].nombre.startsWith(QLatin1String("on"))) t.atributos.removeAt(i);
    }
    if (t.de(QStringLiteral("href")).trimmed().startsWith(QLatin1String("javascript:"), Qt::CaseInsensitive))
        t.quitarAtributo(QStringLiteral("href"));
    if (t.tiene(QStringLiteral("style")) && t.de(QStringLiteral("style")).contains(QLatin1String("url("), Qt::CaseInsensitive)) {
        if (rec) rec->cssDentro << t.de(QStringLiteral("style"));
        if (b) t.poner(QStringLiteral("style"), pagina::montarCss(t.de(QStringLiteral("style")), base, *b));
    }

    const QString &n = t.nombre;
    auto aDatos = [&](const QString &atributo, const QString &url) {
        const QString abs = absoluta(url, base);
        if (abs.isEmpty()) return;
        if (rec) rec->datos << abs;
        if (b) t.poner(atributo, b->datos.value(abs, abs));
    };

    if (n == QLatin1String("base")) {
        t.quitar = true;
    } else if (n == QLatin1String("meta")) {
        // La codificación va de nuevo al principio, y una CSP no dejaría usar
        // `data:`.
        if (t.tiene(QStringLiteral("http-equiv")) || t.tiene(QStringLiteral("charset"))) {
            t.quitar = true;
            return;
        }
        const QString clave = (t.de(QStringLiteral("property")) + QLatin1Char(' ') + t.de(QStringLiteral("name"))).toLower();
        static const QRegularExpression portada(QStringLiteral("\\b(og:image(:secure_url|:url)?|twitter:image(:src)?)\\b"));
        if (rec && rec->portada.isEmpty() && portada.match(clave).hasMatch()) {
            const QString abs = absoluta(t.de(QStringLiteral("content")), base);
            if (!abs.isEmpty()) {
                rec->portada = abs;
                rec->datos << abs;
            }
        }
    } else if (n == QLatin1String("link")) {
        const QStringList rel = tokensRel(t);
        static const QStringList fuera = { "preload", "prefetch", "modulepreload", "dns-prefetch",
                                            "preconnect", "manifest", "serviceworker", "alternate" };
        const bool hoja = rel.contains(QLatin1String("stylesheet"));
        for (const QString &r : rel) {
            // `alternate stylesheet` está apagada; un `alternate` solo (la
            // versión en otro idioma, el RSS) se queda como enlace.
            if (fuera.contains(r) && (r != QLatin1String("alternate") || hoja)) {
                t.quitar = true;
                return;
            }
        }
        if (hoja) {
            const QString abs = absoluta(t.de(QStringLiteral("href")), base);
            if (abs.isEmpty()) return;
            if (rec) rec->hojas << abs;
            if (b) {
                if (b->textos.contains(abs)) {
                    QString estilo = QStringLiteral("<style");
                    const QString medios = t.de(QStringLiteral("media"));
                    if (!medios.isEmpty()) estilo += QStringLiteral(" media=\"") + comillas(medios) + QLatin1Char('"');
                    t.reemplazo = estilo + QStringLiteral(">\n") + pagina::montarCss(b->textos.value(abs), QUrl(abs), *b)
                                  + QStringLiteral("\n</style>");
                } else {
                    t.poner(QStringLiteral("href"), abs);
                }
            }
        } else if (rel.contains(QLatin1String("icon")) || rel.contains(QLatin1String("apple-touch-icon"))) {
            aDatos(QStringLiteral("href"), t.de(QStringLiteral("href")));
        } else if (t.tiene(QStringLiteral("href"))) {
            t.poner(QStringLiteral("href"), entera(t.de(QStringLiteral("href")), base));
        }
    } else if (n == QLatin1String("img")) {
        // Sin scripts nadie carga las perezosas: la de verdad suele estar en
        // un `data-…`, y en `src` un hueco o un difuminado diminuto.
        QString url = t.de(QStringLiteral("src")).trimmed();
        const bool vacia = url.isEmpty() || (url.startsWith(QLatin1String("data:"), Qt::CaseInsensitive) && url.size() < 200);
        if (vacia) {
            for (const char *a : { "data-src", "data-lazy-src", "data-original", "data-url" }) {
                const QString v = t.de(QString::fromLatin1(a)).trimmed();
                if (!v.isEmpty()) {
                    url = v;
                    break;
                }
            }
        }
        QString srcset = t.de(QStringLiteral("data-srcset"));
        if (srcset.isEmpty()) srcset = t.de(QStringLiteral("srcset"));
        const QString mayor = pagina::mayorDeSrcset(srcset);
        if (!mayor.isEmpty()) url = mayor;
        for (const char *a : { "srcset", "sizes", "loading", "decoding", "data-src", "data-srcset",
                               "data-lazy-src", "data-original", "data-url" })
            t.quitarAtributo(QString::fromLatin1(a));
        if (url.isEmpty()) return;
        if (url.startsWith(QLatin1String("data:"), Qt::CaseInsensitive)) {
            if (b) t.poner(QStringLiteral("src"), url);
            return;
        }
        const QString abs = absoluta(url, base);
        if (abs.isEmpty()) return;
        if (rec) rec->datos << abs;
        if (b) t.poner(QStringLiteral("src"), b->datos.value(abs, abs));
    } else if (n == QLatin1String("source")) {
        // Las de `<picture>` pedirían a la web otra que no está dentro: se
        // queda la del `<img>`. Las de un vídeo siguen apuntando a la web.
        if (t.tiene(QStringLiteral("srcset")) || t.tiene(QStringLiteral("data-srcset"))) t.quitar = true;
        else if (t.tiene(QStringLiteral("src"))) t.poner(QStringLiteral("src"), entera(t.de(QStringLiteral("src")), base));
    } else if (n == QLatin1String("video") && t.tiene(QStringLiteral("poster"))) {
        aDatos(QStringLiteral("poster"), t.de(QStringLiteral("poster")));
    }

    // Lo que se queda fuera, con la dirección entera: relativa no apuntaría a
    // nada desde la biblioteca.
    static const QHash<QString, QString> fuera = {
        { "a", "href" },        { "area", "href" },   { "form", "action" }, { "iframe", "src" },
        { "video", "src" },     { "audio", "src" },   { "track", "src" },   { "embed", "src" },
        { "object", "data" },
    };
    const auto it = fuera.constFind(n);
    if (it != fuera.constEnd() && t.tiene(it.value()))
        t.poner(it.value(), entera(t.de(it.value()), base));
}

/// Recorre la página: con `rec`, apunta lo que hay que bajar; con `b`,
/// devuelve la página con lo bajado dentro.
QString recorrer(const QString &html, const QUrl &base, const pagina::Bajado *b, pagina::Recursos *rec)
{
    QString salida;
    if (b) salida.reserve(html.size() * 2);
    const qsizetype n = html.size();
    qsizetype i = 0;
    while (i < n) {
        const qsizetype lt = html.indexOf(QLatin1Char('<'), i);
        if (lt < 0) {
            salida += QStringView(html).mid(i);
            break;
        }
        salida += QStringView(html).mid(i, lt - i);
        if (html.mid(lt, 4) == QLatin1String("<!--")) {
            const qsizetype f = html.indexOf(QLatin1String("-->"), lt + 4);
            const qsizetype fin = f < 0 ? n : f + 3;
            salida += QStringView(html).mid(lt, fin - lt);
            i = fin;
            continue;
        }
        if (lt + 1 >= n) {
            salida += QLatin1Char('<');
            break;
        }
        const QChar c = html[lt + 1];
        if (c == QLatin1Char('/') || c == QLatin1Char('!') || c == QLatin1Char('?')) {
            const qsizetype f = html.indexOf(QLatin1Char('>'), lt);
            const qsizetype fin = f < 0 ? n : f + 1;
            const QString trozo = html.mid(lt, fin - lt);
            // El cierre de un `<noscript>` desenvuelto (ver abajo).
            if (!trozo.startsWith(QLatin1String("</noscript"), Qt::CaseInsensitive)) salida += trozo;
            i = fin;
            continue;
        }
        if (!c.isLetter()) {
            salida += QLatin1Char('<');
            i = lt + 1;
            continue;
        }
        Etiqueta t;
        const qsizetype fin = leerEtiqueta(html, lt, t);
        if (fin < 0) {
            salida += QStringView(html).mid(lt);
            break;
        }
        const QString crudo = html.mid(lt, fin - lt);
        if (t.nombre == QLatin1String("script")) {
            i = trasCierre(html, finDeBloque(html, fin, QStringLiteral("script")));
            continue;
        }
        if (t.nombre == QLatin1String("style") || t.nombre == QLatin1String("title")
                || t.nombre == QLatin1String("textarea")) {
            const qsizetype f = finDeBloque(html, fin, t.nombre);
            const qsizetype g = trasCierre(html, f);
            QString dentro = html.mid(fin, f - fin);
            if (t.nombre == QLatin1String("style")) {
                if (rec) rec->cssDentro << dentro;
                if (b) dentro = pagina::montarCss(dentro, base, *b);
            } else if (t.nombre == QLatin1String("title") && rec && rec->titulo.isEmpty()) {
                rec->titulo = desentidades(dentro).simplified();
            }
            salida += crudo + dentro + html.mid(f, g - f);
            i = g;
            continue;
        }
        if (t.nombre == QLatin1String("noscript")) {
            // Lo que se enseña sin scripts, que es justo esta copia. Si lleva
            // imágenes (la versión sin scripts de una perezosa) se desenvuelve;
            // si no, suele ser un «activa JavaScript» y se quita.
            const qsizetype f = finDeBloque(html, fin, QStringLiteral("noscript"));
            if (html.mid(fin, f - fin).contains(QLatin1String("<img"), Qt::CaseInsensitive)) {
                i = fin;
            } else {
                i = trasCierre(html, f);
            }
            continue;
        }
        tratar(t, base, b, rec);
        if (b) {
            if (!t.reemplazo.isEmpty()) salida += t.reemplazo;
            else if (!t.quitar) salida += t.texto();
        }
        i = fin;
    }
    return salida;
}

QString mimeDe(const QString &cabecera, const QString &url, const QByteArray &datos)
{
    QString m = cabecera.section(QLatin1Char(';'), 0, 0).trimmed().toLower();
    if (m.isEmpty() || m == QLatin1String("application/octet-stream") || m == QLatin1String("binary/octet-stream")) {
        static const QMimeDatabase db;
        m = db.mimeTypeForFileNameAndData(QUrl(url).fileName(), datos).name();
    }
    return m;
}

} // namespace

namespace pagina {

QString decodificar(const QByteArray &bytes, const QString &tipo)
{
    static const QRegularExpression cs(QStringLiteral("charset\\s*=\\s*[\"']?([A-Za-z0-9_.:-]+)"),
                                       QRegularExpression::CaseInsensitiveOption);
    QString nombre = cs.match(tipo).captured(1);
    if (nombre.isEmpty()) nombre = cs.match(QString::fromLatin1(bytes.left(4096))).captured(1);
    nombre = nombre.toLower();
    // Qt solo trae las Unicode y Latin-1; Windows-1252 es Latin-1 salvo unas
    // comillas, que es mejor que nada.
    if (nombre == QLatin1String("iso-8859-1") || nombre == QLatin1String("latin1")
            || nombre == QLatin1String("windows-1252") || nombre == QLatin1String("us-ascii"))
        return QString::fromLatin1(bytes);
    auto enc = QStringConverter::encodingForName(nombre.toLatin1().constData());
    if (enc && *enc != QStringConverter::Utf8) {
        QStringDecoder d(*enc);
        return d.decode(bytes);
    }
    return QString::fromUtf8(bytes);
}

QUrl redireccion(const QString &html, const QUrl &base)
{
    static const QRegularExpression meta(QStringLiteral("<meta\\b[^>]*>"), QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression refresh(QStringLiteral("http-equiv\\s*=\\s*[\"']?refresh"),
                                            QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression destino(QStringLiteral("content\\s*=\\s*[\"']\\s*(\\d+)\\s*[;,]\\s*url\\s*=\\s*['\"]?([^\"'>]+)"),
                                            QRegularExpression::CaseInsensitiveOption);
    auto it = meta.globalMatch(html);
    while (it.hasNext()) {
        const QString e = it.next().captured(0);
        if (!refresh.match(e).hasMatch()) continue;
        const auto m = destino.match(e);
        // Una que salta enseguida es una redirección; una que recarga cada
        // minuto es otra cosa.
        if (!m.hasMatch() || m.captured(1).toInt() > 10) continue;
        const QString abs = absoluta(desentidades(m.captured(2)), base);
        if (!abs.isEmpty()) return QUrl(abs);
    }
    return {};
}

QUrl baseDe(const QString &html, const QUrl &url)
{
    static const QRegularExpression base(QStringLiteral("<base\\b[^>]*\\bhref\\s*=\\s*[\"']([^\"']+)[\"']"),
                                         QRegularExpression::CaseInsensitiveOption);
    const auto m = base.match(html);
    if (!m.hasMatch()) return url;
    const QUrl b = url.resolved(QUrl(desentidades(m.captured(1)).trimmed()));
    return b.isValid() ? b : url;
}

Recursos recoger(const QString &html, const QUrl &base)
{
    Recursos r;
    recorrer(html, base, nullptr, &r);
    r.hojas.removeDuplicates();
    r.datos.removeDuplicates();
    return r;
}

namespace {
const QRegularExpression &reImport()
{
    static const QRegularExpression re(
        QStringLiteral("@import\\s+(?:url\\(\\s*([\"']?)([^\"')]+)\\1\\s*\\)|([\"'])([^\"']+)\\3)\\s*([^;]*);"),
        QRegularExpression::CaseInsensitiveOption);
    return re;
}
const QRegularExpression &reUrl()
{
    static const QRegularExpression re(QStringLiteral("url\\(\\s*([\"']?)([^\"')]*?)\\1\\s*\\)"),
                                       QRegularExpression::CaseInsensitiveOption);
    return re;
}
} // namespace

QStringList importsDeCss(const QString &css, const QUrl &base)
{
    QStringList r;
    auto it = reImport().globalMatch(css);
    while (it.hasNext()) {
        const auto m = it.next();
        const QString abs = absoluta(m.captured(2).isEmpty() ? m.captured(4) : m.captured(2), base);
        if (!abs.isEmpty()) r << abs;
    }
    return r;
}

QStringList urlsDeCss(const QString &css, const QUrl &base)
{
    QString sin = css;
    sin.replace(reImport(), QString());
    QStringList r;
    auto it = reUrl().globalMatch(sin);
    while (it.hasNext()) {
        const QString u = it.next().captured(2);
        if (u.startsWith(QLatin1Char('#')) || u.startsWith(QLatin1String("about:"))) continue;
        const QString abs = absoluta(u, base);
        if (!abs.isEmpty()) r << abs;
    }
    r.removeDuplicates();
    return r;
}

QString montarCss(const QString &css, const QUrl &base, const Bajado &b, int profundidad)
{
    QString paso;
    qsizetype ultimo = 0;
    auto it = reImport().globalMatch(css);
    while (it.hasNext()) {
        const auto m = it.next();
        paso += QStringView(css).mid(ultimo, m.capturedStart() - ultimo);
        ultimo = m.capturedEnd();
        const QString abs = absoluta(m.captured(2).isEmpty() ? m.captured(4) : m.captured(2), base);
        const QString medios = m.captured(5).trimmed();
        if (abs.isEmpty()) continue;
        if (profundidad < PROFUNDIDAD_IMPORT && b.textos.contains(abs)) {
            const QString dentro = montarCss(b.textos.value(abs), QUrl(abs), b, profundidad + 1);
            paso += medios.isEmpty() ? dentro : QStringLiteral("@media ") + medios + QStringLiteral(" {\n") + dentro + QStringLiteral("\n}");
        } else {
            paso += QStringLiteral("@import url(\"") + abs + QStringLiteral("\") ") + medios + QLatin1Char(';');
        }
    }
    paso += QStringView(css).mid(ultimo);

    QString r;
    ultimo = 0;
    it = reUrl().globalMatch(paso);
    while (it.hasNext()) {
        const auto m = it.next();
        r += QStringView(paso).mid(ultimo, m.capturedStart() - ultimo);
        ultimo = m.capturedEnd();
        const QString u = m.captured(2);
        const QString abs = (u.startsWith(QLatin1Char('#')) || u.startsWith(QLatin1String("about:"))) ? QString() : absoluta(u, base);
        if (abs.isEmpty()) r += m.captured(0);
        else r += QStringLiteral("url(\"") + b.datos.value(abs, abs) + QStringLiteral("\")");
    }
    r += QStringView(paso).mid(ultimo);
    return r;
}

QString mayorDeSrcset(const QString &srcset)
{
    // Cada candidata es «dirección descriptor», separadas por comas seguidas
    // de espacio (una dirección puede llevar comas dentro).
    const QStringList partes = srcset.split(QRegularExpression(QStringLiteral(",\\s+")), Qt::SkipEmptyParts);
    QString mejor, menor;
    double vMejor = -1, vMenor = 1e18;
    for (QString p : partes) {
        p = p.trimmed();
        if (p.endsWith(QLatin1Char(','))) p.chop(1);
        const QStringList t = p.split(QRegularExpression(QStringLiteral("\\s+")), Qt::SkipEmptyParts);
        if (t.isEmpty()) continue;
        double v = 1;
        bool ancho = false;
        if (t.size() > 1) {
            const QString d = t[1].toLower();
            ancho = d.endsWith(QLatin1Char('w'));
            v = d.left(d.size() - 1).toDouble();
        }
        // Se compara en anchos: «1x» cuenta como una de 700 px, «2x» de 1400.
        const double w = ancho ? v : v * 700;
        if (w <= ANCHO_BUENO && w > vMejor) {
            vMejor = w;
            mejor = t[0];
        }
        if (w < vMenor) {
            vMenor = w;
            menor = t[0];
        }
    }
    return mejor.isEmpty() ? menor : mejor;
}

QString montar(const QString &html, const QUrl &base, const Bajado &b, const QString &origen,
               const QString &fecha, const QString &captura)
{
    QString r = recorrer(html, base, &b, nullptr);
    auto meta = [](const QString &nombre, const QString &valor) {
        return QStringLiteral("<meta name=\"") + nombre + QStringLiteral("\" content=\"") + comillas(valor) + QStringLiteral("\">\n");
    };
    // Al principio del `<head>`: la codificación, que tiene que ir antes de
    // cualquier texto, y lo que Grimorio lee sin abrir el archivo entero —la
    // captura es su miniatura—.
    QString delante = QStringLiteral("\n<meta charset=\"utf-8\">\n") + meta(QStringLiteral("grimorio:origen"), origen)
                      + meta(QStringLiteral("grimorio:guardada"), fecha);
    if (captura.startsWith(QLatin1String("data:image/"))) delante += meta(QStringLiteral("grimorio:captura"), captura);

    QString o = origen;
    const QString comentario = QStringLiteral("<!-- Guardada con Grimorio desde ")
                               + o.replace(QLatin1String("--"), QLatin1String("%2D%2D"))
                               + QStringLiteral(" el ") + fecha + QStringLiteral(" -->\n");
    static const QRegularExpression doctype(QStringLiteral("^\\s*<!doctype[^>]*>\\s*"), QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression head(QStringLiteral("<head(\\s[^>]*)?>"), QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression raiz(QStringLiteral("<html(\\s[^>]*)?>"), QRegularExpression::CaseInsensitiveOption);
    if (const auto m = head.match(r); m.hasMatch()) {
        r.insert(m.capturedEnd(), delante);
    } else if (const auto m2 = raiz.match(r); m2.hasMatch()) {
        r.insert(m2.capturedEnd(), QStringLiteral("<head>") + delante + QStringLiteral("</head>"));
    } else {
        const auto d = doctype.match(r);
        r.insert(d.hasMatch() ? d.capturedEnd() : 0, QStringLiteral("<head>") + delante + QStringLiteral("</head>\n"));
    }
    if (const auto m = doctype.match(r); m.hasMatch()) {
        r.insert(m.capturedEnd(), comentario);
    } else {
        r.prepend(QStringLiteral("<!DOCTYPE html>\n") + comentario);
    }
    return r;
}

} // namespace pagina

ArchivarPagina::ArchivarPagina(QNetworkAccessManager *red, QObject *padre)
    : QObject(padre), m_red(red)
{
}

void ArchivarPagina::empezar(const QByteArray &html, const QString &tipo, const QUrl &url, const QString &origen)
{
    m_url = url;
    m_origen = origen;
    m_html = pagina::decodificar(html, tipo);
    m_base = pagina::baseDe(m_html, url);
    m_rec = pagina::recoger(m_html, m_base);
    for (const QString &h : std::as_const(m_rec.hojas)) pedir(h, true, 0);
    for (const QString &css : std::as_const(m_rec.cssDentro))
        for (const QString &i : pagina::importsDeCss(css, m_base)) pedir(i, true, 1);
    if (m_pendientes == 0) cuandoTermine();
}

void ArchivarPagina::pedir(const QString &url, bool texto, int profundidad)
{
    const QString clave = (texto ? QStringLiteral("t ") : QStringLiteral("d ")) + url;
    if (m_pedidos.contains(clave) || m_gastado >= PRESUPUESTO) return;
    m_pedidos.insert(clave);
    QNetworkRequest pide { QUrl(url) };
    pide.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("Mozilla/5.0 (X11; Linux x86_64) Grimorio"));
    // Hay CDN que no sirven una imagen si no saben desde qué página se pide.
    pide.setRawHeader("Referer", m_url.toString(QUrl::FullyEncoded).toUtf8());
    pide.setTransferTimeout(20000);
    QNetworkReply *r = m_red->get(pide);
    ++m_pendientes;
    connect(r, &QNetworkReply::downloadProgress, r, [r](qint64 recibido, qint64 total) {
        if (recibido > TOPE_RECURSO || total > TOPE_RECURSO) r->abort();
    });
    connect(r, &QNetworkReply::finished, this, [=] {
        r->deleteLater();
        if (r->error() == QNetworkReply::NoError) {
            const QByteArray bytes = r->readAll();
            if (texto) {
                const QString css = QString::fromUtf8(bytes);
                if (m_gastado + css.size() <= PRESUPUESTO) {
                    m_bajado.textos.insert(url, css);
                    m_gastado += css.size();
                    if (profundidad < PROFUNDIDAD_IMPORT)
                        for (const QString &i : pagina::importsDeCss(css, QUrl(url))) pedir(i, true, profundidad + 1);
                }
            } else if (!bytes.isEmpty()) {
                const QString mime = mimeDe(r->header(QNetworkRequest::ContentTypeHeader).toString(), url, bytes);
                const QString d = QStringLiteral("data:") + mime + QStringLiteral(";base64,") + QString::fromLatin1(bytes.toBase64());
                if (m_gastado + d.size() <= PRESUPUESTO) {
                    m_bajado.datos.insert(url, d);
                    m_gastado += d.size();
                }
            }
        }
        if (--m_pendientes == 0) cuandoTermine();
    });
}

void ArchivarPagina::cuandoTermine()
{
    if (m_fase == 0) {
        // Ya están todas las hojas: ahora lo que piden ellas y la página.
        m_fase = 1;
        QStringList datos = m_rec.datos;
        for (auto it = m_bajado.textos.cbegin(); it != m_bajado.textos.cend(); ++it)
            datos += pagina::urlsDeCss(it.value(), QUrl(it.key()));
        for (const QString &css : std::as_const(m_rec.cssDentro)) datos += pagina::urlsDeCss(css, m_base);
        // La portada primero: es la miniatura, y si el presupuesto se acaba
        // que no sea con ella fuera.
        if (!m_rec.portada.isEmpty()) datos.prepend(m_rec.portada);
        datos.removeDuplicates();
        for (const QString &d : std::as_const(datos)) pedir(d, false, 0);
        if (m_pendientes == 0) montarYAcabar();
    } else if (m_fase == 1) {
        montarYAcabar();
    }
}

void ArchivarPagina::montarYAcabar()
{
    m_fase = 2;
    // La miniatura: la imagen que la página dice que la representa. Grimorio la lee con su núcleo, que no dibuja SVG.
    auto vale = [](const QString &d) {
        return d.startsWith(QLatin1String("data:image/")) && !d.startsWith(QLatin1String("data:image/svg"))
               && d.size() < (6 << 20);
    };
    QString captura;
    if (!m_rec.portada.isEmpty() && vale(m_bajado.datos.value(m_rec.portada)))
        captura = m_bajado.datos.value(m_rec.portada);
    // Si no dice, la imagen más pesada de las suyas, que suele ser la foto del
    // artículo y no el logo ni un icono.
    if (captura.isEmpty()) {
        for (const QString &u : std::as_const(m_rec.datos)) {
            const QString d = m_bajado.datos.value(u);
            if (vale(d) && d.size() > 8000 && d.size() > captura.size()) captura = d;
        }
    }
    const QString html = pagina::montar(m_html, m_base, m_bajado, m_origen,
                                        QDateTime::currentDateTimeUtc().toString(Qt::ISODate), captura);
    emit hecha(html.toUtf8(), m_rec.titulo);
    deleteLater();
}
