// El botón de Instagram: al pasar por encima de una foto sale «a Grimorio».
//
// El clic derecho no sirve en Instagram: cada foto está tapada por capas
// transparentes y el navegador nunca ve una imagen debajo. Así que se mira qué
// hay bajo el ratón atravesando las capas (`medioEnPunto`), y se pinta un botón
// propio encima. El botón vive en una sombra (shadow DOM): los estilos de
// Instagram no lo tocan y los suyos no se salen.
"use strict";

(() => {
  const anfitrion = document.createElement("div");
  anfitrion.dataset.grimorio = "";
  anfitrion.style.cssText = "position:fixed;top:0;left:0;width:0;height:0;z-index:2147483647;";
  // Abierta y no cerrada: no protege nada —la página puede tapar el botón
  // igual—, y así las pruebas lo encuentran.
  const sombra = anfitrion.attachShadow({ mode: "open" });
  sombra.innerHTML = `
    <style>
      .barra { position: fixed; display: none; gap: 6px; }
      button {
        font: 600 13px/1 system-ui, sans-serif; color: #f2efe9;
        background: rgba(20, 20, 22, .82); border: 1px solid rgba(255,255,255,.25);
        border-radius: 999px; padding: 7px 11px; cursor: pointer;
      }
      button:hover { background: rgba(20, 20, 22, .95); border-color: rgba(255,255,255,.6); }
      button[disabled] { opacity: .6; cursor: default; }
    </style>
    <div class="barra"><button class="una">a Grimorio</button><button class="todas"></button></div>`;
  document.documentElement.appendChild(anfitrion);

  const barra = sombra.querySelector(".barra");
  const una = sombra.querySelector(".una");
  const todas = sombra.querySelector(".todas");
  let actual = null;
  let carrusel = [];

  /// Lo que va pasando, en el aviso común (aviso.js).
  function avisar(titulo, estado = "en curso", detalle = "") {
    avisoGrimorio({ estado, titulo, detalle });
  }

  function colocar() {
    if (!actual || !actual.isConnected) return esconder();
    const r = actual.getBoundingClientRect();
    if (r.bottom < 0 || r.top > innerHeight) return esconder();
    barra.style.display = "flex";
    barra.style.top = Math.max(8, r.top + 10) + "px";
    barra.style.left = Math.max(8, r.right - barra.offsetWidth - 10) + "px";
  }

  function esconder() {
    actual = null;
    barra.style.display = "none";
  }

  function mostrarPara(medio) {
    actual = medio;
    una.textContent = "a Grimorio";
    una.disabled = false;
    carrusel = medio.tagName === "IMG" ? imagenesDelCarrusel(medio) : [];
    // «todas» y no un número: la API trae las del carrusel aunque no se hayan
    // cargado, así que las que se ven no son todas las que hay.
    todas.style.display = carrusel.length > 1 ? "" : "none";
    todas.textContent = "todas";
    colocar();
  }

  // El ratón por encima: lo que haya debajo, atravesando las capas. Se mira
  // un fotograma sí y otro no, que con mirar a cada movimiento sobra.
  let pendiente = false;
  document.addEventListener("mousemove", (e) => {
    if (pendiente) return;
    pendiente = true;
    requestAnimationFrame(() => {
      pendiente = false;
      // Encima del propio botón no se cambia de foto.
      if (e.composedPath().includes(anfitrion)) return;
      const medio = medioEnPunto(document, e.clientX, e.clientY);
      if (medio && medio !== actual) mostrarPara(medio);
      else if (!medio && actual) {
        const r = actual.getBoundingClientRect();
        const dentro = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
        if (!dentro) esconder();
      }
    });
  }, { passive: true });
  addEventListener("scroll", colocar, { passive: true });
  addEventListener("resize", colocar, { passive: true });

  /// Todo lo que el navegador ha bajado en esta página —lo que sale en la
  /// pestaña «Img» de las herramientas de desarrollo—, por si la API no
  /// contesta: abrir un post o pasar un carrusel baja la foto grande, y de
  /// ahí se puede guardar en vez de la miniatura. Lo lee el fondo en el mundo
  /// de la página: desde aquí, el mundo aislado de la extensión, no se ve.
  async function cargadas() {
    try {
      const r = await chrome.runtime.sendMessage({ tipo: "cargadas" });
      return (r && r.entradas) || [];
    } catch (_) {
      return [];
    }
  }

  /// Lo que la API dice del post, o por qué no lo dijo.
  async function mediosDeLaApi(enlace) {
    const id = idDeCodigo(codigoDelEnlace(enlace));
    if (!id) return { motivo: "no sé qué post es" };
    try {
      const r = await fetch(`/api/v1/media/${id}/info/`, {
        credentials: "include",
        headers: cabecerasApi(document.cookie),
      });
      if (!r.ok) return { motivo: `la API contestó ${r.status}` };
      const j = await r.json();
      const medios = mediosDelPost(j && j.items && j.items[0]);
      return medios.length ? { medios } : { motivo: "la API no trajo fotos" };
    } catch (e) {
      return { motivo: `la API no contestó (${e.message})` };
    }
  }

  function srcVisto(medio) {
    if (medio.tagName === "IMG") return medio.currentSrc || medio.src;
    // Un vídeo se reproduce desde un «blob:»; lo que lleva su nombre de
    // archivo es la portada.
    return medio.getAttribute("poster") || "";
  }

  function conIndice(nombre, i, n) {
    return n > 1 ? `${nombre} (${i + 1} de ${n})` : nombre;
  }

  /// Sin la API: la mayor versión de esta foto entre lo que se ve (`srcset`)
  /// y lo que el navegador ya bajó.
  function sinApi(img, origen, nombre, entradas) {
    const deSrcset = mejorDeImagen(img);
    const cargada = mejorCargada(entradas, archivoDe(srcVisto(img)));
    const url = cargada && tamanoDeUrl(cargada.url) > tamanoDeUrl(deSrcset) ? cargada.url : deSrcset;
    return { url, origen, nombre };
  }

  /// Lo que se manda a guardar para `medio`, o para todo su post si `todos`,
  /// y cómo se ha conseguido, para decirlo.
  async function pedidos(medio, todos) {
    const origen = enlaceDelPost(medio, location);
    const nombre = nombreDelMedio(medio, location);
    const api = await mediosDeLaApi(origen);
    const medios = api.medios;
    if (medios) {
      if (todos) {
        return { lista: medios.map((m, i) => ({ url: m.url, origen, nombre: conIndice(nombre, i, medios.length) })),
                 como: "originales" };
      }
      let i = elegirDelPost(medios, srcVisto(medio), indiceDeLaPagina(location));
      // Un vídeo sin portada reconocible: el primer vídeo del post.
      if (i === null && medio.tagName === "VIDEO") i = medios.findIndex((m) => m.tipo === "video");
      if (i !== null && i >= 0) {
        const m = medios[i];
        return { lista: [{ url: m.url, origen, nombre: conIndice(nombre, i, medios.length) }],
                 como: `${m.ancho}×${m.alto} · original` };
      }
      api.motivo = "no reconozco cuál del carrusel es";
    }
    if (medio.tagName !== "IMG") return { lista: [], como: api.motivo };
    const entradas = await cargadas();
    if (todos) {
      return { lista: carrusel.map((img) => sinApi(img, origen, nombreDelMedio(img, location), entradas)),
               como: `lo ya bajado (${api.motivo})` };
    }
    const p = sinApi(medio, origen, nombre, entradas);
    const grande = p.url !== mejorDeImagen(medio);
    return { lista: [p], como: `${grande ? "lo ya bajado" : "lo que se ve"} (${api.motivo})` };
  }

  async function guardar(medio, todos) {
    avisar("Buscando el original…");
    const { lista: encontrados, como } = await pedidos(medio, todos);
    const lista = encontrados.filter((p) => p.url);
    if (lista.length === 0) {
      avisar("No encuentro qué guardar", "error", como || "");
      return { ok: false, error: "nada que guardar" };
    }
    avisar(lista.length === 1 ? "Guardando en Grimorio…" : `Guardando ${lista.length} en Grimorio…`);
    const r = await chrome.runtime.sendMessage({ tipo: "guardar", lista });
    // Lo que pasó de verdad, y debajo cómo se consiguió (original, lo ya
    // bajado…), que es lo que dice si la calidad es la buena.
    const a = avisoDeResultado(r, lista.length);
    if (a.estado !== "error" && como) a.detalle = [a.detalle, como].filter(Boolean).join(" · ");
    avisoGrimorio(a);
    return r;
  }

  // El clic derecho: Instagram tapa cada foto con capas y el navegador no
  // sabe que debajo hay una imagen. Se apunta qué había bajo el ratón al
  // abrir el menú, y «Guardar en Grimorio» guarda eso (ver fondo.js).
  let bajoElClicDerecho = null;
  document.addEventListener("contextmenu", (e) => {
    bajoElClicDerecho = medioEnPunto(document, e.clientX, e.clientY);
  }, { capture: true, passive: true });
  chrome.runtime.onMessage.addListener((m, _de, responder) => {
    if (m.tipo !== "guardarClicDerecho") return false;
    if (!bajoElClicDerecho) {
      avisar("No había ninguna foto ni vídeo bajo el ratón", "error");
      responder({ ok: false, error: "nada bajo el ratón" });
      return false;
    }
    guardar(bajoElClicDerecho, false).then(responder);
    return true;
  });

  una.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (actual) guardar(actual, false);
  });
  todas.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (actual) guardar(actual, true);
  });
})();
