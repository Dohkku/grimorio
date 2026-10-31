// Los textos de _locales/: los dos idiomas tienen lo mismo, y todo lo que el
// código pide existe.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { mensajes, ponerIdioma } from "./idioma.mjs";

const es = mensajes("es");
const en = mensajes("en");
const raiz = new URL("../", import.meta.url);

test("español e inglés tienen las mismas claves y los mismos huecos", () => {
  assert.deepEqual(Object.keys(en).sort(), Object.keys(es).sort());
  for (const clave of Object.keys(es)) {
    const huecos = (e) => Object.keys(e.placeholders || {}).sort();
    assert.deepEqual(huecos(en[clave]), huecos(es[clave]), clave);
    assert.ok(en[clave].message, `${clave} vacío en inglés`);
  }
});

test("cada texto que pide el código está en _locales", () => {
  const fuentes = readdirSync(raiz).filter((f) => /\.(js|html|json)$/.test(f) && !f.startsWith("package"));
  const pedidas = new Set();
  for (const f of fuentes) {
    const s = readFileSync(new URL(f, raiz), "utf8");
    for (const m of s.matchAll(/(?:textoGrimorio|\bt|getMessage)\(\s*"([A-Za-z0-9_]+)"/g)) pedidas.add(m[1]);
    for (const m of s.matchAll(/__MSG_([A-Za-z0-9_]+)__/g)) pedidas.add(m[1]);
    for (const m of s.matchAll(/data-texto="([A-Za-z0-9_]+)"/g)) pedidas.add(m[1]);
    for (const m of s.matchAll(/textoGrimorio\(\s*[^,)]*\?\s*"([A-Za-z0-9_]+)"\s*:\s*"([A-Za-z0-9_]+)"/g)) {
      pedidas.add(m[1]);
      pedidas.add(m[2]);
    }
  }
  assert.ok(pedidas.size > 30, "se encontraron las claves");
  for (const clave of pedidas) assert.ok(es[clave], `falta «${clave}» en _locales`);
});

test("en un navegador en inglés el aviso sale en inglés", () => {
  ponerIdioma("en");
  const { avisoDeResultado } = createRequire(import.meta.url)("../aviso.js");
  const a = avisoDeResultado({ ok: true, resultados: [
    { ok: true, estado: "guardada", id: "1" },
    { ok: true, estado: "repetida", id: "2" },
    { ok: true, estado: "repetida", id: "3" },
    { ok: false, error: "x" },
  ] }, 4);
  assert.equal(a.titulo, "3 of 4 in Grimorio");
  assert.equal(a.detalle, "1 new · 2 already there · 1 failed");
  ponerIdioma("es");
});
