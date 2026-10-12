// La ventanita del botón de la extensión: si Grimorio está, y dónde guardar.
"use strict";

/// Hace cuánto, en corto: «ahora», «5 min», «2 h», «3 d».
function haceCuanto(ms) {
  const s = Math.max(0, (Date.now() - ms) / 1000);
  if (s < 60) return "ahora";
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86400)} d`;
}

/// Lo último guardado: qué pasó con cada cosa, y pulsar lleva a Grimorio.
async function pintarUltimas() {
  const { ultimas = [] } = await chrome.storage.local.get("ultimas");
  if (ultimas.length === 0) return;
  document.getElementById("ultimas").hidden = false;
  const lista = document.getElementById("lista");
  const MARCA = { guardada: "✓", repetida: "●", error: "✕", "en camino": "◌" };
  const TEXTO = { guardada: "guardada", repetida: "ya la tenías", error: "no se guardó", "en camino": "en camino" };
  for (const u of ultimas) {
    const fila = document.createElement("div");
    fila.className = "ultima " + (u.estado === "en camino" ? "camino" : u.estado);
    fila.title = u.estado === "error" ? u.error : TEXTO[u.estado] || "";
    const marca = document.createElement("span");
    marca.className = "marca";
    marca.textContent = MARCA[u.estado] || "·";
    const nombre = document.createElement("span");
    nombre.className = "nombre";
    nombre.textContent = u.nombre || TEXTO[u.estado] || "";
    const cuando = document.createElement("span");
    cuando.className = "cuando";
    cuando.textContent = haceCuanto(u.cuando);
    fila.append(marca, nombre, cuando);
    if (u.id) {
      fila.dataset.id = u.id;
      fila.addEventListener("click", () => {
        chrome.runtime.sendMessage({ tipo: "mostrar", id: u.id });
        window.close();
      });
    }
    lista.append(fila);
  }
}

(async () => {
  pintarUltimas();
  const estado = document.getElementById("estado");
  const destino = document.getElementById("destino");
  const select = document.getElementById("carpeta");
  const r = await chrome.runtime.sendMessage({ tipo: "estado" });

  if (!r || !r.hola.ok) {
    estado.classList.add("mal");
    if (r && r.hola.sinConectar) {
      estado.innerHTML = "Falta conectar Grimorio con el navegador. En una terminal: <code>grim navegador instalar</code>";
    } else if (r && r.hola.cerrada) {
      estado.textContent = "Grimorio no está abierto. Al guardar algo se abre solo.";
    } else {
      estado.textContent = (r && r.hola.error) || "no hay respuesta de Grimorio";
    }
    return;
  }

  estado.textContent = `Conectado con «${r.hola.biblioteca}».`;
  destino.hidden = false;
  const sinCarpeta = new Option("ninguna carpeta (Todo)", "");
  select.add(sinCarpeta);
  for (const c of r.carpetas) {
    select.add(new Option("  ".repeat(c.nivel || 0) + c.nombre, c.id));
  }
  select.value = r.carpetas.some((c) => c.id === r.carpeta) ? r.carpeta : "";
  select.addEventListener("change", () => chrome.storage.local.set({ carpeta: select.value }));
})();
