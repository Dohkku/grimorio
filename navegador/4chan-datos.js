// Qué guardar de 4chan: el archivo entero, no la miniatura, con su nombre
// original y el enlace al post.
//
// Va aparte de 4chan.js para poder probarlo sin navegador (ver
// pruebas/4chan.test.mjs). En un hilo cada archivo es un `.file` con dos
// enlaces al original —el del nombre y el que envuelve la miniatura— y la
// miniatura es otra imagen, «…s.jpg», de 250 px como mucho. El clic derecho
// del navegador da la miniatura; esto da el original.
"use strict";

/// El medio que se guardaría bajo el ratón: la miniatura o el archivo ya
/// abierto en su sitio (imagen o vídeo). Sin capas encima, basta el destino.
function medioDe4chan(el) {
  if (!el || !el.closest) return null;
  const m = el.closest("img, video");
  if (!m) return null;
  // Iconos de la página —banderas, chinchetas, el logo— no.
  if (m.closest(".file") || m.closest("a.catalog-thumb, .thread a[href*='/thread/']") || m.classList.contains("thumb")) return m;
  return null;
}

/// «nombre.png» sin la extensión, que Grimorio ya sabe por el archivo.
function sinExtension(nombre) {
  return String(nombre || "").replace(/\.[A-Za-z0-9]{2,5}$/, "").trim();
}

/// Lo que se manda a guardar para `el`, con `como` para el aviso; null si no
/// es de un post.
function archivo4chan(el, loc) {
  const caja = el.closest(".file");
  if (!caja) {
    // El catálogo solo tiene la miniatura y el enlace al hilo.
    const src = el.currentSrc || el.getAttribute("src");
    if (!src) return null;
    const hilo = el.closest("a[href*='/thread/']");
    // El nombre, del asunto del hilo o, sin asunto, de cómo empieza.
    const tarjeta = el.closest(".thread");
    const resumen = tarjeta && tarjeta.querySelector(".teaser");
    const asunto = resumen && ((resumen.querySelector("b") || {}).textContent || resumen.textContent);
    const tablon = (loc.pathname.match(/^\/([^/]+)\//) || [])[1];
    const nombre = [tablon ? "/" + tablon + "/" : null, String(asunto || "").replace(/\s+/g, " ").slice(0, 80).trim() || null]
      .filter(Boolean).join(" · ");
    return { url: new URL(src, loc.href).href, origen: hilo ? new URL(hilo.getAttribute("href"), loc.href).href : loc.href,
             nombre, como: "miniatura del catálogo" };
  }
  const enlace = caja.querySelector("a.fileThumb[href]") || caja.querySelector(".fileText a[href]");
  if (!enlace) return null;
  const url = new URL(enlace.getAttribute("href"), loc.href).href;

  // El nombre original: si el archivo va con spoiler, el enlace dice «Spoiler
  // Image» y el nombre está en el `title` de la caja; si es largo, el enlace
  // lo recorta y el entero está en su `title`.
  const texto = caja.querySelector(".fileText");
  const a = texto && texto.querySelector("a");
  const original = (texto && texto.getAttribute("title")) || (a && (a.getAttribute("title") || a.textContent)) || "";

  // El post: «f123» en la caja, o el `.post` que la tiene.
  const post = caja.closest(".post");
  const id = (caja.id.match(/^f(\d+)$/) || [])[1] || (post && (post.id.match(/(\d+)$/) || [])[1]);
  const tablon = (loc.pathname.match(/^\/([^/]+)\//) || [])[1];
  const hiloPagina = (loc.pathname.match(/^\/[^/]+\/thread\/(\d+)/) || [])[1];
  const cajaHilo = caja.closest(".thread");
  const hilo = hiloPagina || (cajaHilo && (cajaHilo.id.match(/(\d+)$/) || [])[1]);
  const origen = tablon && hilo ? `${loc.origin}/${tablon}/thread/${hilo}${id ? "#p" + id : ""}` : loc.href;

  const nombre = [tablon ? "/" + tablon + "/" : null, sinExtension(original) || null].filter(Boolean).join(" · ");
  return { url, origen, nombre, como: "original" };
}

// Para las pruebas en Node; en el navegador esto no existe y no hace nada.
if (typeof module !== "undefined") {
  module.exports = { medioDe4chan, archivo4chan, sinExtension };
}
