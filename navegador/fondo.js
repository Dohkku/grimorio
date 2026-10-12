// El fondo de la extensión: el menú del clic derecho y el puente con Grimorio.
//
// Con Grimorio se habla por «native messaging»: el navegador lanza `grim`, que
// lleva el mensaje a la aplicación abierta (ver crates/grimorio-cli/src/
// nativo.rs). Para eso `grim navegador instalar` tiene que haberse ejecutado
// una vez; si no, el navegador dice que no conoce el programa y aquí se
// traduce a algo que se pueda hacer.
"use strict";

const PROGRAMA = "com.grimorio.puerta";
/// Por encima de esto no se mandan los bytes, se manda la dirección y la baja
/// Grimorio: un mensaje al programa nativo no puede pasar de 64 MB, y en base64
/// los bytes crecen un tercio.
const TOPE_BYTES = 40 * 1024 * 1024;

// Una sola entrada para todo: en una imagen o un vídeo guarda ese archivo; en
// una página o un enlace, la imagen que los representa; en Instagram, lo que
// hay bajo el ratón, que el navegador no ve porque está tapado por capas.
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "guardar",
      title: "Guardar en Grimorio",
      contexts: ["image", "video", "link", "page"],
    });
  });
});

async function hablar(mensaje) {
  try {
    return await chrome.runtime.sendNativeMessage(PROGRAMA, mensaje);
  } catch (e) {
    const texto = String((e && e.message) || e);
    if (/not found|no se encontr/i.test(texto)) {
      return { ok: false, sinConectar: true,
               error: "falta conectar Grimorio con el navegador: ejecuta «grim navegador instalar»" };
    }
    return { ok: false, error: texto };
  }
}

function base64(bytes) {
  // Por trozos: `String.fromCharCode(...todo)` revienta la pila con una foto.
  let s = "";
  const paso = 0x8000;
  for (let i = 0; i < bytes.length; i += paso) {
    s += String.fromCharCode.apply(null, bytes.subarray(i, i + paso));
  }
  return btoa(s);
}

/// Baja la imagen aquí, con la sesión de quien mira, y manda los bytes. Si no
/// se puede —demasiado grande, o algo que no es una imagen—, manda la
/// dirección y que la baje Grimorio.
async function guardarUno({ url, origen, nombre }, carpeta) {
  const pedido = { accion: "traer", origen: origen || "", nombre: nombre || "", carpeta: carpeta || "" };
  try {
    const r = await fetch(url, { credentials: "include" });
    const tipo = (r.headers.get("content-type") || "").split(";")[0].trim().toLowerCase();
    const largo = Number(r.headers.get("content-length") || 0);
    if (r.ok && /^(image|video)\//.test(tipo) && largo <= TOPE_BYTES) {
      const bytes = new Uint8Array(await r.arrayBuffer());
      if (bytes.length <= TOPE_BYTES) {
        return hablar({ ...pedido, datos: base64(bytes), tipo });
      }
    }
  } catch (_) {
    // Sin poder bajarla aquí, que lo intente Grimorio con la dirección.
  }
  if (!/^https?:/.test(url)) return { ok: false, error: "esa imagen no tiene dirección que bajar" };
  return hablar({ ...pedido, url });
}

async function carpetaElegida() {
  const { carpeta } = await chrome.storage.local.get("carpeta");
  return carpeta || "";
}

async function guardarLista(lista) {
  const carpeta = await carpetaElegida();
  let fallo = null;
  let hechos = 0;
  for (const p of lista) {
    const r = await guardarUno(p, carpeta);
    if (r && r.ok) hechos++;
    else fallo = fallo || r;
  }
  return fallo && hechos === 0 ? fallo : { ok: true, hechos };
}

async function marcar(tabId, bien) {
  await chrome.action.setBadgeBackgroundColor({ color: bien ? "#3b7d5a" : "#a33a3a", tabId });
  await chrome.action.setBadgeText({ text: bien ? "✓" : "!", tabId });
  setTimeout(() => chrome.action.setBadgeText({ text: "", tabId }), 3000);
}

const RE_INSTAGRAM = /^https:\/\/www\.instagram\.com\//;

/// Lo que hace la entrada del clic derecho. Aparte del oyente para poder
/// llamarla en las pruebas sin abrir un menú de verdad.
async function alPulsarMenu(info, tab) {
  if (tab && RE_INSTAGRAM.test(info.pageUrl || tab.url || "")) {
    // Lo guarda la pestaña, que sabe qué había bajo el ratón y dónde está el
    // original entero; el aviso sale allí, como con el botón.
    try {
      return await chrome.tabs.sendMessage(tab.id, { tipo: "guardarClicDerecho" }, { frameId: info.frameId || 0 });
    } catch (e) {
      return { ok: false, error: String(e.message || e) };
    }
  }
  if ((info.mediaType === "image" || info.mediaType === "video") && info.srcUrl) {
    return guardarLista([{ url: info.srcUrl, origen: info.pageUrl, nombre: tab && tab.title }]);
  }
  // Un enlace o la página: Grimorio busca su imagen principal (og:image).
  const url = info.linkUrl || info.pageUrl;
  return hablar({ accion: "traer", url, origen: url, carpeta: await carpetaElegida() });
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const r = await alPulsarMenu(info, tab);
  if (r && tab) await marcar(tab.id, r.ok);
});

chrome.runtime.onMessage.addListener((m, de, responder) => {
  if (m.tipo === "guardar") {
    guardarLista(m.lista).then(responder);
    return true;
  }
  if (m.tipo === "cargadas" && de.tab) {
    // Las imágenes que la página ha bajado, leídas en su propio mundo: el
    // registro de la página no llega al mundo aislado de la extensión.
    chrome.scripting.executeScript({
      target: { tabId: de.tab.id, frameIds: [de.frameId || 0] },
      world: "MAIN",
      func: () => performance.getEntriesByType("resource")
        .filter((e) => e.initiatorType === "img")
        .map((e) => ({ name: e.name, decodedBodySize: e.decodedBodySize, encodedBodySize: e.encodedBodySize })),
    }).then((r) => responder({ entradas: (r && r[0] && r[0].result) || [] }),
            () => responder({ entradas: [] }));
    return true;
  }
  if (m.tipo === "estado") {
    (async () => {
      const hola = await hablar({ accion: "hola" });
      const carpetas = hola.ok ? await hablar({ accion: "carpetas" }) : { carpetas: [] };
      responder({ hola, carpetas: carpetas.carpetas || [], carpeta: await carpetaElegida() });
    })();
    return true;
  }
  return false;
});
