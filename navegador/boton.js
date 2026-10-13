// El botón «a Grimorio» encima de lo que hay bajo el ratón, y el clic derecho
// que guarda eso mismo. Lo comparten TikTok y 4chan; cada uno dice qué cuenta
// como medio y cómo se guarda. (Instagram tiene el suyo, con «todas».)
//
// Vive en una sombra (shadow DOM): los estilos de la página no lo tocan y los
// suyos no se salen.
"use strict";

/// `medioBajo(evento)` da el elemento que se guardaría con ese ratón, o null;
/// `guardar(elemento)` lo guarda y devuelve lo que contestó el fondo.
function botonGrimorio({ medioBajo, guardar }) {
  const anfitrion = document.createElement("div");
  anfitrion.dataset.grimorio = "";
  anfitrion.style.cssText = "position:fixed;top:0;left:0;width:0;height:0;z-index:2147483647;";
  const sombra = anfitrion.attachShadow({ mode: "open" });
  sombra.innerHTML = `
    <style>
      .barra { position: fixed; display: none; }
      button {
        font: 600 13px/1 system-ui, sans-serif; color: #f2efe9;
        background: rgba(20, 20, 22, .82); border: 1px solid rgba(255,255,255,.25);
        border-radius: 999px; padding: 7px 11px; cursor: pointer;
      }
      button:hover { background: rgba(20, 20, 22, .95); border-color: rgba(255,255,255,.6); }
    </style>
    <div class="barra"><button class="una">a Grimorio</button></div>`;
  document.documentElement.appendChild(anfitrion);

  const barra = sombra.querySelector(".barra");
  const una = sombra.querySelector(".una");
  let actual = null;

  function colocar() {
    if (!actual || !actual.isConnected) return esconder();
    const r = actual.getBoundingClientRect();
    if (r.bottom < 0 || r.top > innerHeight) return esconder();
    barra.style.display = "block";
    // En una miniatura pequeña el botón no cabe dentro: se apoya en su borde
    // de arriba, sin salirse de la ventana.
    barra.style.top = Math.max(8, r.top + 6) + "px";
    barra.style.left = Math.max(8, Math.min(innerWidth - barra.offsetWidth - 8, r.right - barra.offsetWidth - 6)) + "px";
  }

  function esconder() {
    actual = null;
    barra.style.display = "none";
  }

  // Se mira un fotograma sí y otro no, que con mirar a cada movimiento sobra.
  let pendiente = false;
  document.addEventListener("mousemove", (e) => {
    if (pendiente) return;
    pendiente = true;
    requestAnimationFrame(() => {
      pendiente = false;
      // Encima del propio botón no se cambia de medio.
      if (e.composedPath().includes(anfitrion)) return;
      const medio = medioBajo(e);
      if (medio && medio !== actual) {
        actual = medio;
        colocar();
      } else if (!medio && actual) {
        const r = actual.getBoundingClientRect();
        const dentro = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
        if (!dentro) esconder();
      }
    });
  }, { passive: true });
  // En captura: TikTok desplaza una caja interior, no la página.
  addEventListener("scroll", colocar, { passive: true, capture: true });
  addEventListener("resize", colocar, { passive: true });

  // El clic derecho: se apunta qué había bajo el ratón al abrir el menú, y
  // «Guardar en Grimorio» guarda eso (ver `alPulsarMenu` en fondo.js).
  let bajoElClicDerecho = null;
  document.addEventListener("contextmenu", (e) => {
    bajoElClicDerecho = medioBajo(e);
  }, { capture: true, passive: true });
  chrome.runtime.onMessage.addListener((m, _de, responder) => {
    if (m.tipo !== "guardarClicDerecho") return false;
    if (!bajoElClicDerecho) {
      avisoGrimorio({ estado: "error", titulo: "No había ningún vídeo ni foto bajo el ratón" });
      responder({ ok: false, error: "nada bajo el ratón" });
      return false;
    }
    guardar(bajoElClicDerecho).then(responder);
    return true;
  });

  una.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (actual) guardar(actual);
  });
}

/// Guarda una lista ya resuelta y lo cuenta en el aviso, con `como` —cómo se
/// consiguió: original, lo que se ve…— debajo.
async function guardarYAvisar(lista, como) {
  if (lista.length === 0) {
    avisoGrimorio({ estado: "error", titulo: "No encuentro qué guardar", detalle: como || "" });
    return { ok: false, error: "nada que guardar" };
  }
  avisoGrimorio({ estado: "en curso", titulo: "Guardando en Grimorio…" });
  const r = await chrome.runtime.sendMessage({ tipo: "guardar", lista });
  const a = avisoDeResultado(r, lista.length);
  if (a.estado !== "error" && como) a.detalle = [a.detalle, como].filter(Boolean).join(" · ");
  avisoGrimorio(a);
  return r;
}
