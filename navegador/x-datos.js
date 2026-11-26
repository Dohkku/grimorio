// Qué guardar de X (Twitter): de qué tuit es lo que hay bajo el ratón, la foto
// a su tamaño original y dónde está el vídeo entero.
//
// Va aparte de x.js para poder probarlo sin navegador (ver
// pruebas/x.test.mjs). Como con Instagram y TikTok, nada de aquí usa las
// clases de X, que son nombres sin sentido que cambian: se apoya en los
// enlaces a los tuits, en las direcciones de pbs.twimg.com y en los datos que
// da el mismo servicio con el que X pinta los tuits incrustados en otras webs.
//
// Una foto se ve en una dirección con su tamaño («?format=jpg&name=small»):
// basta pedir «name=orig» para tener la que se subió. Un vídeo se reproduce
// desde un «blob:», sin dirección que bajar; su portada sí la tiene, y lleva
// dentro el id del vídeo, con el que se reconoce entre los del tuit.
"use strict";

const RE_TUIT = /^\/(?:([A-Za-z0-9_]{1,30})|i(?:\/web)?)\/status\/(\d+)/;

/// El enlace limpio de un tuit («https://x.com/autor/status/123») a partir de
/// un enlace o de la dirección de la página, o null si no lo es.
function enlaceX(href, loc) {
  if (!href) return null;
  let u;
  try {
    u = new URL(href, loc.href);
  } catch (_) {
    return null;
  }
  if (!/^((www|mobile)\.)?(x|twitter)\.com$/.test(u.hostname)) return null;
  const m = u.pathname.match(RE_TUIT);
  if (!m) return null;
  return m[1] && m[1] !== "i" ? `https://x.com/${m[1]}/status/${m[2]}` : `https://x.com/i/status/${m[2]}`;
}

function idDeTuit(enlace) {
  return (String(enlace || "").match(/\/status\/(\d+)/) || [])[1] || null;
}

/// El tuit de lo que hay bajo el ratón:
/// - una foto es un enlace a «/autor/status/ID/photo/N»;
/// - si no, el tuit en el que está: en su `article`, el enlace que envuelve
///   la hora, que es el enlace permanente (los demás pueden ser de un tuit
///   citado o de respuestas);
/// - si no, la página, cuando es la de un tuit (también la del visor de fotos).
function tuitX(el, loc) {
  for (let a = el.closest("a[href]"); a; a = a.parentElement && a.parentElement.closest("a[href]")) {
    const e = enlaceX(a.getAttribute("href"), loc);
    if (e) return e;
  }
  const caja = el.closest("article");
  if (caja) {
    for (const t of caja.querySelectorAll("a[href] time")) {
      const e = enlaceX(t.closest("a").getAttribute("href"), loc);
      if (e) return e;
    }
    for (const a of caja.querySelectorAll('a[href*="/status/"]')) {
      const e = enlaceX(a.getAttribute("href"), loc);
      if (e) return e;
    }
  }
  return enlaceX(loc.href, loc);
}

/// La foto a su tamaño original. Vale para las del tuit (`/media/`), las
/// portadas de los vídeos y las tarjetas; lo demás se deja como está.
function fotoOriginalX(url) {
  let u;
  try {
    u = new URL(url);
  } catch (_) {
    return url;
  }
  if (u.hostname !== "pbs.twimg.com") return url;
  // La forma antigua: «/media/ID.jpg:large».
  const antigua = u.pathname.match(/^(\/media\/[^.:/]+)\.(\w+):\w+$/);
  if (antigua) return `${u.origin}${antigua[1]}?format=${antigua[2]}&name=orig`;
  if (!/^\/(media|ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb|card_img)\//.test(u.pathname)) return url;
  // «/media/ID.jpg» sin parámetros también es la forma antigua.
  const ext = u.pathname.match(/^(.*)\.(jpg|jpeg|png|webp)$/);
  if (ext && !u.searchParams.get("format")) {
    u.pathname = ext[1];
    u.searchParams.set("format", ext[2] === "jpeg" ? "jpg" : ext[2]);
  }
  // X manda webp a los navegadores que lo leen, pero el original es un jpg
  // (o un png, que se queda): mejor el de verdad, que lee cualquier programa.
  if (u.searchParams.get("format") === "webp") u.searchParams.set("format", "jpg");
  u.searchParams.set("name", "orig");
  return u.toString();
}

/// Lo que hay bajo el ratón en X. Las fotos no salen en `elementsFromPoint`
/// —X les quita los eventos del ratón (`pointer-events: none`) y pinta la
/// misma foto como fondo de una capa—, así que si `medioEnPunto` no da nada
/// se busca la foto de X cuyo trozo visible tiene debajo el ratón.
function medioBajoX(doc, x, y) {
  const medio = medioEnPunto(doc, x, y);
  if (medio) return medio;
  for (const img of doc.querySelectorAll('img[src^="https://pbs.twimg.com/"]')) {
    if (!/\/(media|card_img|ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb)\//.test(img.src)) continue;
    if (esContenido(img) && seVeEn(img, x, y)) return img;
  }
  return null;
}

/// El token que pide el servicio de los tuits incrustados: el id del tuit,
/// partido por 1e15, por π, escrito en base 36 y sin ceros ni punto. Es lo
/// que calcula su propio script.
function tokenX(id) {
  return ((Number(id) / 1e15) * Math.PI).toString(36).replace(/(0+|\.)/g, "");
}

function direccionSindicacion(id) {
  return `https://cdn.syndication.twimg.com/tweet-result?id=${id}&token=${tokenX(id)}&lang=es`;
}

/// Lo que identifica un vídeo o un gif en sus direcciones: el id que va tras
/// «_thumb/» en la portada, y tras «amplify_video/», «ext_tw_video/» o
/// «tweet_video/» en el vídeo.
function claveX(url) {
  const m = String(url || "").match(/\/(?:ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb|ext_tw_video|amplify_video|tweet_video)\/([^/.?]+)/);
  return m ? m[1] : null;
}

/// Un medio de los datos del tuit, a su mayor tamaño: de un vídeo o un gif el
/// mp4 de más calidad y, detrás, los demás por si ese no contesta.
function medioX(m) {
  if (!m) return null;
  const variantes = ((m.video_info && m.video_info.variants) || [])
    .filter((v) => v && /^https:/.test(v.url || "") && /mp4/.test(v.content_type || v.url));
  if (m.type === "video" || m.type === "animated_gif" || variantes.length) {
    if (!variantes.length) return null;
    variantes.sort((a, b) => (b.bitrate || 0) - (a.bitrate || 0));
    // El tamaño de lo que se baja va en su dirección («/vid/1280x720/»);
    // `original_info` es el de lo que se subió, que X no siempre sirve.
    const info = m.original_info || {};
    const t = variantes[0].url.match(/\/(\d+)x(\d+)\//);
    return {
      tipo: "video", url: variantes[0].url, alternativas: variantes.slice(1).map((v) => v.url),
      ancho: t ? Number(t[1]) : info.width || 0, alto: t ? Number(t[2]) : info.height || 0,
      claves: [m.media_url_https, ...variantes.map((v) => v.url)].map(claveX).filter(Boolean),
      portada: m.media_url_https || "",
    };
  }
  if (!m.media_url_https) return null;
  const info = m.original_info || {};
  return { tipo: "imagen", url: fotoOriginalX(m.media_url_https), alternativas: [], ancho: info.width || 0,
           alto: info.height || 0, claves: [], portada: m.media_url_https };
}

/// Los medios de un vídeo que va en una tarjeta («unified_card»): los de
/// anunciantes y medios grandes no van en `mediaDetails`.
function mediosDeTarjeta(card) {
  const v = card && card.binding_values && card.binding_values.unified_card;
  if (!v || !v.string_value) return [];
  try {
    return Object.values(JSON.parse(v.string_value).media_entities || {});
  } catch (_) {
    return [];
  }
}

/// Todo lo que trae un tuit según el servicio de los incrustados: sus fotos y
/// vídeos, los de su tarjeta y los del tuit que cita, que en la página se ven
/// dentro del mismo `article`.
function mediosX(datos) {
  if (!datos || datos.__typename === "TweetTombstone") return [];
  const crudos = [];
  for (const t of [datos, datos.quoted_tweet]) {
    if (!t) continue;
    crudos.push(...(t.mediaDetails || []), ...mediosDeTarjeta(t.card));
  }
  return crudos.map(medioX).filter(Boolean);
}

/// Cuál de los vídeos del tuit es el que se ve, por la portada (`poster`) del
/// reproductor; con uno solo, ese. null si no se reconoce.
function elegirVideoX(medios, poster) {
  const videos = medios.filter((m) => m.tipo === "video");
  const clave = claveX(poster);
  if (clave) {
    const v = videos.find((m) => m.claves.includes(clave));
    if (v) return v;
  }
  return videos.length === 1 ? videos[0] : null;
}

/// Un nombre que se pueda buscar luego: el autor y el texto del tuit, sin
/// los enlaces cortos «t.co» que X le pega al final.
function nombreX(datos, enlace) {
  const autor = (datos && datos.user && datos.user.screen_name) || (String(enlace || "").match(/x\.com\/([^/]+)\/status/) || [])[1];
  const texto = String((datos && datos.text) || "").replace(/https:\/\/t\.co\/\w+/g, "").replace(/\s+/g, " ").slice(0, 80).trim();
  const id = idDeTuit(enlace);
  const partes = [autor && autor !== "i" ? "@" + autor : null, texto || (id ? "x " + id : null)];
  return partes.filter(Boolean).join(" · ") || "x";
}

/// El texto del tuit en la página, por si no hay datos: el bloque marcado
/// para lectores de pantalla como el texto del tuit.
function textoEnPagina(el) {
  const caja = el.closest("article");
  const t = caja && caja.querySelector('[data-testid="tweetText"], [lang]');
  return t ? t.textContent : "";
}

// Para las pruebas en Node; en el navegador esto no existe y no hace nada.
if (typeof module !== "undefined") {
  module.exports = {
    enlaceX, idDeTuit, tuitX, medioBajoX, fotoOriginalX, tokenX, direccionSindicacion, claveX, medioX,
    mediosDeTarjeta, mediosX, elegirVideoX, nombreX, textoEnPagina,
  };
}
