// La ventanita del botón de la extensión: si Grimorio está, y dónde guardar.
"use strict";

/// Un texto en el idioma del navegador (_locales/).
const t = (clave, ...huecos) => chrome.i18n.getMessage(clave, huecos.map(String)) || clave;

// Los textos fijos de ventana.html, que en una página de la extensión no se
// pueden poner con __MSG_…__.
document.documentElement.lang = t("idioma");
for (const el of document.querySelectorAll("[data-texto]")) el.textContent = t(el.dataset.texto);

/// Hace cuánto, en corto: «ahora», «5 min», «2 h», «3 d».
function haceCuanto(ms) {
  const s = Math.max(0, (Date.now() - ms) / 1000);
  if (s < 60) return t("haceAhora");
  if (s < 3600) return t("haceMinutos", Math.floor(s / 60));
  if (s < 86400) return t("haceHoras", Math.floor(s / 3600));
  return t("haceDias", Math.floor(s / 86400));
}

/// Lo último guardado: qué pasó con cada cosa, y pulsar lleva a Grimorio.
async function pintarUltimas() {
  const { ultimas = [] } = await chrome.storage.local.get("ultimas");
  if (ultimas.length === 0) return;
  document.getElementById("ultimas").hidden = false;
  const lista = document.getElementById("lista");
  const MARCA = { guardada: "✓", repetida: "●", error: "✕", "en camino": "◌" };
  const TEXTO = { guardada: t("ultimaGuardada"), repetida: t("ultimaRepetida"), error: t("ultimaError"),
                  "en camino": t("ultimaEnCamino") };
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
      const orden = document.createElement("code");
      orden.textContent = "grim navegador instalar";
      estado.replaceChildren(t("ventanaSinConectar") + " ", orden);
    } else if (r && r.hola.cerrada) {
      estado.textContent = t("ventanaCerrada");
    } else {
      estado.textContent = (r && r.hola.error) || t("ventanaSinRespuesta");
    }
    return;
  }

  estado.textContent = t("ventanaConectado", r.hola.biblioteca);
  destino.hidden = false;
  const sinCarpeta = new Option(t("ventanaSinCarpeta"), "");
  select.add(sinCarpeta);
  for (const c of r.carpetas) {
    select.add(new Option("  ".repeat(c.nivel || 0) + c.nombre, c.id));
  }
  select.value = r.carpetas.some((c) => c.id === r.carpeta) ? r.carpeta : "";
  select.addEventListener("change", () => chrome.storage.local.set({ carpeta: select.value }));
})();
