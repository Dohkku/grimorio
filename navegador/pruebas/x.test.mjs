// Pruebas de x-datos.js con la forma de lo que X sirve: un tuit en el feed
// con su hora enlazada, una foto que enlaza a «/photo/1», un tuit citado y los
// datos del servicio de los incrustados. Sin clases de X, igual que el código.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const x = require("../x-datos.js");

const HTML = `
<main>
  <article>
    <a href="/aranita"><img class="avatar" src="https://pbs.twimg.com/profile_images/1/a_normal.jpg"></a>
    <a href="/aranita/status/1800000000000000001"><time datetime="2026-10-01">1 oct</time></a>
    <div data-testid="tweetText" lang="es">Bocetos de hoy https://t.co/AbC123</div>
    <a href="/aranita/status/1800000000000000001/photo/1">
      <div><img class="foto" src="https://pbs.twimg.com/media/GaBcDeF?format=jpg&amp;name=small"></div>
    </a>
    <div><video class="video" poster="https://pbs.twimg.com/ext_tw_video_thumb/1575559336759263233/pu/img/Kvc2.jpg" src="blob:https://x.com/1"></video></div>
    <div role="link"><a href="/otro/status/1700000000000000002">citado</a></div>
  </article>
  <img class="suelta" src="https://pbs.twimg.com/media/Zz?format=png&amp;name=900x900">
</main>`;

function pagina(url = "https://x.com/home") {
  const dom = new JSDOM(HTML, { url });
  return { doc: dom.window.document, loc: dom.window.location };
}

const SINDICACION = {
  __typename: "Tweet",
  text: "Absolutely heartbreaking footage https://t.co/xyz",
  user: { screen_name: "MesoMax919" },
  mediaDetails: [{
    type: "video",
    media_url_https: "https://pbs.twimg.com/ext_tw_video_thumb/1575559336759263233/pu/img/Kvc2iofyjHrt1DmI.jpg",
    original_info: { width: 1920, height: 1080 },
    video_info: { variants: [
      { content_type: "application/x-mpegURL", url: "https://video.twimg.com/ext_tw_video/1575559336759263233/pu/pl/3qM.m3u8?tag=12" },
      { bitrate: 256000, content_type: "video/mp4", url: "https://video.twimg.com/ext_tw_video/1575559336759263233/pu/vid/480x270/Yq.mp4?tag=12" },
      { bitrate: 2176000, content_type: "video/mp4", url: "https://video.twimg.com/ext_tw_video/1575559336759263233/pu/vid/1280x720/AO.mp4?tag=12" },
    ] },
  }],
  quoted_tweet: {
    mediaDetails: [{ type: "photo", media_url_https: "https://pbs.twimg.com/media/Cita.jpg", original_info: { width: 800, height: 600 } }],
  },
};

test("el enlace de un tuit, limpio, venga de x.com o de twitter.com", () => {
  const { loc } = pagina();
  assert.equal(x.enlaceX("/aranita/status/18/photo/1?s=20", loc), "https://x.com/aranita/status/18");
  assert.equal(x.enlaceX("https://twitter.com/a_b/status/123", loc), "https://x.com/a_b/status/123");
  assert.equal(x.enlaceX("/i/web/status/99", loc), "https://x.com/i/status/99");
  assert.equal(x.enlaceX("/aranita", loc), null);
  assert.equal(x.enlaceX("https://example.com/a/status/1", loc), null);
});

test("el tuit de una foto es el de su enlace; el de un vídeo, el de la hora", () => {
  const { doc, loc } = pagina();
  assert.equal(x.tuitX(doc.querySelector(".foto"), loc), "https://x.com/aranita/status/1800000000000000001");
  assert.equal(x.tuitX(doc.querySelector(".video"), loc), "https://x.com/aranita/status/1800000000000000001");
});

test("fuera de un tuit, la página si es la de uno (el visor de fotos)", () => {
  const { doc, loc } = pagina("https://x.com/aranita/status/555/photo/2");
  assert.equal(x.tuitX(doc.querySelector(".suelta"), loc), "https://x.com/aranita/status/555");
  assert.equal(x.tuitX(pagina().doc.querySelector(".suelta"), pagina().loc), null);
});

test("la foto a su tamaño original", () => {
  assert.equal(x.fotoOriginalX("https://pbs.twimg.com/media/GaBcDeF?format=jpg&name=small"),
               "https://pbs.twimg.com/media/GaBcDeF?format=jpg&name=orig");
  assert.equal(x.fotoOriginalX("https://pbs.twimg.com/media/GaBcDeF.png:large"),
               "https://pbs.twimg.com/media/GaBcDeF?format=png&name=orig");
  assert.equal(x.fotoOriginalX("https://pbs.twimg.com/media/FeWr?format=webp&name=small"),
               "https://pbs.twimg.com/media/FeWr?format=jpg&name=orig");
  assert.equal(x.fotoOriginalX("https://pbs.twimg.com/media/Cita.jpg"),
               "https://pbs.twimg.com/media/Cita?format=jpg&name=orig");
  // Los avatares y lo que no es de X se quedan como están.
  assert.equal(x.fotoOriginalX("https://pbs.twimg.com/profile_images/1/a_normal.jpg"),
               "https://pbs.twimg.com/profile_images/1/a_normal.jpg");
  assert.equal(x.fotoOriginalX("https://example.com/a.jpg?name=small"), "https://example.com/a.jpg?name=small");
});

test("el token del servicio de los incrustados, como lo calcula X", () => {
  assert.equal(x.tokenX("1575560063510810624"), "3thrn8435ls");
  assert.equal(x.tokenX("1349794411333394432"), "39si5g9b29u");
});

test("del vídeo, el mp4 más grande con su tamaño de verdad, y los demás de repuesto", () => {
  const medios = x.mediosX(SINDICACION);
  assert.equal(medios.length, 2);
  const v = x.elegirVideoX(medios, "https://pbs.twimg.com/ext_tw_video_thumb/1575559336759263233/pu/img/Kvc2.jpg");
  assert.match(v.url, /1280x720/);
  assert.deepEqual([v.ancho, v.alto], [1280, 720]);
  assert.equal(v.alternativas.length, 1);
  assert.ok(!v.alternativas.some((u) => /m3u8/.test(u)));
  // La foto del tuit citado, original.
  assert.equal(medios[1].url, "https://pbs.twimg.com/media/Cita?format=jpg&name=orig");
});

test("un vídeo en una tarjeta también cuenta", () => {
  const tarjeta = { __typename: "Tweet", card: { binding_values: { unified_card: { string_value: JSON.stringify({
    media_entities: { "13_1": { type: "video", media_url_https: "https://pbs.twimg.com/amplify_video_thumb/1349/img/b.jpg",
      video_info: { variants: [{ bitrate: 2176000, content_type: "video/mp4", url: "https://video.twimg.com/amplify_video/1349/vid/1280x720/AV.mp4" }] } } },
  }) } } } };
  const v = x.elegirVideoX(x.mediosX(tarjeta), "https://pbs.twimg.com/amplify_video_thumb/1349/img/otra.jpg");
  assert.match(v.url, /amplify_video\/1349\/vid/);
});

test("sin datos del tuit no hay medios, y no se elige un vídeo ajeno", () => {
  assert.deepEqual(x.mediosX(null), []);
  assert.deepEqual(x.mediosX({ __typename: "TweetTombstone" }), []);
  const dos = [{ tipo: "video", claves: ["1"] }, { tipo: "video", claves: ["2"] }];
  assert.equal(x.elegirVideoX(dos, "https://pbs.twimg.com/amplify_video_thumb/3/img/a.jpg"), null);
});

test("el nombre: autor y texto, sin los enlaces t.co", () => {
  assert.equal(x.nombreX(SINDICACION, "https://x.com/MesoMax919/status/1"), "@MesoMax919 · Absolutely heartbreaking footage");
  assert.equal(x.nombreX({ text: "" }, "https://x.com/aranita/status/18"), "@aranita · x 18");
  assert.equal(x.nombreX({}, "https://x.com/i/status/18"), "x 18");
  const { doc } = pagina();
  assert.equal(x.textoEnPagina(doc.querySelector(".foto")).trim(), "Bocetos de hoy https://t.co/AbC123");
});
