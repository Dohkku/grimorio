// La ventanita del botón de la extensión: si Grimorio está, y dónde guardar.
"use strict";

(async () => {
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
