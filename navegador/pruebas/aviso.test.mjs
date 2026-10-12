import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { avisoDeResultado } = require("../aviso.js");

test("una nueva: guardada, con nombre e id para verla", () => {
  const a = avisoDeResultado({ ok: true, resultados: [{ ok: true, estado: "guardada", id: "A", nombre: "foto" }] }, 1);
  assert.equal(a.estado, "guardada");
  assert.equal(a.id, "A");
  assert.equal(a.detalle, "foto");
});

test("una que ya estaba: repetida, señala la que había", () => {
  const a = avisoDeResultado({ ok: true, resultados: [{ ok: true, estado: "repetida", id: "B", nombre: "vieja" }] }, 1);
  assert.equal(a.estado, "repetida");
  assert.equal(a.id, "B");
  assert.match(a.titulo, /Ya la tenías/);
});

test("todo mal o sin respuesta: error con el motivo", () => {
  assert.equal(avisoDeResultado(undefined, 1).estado, "error");
  const a = avisoDeResultado({ ok: false, resultados: [{ ok: false, error: "no pude descargar" }] }, 1);
  assert.equal(a.estado, "error");
  assert.equal(a.detalle, "no pude descargar");
  assert.equal(a.id, undefined);
});

test("carrusel mezclado: cuenta cada cosa", () => {
  const a = avisoDeResultado({ ok: true, resultados: [
    { ok: true, estado: "guardada", id: "1" },
    { ok: true, estado: "repetida", id: "2" },
    { ok: false, error: "x" },
  ] }, 3);
  assert.equal(a.titulo, "2 de 3 en Grimorio");
  assert.equal(a.detalle, "1 nueva · 1 ya la tenías · 1 con error");
  assert.equal(a.id, "1");
});

test("un Grimorio viejo que solo dice ok: guardada, sin botón de ver", () => {
  const a = avisoDeResultado({ ok: true, resultados: [{ ok: true, nombre: "x" }] }, 1);
  assert.equal(a.estado, "guardada");
  assert.equal(a.id, undefined);
});

test("descarga larga: en camino, sin botón de ver", () => {
  const a = avisoDeResultado({ ok: true, resultados: [{ ok: true, estado: "en camino" }] }, 1);
  assert.equal(a.estado, "en curso");
  assert.equal(a.id, undefined);
});
