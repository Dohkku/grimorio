[English](LEEME.md) · Español

# La extensión de Grimorio para el navegador

Guarda imágenes en tu biblioteca sin descargarlas antes:

- **Clic derecho → «Guardar en Grimorio»**, en cualquier web: en una imagen o
  un vídeo guarda ese archivo; en la página, **la página entera** (ver abajo);
  en un enlace, la imagen que lo representa (`og:image`). En Instagram guarda lo que hay bajo el ratón, con el
  mismo camino que el botón, aunque la foto esté tapada por capas.
- **En Instagram:** al pasar por encima de una foto o un vídeo sale un botón
  «a Grimorio», también en la cuadrícula de un perfil. Se lleva el original
  **entero**: lo que se ve en pantalla suele ser una miniatura recortada por
  el propio servidor —cuadrada en la cuadrícula—, así que la foto se pide a la
  misma API que usa la web de Instagram, con tu sesión. En un carrusel,
  «todas» se lleva todas, aunque no se hayan cargado. Si la API no contesta,
  se guarda la versión más grande de esa foto que el navegador ya haya bajado
  —lo mismo que se buscaría a mano en la pestaña «Img» de las herramientas de
  desarrollo—, y si no hay ninguna, lo que se ve. El aviso dice qué se guardó
  y cómo: «guardada · 1080×1350 · original», o «lo ya bajado (la API contestó
  403)». El origen que queda apuntado es el enlace del post, que no caduca; la
  dirección de la imagen sí.
- **En TikTok:** el mismo botón «a Grimorio» encima de cada vídeo, en el feed
  y en las cuadrículas, y el clic derecho guarda lo que hay bajo el ratón. El
  vídeo se reproduce desde un `blob:`, que no se puede bajar, así que se pide
  la página del post, que trae dentro dónde está el vídeo entero: se guarda la
  versión más grande —a menudo 1080p en H.265—, sin marca de agua. De un
  carrusel de fotos, la que se ve. Si la página no trae los datos (pide
  iniciar sesión, está borrado), una foto se guarda como se ve y un vídeo no.
- **En X (Twitter):** «a Grimorio» encima de cada foto, vídeo o gif, y el
  clic derecho guarda lo que hay bajo el ratón. De una foto, la original
  («name=orig», en jpg), no la reducida que se ve. De un vídeo, el mp4 más
  grande que da el servicio con el que X pinta los tuits incrustados (lo pide
  el fondo, sin tu sesión). Los tuits que ese servicio no enseña sin sesión,
  o marcados como sensibles, no dan su vídeo, y el aviso lo dice.
- **En 4chan:** al pasar por encima de una miniatura sale «a Grimorio», y el
  clic derecho guarda lo mismo: el archivo entero —imagen, gif o webm—, no la
  miniatura «…s.jpg» que se ve, con su nombre original (también si va con
  spoiler) y el enlace al post («/po/thread/123#p456»). En el catálogo solo
  hay miniatura, y se guarda esa con el asunto del hilo.
- **Dónde:** el botón de la extensión enseña si Grimorio está abierto y deja
  elegir la carpeta en la que entra todo.


## Instalar

1. **Conectar Grimorio con el navegador**, una vez: Ajustes → navegador →
   «conectar los navegadores», o en una terminal `grim navegador instalar`.
   Registra `grim` como programa nativo en Brave, Chrome, Chromium, Edge,
   Vivaldi y Firefox, los que estén instalados.
2. **Cargar la extensión**: en `brave://extensions` (o `chrome://extensions`)
   activa el modo de desarrollador, «cargar descomprimida» y elige esta
   carpeta (Ajustes → navegador → «abrir la carpeta de la extensión» la abre).

Su id es siempre `nhmdhkmjpnpclpiopkajinahdcmacfko`: sale de la clave pública
de `manifest.json`, y es el que `grim` deja hablar con Grimorio.

## Cómo funciona

```
extensión ──native messaging──▶ grim ──socket local──▶ Grimorio abierto
```

- La extensión no puede hablar con otros programas; el navegador solo la deja
  hablar con uno registrado para ella, que lanza él mismo. Ese es `grim` (ver
  `crates/grimorio-cli/src/nativo.rs`).
- `grim` le pasa cada mensaje a la aplicación por un socket local en
  `$XDG_RUNTIME_DIR` que solo puede abrir tu usuario (ver
  `app/src/puerta.h`). No es un puerto: un servidor en `localhost` lo podría
  llamar cualquier web que visites.
- La imagen la baja la extensión, con tu sesión, y la manda entera; si pasa
  de 40 MB manda la dirección y la baja Grimorio.
- Si Grimorio está cerrado, guardar algo lo abre. Mirar si está o pedir sus
  carpetas no abre nada.

Nada de esto sale de tu equipo: la foto va del navegador a tu biblioteca.

## Páginas enteras

En la página (clic derecho fuera de una imagen o un enlace) se guarda lo que se
está viendo, no lo que manda el servidor: el HTML ya pintado —con lo que sale
después de iniciar sesión o de bajar—, sin scripts, y con las hojas de estilo,
las imágenes y las tipografías dentro (`pagina.js`). Sale un solo `.html` que
se abre con cualquier navegador sin conexión. Su miniatura en Grimorio es una
captura de la pestaña, que va dentro del propio archivo
(`docs/FORMATO.es.md`).

Se quedan apuntando a la web, y solo se ven si sigue ahí, los vídeos, los
`iframe` (un vídeo de YouTube metido en un artículo) y lo que no quepa: se
meten hasta unos 24 MB, y cada archivo hasta 8 MB.

## Pruebas

```sh
cd navegador && npm install && npm test
```

Prueban `elegir.js` —qué imagen hay bajo el ratón atravesando las capas, cuál
es el tamaño mayor, el enlace del post, el nombre— contra un HTML con la forma
del de Instagram, sin sus clases, igual que el código; y `tiktok-datos.js`
—de qué post es cada vídeo, qué versión bajar— con la forma de lo que sirve
TikTok; y `4chan-datos.js` —el original detrás de cada miniatura, su nombre y
su post— con el HTML de un hilo; y `x-datos.js` —de qué tuit es una foto o un
vídeo, la foto original, el vídeo más grande— con la forma del timeline de X y
de los datos de los tuits incrustados.
