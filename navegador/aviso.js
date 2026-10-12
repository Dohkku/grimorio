// El aviso de lo que pasó al guardar, abajo a la derecha de la página.
//
// Uno solo para Instagram y para cualquier otra web: lo pinta el script de
// Instagram, y en las demás lo mete el fondo al guardar con el clic derecho.
// Vive en una sombra para que los estilos de la página no lo toquen.
//
// Estados: «en curso» mientras se guarda, «guardada», «repetida» (ya estaba
// en la biblioteca) y «error». Con un id, ofrece verlo en Grimorio.
"use strict";

var avisoGrimorio = avisoGrimorio || (() => {
  let raiz = null;
  let caja = null;
  let olvido = null;

  const COLORES = {
    "en curso": "#8d8a83",
    guardada: "#4caf7d",
    repetida: "#5b9bd5",
    error: "#e0675a",
  };

  function montar() {
    if (raiz && raiz.isConnected) return;
    raiz = document.createElement("div");
    raiz.dataset.grimorioAviso = "";
    raiz.style.cssText = "position:fixed;right:20px;bottom:20px;z-index:2147483647;";
    const sombra = raiz.attachShadow({ mode: "open" });
    sombra.innerHTML = `
      <style>
        .caja {
          display: none; min-width: 260px; max-width: 360px;
          font: 13px/1.35 system-ui, sans-serif; color: #efece6;
          background: rgba(22, 22, 25, .96); border-radius: 12px;
          box-shadow: 0 8px 28px rgba(0,0,0,.45); overflow: hidden;
          border-left: 5px solid var(--color);
        }
        .dentro { padding: 12px 14px; display: flex; gap: 10px; align-items: flex-start; }
        .icono { font-size: 18px; line-height: 1; color: var(--color); margin-top: 1px; }
        .textos { flex: 1; min-width: 0; }
        .titulo { font-weight: 650; }
        .detalle { color: #b8b4ab; margin-top: 2px; overflow: hidden; text-overflow: ellipsis;
                   display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
        .ver { margin-top: 8px; font: 600 12px system-ui, sans-serif; color: #efece6;
               background: rgba(255,255,255,.08); border: 1px solid rgba(255,255,255,.22);
               border-radius: 999px; padding: 5px 11px; cursor: pointer; }
        .ver:hover { background: rgba(255,255,255,.16); }
        .cerrar { background: none; border: none; color: #8d8a83; cursor: pointer; font-size: 15px; }
        .girando { display: inline-block; animation: g 1s linear infinite; }
        @keyframes g { to { transform: rotate(360deg); } }
      </style>
      <div class="caja"><div class="dentro">
        <div class="icono"></div>
        <div class="textos"><div class="titulo"></div><div class="detalle"></div>
          <button class="ver" hidden>ver en Grimorio</button></div>
        <button class="cerrar" title="cerrar">×</button>
      </div></div>`;
    (document.body || document.documentElement).appendChild(raiz);
    caja = sombra.querySelector(".caja");
    sombra.querySelector(".cerrar").addEventListener("click", () => { caja.style.display = "none"; });
    // Con el ratón encima no se va: da tiempo a leerlo y a pulsar «ver».
    caja.addEventListener("mouseenter", () => clearTimeout(olvido));
    caja.addEventListener("mouseleave", () => programarOlvido(3000));
  }

  function programarOlvido(ms) {
    clearTimeout(olvido);
    olvido = setTimeout(() => { if (caja) caja.style.display = "none"; }, ms);
  }

  /// `estado`, `titulo`, `detalle` y, si se puede ver, `id`.
  return function avisar({ estado, titulo, detalle, id }) {
    montar();
    const s = caja.getRootNode();
    caja.style.setProperty("--color", COLORES[estado] || COLORES["en curso"]);
    s.querySelector(".icono").innerHTML =
      estado === "en curso" ? '<span class="girando">◌</span>'
        : estado === "error" ? "✕" : estado === "repetida" ? "●" : "✓";
    s.querySelector(".titulo").textContent = titulo;
    s.querySelector(".detalle").textContent = detalle || "";
    s.querySelector(".detalle").hidden = !detalle;
    const ver = s.querySelector(".ver");
    ver.hidden = !id;
    ver.onclick = () => {
      chrome.runtime.sendMessage({ tipo: "mostrar", id });
      caja.style.display = "none";
    };
    caja.style.display = "block";
    if (estado === "en curso") clearTimeout(olvido);
    else programarOlvido(estado === "error" ? 9000 : 7000);
  };
})();

/// El aviso que toca para lo que contestó el fondo al guardar.
function avisoDeResultado(r, cuantos) {
  // Un Grimorio de antes de la 0.1.7 contesta solo `{ok:true}`, sin estado ni id.
  const res = ((r && r.resultados) || []).map((x) => (x.ok && !x.estado ? { ...x, estado: "guardada" } : x));
  const bien = res.filter((x) => x.ok && x.estado !== "en camino");
  const nuevas = res.filter((x) => x.ok && x.estado === "guardada");
  const ya = res.filter((x) => x.ok && x.estado === "repetida");
  const camino = res.filter((x) => x.ok && x.estado === "en camino");
  const mal = res.filter((x) => !x.ok);
  const primero = nuevas[0] || ya[0];
  if (res.length === 0 || (mal.length === res.length)) {
    return { estado: "error", titulo: "No se ha guardado",
             detalle: (mal[0] && mal[0].error) || (r && r.error) || "Grimorio no contestó" };
  }
  if (cuantos === 1 && camino.length === 1) {
    return { estado: "en curso", titulo: "Grimorio sigue bajándola",
             detalle: "Es grande; entrará en unos momentos." };
  }
  if (cuantos === 1) {
    return primero.estado === "repetida"
      ? { estado: "repetida", titulo: "Ya la tenías en Grimorio", detalle: primero.nombre, id: primero.id }
      : { estado: "guardada", titulo: "Guardada en Grimorio", detalle: primero.nombre, id: primero.id };
  }
  const partes = [];
  if (nuevas.length) partes.push(`${nuevas.length} nueva${nuevas.length === 1 ? "" : "s"}`);
  if (ya.length) partes.push(`${ya.length} ya la${ya.length === 1 ? "" : "s"} tenías`);
  if (camino.length) partes.push(`${camino.length} en camino`);
  if (mal.length) partes.push(`${mal.length} con error`);
  return { estado: mal.length ? "error" : nuevas.length ? "guardada" : "repetida",
           titulo: `${bien.length + camino.length} de ${res.length} en Grimorio`,
           detalle: partes.join(" · "), id: primero && primero.id };
}

if (typeof module !== "undefined") module.exports = { avisoDeResultado };
