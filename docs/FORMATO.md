# El formato de biblioteca de Grimorio

Versión de esquema: **1**

Esta es la parte del proyecto que promete durar más que el propio programa. Si
Grimorio desaparece, esta especificación tiene que bastar para recuperar todo
con un script de veinte líneas.

## Estructura en disco

```
MiBiblioteca.grimorio/
├── library.json          identidad y ajustes de la biblioteca
├── folders.json          el árbol de carpetas
├── etiquetas.json        grupos de etiquetas con su color
├── busquedas.json        carpetas dinámicas (búsquedas con nombre)
├── vigiladas.json        carpetas del disco que se importan solas
├── abierta.lock          cerrojo mientras la aplicación la tiene abierta
├── index.sqlite          ÍNDICE DERIVADO — borrable, se reconstruye
├── items/
│   └── 01/JK/01JKX7…/    sharding de dos niveles por los 4 primeros
│       ├── item.json     caracteres del id
│       └── original.jpg
├── cache/                DERIVADO — borrable, se rehace al abrir cada cosa
│   ├── proxy/01/JK/…mp4  copia en H.264 de los vídeos que Qt no abre
│   ├── tira/             fotogramas en rejilla para la barra del vídeo
│   ├── onda/             onda y espectro de cada sonido (`.onda`)
│   ├── pdf/              páginas dibujadas, por número y ancho
│   ├── malla/            modelos 3D listos para el visor (`.malla`)
│   └── versiones/        temporal de una versión mientras se importa
├── thumbs/
│   ├── grid.pack         miniaturas de 320 px concatenadas
│   └── preview/01/…jpg   previsualizaciones de 1024 px
├── trash/                borrado reversible
├── themes/
└── plugins/
```

**El sharding no es decoración.** Un directorio con 100.000 entradas hace lento
cualquier `readdir`, y las herramientas de escritorio se atragantan. Con dos
niveles de dos caracteres, ninguna carpeta pasa de unos cientos de entradas.

## Identificadores

ULID de 26 caracteres en Crockford base32: 48 bits de milisegundos + 80 bits de
azar. Dos propiedades que usamos en todas partes:

- **Ordenar por id es ordenar por fecha de importación.** La vista por defecto
  de la malla no ordena nada: lee el índice en orden de clave primaria.
- Son únicos sin coordinación, así que se pueden generar en paralelo durante una
  importación de miles de archivos.

## `library.json`

```json
{
  "schema": 1,
  "id": "01JKF8...",
  "name": "Referencias",
  "createdAt": "2026-08-25T18:40:12Z",
  "importMode": "copy",
  "appVersion": "0.1.0"
}
```

`importMode` es `copy` (por defecto, el original vive dentro), `move` o `ref`
(el original se queda donde está).

## `item.json`

```json
{
  "id": "01JKX7Q2M8V3N4P5R6S7T8",
  "schema": 1,
  "name": "estudio de iluminación nocturna",
  "ext": "jpg",
  "kind": "imagen",
  "size": 4821904,
  "width": 4032,
  "height": 3024,
  "hash": "9f2c…",
  "phash": "d4a1c39e7b208f16",
  "importedAt": "2026-08-25T10:12:33Z",
  "modifiedAt": "2026-08-25T10:12:33Z",
  "source": "https://…",
  "folders": ["01JKF…"],
  "tags": ["nocturno", "referencia"],
  "stars": 4,
  "note": "textura del reflejo en el asfalto",
  "palette": [{ "rgb": [18, 32, 64], "w": 0.41 }],
  "origin": { "mode": "copy", "path": "/home/…", "inode": 5241923 },
  "trashed": false
}
```

- `hash` es blake3 del contenido en hexadecimal: la identidad real del archivo,
  y la base de la detección de duplicados.
- `phash` es un dHash de 64 bits en hexadecimal, para "parecidas a esta".
- `palette` va ordenada por peso; el primer color es el dominante.
- `kind` es la familia: `imagen`, `raw`, `video`, `audio`, `documento`,
  `tipografia` o `modelo`. Se saca de la extensión; está escrita para no
  tener que saber cuál es cuál.
- `adult` (solo si es `true`) marca lo que el modo seguro difumina.
- `rotation` (0, 90, 180 o 270) es el giro con el que se mira. El original no
  se gira.
- `orden` guarda el orden a mano: por cada carpeta (su id, o `""` para
  «Todo»), la posición del elemento. Son números con hueco entre ellos para
  meter uno en medio sin renumerar.
- `origin.inode` permite reencontrar un original en modo `ref` que alguien haya
  movido de sitio.
- `durationMs` y `pages` van en lo que dura y en lo que tiene páginas.
- `pila` y `receta` van en las versiones (ver «Pilas de versiones», abajo).
- `triangles` y `sizeMm` van en los modelos 3D. `sizeMm` es ancho, fondo y
  alto en milímetros, y solo está si el formato dice sus unidades: STL (que se
  escribe en milímetros por costumbre), 3MF, glTF y lo que sale de Blender. OBJ
  y PLY no lo dicen, y ahí no se inventa.
- Los campos vacíos se omiten al escribir. Un lector debe tolerar que falten.

### Pilas de versiones

La misma imagen con distinto peso —una exportación para la web, la copia que
alguien mandó recomprimida— se guarda como una **pila**: una principal y sus
versiones. Cada versión es un elemento completo, con su carpeta en `items/`, su
original y su miniatura; lo único que la distingue es que apunta a su
principal:

```json
{
  "id": "01JKX9…",
  "name": "estudio de iluminación nocturna (jpg 1600 px 82 %)",
  "ext": "jpg",
  "pila": "01JKX7Q2M8V3N4P5R6S7T8",
  "receta": { "formato": "jpg", "calidad": 82, "lado": 1600 }
}
```

- `pila` es el id de la principal. Solo lo llevan las versiones; la principal
  no tiene lista de versiones, así que no hay dos sitios que puedan dejar de
  coincidir. Las pilas son planas: una versión nunca es principal de otra.
- `receta` solo está si la versión la hizo Grimorio: `formato` (`jpg`, `png`
  o `webp`), `calidad` (de 1 a 100; cuenta en JPEG y en WebP, donde 100 es
  sin pérdida) y `lado` (el lado mayor en píxeles; 0 deja el tamaño). Nunca
  amplía. El original no se toca.
- `receta.recorte`, si está, dice con qué modelo se quitó el fondo
  (`isnet-general-use`): la versión es un PNG del mismo tamaño con
  transparencia donde estaba el fondo.
- `receta.tramo`, si está, es un trozo de un vídeo o de un sonido sacado con
  el editor: `desde_ms` y `hasta_ms` sobre el original, y `exacto` si se
  recodificó para cortar en el fotograma justo (si no, se copió sin
  recodificar y el archivo arranca en el fotograma clave de antes, con una
  lista de edición que lo deja empezar donde se pidió). El formato es el del
  original.
- Las etiquetas, carpetas, estrellas y nota son de la principal: es lo que se
  ve en la malla. Las versiones no salen en la malla ni en los recuentos.
- Una versión cuya principal no existe o está en la papelera se ve suelta:
  apuntar a nada no puede esconder un archivo.

Para sacar las pilas sin Grimorio:

```sh
find MiBiblioteca.grimorio/items -name item.json -exec cat {} + \
  | jq -c 'select(.pila) | {version: .id, principal: .pila, receta}'
```

Toda escritura es atómica: archivo temporal en el mismo directorio y `rename`.
Un corte de corriente deja el archivo anterior intacto, nunca uno a medias.

## Los otros archivos de la raíz

Todos son JSON con un `schema` y se escriben enteros cada vez, de forma
atómica. Ninguno es imprescindible: sin ellos la biblioteca abre igual, sin
carpetas, grupos, carpetas dinámicas ni carpetas vigiladas.

- **`folders.json`**: qué carpetas existen: `{"schema", "folders": [{"id",
  "name", "parent", "color", "pos"}]}`. `parent` falta en las de la raíz y
  `pos` ordena entre hermanas. En qué carpetas está cada elemento lo dice su
  `item.json` (`folders`), no este archivo.
- **`etiquetas.json`**: grupos de etiquetas: `{"schema", "grupos": [{"id",
  "nombre", "color", "etiquetas"}]}`. Una etiqueta está en un grupo o en
  ninguno; su color es el del grupo.
- **`busquedas.json`**: las carpetas dinámicas: `{"schema", "busquedas":
  [{"id", "nombre", "consulta"}]}`. `consulta` es la línea del buscador tal
  cual (`#comida -#picante tipo:video`); puede estar vacía en una recién
  creada.
- **`vigiladas.json`**: carpetas del disco que se importan solas:
  `{"schema", "vigiladas": [{"id", "ruta", "carpeta", "desde_ms"}]}`.
  `carpeta` es la carpeta de la biblioteca donde caen (o `null`) y `desde_ms`
  el momento de la última pasada.
- **`abierta.lock`**: lo pone la aplicación mientras tiene la biblioteca
  abierta (formato de `QLockFile`: el pid en la primera línea, la máquina en
  la tercera). `grim` lo mira antes de escribir. Uno de un proceso que ya no
  existe no cuenta.

## `thumbs/grid.pack`

Archivo *append-only* con todas las miniaturas de malla concatenadas. Existe
porque con 100.000 elementos, un archivo por miniatura significa 100.000
`open()` durante un scroll; así la interfaz mapea un archivo una vez y el
sistema operativo se encarga del caché.

```
cabecera (16 bytes)
  0..8    "GRIMPK01"
  8..16   reservado

registro (repetido)
  0       0xA5   marca de inicio
  1       flags  0 = JPEG
  2..4    u16 LE longitud del id
  4..8    u32 LE longitud de los datos
  8..     id en UTF-8, sin terminador
  …       datos de la miniatura
```

Cada registro lleva su propio id, así que el pack se puede recorrer de principio
a fin sin ayuda de nadie. Eso es lo que permite reconstruir el índice sin volver
a generar miniaturas. Si el archivo se trunca a media escritura, el recorrido
para limpiamente en el último registro completo.

Una miniatura sustituida se añade al final; la última entrada de un id gana. El
espacio anterior se recupera al compactar (todavía no implementado).

**Un solo escritor.** Quien abre el pack para escribir toma un cerrojo exclusivo
sobre el archivo. Dos procesos añadiendo a la vez se pisarían las posiciones y
dejarían miniaturas cruzadas, que es de los pocos daños que `reindex` no sabe
arreglar. Los lectores no necesitan cerrojo: el archivo solo crece.

## `index.sqlite`

Derivado. Contiene `items`, `tags`, `item_tags`, `folders`, `item_folders`,
`meta` y una tabla FTS5 `items_fts`.

`items.pila` (desde la versión 5 del índice, `PRAGMA user_version`) copia el
campo del `item.json` y tiene un índice parcial que solo lleva las versiones:
en una biblioteca sin pilas está vacío, y entonces las consultas ni siquiera
miran la columna.

Detalle que importa: `items_fts` **no** tiene columna de id. Comparte `rowid`
con `items`, de modo que borrar y unir son operaciones de clave primaria. En la
primera versión sí tenía una columna `id UNINDEXED`, y cada borrado escaneaba la
tabla entera: la importación se volvía cuadrática y el índice tardaba diez veces
más de la cuenta.

Para reconstruirlo: `grim reindex`. Recorre todos los `item.json`, lee el pack
de miniaturas y vuelve a montar el índice entero.

## Recuperar una biblioteca sin Grimorio

```sh
# todos los metadatos, en un JSON por línea
find MiBiblioteca.grimorio/items -name item.json -exec cat {} + | jq -c .

# los originales, con su nombre real
find MiBiblioteca.grimorio/items -name 'original.*'
```
