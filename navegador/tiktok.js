// El botón de TikTok: al pasar por encima de un vídeo o una foto sale
// «a Grimorio».
//
// El clic derecho no sirve en TikTok: el vídeo se reproduce desde un «blob:»,
// que no tiene dirección que bajar, y está tapado por capas. Así que se mira
// qué hay bajo el ratón atravesando las capas (`medioEnPunto`, de elegir.js),
// se averigua de qué post es y se pide la página del post, que trae dentro
// la dirección del vídeo entero (ver tiktok-datos.js). El botón es el de
// boton.js.
"use strict";

(() => {
  /// Los datos del post, de su página, o por qué no se tienen.
  async function datosDelPost(enlace) {
    try {
      const r = await fetch(enlace, { credentials: "include" });
      if (!r.ok) return { motivo: textoGrimorio("motivoTikTokContesto", r.status) };
      const item = itemDelHtml(await r.text());
      return item ? { item } : { motivo: textoGrimorio("motivoSinDatosDelPost") };
    } catch (e) {
      return { motivo: textoGrimorio("motivoTikTokNoContesto", e.message) };
    }
  }

  /// Lo que se manda a guardar para `medio`, y cómo se ha conseguido.
  async function pedidos(medio) {
    const visto = medio.tagName === "IMG" ? mejorDeImagen(medio) : "";
    const origen = postTikTok(medio, location);
    let motivo = textoGrimorio("motivoDeQuePost");
    if (origen) {
      const { item, motivo: porque } = await datosDelPost(origen);
      const medios = mediosTikTok(item);
      if (medios.length) {
        const i = elegirTikTok(medios, visto);
        const m = medios[i];
        let nombre = nombreTikTok(item, origen);
        if (medios.length > 1) nombre = textoGrimorio("nombreConIndice", nombre, i + 1, medios.length);
        return { lista: [{ url: m.url, alternativas: m.alternativas, origen, nombre }],
                 como: m.ancho ? textoGrimorio("comoTamanoOriginal", m.ancho, m.alto) : textoGrimorio("comoOriginal") };
      }
      motivo = porque || textoGrimorio("motivoSinArchivo");
    }
    // Sin datos: una foto se guarda como se ve; un vídeo no tiene de dónde.
    if (!visto) return { lista: [], como: motivo };
    const nombre = (medio.getAttribute("alt") || "").replace(/\s+/g, " ").slice(0, 80).trim() || "tiktok";
    return { lista: [{ url: visto, origen: origen || location.href, nombre }], como: textoGrimorio("comoLoQueSeVe", motivo) };
  }

  async function guardar(medio) {
    avisoGrimorio({ estado: "en curso", titulo: textoGrimorio("avisoBuscando") });
    const { lista, como } = await pedidos(medio);
    return guardarYAvisar(lista, como);
  }

  botonGrimorio({ medioBajo: (e) => medioEnPunto(document, e.clientX, e.clientY), guardar });
})();
