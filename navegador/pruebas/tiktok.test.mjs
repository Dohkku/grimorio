// Pruebas de tiktok-datos.js con la forma de lo que TikTok sirve: el feed con
// el id en el reproductor, la cuadrícula con enlaces, y los datos que la
// página de un post trae dentro. Sin clases de TikTok, igual que el código.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const t = require("../tiktok-datos.js");

const HTML = `
<main>
  <article>
    <section>
      <div id="xgwrapper-0-7655304989836922132"><div><video class="feed" src="blob:https://www.tiktok.com/x"></video></div></div>
    </section>
    <a href="/@julia.b"><img class="avatar" src="https://cdn/avatar.jpeg"></a>
    <a href="/@julia.b">julia.b</a>
  </article>
  <div id="xgwrapper-0-7000000000000000001"><video class="solo"></video></div>
  <a href="https://www.tiktok.com/@otra/video/7693300954975751432?lang=es">
    <img class="miniatura" src="https://p16/tos-alisg-p-0037/ow6Ew1kVivF~tplv-photomode-zoomcover:720:720.jpeg">
  </a>
</main>`;

function pagina(url = "https://www.tiktok.com/foryou") {
  const dom = new JSDOM(HTML, { url });
  return { doc: dom.window.document, loc: dom.window.location };
}

const VIDEO = {
  id: "7693389183720721684",
  desc: "CIENTÍFICOS   DESCUBREN\nEL ARCA",
  author: { uniqueId: "saulbaena" },
  video: {
    width: 576, height: 658,
    playAddr: "https://v16-webapp-prime.tiktok.com/video/play-540",
    downloadAddr: "https://v16-webapp-prime.tiktok.com/video/descarga",
    bitrateInfo: [
      { CodecType: "h265_hvc1", PlayAddr: { Width: 720, Height: 822, UrlList: ["https://v16/720", "https://v19/720", "https://www.tiktok.com/aweme/v1/play/?id=1"] } },
      { CodecType: "h264", PlayAddr: { Width: 576, Height: 658, UrlList: ["https://v16/576-h264", "https://v19/576-h264"] } },
      { CodecType: "h265_hvc1", PlayAddr: { Width: 1080, Height: 1232, UrlList: ["https://v16/1080", "https://v19/1080", "https://www.tiktok.com/aweme/v1/play/?id=2"] } },
      { CodecType: "h265_hvc1", PlayAddr: { Width: 576, Height: 658, UrlList: ["https://v16/576-h265"] } },
    ],
  },
};

const FOTOS = {
  id: "7600000000000000000",
  desc: "",
  author: { uniqueId: "paisajes" },
  video: { playAddr: "", bitrateInfo: [] },
  imagePost: { images: [
    { imageWidth: 1080, imageHeight: 1440, imageURL: { urlList: ["https://p16/tos/aaa111~tplv-photomode-image.jpeg", "https://p19/tos/aaa111~tplv-photomode-image.jpeg"] } },
    { imageWidth: 1080, imageHeight: 1350, imageURL: { urlList: ["https://p16/tos/bbb222~tplv-photomode-image.jpeg"] } },
  ] },
};

test("en la cuadrícula el post es el del enlace de la miniatura, sin lo de detrás", () => {
  const { doc, loc } = pagina();
  assert.equal(t.postTikTok(doc.querySelector(".miniatura"), loc), "https://www.tiktok.com/@otra/video/7693300954975751432");
});

test("en el feed el post sale del id del reproductor y el autor de su enlace", () => {
  const { doc, loc } = pagina();
  assert.equal(t.postTikTok(doc.querySelector(".feed"), loc), "https://www.tiktok.com/@julia.b/video/7655304989836922132");
});

test("sin autor a la vista vale cualquiera; con la página del post, la página", () => {
  const { doc, loc } = pagina();
  assert.equal(t.postTikTok(doc.querySelector(".solo"), loc), "https://www.tiktok.com/@_/video/7000000000000000001");
  const p = pagina("https://www.tiktok.com/@alguien/video/7000000000000000001?is_from_webapp=1");
  assert.equal(t.postTikTok(p.doc.querySelector(".solo"), p.loc), "https://www.tiktok.com/@alguien/video/7000000000000000001");
});

test("fuera de un post y sin enlace no hay post", () => {
  const { doc, loc } = pagina("https://www.tiktok.com/explore");
  assert.equal(t.postTikTok(doc.querySelector(".avatar"), loc), null);
});

test("los datos del post salen de la página; sin ellos, null", () => {
  const datos = { __DEFAULT_SCOPE__: { "webapp.video-detail": { statusCode: 0, itemInfo: { itemStruct: VIDEO } } } };
  const html = `<html><script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">${JSON.stringify(datos)}</script></html>`;
  assert.equal(t.itemDelHtml(html).id, VIDEO.id);
  assert.equal(t.itemDelHtml("<html>inicia sesión</html>"), null);
  assert.equal(t.itemDelHtml('<script id="__UNIVERSAL_DATA_FOR_REHYDRATION__">{roto</script>'), null);
});

test("de un vídeo, la versión más grande primero y fuera las que dan una página", () => {
  const [m] = t.mediosTikTok(VIDEO);
  assert.equal(m.tipo, "video");
  assert.equal(m.url, "https://v16/1080");
  assert.deepEqual([m.ancho, m.alto], [1080, 1232]);
  assert.equal(m.alternativas[0], "https://v19/1080");
  assert.ok(!m.alternativas.some((u) => u.includes("/aweme/v1/play/")));
  // A igual tamaño, H.264 antes que H.265; la de descarga, con marca de agua, al final.
  assert.ok(m.alternativas.indexOf("https://v16/576-h264") < m.alternativas.indexOf("https://v16/576-h265"));
  assert.equal(m.alternativas.at(-1), "https://v16-webapp-prime.tiktok.com/video/descarga");
});

test("un vídeo sin versiones se queda con la de reproducir", () => {
  const [m] = t.mediosTikTok({ video: { width: 576, height: 1024, playAddr: "https://v16/play", bitrateInfo: [] } });
  assert.equal(m.url, "https://v16/play");
  assert.deepEqual([m.ancho, m.alto], [576, 1024]);
  assert.deepEqual(t.mediosTikTok({ video: {} }), []);
  assert.deepEqual(t.mediosTikTok(null), []);
});

test("de un carrusel de fotos, cada foto, y se reconoce la que se ve aunque esté recortada", () => {
  const medios = t.mediosTikTok(FOTOS);
  assert.equal(medios.length, 2);
  assert.equal(medios[0].tipo, "imagen");
  assert.deepEqual(medios[0].alternativas, ["https://p19/tos/aaa111~tplv-photomode-image.jpeg"]);
  assert.equal(t.elegirTikTok(medios, "https://p16/tos/bbb222~tplv-photomode-zoomcover:720:720.jpeg"), 1);
  assert.equal(t.elegirTikTok(medios, "https://otra/cosa.jpg"), 0);
});

test("el nombre lleva el autor y el texto del post, en una línea", () => {
  assert.equal(t.nombreTikTok(VIDEO, ""), "@saulbaena · CIENTÍFICOS DESCUBREN EL ARCA");
  assert.equal(t.nombreTikTok(FOTOS, "https://www.tiktok.com/@paisajes/photo/7600000000000000000"), "@paisajes · tiktok 7600000000000000000");
  assert.equal(t.nombreTikTok(null, ""), "tiktok");
});
