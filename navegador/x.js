// El botón de X (Twitter): al pasar por encima de una foto, un vídeo o un gif
// sale «a Grimorio».
//
// El clic derecho no basta en X: la foto que se ve es una versión reducida
// («name=small») y está tapada por capas, y el vídeo se reproduce desde un
// «blob:», sin dirección que bajar. Así que se mira qué hay bajo el ratón
// atravesando las capas (`medioBajoX`, de x-datos.js) y:
// - de una foto se pide la original («name=orig»);
// - de un vídeo, el mp4 más grande que da el servicio de los tuits
//   incrustados (lo pide el fondo: desde la página no deja). Los tuits que
//   ese servicio no enseña sin sesión, o marcados como sensibles, no traen el
//   vídeo, y se dice por qué.
// El botón es el de boton.js.
"use strict";

(() => {
  /// Los datos del tuit según el servicio de los incrustados, o por qué no.
  async function datosDelTuit(id) {
    try {
      const r = await chrome.runtime.sendMessage({ tipo: "x", id });
      return r && r.datos ? { datos: r.datos } : { motivo: (r && r.motivo) || textoGrimorio("motivoXSinDatos") };
    } catch (e) {
      return { motivo: String(e.message || e) };
    }
  }

  function nombreEnPagina(medio, origen) {
    return nombreX({ text: textoEnPagina(medio) }, origen);
  }

  async function guardarFoto(img) {
    const origen = tuitX(img, location) || location.href;
    const vista = mejorDeImagen(img);
    const url = fotoOriginalX(vista);
    const lista = [{ url, alternativas: url !== vista ? [vista] : [], origen, nombre: nombreEnPagina(img, origen) }];
    return guardarYAvisar(lista, url !== vista ? textoGrimorio("comoOriginal")
                                               : textoGrimorio("comoLoQueSeVe", textoGrimorio("motivoXNoEsDeX")));
  }

  async function guardarVideo(video) {
    const origen = tuitX(video, location);
    const poster = video.getAttribute("poster") || "";
    // Un gif es un mp4 con dirección de verdad: se baja tal cual.
    if (/^https:\/\/video\.twimg\.com\//.test(video.currentSrc || video.src || "")) {
      const url = video.currentSrc || video.src;
      return guardarYAvisar([{ url, origen: origen || location.href, nombre: nombreEnPagina(video, origen) }],
                            textoGrimorio("comoOriginal"));
    }
    let motivo = textoGrimorio("motivoDeQueTuit");
    if (origen) {
      const { datos, motivo: porque } = await datosDelTuit(idDeTuit(origen));
      const v = elegirVideoX(mediosX(datos), poster);
      if (v) {
        const nombre = datos ? nombreX(datos, origen) : nombreEnPagina(video, origen);
        return guardarYAvisar([{ url: v.url, alternativas: v.alternativas, origen, nombre }],
                              v.ancho ? textoGrimorio("comoTamanoOriginal", v.ancho, v.alto) : textoGrimorio("comoOriginal"));
      }
      motivo = porque || textoGrimorio("motivoXSinVideo");
    }
    avisoGrimorio({ estado: "error", titulo: textoGrimorio("avisoNoGuardado"), detalle: motivo });
    return { ok: false, error: motivo };
  }

  async function guardar(medio) {
    if (medio.tagName === "IMG") return guardarFoto(medio);
    avisoGrimorio({ estado: "en curso", titulo: textoGrimorio("avisoBuscando") });
    return guardarVideo(medio);
  }

  botonGrimorio({ medioBajo: (e) => medioBajoX(document, e.clientX, e.clientY), guardar });
})();
