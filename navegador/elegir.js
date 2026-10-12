// Qué guardar de una página: la imagen de verdad, a su mayor tamaño, y de
// dónde sale.
//
// Va aparte de instagram.js para poder probarlo sin navegador (ver
// pruebas/elegir.test.mjs). Instagram cambia sus clases cada poco, así que nada
// de aquí las usa: se apoya en lo que tiene que estar para que la página
// funcione —una <img> con su `srcset`, el enlace del post, el texto
// alternativo— y en lo que hay bajo el ratón, no en cómo se llama.
"use strict";

/// La dirección de mayor anchura de un `srcset` («url 640w, url 1080w»), o
/// null si no hay nada que leer.
function mejorDeSrcset(srcset) {
  if (!srcset) return null;
  let mejor = null;
  let ancho = -1;
  // Las comas pueden ir dentro de una URL; las que separan candidatos van
  // detrás de un descriptor («640w,» o «2x,»).
  for (const trozo of srcset.split(/,\s+(?=\S)/)) {
    const partes = trozo.trim().split(/\s+/);
    const url = partes[0];
    if (!url) continue;
    const d = partes[1] || "1x";
    const n = d.endsWith("w") ? parseFloat(d) : d.endsWith("x") ? parseFloat(d) * 1000 : 0;
    if (n > ancho) {
      ancho = n;
      mejor = url;
    }
  }
  return mejor;
}

/// La mejor dirección de una imagen: lo más grande de su `srcset`, o lo que
/// esté pintando.
function mejorDeImagen(img) {
  return mejorDeSrcset(img.getAttribute("srcset")) || img.currentSrc || img.src || null;
}

/// Si una imagen es contenido y no un adorno: un avatar, un icono, un emoji.
function esContenido(img, minimo = 150) {
  const r = img.getBoundingClientRect();
  const lado = Math.max(r.width, r.height, img.naturalWidth || 0);
  return lado >= minimo;
}

/// La imagen o el vídeo que hay de verdad bajo un punto, aunque haya capas
/// transparentes encima —Instagram tapa sus fotos con varias para que el
/// clic derecho no las encuentre—.
///
/// Solo cuenta lo que se ve en ese punto: en un carrusel las fotos de los
/// lados están en la página, desplazadas y recortadas por el borde del
/// carrusel, y el navegador a veces las da también. Una que en ese punto
/// queda fuera de lo que deja ver su caja no es la que se está mirando.
function medioEnPunto(doc, x, y) {
  for (const el of doc.elementsFromPoint(x, y)) {
    const esMedio = (el.tagName === "IMG" && esContenido(el)) || el.tagName === "VIDEO";
    if (esMedio && seVeEn(el, x, y)) return el;
  }
  return null;
}

/// Si el punto cae dentro de lo que se ve de `el`: dentro de su caja y de la
/// de cada antepasado que recorta lo que se le sale.
function seVeEn(el, x, y) {
  const dentro = (r) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
  if (!dentro(el.getBoundingClientRect())) return false;
  const vista = el.ownerDocument.defaultView;
  for (let p = el.parentElement; p && p !== el.ownerDocument.body; p = p.parentElement) {
    const estilo = vista && vista.getComputedStyle ? vista.getComputedStyle(p) : null;
    const recorta = estilo && /(hidden|clip|auto|scroll)/.test(estilo.overflow + estilo.overflowX);
    if (recorta && !dentro(p.getBoundingClientRect())) return false;
  }
  return true;
}

/// El contenedor del post: el `article` del feed, o el diálogo de un post
/// abierto encima del perfil, o la página entera de un post.
function contenedorDelPost(el) {
  return el.closest("article") || el.closest('[role="dialog"]') || el.closest("main") || null;
}

const RE_POST = /^\/(?:[^/]+\/)?(p|reel|tv)\/([^/?#]+)/;

function enlaceDe(href, loc) {
  const m = new URL(href, loc.href).pathname.match(RE_POST);
  return m ? `https://www.instagram.com/${m[1]}/${m[2]}/` : null;
}

/// El enlace permanente del post, que es lo que se guarda como origen: la
/// dirección de la imagen caduca en días, la del post no.
///
/// Por orden: el enlace que envuelve a la propia foto —en la cuadrícula de un
/// perfil cada miniatura es un enlace a su post—; el del post en el que está,
/// en el feed o abierto encima; y la página, si es la de un post. Nunca «el
/// primero que haya»: en la cuadrícula eso era el post de otra foto.
function enlaceDelPost(el, loc) {
  for (let a = el.closest("a[href]"); a; a = a.parentElement && a.parentElement.closest("a[href]")) {
    const e = enlaceDe(a.getAttribute("href"), loc);
    if (e) return e;
  }
  const caja = el.closest("article") || el.closest('[role="dialog"]');
  if (caja) {
    for (const a of caja.querySelectorAll("a[href]")) {
      const e = enlaceDe(a.getAttribute("href"), loc);
      if (e) return e;
    }
  }
  return enlaceDe(loc.pathname, loc) || loc.href;
}

// ------------------------------------------------- la foto sin recortar
//
// Lo que se ve en pantalla no es la foto: en la cuadrícula de un perfil y en
// explorar son miniaturas cuadradas recortadas por el propio servidor, y en el
// feed a veces también. El recorte va dentro de la dirección, que está
// firmada, así que no se puede quitar. La foto de verdad —entera, en su
// proporción, y todas las de un carrusel aunque no se hayan cargado— la da la
// misma API que usa la web de Instagram, preguntando con la sesión de quien
// mira. Estas funciones leen su respuesta; instagram.js hace la pregunta.

const ALFABETO = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// El código del post («C9xYz12AbC») como el número que pide la API: el
/// código es ese número escrito en base 64 con este alfabeto. Los posts
/// privados llevan algo más detrás; el número son los once primeros.
function idDeCodigo(codigo) {
  if (!codigo) return null;
  let id = 0n;
  for (const c of codigo.slice(0, 11)) {
    const i = ALFABETO.indexOf(c);
    if (i < 0) return null;
    id = id * 64n + BigInt(i);
  }
  return id.toString();
}

function codigoDelEnlace(enlace) {
  const m = (enlace || "").match(/\/(?:p|reel|tv)\/([^/?#]+)/);
  return m ? m[1] : null;
}

/// El archivo de una dirección del CDN, sin parámetros: es el mismo en todas
/// las variantes de una foto —miniatura, recorte, tamaño completo—, que solo
/// cambian lo que va detrás de la «?».
function archivoDe(url) {
  try {
    return new URL(url).pathname.split("/").pop() || null;
  } catch (_) {
    return null;
  }
}

function mayor(candidatos) {
  let mejor = null;
  for (const c of candidatos || []) {
    if (!c || !c.url) continue;
    if (!mejor || (c.width || 0) * (c.height || 0) > (mejor.width || 0) * (mejor.height || 0)) mejor = c;
  }
  return mejor;
}

/// Cada foto o vídeo del post, a su mayor tamaño, con los nombres de archivo
/// de todas sus variantes para poder reconocer cuál se estaba mirando.
function mediosDelPost(item) {
  if (!item) return [];
  const piezas = Array.isArray(item.carousel_media) && item.carousel_media.length ? item.carousel_media : [item];
  const out = [];
  for (const p of piezas) {
    const imagenes = (p.image_versions2 && p.image_versions2.candidates) || [];
    const archivos = new Set(imagenes.map((c) => archivoDe(c.url)).filter(Boolean));
    const video = p.media_type === 2 ? mayor(p.video_versions) : null;
    const foto = mayor(imagenes);
    const elegido = video || foto;
    if (!elegido) continue;
    out.push({
      url: elegido.url,
      tipo: video ? "video" : "imagen",
      ancho: elegido.width || 0,
      alto: elegido.height || 0,
      archivos,
    });
  }
  return out;
}

/// Cuál de los medios del post es el que se estaba mirando: por el nombre de
/// archivo de lo que había en pantalla. Con uno solo, ese. Si no se reconoce,
/// null: mejor guardar lo que se ve que otra foto del carrusel.
function elegirDelPost(medios, urlVista, indiceDeLaPagina = null) {
  if (medios.length === 1) return 0;
  const archivo = archivoDe(urlVista);
  const i = archivo ? medios.findIndex((m) => m.archivos.has(archivo)) : -1;
  if (i >= 0) return i;
  // Un post abierto dice en su dirección qué foto del carrusel se ve.
  if (indiceDeLaPagina !== null && indiceDeLaPagina < medios.length) return indiceDeLaPagina;
  return null;
}

/// La foto del carrusel que dice la dirección de un post abierto
/// (`?img_index=3` es la tercera), contando desde cero; null si no lo dice.
function indiceDeLaPagina(loc) {
  const n = parseInt(new URL(loc.href).searchParams.get("img_index") || "", 10);
  return Number.isFinite(n) && n >= 1 ? n - 1 : null;
}

/// Las cabeceras con las que la propia web de Instagram pregunta a su API.
/// Sin el token de la cookie y estos identificadores, la API contesta que no
/// y solo quedaba guardar la miniatura.
function cabecerasApi(cookie) {
  const token = (/(?:^|;\s*)csrftoken=([^;]+)/.exec(cookie || "") || [])[1] || "";
  const h = {
    "X-IG-App-ID": "936619743392459",
    "X-ASBD-ID": "129477",
    "X-IG-WWW-Claim": "0",
    "X-Requested-With": "XMLHttpRequest",
  };
  if (token) h["X-CSRFToken"] = token;
  return h;
}

/// De lo que el navegador ya ha bajado (lo que sale en la pestaña «Img» de
/// las herramientas de desarrollo), la versión más pesada de la foto con este
/// nombre de archivo. Más pesada es más grande: son la misma foto en otro
/// tamaño. `entradas` son `PerformanceResourceTiming`, o lo que se les parezca.
function mejorCargada(entradas, archivo) {
  if (!archivo) return null;
  let mejor = null;
  for (const e of entradas || []) {
    if (archivoDe(e.name) !== archivo) continue;
    // El navegador solo dice cuánto pesa algo de otro dominio si ese dominio
    // lo permite; si no, cero. Entonces manda el tamaño escrito en la URL.
    const peso = e.decodedBodySize || e.encodedBodySize || 0;
    const valor = peso > 0 ? peso : tamanoDeUrl(e.name);
    if (!mejor || valor > mejor.valor) mejor = { url: e.name, valor };
  }
  return mejor;
}

/// Lo grande que dice ser una variante del CDN por su dirección: «s640x640»,
/// «p1080x1080», «e35_s150x150». Sin tamaño escrito es la de tamaño completo,
/// la más grande.
function tamanoDeUrl(url) {
  let mayorArea = 0;
  for (const m of String(url).matchAll(/[_.=-][sp](\d{2,5})x(\d{2,5})/g)) {
    mayorArea = Math.max(mayorArea, Number(m[1]) * Number(m[2]));
  }
  return mayorArea > 0 ? mayorArea : Number.MAX_SAFE_INTEGER;
}

/// Quién lo publicó: el primer enlace a un perfil de la cabecera del post.
function autorDelPost(el) {
  const caja = contenedorDelPost(el);
  if (!caja) return null;
  for (const a of caja.querySelectorAll("header a[href], a[role='link'][href]")) {
    const m = (a.getAttribute("href") || "").match(/^\/([A-Za-z0-9._]+)\/?$/);
    if (m && !["explore", "reels", "p", "reel", "stories"].includes(m[1])) return m[1];
  }
  return null;
}

/// Un nombre que se pueda buscar luego: el autor y lo que dice el texto
/// alternativo, sin la coletilla automática de Instagram.
function nombreDelMedio(img, loc) {
  const autor = autorDelPost(img);
  let alt = (img.getAttribute("alt") || "").trim();
  // «Photo by X on May 3, 2024. May be an image of …»: lo útil es lo de después.
  alt = alt.replace(/^(photo|foto|video|vídeo) (by|de|shared by|compartida por)[^.]*\.\s*/i, "");
  alt = alt.replace(/^(may be an image of|puede ser una imagen de)\s*/i, "");
  alt = alt.slice(0, 80).trim();
  const codigo = (enlaceDelPost(img, loc).match(/\/(p|reel|tv)\/([^/]+)/) || [])[2];
  const partes = [autor ? "@" + autor : null, alt || (codigo ? "instagram " + codigo : null)];
  return partes.filter(Boolean).join(" · ") || "instagram";
}

/// Las fotos de un carrusel que ya están cargadas, en orden y sin repetir.
/// Instagram carga las de los lados según se pasan, así que «todas» son las
/// que el navegador ya tiene; se dice cuántas antes de guardarlas.
function imagenesDelCarrusel(el) {
  const caja = contenedorDelPost(el);
  if (!caja) return [];
  const lista = el.closest("ul") || caja.querySelector("ul");
  const fuente = lista && lista.querySelectorAll("li img").length > 1 ? lista : null;
  if (!fuente) return [];
  const vistas = new Set();
  const out = [];
  for (const img of fuente.querySelectorAll("li img")) {
    if (!esContenido(img)) continue;
    const url = mejorDeImagen(img);
    if (!url || vistas.has(url)) continue;
    vistas.add(url);
    out.push(img);
  }
  return out;
}

// Para las pruebas en Node; en el navegador esto no existe y no hace nada.
if (typeof module !== "undefined") {
  module.exports = {
    mejorDeSrcset, mejorDeImagen, esContenido, medioEnPunto, contenedorDelPost,
    enlaceDelPost, autorDelPost, nombreDelMedio, imagenesDelCarrusel,
    idDeCodigo, codigoDelEnlace, archivoDe, mediosDelPost, elegirDelPost,
    seVeEn, indiceDeLaPagina, cabecerasApi, mejorCargada, tamanoDeUrl,
  };
}
