// El botón de 4chan: al pasar por encima de una miniatura sale «a Grimorio»,
// y el clic derecho guarda lo mismo. Las dos cosas guardan el archivo entero
// con su nombre original, no la miniatura (ver 4chan-datos.js). El botón es
// el de boton.js.
"use strict";

(() => {
  async function guardar(medio) {
    const a = archivo4chan(medio, location);
    if (!a) return guardarYAvisar([], "no sé de qué post es");
    return guardarYAvisar([{ url: a.url, origen: a.origen, nombre: a.nombre || document.title }], a.como);
  }

  botonGrimorio({ medioBajo: (e) => medioDe4chan(e.target), guardar });
})();
