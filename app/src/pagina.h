// Una página web entera desde su dirección, para leerla sin conexión.
//
// Es lo mismo que hace la extensión del navegador (navegador/pagina.js) pero
// sin navegador: al pegar o soltar en Grimorio la dirección de una página, se
// baja su HTML, se le quitan los scripts y se le meten dentro las hojas de
// estilo, las imágenes y las tipografías como `data:`. Sale un solo `.html`
// con el mismo formato que el de la extensión (ver
// crates/grimorio-core/src/web.rs), que entra en la biblioteca como página web.
//
// La diferencia es que aquí nadie ejecuta los scripts de la página: se guarda
// lo que manda el servidor, no lo que pinta el navegador después. Un artículo,
// un blog o una wiki salen bien; una web hecha entera con JavaScript (X,
// Instagram) puede salir casi vacía, y para esas está la extensión. Las
// imágenes perezosas (`data-src`, las de `<noscript>`) se rescatan, que sin
// scripts nadie las cargaría.
//
// La miniatura es la imagen que la página dice que la representa (og:image),
// o la más grande de las suyas: sin navegador no hay captura de la pestaña.
//
// Lo que no depende de la red (limpiar, recoger, montar) son funciones sueltas
// para poder probarlas (pruebas/pagina_test.cpp); `ArchivarPagina` es quien
// baja.
#pragma once
#include <QByteArray>
#include <QHash>
#include <QObject>
#include <QSet>
#include <QString>
#include <QStringList>
#include <QUrl>

class QNetworkAccessManager;

namespace pagina {

/// Lo que hace falta bajar para montar una página.
struct Recursos {
    /// Hojas de estilo enlazadas, en texto.
    QStringList hojas;
    /// Imágenes, iconos y carteles de vídeo, en datos.
    QStringList datos;
    /// El CSS escrito dentro de la página (`<style>` y atributos `style`), que
    /// también puede pedir cosas: `@import`, `url()`.
    QStringList cssDentro;
    /// La imagen que la representa, para la miniatura ("" si no dice).
    QString portada;
    QString titulo;
};

/// Lo ya bajado: dirección → texto (hojas de estilo) y dirección → URL `data:`.
struct Bajado {
    QHash<QString, QString> textos;
    QHash<QString, QString> datos;
};

/// El HTML en texto, con la codificación que diga la cabecera
/// (`text/html; charset=…`) o la propia página, y si no, UTF-8.
QString decodificar(const QByteArray &bytes, const QString &tipo);

/// A dónde manda un `<meta http-equiv="refresh">` que salta enseguida, o
/// nada. Sin scripts nadie lo seguiría y se guardaría el «redirigiendo…».
QUrl redireccion(const QString &html, const QUrl &base);

/// La dirección contra la que se resuelven las relativas: `<base href>` si la
/// página la tiene, si no, la suya.
QUrl baseDe(const QString &html, const QUrl &url);

/// Lo que hay que bajar de `html`.
Recursos recoger(const QString &html, const QUrl &base);

/// Los `@import` de una hoja, ya en dirección entera.
QStringList importsDeCss(const QString &css, const QUrl &base);
/// Las `url()` de una hoja (sin las de `@import`), ya en dirección entera.
QStringList urlsDeCss(const QString &css, const QUrl &base);
/// La hoja con sus `@import` dentro y cada `url()` cambiada por sus datos, o
/// por la dirección entera si no se bajó.
QString montarCss(const QString &css, const QUrl &base, const Bajado &b, int profundidad = 0);

/// La página en un solo HTML. `captura` es la miniatura (una URL
/// `data:image/…`, o "").
QString montar(const QString &html, const QUrl &base, const Bajado &b, const QString &origen,
               const QString &fecha, const QString &captura);

/// De todas las de un `srcset`, la más grande.
QString mayorDeSrcset(const QString &srcset);

} // namespace pagina

/// Baja una página y lo que lleva y la deja montada. Se borra sola al acabar.
class ArchivarPagina : public QObject {
    Q_OBJECT
public:
    ArchivarPagina(QNetworkAccessManager *red, QObject *padre = nullptr);

    /// `html`, lo que contestó `url` (ya bajado); `tipo`, su Content-Type.
    void empezar(const QByteArray &html, const QString &tipo, const QUrl &url, const QString &origen);

signals:
    /// La página montada, en UTF-8, y su título.
    void hecha(const QByteArray &html, const QString &titulo);

private:
    void pedir(const QString &url, bool texto, int profundidad);
    void cuandoTermine();
    void montarYAcabar();

    QNetworkAccessManager *m_red;
    QString m_html;
    QUrl m_base;
    QUrl m_url;
    QString m_origen;
    pagina::Recursos m_rec;
    pagina::Bajado m_bajado;
    QSet<QString> m_pedidos;
    /// Hojas bajadas y desde qué profundidad de `@import`.
    QHash<QString, int> m_profundidad;
    int m_pendientes = 0;
    /// En qué vuelta está: 0 las hojas (y sus `@import`), 1 lo demás.
    int m_fase = 0;
    qint64 m_gastado = 0;
};
