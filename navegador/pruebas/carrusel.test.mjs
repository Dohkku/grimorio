// La foto de verdad en un carrusel y el plan B de cuando la API no contesta.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const e = require("../elegir.js");

test("en un carrusel solo cuenta la foto que se ve, no las recortadas por el borde", () => {
  const dom = new JSDOM(`<body><div class="marco" style="overflow:hidden">
    <ul><li><img class="antes"></li><li><img class="esta"></li><li><img class="despues"></li></ul></div></body>`);
  const doc = dom.window.document;
  const caja = (izq) => () => ({ left: izq, right: izq + 400, top: 0, bottom: 400, width: 400, height: 400 });
  doc.querySelector(".marco").getBoundingClientRect = caja(0);
  doc.querySelector("ul").getBoundingClientRect = caja(-400);
  doc.querySelectorAll("li").forEach((li, i) => { li.getBoundingClientRect = caja(-400 + i * 400); });
  // Las tres en su sitio del carril: la de antes asoma por la izquierda.
  doc.querySelector(".antes").getBoundingClientRect = caja(-400);
  doc.querySelector(".esta").getBoundingClientRect = caja(0);
  doc.querySelector(".despues").getBoundingClientRect = caja(400);
  for (const img of doc.querySelectorAll("img")) {
    Object.defineProperty(img, "naturalWidth", { value: 1080 });
  }
  // El navegador da en ese punto la de antes también (por cómo se apila).
  doc.elementsFromPoint = () => [doc.querySelector(".antes"), doc.querySelector(".esta")];
  assert.equal(e.medioEnPunto(doc, 200, 200), doc.querySelector(".esta"));
  assert.equal(e.seVeEn(doc.querySelector(".despues"), 200, 200), false);
});

test("un post abierto dice en su dirección qué foto del carrusel se ve", () => {
  assert.equal(e.indiceDeLaPagina(new URL("https://www.instagram.com/p/X/?img_index=3")), 2);
  assert.equal(e.indiceDeLaPagina(new URL("https://www.instagram.com/p/X/")), null);
  const medios = [0, 1, 2].map((i) => ({ archivos: new Set([`${i}_n.jpg`]) }));
  assert.equal(e.elegirDelPost(medios, "https://cdn/zzz_n.jpg", 2), 2, "si el archivo no se reconoce, manda la dirección");
  assert.equal(e.elegirDelPost(medios, "https://cdn/1_n.jpg", 2), 1, "pero el archivo manda sobre la dirección");
});

test("las cabeceras llevan el token de la cookie, como las de la propia web", () => {
  const h = e.cabecerasApi("ig_did=abc; csrftoken=T0K3N; mid=x");
  assert.equal(h["X-CSRFToken"], "T0K3N");
  assert.equal(h["X-IG-App-ID"], "936619743392459");
  assert.ok(h["X-ASBD-ID"]);
  assert.equal(e.cabecerasApi("").hasOwnProperty("X-CSRFToken"), false);
});

test("de lo ya bajado se queda con la variante más grande de la misma foto", () => {
  const cdn = "https://scontent-mad1-1.cdninstagram.com/v/t51.2885-15/555_n.jpg";
  const sinPeso = [
    { name: `${cdn}?stp=dst-jpg_e35_s150x150_tt6`, decodedBodySize: 0 },
    { name: `${cdn}?stp=dst-jpg_e35_p1080x1080_tt6`, decodedBodySize: 0 },
    { name: `${cdn}?stp=dst-jpg_e35_s640x640_tt6`, decodedBodySize: 0 },
    { name: "https://scontent.cdninstagram.com/v/t51.2885-15/OTRA_n.jpg?stp=dst-jpg_e35_s2000x2000", decodedBodySize: 0 },
  ];
  assert.match(e.mejorCargada(sinPeso, "555_n.jpg").url, /p1080x1080/, "por el tamaño escrito en la URL");
  const conPeso = [
    { name: `${cdn}?a`, decodedBodySize: 20000 },
    { name: `${cdn}?b`, decodedBodySize: 310000 },
  ];
  assert.match(e.mejorCargada(conPeso, "555_n.jpg").url, /\?b$/, "y por el peso cuando el navegador lo da");
  assert.equal(e.mejorCargada(sinPeso, "nada.jpg"), null);
  assert.ok(e.tamanoDeUrl(`${cdn}?stp=dst-jpg`) > e.tamanoDeUrl(`${cdn}?stp=dst-jpg_e35_p1080x1080`),
            "sin tamaño escrito es la completa");
});
