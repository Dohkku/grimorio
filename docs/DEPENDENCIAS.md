# Política de dependencias

## La regla

Una dependencia entra solo si cumple las tres:

1. Tiene mantenimiento activo y alguien serio la usa en producción.
2. Escribirla nosotros costaría más de dos semanas.
3. Su licencia es compatible con la AGPL-3.0 de Grimorio (ver abajo).

Si no, se escribe en casa. Ya hemos escrito: los ULID, el formato de fechas
RFC 3339, el pack de miniaturas, la extracción de paleta en Oklab, el hash
perceptual, el recorrido de directorios y el motor de consultas.

El árbol completo de dependencias debe caber en una revisión de una tarde. Se
audita en cada hito con `cargo tree --duplicates` y `cargo deny` cuando se añada.

## Licencia del proyecto y qué se puede enlazar

Grimorio empezó como binario cerrado y gratuito (modelo Obsidian), y de ahí
viene la prudencia de esta lista. Desde la primera versión pública es software
libre bajo la **AGPL-3.0**. Eso amplía lo que se puede usar, pero se mantienen
las reglas de antes porque siguen siendo buenas (pocas dependencias, Qt
dinámico) y porque dejan abierta la puerta a ofrecer Grimorio con otra
licencia más adelante, algo que solo puede hacer quien tiene el copyright de
todo el código.

| Licencia | ¿Sirve? | Condición |
|---|---|---|
| MIT / Apache-2 / BSD / dominio público | sí | ninguna |
| LGPL 2.1 / 3 | sí | **solo enlazado dinámico**, con aviso y posibilidad de sustituir la biblioteca |
| MPL-2.0 | sí | los cambios en sus archivos se publican |
| GPL-3.0 / AGPL-3.0 | sí, con cuidado | son compatibles con la AGPL, pero atan cualquier licencia futura a ellas; solo si no hay alternativa |

**Regla dura del proyecto: nunca enlazar Qt ni ninguna LGPL de forma estática.**
Enlazado estático de Qt exige licencia comercial. En los paquetes (`.deb`,
Flatpak, AppImage) las bibliotecas LGPL van como `.so` separadas y se incluyen
sus textos de licencia.

## Estado actual

Dependencias directas del núcleo y el CLI:

| Crate | Para qué | Licencia | Nota |
|---|---|---|---|
| `rusqlite` (+ `bundled`) | índice y búsqueda | MIT | compila SQLite dentro; SQLite es dominio público |
| `serde` / `serde_json` | `item.json`, `library.json`, temas | MIT/Apache-2 | |
| `image` | decodificar y escalar JPEG/PNG/GIF/WebP/BMP | MIT/Apache-2 | con `default-features = false` |
| `blake3` | identidad de contenido | CC0/Apache-2 | |
| `rayon` | importación en paralelo | MIT/Apache-2 | |
| `memmap2` | mapear el pack de miniaturas | MIT/Apache-2 | |
| `thiserror` | errores del núcleo | MIT/Apache-2 | |
| `miniz_oxide` | inflar los zip de 3MF, y de `.kra`, `.ora` y `.sketch` (sin `unzip`, que en Windows no está) | MIT/Apache-2/Zlib | ya venía compilado dentro por el PNG de `image`; nombrarla no añade código |
| `clap` | interfaz del CLI | MIT/Apache-2 | solo en el binario `grim` |
| `tempfile` | pruebas | MIT/Apache-2 | solo `dev-dependencies` |

Solo en los spikes (no forman parte del producto):

- `spikes/grid-wgpu`: `wgpu`, `winit`, `pollster`, `bytemuck`, todas MIT/Apache-2.
- `spikes/grid-qml`: Qt 6 Core/Gui/Quick/Qml, LGPLv3, **enlazadas de forma
  dinámica** desde el primer día, como exige la regla dura de arriba. El puente
  con el núcleo (`grimorio-puente`) sí se enlaza estático, pero es código propio.

### Lo que Qt 6 exige en la práctica, medido

Instalar Qt 6 en Ubuntu 24.04 para compilar y ejecutar el spike de QML deja tres
avisos que hay que tener en cuenta antes de elegirlo como interfaz:

1. `qt6-base-dev` + `qt6-declarative-dev` + `cmake` bastan para **compilar**.
2. Para **ejecutar** hacen falta además siete paquetes `qml6-module-*` que
   Debian no declara como dependencias. Sin ellos, `import QtQuick` falla en
   tiempo de ejecución sin decir qué instalar. Cualquier paquete que hagamos
   (`.deb`, Flatpak, AppImage) tendrá que listarlos a mano.
3. No hay `qsb` ni QtShaderTools en la instalación base: sin un paquete extra no
   se puede compilar ningún `ShaderEffect` propio.

## Decisiones pendientes de cara a v1

| Área | Elección | Licencia | Por qué |
|---|---|---|---|
| Miniaturas rápidas | **libvips** | LGPL-2.1+ | reducción durante la carga: miniaturar un JPEG de 24 MP sin decodificarlo entero |
| RAW | **LibRaw** | LGPL-2.1 o CDDL | estándar de facto, en subproceso aislado |
| PDF | **pdfium** | BSD-3 | MuPDF sería compatible (AGPL), pero ataría la licencia |
| Vídeo | **ffmpeg como binario externo** | LGPL-2.1+ | proceso separado: ni enlazamos ni heredamos su superficie de fallos; nunca una build con banderas GPL |
| Tipografías | Rust puro | MIT/Apache-2 | solo hace falta rasterizar una muestra |
| Interfaz | **Qt 6 / QML** | LGPLv3 | enlazado dinámico obligatorio |

## Herramientas de fuera, opcionales

Se llaman como programas, en procesos aislados y con tope de tiempo. Ninguna
es obligatoria: sin ellas los archivos entran igual y les falta algo.

| Programa | Para qué | Sin él |
|---|---|---|
| `ffmpeg` / `ffprobe` | fotograma y duración de vídeos; copia en H.264 de lo que Qt no abre; onda y espectro | vídeo sin cara, sonido sin onda |
| `pdftoppm` / `pdfinfo` | primera página y páginas del visor | PDF sin cara ni visor |
| `blender` | abrir `.blend` y `.fbx` (`blender -b`, sin ventana) | entran a ciegas |
| ImageMagick (`magick`, o `convert` en la versión 6) | PSD, XCF y muestras de tipografías | entran a ciegas |
| `gs` (Ghostscript; `gswin64c` en Windows) | EPS | entra a ciegas |

En Windows se buscan igual, en el `PATH` (Blender, además, en su carpeta de
instalación), y en Windows ImageMagick se llama siempre como `magick`: allí
`convert` es la herramienta del sistema que convierte discos de FAT a NTFS.
Cómo instalarlas está en `docs/WINDOWS.md`. En el instalador de Windows van
además las DLL de Qt, con sus licencias y los avisos de terceros
(`installer/AVISOS-TERCEROS.txt`).

Un programa al que se llama por ruta absoluta no se busca lanzándolo con
`-version`: Blender no entiende esa opción y abría su ventana entera, una por
cada pregunta.

## Qué no entra, y por qué

- **Qt Quick 3D**: va con licencia GPL o comercial, no LGPL. El visor de
  modelos es OpenGL a mano sobre Qt Gui (`Qt6::OpenGL`, LGPL), que para una
  malla sin texturas es un búfer y dos sombreadores.
- **QtPdf**: sería LGPL y valdría, pero `pdftoppm` ya estaba para las
  miniaturas, en proceso aparte, y un PDF roto no puede tirar la aplicación.

- **Runtimes de JS de terceros para plugins**: Qt ya trae su motor de JS. Traer
  otro son decenas de megas y una superficie de seguridad nueva.
- **Frameworks de UI web**: el punto entero del proyecto es no tener un
  navegador dentro.
- **Bibliotecas de IA**: fuera por decisión de producto, no por licencia.
- **Cajas de utilidades de una sola función** (`itertools`, `lazy_static` y
  compañía): la biblioteca estándar de Rust ya llega.
