// El fondo de la extensión: el menú del clic derecho y el puente con Grimorio.
//
// Con Grimorio se habla por «native messaging»: el navegador lanza `grim`, que
// lleva el mensaje a la aplicación abierta (ver crates/grimorio-cli/src/
// nativo.rs). Para eso `grim navegador instalar` tiene que haberse ejecutado
// una vez; si no, el navegador dice que no conoce el programa y aquí se
// traduce a algo que se pueda hacer.
"use strict";

importScripts("aviso.js");

const PROGRAMA = "com.grimorio.puerta";
/// Por encima de esto no se mandan los bytes, se manda la dirección y la baja
/// Grimorio: un mensaje al programa nativo no puede pasar de 64 MB, y en base64
/// los bytes crecen un tercio.
const TOPE_BYTES = 40 * 1024 * 1024;

// Una sola entrada para todo: en una imagen o un vídeo guarda ese archivo; en
// la página, la página entera (pagina.js); en un enlace, la imagen que lo
// representa, que la página de destino no está abierta; en Instagram y en
// TikTok, lo que hay bajo el ratón, que el navegador no ve porque está tapado
// por capas.
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "guardar",
      title: textoGrimorio("menuGuardar"),
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
               error: textoGrimorio("errorSinConectar") };
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
///
/// Con `alternativas`, si la primera dirección no da el archivo se prueban
/// las demás: TikTok da cada vídeo en varios servidores, y desde aquí unos
/// contestan 403 y otros no.
async function guardarUno({ url, alternativas, origen, nombre }, carpeta) {
  const pedido = { accion: "traer", origen: origen || "", nombre: nombre || "", carpeta: carpeta || "" };
  let grande = null;
  for (const u of [url, ...(alternativas || [])]) {
    try {
      const r = await fetch(u, { credentials: "include" });
      const tipo = (r.headers.get("content-type") || "").split(";")[0].trim().toLowerCase();
      const largo = Number(r.headers.get("content-length") || 0);
      if (!r.ok || !/^(image|video)\//.test(tipo)) continue;
      if (largo <= TOPE_BYTES) {
        const bytes = new Uint8Array(await r.arrayBuffer());
        if (bytes.length <= TOPE_BYTES) {
          return hablar({ ...pedido, datos: base64(bytes), tipo });
        }
      }
      // Demasiado grande para mandarla: esta es la dirección que funciona.
      grande = grande || u;
    } catch (_) {
      // Sin poder bajarla aquí, que lo intente Grimorio con la dirección.
    }
  }
  const direccion = grande || url;
  if (!/^https?:/.test(direccion)) return { ok: false, error: textoGrimorio("errorSinDireccion") };
  return hablar({ ...pedido, url: direccion });
}

async function carpetaElegida() {
  const { carpeta } = await chrome.storage.local.get("carpeta");
  return carpeta || "";
}

/// Guarda cada uno y devuelve lo que pasó con cada uno: guardada, repetida
/// (ya estaba), en camino (sigue bajando) o error. Lo apunta en las últimas.
async function guardarLista(lista) {
  const carpeta = await carpetaElegida();
  const resultados = [];
  for (const p of lista) {
    const r = (await guardarUno(p, carpeta)) || { ok: false, error: textoGrimorio("errorSinRespuesta") };
    resultados.push({ ...r, nombre: r.nombre || p.nombre || "" });
  }
  await apuntar(resultados);
  return { ok: resultados.some((r) => r.ok), resultados };
}

/// Las últimas guardadas, para la ventanita del botón.
async function apuntar(resultados) {
  const { ultimas = [] } = await chrome.storage.local.get("ultimas");
  const nuevas = resultados.map((r) => ({
    estado: r.ok ? r.estado || "guardada" : "error",
    nombre: r.nombre || "",
    error: r.ok ? "" : r.error || "",
    id: r.id || "",
    cuando: Date.now(),
  }));
  await chrome.storage.local.set({ ultimas: [...nuevas.reverse(), ...ultimas].slice(0, 8) });
}

/// El aviso dentro de una página que no es Instagram (allí lo pone su script).
async function avisarEn(tab, aviso) {
  try {
    await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: ["aviso.js"] });
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: (a) => avisoGrimorio(a),
      args: [aviso],
    });
  } catch (_) {
    // Páginas en las que no se puede pintar (las del propio navegador): queda
    // la marca del botón.
  }
}

async function marcar(tabId, bien) {
  await chrome.action.setBadgeBackgroundColor({ color: bien ? "#3b7d5a" : "#a33a3a", tabId });
  await chrome.action.setBadgeText({ text: bien ? "✓" : "!", tabId });
  setTimeout(() => chrome.action.setBadgeText({ text: "", tabId }), 3000);
}

/// Lo que se ve de la pestaña, para la miniatura de una página guardada. En
/// JPEG, que una captura en PNG pesa diez veces más y es solo para reconocerla.
async function capturar(tab) {
  try {
    return await chrome.tabs.captureVisibleTab(tab.windowId, { format: "jpeg", quality: 80 });
  } catch (_) {
    // Las páginas del propio navegador no se dejan.
    return "";
  }
}

/// Un recurso de una página que se está guardando (ver pagina.js), con la
/// sesión de quien mira y sin la política de orígenes de la página: los datos
/// en una URL `data:`, o el texto de una hoja de estilo.
async function bajarRecurso(url, { tope, texto }) {
  const r = await fetch(url, { credentials: "include" });
  if (!r.ok) return null;
  const largo = Number(r.headers.get("content-length") || 0);
  if (largo > tope) return null;
  if (texto) return { texto: await r.text() };
  const bytes = new Uint8Array(await r.arrayBuffer());
  if (bytes.length > tope) return null;
  const tipo = (r.headers.get("content-type") || "application/octet-stream").split(";")[0].trim();
  return { datos: `data:${tipo};base64,${base64(bytes)}` };
}

/// La página de la pestaña, entera, para leerla sin conexión.
async function guardarPagina(tab, captura) {
  let html;
  try {
    await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: ["pagina.js"] });
    const [r] = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: (opciones) => archivarPaginaGrimorio(document, {
        ...opciones,
        bajar: (url, o) => chrome.runtime.sendMessage({ tipo: "recurso", url, ...o }),
      }),
      args: [{ captura }],
    });
    html = r && r.result;
  } catch (e) {
    html = null;
  }
  const nombre = (tab.title || "").trim();
  let resultados;
  const bytes = typeof html === "string" ? new TextEncoder().encode(html) : null;
  if (!bytes) {
    resultados = [{ ok: false, error: textoGrimorio("errorPagina"), nombre }];
  } else if (bytes.length > TOPE_BYTES) {
    resultados = [{ ok: false, error: textoGrimorio("errorPaginaGrande"), nombre }];
  } else {
    const uno = await hablar({ accion: "traer", datos: base64(bytes), tipo: "text/html",
                               nombre, origen: tab.url, carpeta: await carpetaElegida() });
    resultados = [{ ...uno, nombre: uno.nombre || nombre }];
  }
  await apuntar(resultados);
  return { ok: resultados[0].ok, resultados };
}

/// Las webs con script propio: el clic derecho lo atiende él, que sabe qué
/// había bajo el ratón —Instagram y TikTok lo tapan con capas— o dónde está
/// el original —4chan enseña una miniatura—.
const RE_CON_BOTON = /^https:\/\/(www\.(instagram|tiktok)\.com|boards\.4chan(nel)?\.org)\//;

/// Lo que hace la entrada del clic derecho. Aparte del oyente para poder
/// llamarla en las pruebas sin abrir un menú de verdad.
async function alPulsarMenu(info, tab) {
  if (tab && RE_CON_BOTON.test(info.pageUrl || tab.url || "")) {
    // Lo guarda la pestaña, que sabe qué había bajo el ratón y dónde está el
    // original entero; el aviso sale allí, como con el botón.
    try {
      return await chrome.tabs.sendMessage(tab.id, { tipo: "guardarClicDerecho" }, { frameId: info.frameId || 0 });
    } catch (e) {
      return { ok: false, error: String(e.message || e) };
    }
  }
  // La captura antes que el aviso: si no, el aviso sale en la foto.
  const captura = !info.linkUrl && !info.mediaType && tab ? await capturar(tab) : "";
  if (tab) await avisarEn(tab, { estado: "en curso", titulo: textoGrimorio("avisoGuardando") });
  let r;
  if ((info.mediaType === "image" || info.mediaType === "video") && info.srcUrl) {
    r = await guardarLista([{ url: info.srcUrl, origen: info.pageUrl, nombre: tab && tab.title }]);
  } else if (!info.linkUrl && tab) {
    r = await guardarPagina(tab, captura);
  } else {
    // Un enlace: Grimorio busca la imagen principal de su página (og:image).
    const url = info.linkUrl || info.pageUrl;
    const uno = await hablar({ accion: "traer", url, origen: url, carpeta: await carpetaElegida() });
    const resultados = [{ ...uno, nombre: uno.nombre || (tab && tab.title) || "" }];
    await apuntar(resultados);
    r = { ok: uno.ok, resultados };
  }
  if (tab) await avisarEn(tab, avisoDeResultado(r, 1));
  return r;
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const r = await alPulsarMenu(info, tab);
  if (r && tab) await marcar(tab.id, r.ok);
});

chrome.runtime.onMessage.addListener((m, de, responder) => {
  if (m.tipo === "recurso") {
    bajarRecurso(m.url, m).then(responder, () => responder(null));
    return true;
  }
  if (m.tipo === "mostrar") {
    hablar({ accion: "mostrar", id: m.id }).then(responder);
    return true;
  }
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
