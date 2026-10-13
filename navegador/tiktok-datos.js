// Qué guardar de TikTok: de qué post es lo que hay bajo el ratón, y dónde
// está el vídeo o la foto entera.
//
// Va aparte de tiktok.js para poder probarlo sin navegador (ver
// pruebas/tiktok.test.mjs). Como con Instagram, nada de aquí usa las clases
// de TikTok, que cambian: se apoya en los enlaces de los posts, en el id del
// reproductor y en los datos que la propia página del post trae dentro.
//
// El vídeo de TikTok se reproduce desde un «blob:», así que lo que se ve no
// tiene dirección que bajar. La dirección está en la página del post: la
// pide tiktok.js y la leen estas funciones.
"use strict";

const RE_POST_TIKTOK = /^\/@([^/?#]+)\/(video|photo)\/(\d+)/;

/// La dirección limpia de un post («https://www.tiktok.com/@autor/video/123»)
/// a partir de un enlace o de la dirección de la página, o null si no lo es.
function enlaceTikTok(href, loc) {
  if (!href) return null;
  let u;
  try {
    u = new URL(href, loc.href);
  } catch (_) {
    return null;
  }
  const m = u.pathname.match(RE_POST_TIKTOK);
  return m ? `${u.origin}/@${m[1]}/${m[2]}/${m[3]}` : null;
}

/// El autor de un enlace a un perfil («/@autor»), o null.
function autorDeEnlace(href) {
  const m = (href || "").match(/^(?:https:\/\/www\.tiktok\.com)?\/@([^/?#]+)\/?$/);
  return m ? m[1] : null;
}

/// El post de lo que hay bajo el ratón:
/// - en una cuadrícula (perfil, explorar, relacionados), el enlace que
///   envuelve la miniatura;
/// - en el feed, el id que lleva el reproductor («xgwrapper-0-<id>») y el
///   autor del enlace a su perfil; TikTok contesta igual con cualquier autor,
///   así que sin él vale «_»;
/// - si no, la página, cuando es la de un post.
function postTikTok(el, loc) {
  const a = el.closest("a[href]");
  const deEnlace = a && enlaceTikTok(a.getAttribute("href"), loc);
  if (deEnlace) return deEnlace;
  const pagina = enlaceTikTok(loc.href, loc);
  const reproductor = el.closest('[id^="xgwrapper-"]');
  const id = reproductor && (reproductor.id.match(/(\d{6,})$/) || [])[1];
  if (!id) return pagina;
  if (pagina && pagina.endsWith("/" + id)) return pagina;
  let autor = null;
  const caja = el.closest("article");
  if (caja) {
    for (const enlace of caja.querySelectorAll('a[href*="/@"]')) {
      autor = autorDeEnlace(enlace.getAttribute("href"));
      if (autor) break;
    }
  }
  return `${loc.origin}/@${autor || "_"}/video/${id}`;
}

/// Los datos del post que trae su página, o null si no los trae (pide
/// iniciar sesión, está borrado…).
function itemDelHtml(html) {
  const m = String(html).match(/<script[^>]*id="__UNIVERSAL_DATA_FOR_REHYDRATION__"[^>]*>([\s\S]*?)<\/script>/);
  if (!m) return null;
  try {
    const d = JSON.parse(m[1]).__DEFAULT_SCOPE__["webapp.video-detail"];
    return (d && d.itemInfo && d.itemInfo.itemStruct) || null;
  } catch (_) {
    return null;
  }
}

/// Las direcciones de un vídeo que dan el archivo. La de «/aweme/v1/play/»
/// contesta una página, no el vídeo.
function direccionesDeVideo(lista) {
  return (lista || []).filter((u) => typeof u === "string" && /^https:/.test(u) && !/\/aweme\/v1\/play\//.test(u));
}

/// Lo que tiene el post, cada cosa con la dirección buena y las de repuesto:
/// TikTok da cada archivo en varios servidores, y desde el fondo de la
/// extensión unos contestan 403 y otros no (ver `guardarUno` en fondo.js).
///
/// De un vídeo, la versión más grande, aunque sea H.265 —Grimorio lo lee con
/// ffmpeg—; a igual tamaño, H.264. Detrás, las demás versiones, y al final la
/// de descarga, que lleva la marca de agua.
function mediosTikTok(item) {
  if (!item) return [];
  const fotos = item.imagePost && item.imagePost.images;
  if (fotos && fotos.length) {
    return fotos.map((f) => {
      const urls = (f.imageURL && f.imageURL.urlList || []).filter((u) => /^https:/.test(u));
      return { url: urls[0], alternativas: urls.slice(1), ancho: f.imageWidth || 0, alto: f.imageHeight || 0, tipo: "imagen" };
    }).filter((m) => m.url);
  }
  const v = item.video || {};
  const versiones = (v.bitrateInfo || []).map((b) => {
    const p = b.PlayAddr || {};
    return { ancho: p.Width || 0, alto: p.Height || 0, h264: !/265|hevc|hvc/i.test(b.CodecType || ""),
             urls: direccionesDeVideo(p.UrlList) };
  }).filter((x) => x.urls.length);
  versiones.sort((a, b) => b.ancho * b.alto - a.ancho * a.alto || b.h264 - a.h264);
  const urls = [...new Set([...versiones.flatMap((x) => x.urls), ...direccionesDeVideo([v.playAddr, v.downloadAddr])])];
  if (urls.length === 0) return [];
  const mejor = versiones[0] || { ancho: v.width || 0, alto: v.height || 0 };
  return [{ url: urls[0], alternativas: urls.slice(1), ancho: mejor.ancho, alto: mejor.alto, tipo: "video" }];
}

/// Lo que identifica una imagen de TikTok en cualquier tamaño: el trozo de la
/// dirección antes de «~tplv-…», que es lo que cambia con el recorte.
function claveTikTok(url) {
  const m = String(url || "").match(/\/([^/~?]+)~/);
  return m ? m[1] : null;
}

/// Cuál de los medios del post es el que se ve: en un carrusel de fotos, la
/// de la misma clave; si no se reconoce, la primera.
function elegirTikTok(medios, urlVista) {
  const clave = claveTikTok(urlVista);
  if (clave) {
    const i = medios.findIndex((m) => [m.url, ...m.alternativas].some((u) => claveTikTok(u) === clave));
    if (i >= 0) return i;
  }
  return 0;
}

/// Un nombre que se pueda buscar luego: el autor y el texto del post.
function nombreTikTok(item, enlace) {
  const autor = item && item.author && item.author.uniqueId;
  const texto = String((item && item.desc) || "").replace(/\s+/g, " ").slice(0, 80).trim();
  const id = (String(enlace || "").match(/\/(\d+)$/) || [])[1];
  const partes = [autor ? "@" + autor : null, texto || (id ? "tiktok " + id : null)];
  return partes.filter(Boolean).join(" · ") || "tiktok";
}

// Para las pruebas en Node; en el navegador esto no existe y no hace nada.
if (typeof module !== "undefined") {
  module.exports = {
    enlaceTikTok, autorDeEnlace, postTikTok, itemDelHtml, direccionesDeVideo,
    mediosTikTok, claveTikTok, elegirTikTok, nombreTikTok,
  };
}
