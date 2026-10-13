// Pruebas de elegir.js contra un HTML con la forma del de Instagram: capas
// encima de la foto, `srcset`, el enlace del post en la fecha, un avatar
// pequeño y un carrusel. Sin clases de Instagram, igual que el código.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const e = require("../elegir.js");

const HTML = `
<main>
  <article>
    <header>
      <a href="/fotografa.luz/"><img class="avatar" src="https://cdn/avatar.jpg" alt="avatar"></a>
      <a href="/fotografa.luz/">fotografa.luz</a>
    </header>
    <div class="medio">
      <ul>
        <li><img class="f1" alt="Photo by Fotógrafa Luz on May 3, 2024. May be an image of una calle mojada de noche"
             src="https://cdn/a-320.jpg"
             srcset="https://cdn/a-320.jpg 320w, https://cdn/a-1080.jpg?x=1,2 1080w, https://cdn/a-640.jpg 640w"></li>
        <li><img class="f2" alt="" src="https://cdn/b-640.jpg" srcset="https://cdn/b-640.jpg 640w, https://cdn/b-1440.jpg 1440w"></li>
        <li><img class="f2bis" alt="" src="https://cdn/b-640.jpg" srcset="https://cdn/b-640.jpg 640w, https://cdn/b-1440.jpg 1440w"></li>
      </ul>
      <div class="capa"></div><div class="capa2"></div>
    </div>
    <a href="/p/C9xYz12AbC/"><time>3 de mayo</time></a>
  </article>
</main>`;

function pagina(url = "https://www.instagram.com/") {
  const dom = new JSDOM(HTML, { url });
  const doc = dom.window.document;
  // jsdom no pinta: se le dice el tamaño de cada cosa.
  for (const img of doc.querySelectorAll("img")) {
    const lado = img.classList.contains("avatar") ? 32 : 600;
    img.getBoundingClientRect = () => ({ width: lado, height: lado, top: 0, left: 0, right: lado, bottom: lado });
  }
  return { doc, loc: dom.window.location };
}

test("del srcset se queda con lo más ancho, aunque la URL lleve comas", () => {
  assert.equal(
    e.mejorDeSrcset("https://cdn/a-320.jpg 320w, https://cdn/a-1080.jpg?x=1,2 1080w, https://cdn/a-640.jpg 640w"),
    "https://cdn/a-1080.jpg?x=1,2",
  );
  assert.equal(e.mejorDeSrcset("https://cdn/a.jpg 1x, https://cdn/a@2.jpg 2x"), "https://cdn/a@2.jpg");
  assert.equal(e.mejorDeSrcset(""), null);
});

test("atraviesa las capas transparentes y salta los avatares", () => {
  const { doc } = pagina();
  const capa = doc.querySelector(".capa");
  const avatar = doc.querySelector(".avatar");
  const foto = doc.querySelector(".f1");
  doc.elementsFromPoint = () => [capa, doc.querySelector(".capa2"), foto, doc.body];
  assert.equal(e.medioEnPunto(doc, 10, 10), foto);
  doc.elementsFromPoint = () => [avatar, doc.body];
  assert.equal(e.medioEnPunto(doc, 10, 10), null, "un avatar no es contenido");
});

test("el origen es el enlace permanente del post, también desde el feed", () => {
  const { doc, loc } = pagina("https://www.instagram.com/");
  assert.equal(e.enlaceDelPost(doc.querySelector(".f1"), loc), "https://www.instagram.com/p/C9xYz12AbC/");
  const enPost = pagina("https://www.instagram.com/fotografa.luz/p/C9xYz12AbC/?img_index=2");
  enPost.doc.querySelector('a[href^="/p/"]').remove();
  assert.equal(e.enlaceDelPost(enPost.doc.querySelector(".f1"), enPost.loc), "https://www.instagram.com/p/C9xYz12AbC/");
});

test("en la pestaña de Reels el post es el de la página, con su enlace permanente", () => {
  const reels = pagina("https://www.instagram.com/reels/DPq3xYzAbCd/");
  reels.doc.querySelector('a[href^="/p/"]').remove();
  const v = reels.doc.createElement("video");
  reels.doc.body.append(v);
  assert.equal(e.enlaceDelPost(v, reels.loc), "https://www.instagram.com/reel/DPq3xYzAbCd/");
  assert.equal(e.codigoDelEnlace(e.enlaceDelPost(v, reels.loc)), "DPq3xYzAbCd");
  // Las páginas de un audio de Reels no son un post.
  const audio = pagina("https://www.instagram.com/reels/audio/1234567890/");
  audio.doc.querySelector('a[href^="/p/"]').remove();
  assert.equal(e.enlaceDelPost(audio.doc.body, audio.loc), "https://www.instagram.com/reels/audio/1234567890/");
});

test("el nombre lleva el autor y el texto alternativo sin la coletilla automática", () => {
  const { doc, loc } = pagina();
  assert.equal(e.nombreDelMedio(doc.querySelector(".f1"), loc), "@fotografa.luz · una calle mojada de noche");
  assert.equal(e.nombreDelMedio(doc.querySelector(".f2"), loc), "@fotografa.luz · instagram C9xYz12AbC");
});

test("el carrusel da las fotos cargadas, a su mayor tamaño y sin repetir", () => {
  const { doc } = pagina();
  const fotos = e.imagenesDelCarrusel(doc.querySelector(".f1"));
  assert.deepEqual(fotos.map(e.mejorDeImagen), ["https://cdn/a-1080.jpg?x=1,2", "https://cdn/b-1440.jpg"]);
});
