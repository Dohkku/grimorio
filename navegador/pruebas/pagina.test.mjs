// La página entera: que salga un solo HTML que se lee sin conexión.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { JSDOM } from "jsdom";

const require = createRequire(import.meta.url);
const p = require("../pagina.js");

/// Un `bajar` de mentira: lo que hay en `web`, por dirección.
function web(archivos) {
  const pedidos = [];
  const bajar = async (url, { texto }) => {
    pedidos.push(url);
    const a = archivos[url];
    if (a === undefined) return null;
    return texto ? { texto: a } : { datos: `data:${a.tipo};base64,${Buffer.from(a.cuerpo).toString("base64")}` };
  };
  return { bajar, pedidos };
}

function pagina(html, url = "https://ejemplo.com/blog/entrada.html") {
  return new JSDOM(html, { url }).window.document;
}

test("el CSS lleva dentro sus @import y sus url()", async () => {
  const { bajar } = web({
    "https://ejemplo.com/css/base.css": "body { background: url(fondo.png) }",
    "https://ejemplo.com/css/fondo.png": { tipo: "image/png", cuerpo: "PNG" },
  });
  const g = p.guardador(bajar);
  const css = await p.cssConRecursos(
    '@import url("base.css") screen;\nh1 { background: url(\'nada.png\') } a { b: url(data:image/png;base64,AA) }',
    "https://ejemplo.com/css/estilo.css", g);
  assert.match(css, /@media screen \{\nbody \{ background: url\("data:image\/png;base64,UE5H"\) \}\n\}/);
  assert.match(css, /url\("https:\/\/ejemplo\.com\/css\/nada\.png"\)/, "lo que no se pudo bajar, con la dirección entera");
  assert.match(css, /url\(data:image\/png;base64,AA\)/, "lo que ya va dentro no se toca");
});

test("el presupuesto corta, y nada se baja dos veces", async () => {
  const { bajar, pedidos } = web({
    "https://a.com/1.png": { tipo: "image/png", cuerpo: "x".repeat(300) },
    "https://a.com/2.png": { tipo: "image/png", cuerpo: "y".repeat(300) },
  });
  const g = p.guardador(bajar, 500);
  assert.ok(await g.datos("https://a.com/1.png"));
  assert.ok(await g.datos("https://a.com/1.png"));
  assert.equal(pedidos.length, 1);
  assert.equal(await g.datos("https://a.com/2.png"), null, "ya no queda presupuesto");
  assert.equal(await g.datos("file:///etc/passwd"), null);
});

test("la página sale entera, sin scripts y con la captura delante", async () => {
  const doc = pagina(`<!DOCTYPE html><html><head>
    <meta charset="iso-8859-1"><base href="/blog/">
    <meta http-equiv="Content-Security-Policy" content="img-src 'self'">
    <title>Entrada</title>
    <link rel="stylesheet" href="https://cdn.otro.com/estilo.css">
    <link rel="preload" href="/x.js" as="script">
    <script src="/app.js"></script>
  </head><body onload="pasa()">
    <div data-grimorio-aviso></div>
    <h1>Título</h1>
    <p>Texto <a href="otra.html">enlace</a> <a href="javascript:alert(1)">mal</a></p>
    <img src="foto.jpg" srcset="foto-2x.jpg 2x" loading="lazy" alt="foto">
    <img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" data-src="perezosa.jpg">
    <iframe src="/video"></iframe>
    <div style="background-image: url('/fondo.png')"></div>
    <input value="escrito"><textarea>viejo</textarea>
    <script>alert("no")</script>
  </body></html>`);
  doc.querySelector("input").value = "cambiado";
  doc.querySelector("textarea").value = "nuevo";
  const { bajar } = web({
    "https://cdn.otro.com/estilo.css": "h1 { color: red }",
    "https://ejemplo.com/blog/foto.jpg": { tipo: "image/jpeg", cuerpo: "JPG" },
    "https://ejemplo.com/blog/perezosa.jpg": { tipo: "image/jpeg", cuerpo: "PER" },
    "https://ejemplo.com/fondo.png": { tipo: "image/png", cuerpo: "PNG" },
  });
  const html = await p.archivarPaginaGrimorio(doc, {
    bajar, captura: "data:image/jpeg;base64,Q0FQ", fecha: "2026-11-17T12:00:00Z",
  });

  assert.match(html, /^<!DOCTYPE html>\n<!-- Guardada con Grimorio desde https:\/\/ejemplo\.com\/blog\/entrada\.html/);
  const salida = new JSDOM(html).window.document;
  const head = salida.head.children;
  assert.equal(head[0].getAttribute("charset"), "utf-8", "la codificación, la primera");
  assert.equal(head[1].getAttribute("content"), "https://ejemplo.com/blog/entrada.html");
  assert.equal(salida.querySelector("meta[name='grimorio:captura']").getAttribute("content"),
               "data:image/jpeg;base64,Q0FQ");
  assert.equal(salida.querySelectorAll("meta[charset]").length, 1);
  assert.equal(salida.querySelectorAll("script, base, meta[http-equiv], link[rel=preload], [data-grimorio-aviso]").length, 0);
  assert.equal(salida.body.getAttribute("onload"), null);
  assert.equal(salida.querySelector("style").textContent, "h1 { color: red }");
  assert.equal(salida.querySelector("link[rel=stylesheet]"), null);

  const [foto, perezosa] = salida.querySelectorAll("img");
  assert.equal(foto.getAttribute("src"), "data:image/jpeg;base64,SlBH");
  assert.equal(foto.getAttribute("srcset"), null);
  assert.equal(foto.getAttribute("alt"), "foto");
  assert.equal(perezosa.getAttribute("src"), "data:image/jpeg;base64,UEVS");

  const [enlace, malo] = salida.querySelectorAll("a");
  assert.equal(enlace.getAttribute("href"), "https://ejemplo.com/blog/otra.html");
  assert.equal(malo.getAttribute("href"), null);
  assert.equal(salida.querySelector("iframe").getAttribute("src"), "https://ejemplo.com/video");
  assert.match(salida.querySelector("div[style]").getAttribute("style"), /url\("data:image\/png;base64,UE5H"\)/);
  assert.equal(salida.querySelector("input").getAttribute("value"), "cambiado");
  assert.equal(salida.querySelector("textarea").textContent, "nuevo");
  assert.equal(salida.querySelectorAll("[data-grimorio-i]").length, 0, "sin marcas");
  assert.equal(doc.querySelectorAll("[data-grimorio-i]").length, 0, "ni en la página de verdad");
});
