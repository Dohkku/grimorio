// Pruebas de 4chan-datos.js con el HTML de un hilo de 4chan tal cual lo sirve:
// cada archivo en su `.file`, el enlace al original en el nombre y en la
// miniatura, y la miniatura «…s.jpg».
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const c = require("../4chan-datos.js");

const HTML = `
<div class="board">
<div class="thread" id="t627472">
  <div class="postContainer opContainer" id="pc627472"><div id="p627472" class="post op">
    <div class="file" id="f627472"><div class="fileText" id="fT627472">File: <a href="//i.4cdn.org/po/1745614740915414.png" target="_blank">yotsuba_folding.png</a> (179 KB, 530x449)</div><a class="fileThumb" href="//i.4cdn.org/po/1745614740915414.png" target="_blank"><img class="miniatura" src="//i.4cdn.org/po/1745614740915414s.jpg" alt="179 KB" loading="lazy"></a></div>
    <img class="bandera" src="//s.4cdn.org/image/sticky.gif">
  </div></div>
  <div class="postContainer replyContainer" id="pc627480"><div id="p627480" class="post reply">
    <div class="file" id="f627480"><div class="fileText" id="fT627480" title="mi grulla secreta.jpg">File: <a href="//i.4cdn.org/po/1745620000000001.jpg" target="_blank">Spoiler Image</a> (80 KB, 1200x900)</div><a class="fileThumb imgspoiler" href="//i.4cdn.org/po/1745620000000001.jpg" target="_blank"><img class="spoiler" src="//s.4cdn.org/image/spoiler-po1.png"></a></div>
  </div></div>
  <div class="postContainer replyContainer" id="pc627490"><div id="p627490" class="post reply">
    <div class="file" id="f627490"><div class="fileText" id="fT627490">File: <a title="un nombre muy largo que 4chan recorta en pantalla.webm" href="//i.4cdn.org/po/1745630000000002.webm" target="_blank">un nombre muy lar(...).webm</a></div><a class="fileThumb" href="//i.4cdn.org/po/1745630000000002.webm"><img class="mini-webm" src="//i.4cdn.org/po/1745630000000002s.jpg"></a><video class="expandedWebm" src="//i.4cdn.org/po/1745630000000002.webm"></video></div>
  </div></div>
</div>
</div>
<div id="catalogo"><div class="thread" id="thread-1"><a href="//boards.4chan.org/po/thread/600000"><img class="thumb" src="//i.4cdn.org/po/1700000000000000s.jpg"></a><div class="teaser"><b>Grullas de mil colores</b>: hilo de grullas</div></div></div>`;

function pagina(url = "https://boards.4chan.org/po/thread/627472/welcome-to-po") {
  const dom = new JSDOM(HTML, { url });
  return { doc: dom.window.document, loc: dom.window.location };
}

test("de la miniatura se guarda el original, con su nombre y el enlace al post", () => {
  const { doc, loc } = pagina();
  const a = c.archivo4chan(doc.querySelector(".miniatura"), loc);
  assert.equal(a.url, "https://i.4cdn.org/po/1745614740915414.png");
  assert.equal(a.origen, "https://boards.4chan.org/po/thread/627472#p627472");
  assert.equal(a.nombre, "/po/ · yotsuba_folding");
});

test("con spoiler, el nombre de verdad; recortado, el entero", () => {
  const { doc, loc } = pagina();
  assert.equal(c.archivo4chan(doc.querySelector(".spoiler"), loc).nombre, "/po/ · mi grulla secreta");
  const w = c.archivo4chan(doc.querySelector(".mini-webm"), loc);
  assert.equal(w.nombre, "/po/ · un nombre muy largo que 4chan recorta en pantalla");
  assert.equal(w.url, "https://i.4cdn.org/po/1745630000000002.webm");
  assert.equal(w.origen, "https://boards.4chan.org/po/thread/627472#p627490");
});

test("un vídeo ya abierto en su sitio es el mismo archivo", () => {
  const { doc, loc } = pagina();
  const v = doc.querySelector(".expandedWebm");
  assert.equal(c.medioDe4chan(v), v);
  assert.equal(c.archivo4chan(v, loc).url, "https://i.4cdn.org/po/1745630000000002.webm");
});

test("en el índice del tablón el hilo sale de su caja", () => {
  const { doc, loc } = pagina("https://boards.4chan.org/po/");
  assert.equal(c.archivo4chan(doc.querySelector(".mini-webm"), loc).origen, "https://boards.4chan.org/po/thread/627472#p627490");
});

test("los iconos de la página no son medios; el catálogo da su miniatura", () => {
  const { doc, loc } = pagina("https://boards.4chan.org/po/catalog");
  assert.equal(c.medioDe4chan(doc.querySelector(".bandera")), null);
  const mini = doc.querySelector(".thumb");
  assert.equal(c.medioDe4chan(mini), mini);
  const a = c.archivo4chan(mini, loc);
  assert.equal(a.url, "https://i.4cdn.org/po/1700000000000000s.jpg");
  assert.equal(a.origen, "https://boards.4chan.org/po/thread/600000");
  assert.equal(a.como, "miniatura del catálogo");
  assert.equal(a.nombre, "/po/ · Grullas de mil colores");
});

test("sin extensión, solo la última", () => {
  assert.equal(c.sinExtension("foto.final.jpeg"), "foto.final");
  assert.equal(c.sinExtension("sin punto"), "sin punto");
});
