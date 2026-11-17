// La página entera, para leerla sin conexión.
//
// Se guarda lo que se está viendo, no lo que el servidor manda: el HTML ya
// pintado por los scripts de la página (lo que sale después de iniciar sesión,
// lo que se ha cargado al bajar), sin los scripts, y con las hojas de estilo,
// las imágenes y las tipografías metidas dentro como `data:`. Sale un solo
// `.html` que cualquier navegador abre igual aunque la web cambie o
// desaparezca, y que se puede copiar o leer como cualquier archivo.
//
// Lo que no cabe en un archivo así se queda apuntando a la web: los vídeos,
// los `iframe` (un vídeo de YouTube metido en un artículo) y lo que pase del
// presupuesto. Si la web sigue ahí, se ve; si no, queda el hueco.
//
// Los recursos los baja el fondo de la extensión (`bajar`), que puede pedir a
// cualquier dominio con la sesión de quien mira; desde aquí, la política de
// orígenes de la página no dejaría leer las hojas de estilo de otro dominio.
//
// Va aparte del fondo para poder probarlo con jsdom (pruebas/pagina.test.mjs).
"use strict";

/// Lo que se mete dentro, en total y por archivo: el mensaje a Grimorio no
/// puede pasar de 40 MB (ver fondo.js) y en base64 todo crece un tercio.
const PRESUPUESTO_PAGINA = 24 * 1024 * 1024;
const TOPE_RECURSO_PAGINA = 8 * 1024 * 1024;
/// Hojas de estilo que importan otras, que importan otras…
const PROFUNDIDAD_IMPORT = 4;

/// Un `replace` con una función asíncrona: las sustituciones se piden todas a
/// la vez y se ponen en orden.
async function reemplazarAsync(texto, re, f) {
  const trozos = [];
  let ultimo = 0;
  for (const m of texto.matchAll(re)) {
    trozos.push(texto.slice(ultimo, m.index));
    trozos.push(f(...m));
    ultimo = m.index + m[0].length;
  }
  trozos.push(texto.slice(ultimo));
  return (await Promise.all(trozos)).join("");
}

function absoluta(url, base) {
  try {
    return new URL(url, base).href;
  } catch (_) {
    return null;
  }
}

/// El CSS con sus `@import` dentro y cada `url()` cambiada por los datos, o
/// por la dirección entera si no se pudo bajar: relativa ya no apuntaría a
/// nada, porque el archivo vive en otro sitio.
async function cssConRecursos(texto, base, guardar, profundidad = 0) {
  texto = await reemplazarAsync(
    texto,
    /@import\s+(?:url\(\s*(["']?)([^"')]+)\1\s*\)|(["'])([^"']+)\3)\s*([^;]*);/gi,
    async (todo, _q1, u1, _q2, u2, medios) => {
      const abs = absoluta(u1 || u2, base);
      if (!abs) return "";
      const dentro = profundidad < PROFUNDIDAD_IMPORT ? await guardar.texto(abs) : null;
      if (dentro == null) return `@import url("${abs}") ${medios};`;
      const css = await cssConRecursos(dentro, abs, guardar, profundidad + 1);
      medios = medios.trim();
      return medios ? `@media ${medios} {\n${css}\n}` : css;
    },
  );
  return reemplazarAsync(texto, /url\(\s*(["']?)([^"')]*?)\1\s*\)/gi, async (todo, _q, u) => {
    if (!u || /^(data:|#|about:)/i.test(u)) return todo;
    const abs = absoluta(u, base);
    if (!abs) return todo;
    return `url("${(await guardar.datos(abs)) || abs}")`;
  });
}

/// El texto de una hoja de estilo tal y como la tiene el navegador. Por sus
/// reglas y no por el archivo: así sale también lo que los scripts le añaden
/// (los estilos de CSS-in-JS van por `insertRule` y no aparecen en el texto).
/// Si es de otro dominio, el navegador no deja leer sus reglas: se baja.
async function textoDeHoja(hoja, guardar) {
  try {
    if (hoja && hoja.cssRules) {
      return Array.from(hoja.cssRules, (r) => r.cssText).join("\n");
    }
  } catch (_) {
    // De otro dominio.
  }
  return hoja && hoja.href ? guardar.texto(hoja.href) : null;
}

/// Quien baja los recursos, con el presupuesto y sin bajar dos veces lo mismo.
/// `bajar(url, {tope, texto})` contesta `{datos}` (una URL `data:`) o
/// `{texto}`, o nada si no pudo.
function guardador(bajar, presupuesto = PRESUPUESTO_PAGINA) {
  let gastado = 0;
  const hechos = new Map();
  const una = (url, texto) => {
    const clave = (texto ? "t " : "d ") + url;
    if (!hechos.has(clave)) {
      hechos.set(clave, (async () => {
        if (!/^(https?|blob):/i.test(url)) return null;
        // El presupuesto se cuenta en lo que ocupa dentro, ya en base64; a
        // quien baja se le dice en bytes.
        const queda = presupuesto - gastado;
        if (queda <= 0) return null;
        const tope = Math.min(TOPE_RECURSO_PAGINA, Math.floor(queda * 3 / 4));
        let r = null;
        try {
          r = await bajar(url, { tope, texto });
        } catch (_) {
          return null;
        }
        const valor = r && (texto ? r.texto : r.datos);
        if (typeof valor !== "string" || valor.length > queda) return null;
        gastado += valor.length;
        return valor;
      })());
    }
    return hechos.get(clave);
  };
  return {
    datos: (url) => (/^data:/i.test(url) ? Promise.resolve(url) : una(url, false)),
    texto: (url) => una(url, true),
  };
}

/// La página `doc` en un solo HTML. `opciones`:
///   bajar     ver `guardador`
///   captura   la captura de la pestaña, una URL `data:image/…`, o ""
///   origen    la dirección de la página
///   fecha     cuándo se guarda, en ISO 8601
async function archivarPaginaGrimorio(doc, opciones) {
  const { bajar, captura = "", origen = doc.URL, fecha = new Date().toISOString(),
          presupuesto } = opciones || {};
  const guardar = guardador(bajar, presupuesto);
  const base = doc.baseURI;

  // Lo que hay que leer del original y no de la copia —la imagen que eligió
  // el navegador entre las de `srcset`, el texto escrito en un campo, lo
  // pintado en un `canvas`, las reglas de cada hoja— se marca antes de copiar
  // para encontrar luego su pareja.
  const MARCA = "data-grimorio-i";
  const originales = Array.from(doc.querySelectorAll(
    "img, canvas, input, textarea, select, style, link"));
  originales.forEach((el, i) => el.setAttribute(MARCA, String(i)));
  const raiz = doc.documentElement.cloneNode(true);
  originales.forEach((el) => el.removeAttribute(MARCA));
  const original = (el) => originales[Number(el.getAttribute(MARCA))];

  const trabajos = [];
  const quitar = (sel) => raiz.querySelectorAll(sel).forEach((el) => el.remove());

  // Nada que se ejecute ni que cargue por su cuenta: la copia tiene que verse
  // igual hoy que dentro de diez años. Fuera también lo que impediría ver lo
  // que va dentro (una CSP que no deja usar `data:`) o mandaría a otra página.
  quitar("script, noscript, base, meta[http-equiv], meta[charset], "
         + "link[rel~='preload'], link[rel~='prefetch'], link[rel~='modulepreload'], "
         + "link[rel~='dns-prefetch'], link[rel~='preconnect'], link[rel~='manifest'], "
         + "link[rel~='serviceworker'], "
         // Lo que pone la propia extensión: el aviso de «guardando» y el botón.
         + "[data-grimorio-aviso], [data-grimorio]");
  for (const el of raiz.querySelectorAll("*")) {
    for (const a of Array.from(el.attributes)) {
      if (/^on/i.test(a.name)) el.removeAttribute(a.name);
    }
  }
  for (const el of raiz.querySelectorAll("[href^='javascript:' i]")) el.removeAttribute("href");

  // Las hojas de estilo, dentro y en su sitio, que el orden cuenta.
  for (const el of raiz.querySelectorAll("link[rel~='stylesheet' i], style")) {
    const o = original(el);
    const hoja = o && o.sheet;
    const enlazada = el.localName === "link";
    if (enlazada && o && o.disabled) {
      el.remove();
      continue;
    }
    const href = enlazada ? absoluta(el.getAttribute("href"), base) : null;
    trabajos.push((async () => {
      // Sin hoja, una enlazada no llegó a cargar: se intenta bajar.
      const texto = hoja ? await textoDeHoja(hoja, guardar)
                    : enlazada ? href && await guardar.texto(href) : el.textContent;
      if (texto == null) return;
      const css = await cssConRecursos(texto, (hoja && hoja.href) || href || base, guardar);
      const estilo = doc.createElement("style");
      const medios = el.getAttribute("media");
      if (medios) estilo.setAttribute("media", medios);
      estilo.textContent = css;
      el.replaceWith(estilo);
    })());
  }
  // Las que los scripts ponen sin elemento (`adoptedStyleSheets`).
  for (const hoja of doc.adoptedStyleSheets || []) {
    trabajos.push((async () => {
      const texto = await textoDeHoja(hoja, guardar);
      if (texto == null) return;
      const estilo = doc.createElement("style");
      estilo.textContent = await cssConRecursos(texto, base, guardar);
      (raiz.querySelector("head") || raiz).append(estilo);
    })());
  }
  for (const el of raiz.querySelectorAll("[style]")) {
    if (!/url\(/i.test(el.getAttribute("style"))) continue;
    trabajos.push(cssConRecursos(el.getAttribute("style"), base, guardar)
      .then((css) => el.setAttribute("style", css)));
  }

  // Las imágenes: la que el navegador eligió y está viendo, sin las demás
  // de `srcset`, que pedirían a la web una que no está dentro.
  quitar("picture > source");
  for (const el of raiz.querySelectorAll("img")) {
    const o = original(el);
    const perezosa = el.getAttribute("data-src");
    let url = (o && (o.currentSrc || o.src)) || el.getAttribute("src") || "";
    if (perezosa && (!url || /^data:/i.test(url) && url.length < 200)) url = perezosa;
    for (const a of ["srcset", "sizes", "loading", "decoding", "data-src", "data-srcset"]) {
      el.removeAttribute(a);
    }
    const abs = absoluta(url, base);
    if (!abs) continue;
    el.setAttribute("src", abs);
    trabajos.push(guardar.datos(abs).then((d) => d && el.setAttribute("src", d)));
  }
  // Lo pintado con scripts, como imagen. Uno con imágenes de otro dominio no
  // deja leerse: se queda vacío.
  for (const el of raiz.querySelectorAll("canvas")) {
    const o = original(el);
    try {
      const img = doc.createElement("img");
      img.setAttribute("src", o.toDataURL("image/png"));
      for (const a of ["class", "style", "width", "height", "id"]) {
        if (el.hasAttribute(a)) img.setAttribute(a, el.getAttribute(a));
      }
      el.replaceWith(img);
    } catch (_) {
      // Sin poder leerlo.
    }
  }
  for (const el of raiz.querySelectorAll("video[poster]")) {
    const abs = absoluta(el.getAttribute("poster"), base);
    if (abs) trabajos.push(guardar.datos(abs).then((d) => el.setAttribute("poster", d || abs)));
  }
  for (const el of raiz.querySelectorAll("link[rel~='icon' i]")) {
    const abs = absoluta(el.getAttribute("href"), base);
    if (abs) trabajos.push(guardar.datos(abs).then((d) => el.setAttribute("href", d || abs)));
  }

  // Lo escrito en los formularios, que vive en el elemento y no en el HTML.
  for (const el of raiz.querySelectorAll("input, textarea, select")) {
    const o = original(el);
    if (!o) continue;
    if (el.localName === "textarea") el.textContent = o.value;
    else if (el.localName === "select") {
      el.querySelectorAll("option").forEach((op, i) => {
        if (o.options[i] && o.options[i].selected) op.setAttribute("selected", "");
        else op.removeAttribute("selected");
      });
    } else if (/^(checkbox|radio)$/i.test(o.type)) {
      if (o.checked) el.setAttribute("checked", "");
      else el.removeAttribute("checked");
    } else if (!/^(password|file)$/i.test(o.type)) {
      el.setAttribute("value", o.value);
    }
  }

  // Lo que se queda fuera, con la dirección entera: relativa no apuntaría a
  // nada desde la biblioteca.
  for (const [sel, a] of [["a[href]", "href"], ["area[href]", "href"], ["form[action]", "action"],
                          ["iframe[src]", "src"], ["video[src]", "src"], ["audio[src]", "src"],
                          ["source[src]", "src"], ["track[src]", "src"], ["embed[src]", "src"],
                          ["object[data]", "data"], ["link[href]", "href"]]) {
    for (const el of raiz.querySelectorAll(sel)) {
      const v = el.getAttribute(a);
      if (v && !/^(data:|#|mailto:|tel:)/i.test(v)) {
        const abs = absoluta(v, base);
        if (abs) el.setAttribute(a, abs);
      }
    }
  }
  raiz.querySelectorAll("[" + MARCA + "]").forEach((el) => el.removeAttribute(MARCA));

  await Promise.all(trabajos);

  // Al principio del `<head>`: la codificación, que tiene que ir antes de
  // cualquier texto, y lo que Grimorio lee sin abrir el archivo entero —la
  // captura es su miniatura—.
  let cabeza = raiz.querySelector("head");
  if (!cabeza) {
    cabeza = doc.createElement("head");
    raiz.prepend(cabeza);
  }
  const meta = (atributos) => {
    const m = doc.createElement("meta");
    for (const [k, v] of Object.entries(atributos)) m.setAttribute(k, v);
    return m;
  };
  const delante = [meta({ charset: "utf-8" }),
                   meta({ name: "grimorio:origen", content: origen }),
                   meta({ name: "grimorio:guardada", content: fecha })];
  if (/^data:image\//i.test(captura)) delante.push(meta({ name: "grimorio:captura", content: captura }));
  cabeza.prepend(...delante);

  const tipo = doc.doctype ? `<!DOCTYPE ${doc.doctype.name}>` : "<!DOCTYPE html>";
  return `${tipo}\n<!-- Guardada con Grimorio desde ${origen.replace(/--/g, "%2D%2D")} el ${fecha} -->\n`
         + raiz.outerHTML;
}

if (typeof module !== "undefined") {
  module.exports = { archivarPaginaGrimorio, cssConRecursos, guardador, reemplazarAsync };
}
