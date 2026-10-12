// La foto sin recortar: leer lo que contesta la API de Instagram y reconocer
// cuál de las fotos del post era la que se estaba mirando.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const e = require("../elegir.js");

test("el código del post es su id en base 64", () => {
  assert.equal(e.idDeCodigo("A"), "0");
  assert.equal(e.idDeCodigo("B"), "1");
  assert.equal(e.idDeCodigo("_"), "63");
  assert.equal(e.idDeCodigo("BA"), "64");
  // Los ids de verdad pasan de 2^53: tiene que ir en BigInt y no perder nada.
  assert.equal(e.idDeCodigo("C9xYz12AbC"), (() => {
    const A = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let n = 0n;
    for (const c of "C9xYz12AbC") n = n * 64n + BigInt(A.indexOf(c));
    return n.toString();
  })());
  assert.equal(e.idDeCodigo("C9xYz12AbCdPrivado"), e.idDeCodigo("C9xYz12AbCd"), "solo cuentan los once primeros");
  assert.equal(e.idDeCodigo("con espacio"), null);
  assert.equal(e.codigoDelEnlace("https://www.instagram.com/reel/XyZ_1-2/"), "XyZ_1-2");
});

const CDN = "https://scontent.cdninstagram.com/v/t51.2885-15";
const CARRUSEL = {
  carousel_media: [
    {
      media_type: 1,
      image_versions2: { candidates: [
        { url: `${CDN}/111_n.jpg?stp=dst-jpg_e35_p1080x1080`, width: 1080, height: 1350 },
        { url: `${CDN}/111_n.jpg?stp=c0.135.1080.1080a_s640x640`, width: 640, height: 640 },
      ] },
    },
    {
      media_type: 2,
      image_versions2: { candidates: [{ url: `${CDN}/222_n.jpg?stp=s640x640`, width: 640, height: 800 }] },
      video_versions: [
        { url: "https://scontent.cdninstagram.com/o1/v/222.mp4?efg=480", width: 480, height: 600 },
        { url: "https://scontent.cdninstagram.com/o1/v/222.mp4?efg=720", width: 720, height: 900 },
      ],
    },
  ],
};

test("de cada medio del post sale lo más grande: la foto entera y el vídeo", () => {
  const m = e.mediosDelPost(CARRUSEL);
  assert.equal(m.length, 2);
  assert.deepEqual([m[0].tipo, m[0].ancho, m[0].alto], ["imagen", 1080, 1350]);
  assert.match(m[0].url, /p1080x1080/, "la entera, no el recorte cuadrado");
  assert.deepEqual([m[1].tipo, m[1].ancho], ["video", 720]);
  assert.equal(e.mediosDelPost({ image_versions2: { candidates: [] } }).length, 0);
  assert.equal(e.mediosDelPost(null).length, 0);
});

test("se reconoce la foto mirada aunque en pantalla estuviera recortada", () => {
  const m = e.mediosDelPost(CARRUSEL);
  assert.equal(e.elegirDelPost(m, `${CDN}/111_n.jpg?stp=c0.135.1080.1080a_s320x320&_nc_ht=x`), 0);
  assert.equal(e.elegirDelPost(m, `${CDN}/222_n.jpg?stp=s150x150`), 1, "por la portada del vídeo");
  assert.equal(e.elegirDelPost(m, `${CDN}/999_n.jpg`), null, "si no se reconoce, no se inventa");
  assert.equal(e.elegirDelPost([m[0]], "lo-que-sea"), 0, "con uno solo, ese");
});

test("en la cuadrícula de un perfil el origen es el post de esa miniatura", () => {
  const dom = new JSDOM(`<main>
    <header><a href="/fotografa.luz/">fotografa.luz</a> <a href="/p/OTRO1/">fijado</a></header>
    <div><a href="/p/OTRO2/"><img class="a" src="x"></a><a href="/fotografa.luz/p/ESTA/"><div><img class="b" src="y"></div></a></div>
  </main>`, { url: "https://www.instagram.com/fotografa.luz/" });
  const doc = dom.window.document;
  assert.equal(e.enlaceDelPost(doc.querySelector(".b"), dom.window.location), "https://www.instagram.com/p/ESTA/");
  assert.equal(e.enlaceDelPost(doc.querySelector(".a"), dom.window.location), "https://www.instagram.com/p/OTRO2/");
});
