English · [Español](FORMATO.es.md)

# The Grimorio library format

Schema version: **1**

This is the part of the project that promises to outlive the program itself. If
Grimorio disappears, this specification has to be enough to recover everything
with a twenty-line script.

## Layout on disk

```
MiBiblioteca.grimorio/
├── library.json          identity and settings of the library
├── folders.json          the folder tree
├── etiquetas.json        tag groups with their color
├── busquedas.json        smart folders (named searches)
├── vigiladas.json        disk folders that are imported automatically
├── abierta.lock          lock held while the app has the library open
├── index.sqlite          DERIVED INDEX: can be deleted, it is rebuilt
├── items/
│   └── 01/JK/01JKX7…/    two-level sharding by the first 4
│       ├── item.json     characters of the id
│       └── original.jpg
├── cache/                DERIVED: can be deleted, remade when each thing opens
│   ├── proxy/01/JK/…mp4  H.264 copy of the videos Qt cannot open
│   ├── tira/             frames in a grid for the video scrub bar
│   ├── onda/             waveform and spectrum of each sound (`.onda`)
│   ├── pdf/              rendered pages, by number and width
│   ├── malla/            3D models ready for the viewer (`.malla`)
│   └── versiones/        temporary space for a version while it is imported
├── thumbs/
│   ├── grid.pack         320 px thumbnails, concatenated
│   └── preview/01/…jpg   1024 px previews
├── trash/                reversible deletion
├── themes/
└── plugins/
```

**The sharding is there for a reason.** A directory with 100,000 entries makes
any `readdir` slow, and desktop tools choke on it. With two levels of two
characters, no folder goes over a few hundred entries.

## Identifiers

26-character ULIDs in Crockford base32: 48 bits of milliseconds + 80 random
bits. Two properties we rely on everywhere:

- **Sorting by id is sorting by import date.** The default grid view does not
  sort anything: it reads the index in primary key order.
- They are unique without coordination, so they can be generated in parallel
  during an import of thousands of files.

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

`importMode` is `copy` (the default, the original lives inside), `move` or `ref`
(the original stays where it is).

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

- `hash` is the blake3 of the content in hexadecimal: the real identity of the
  file, and the basis of duplicate detection.
- `phash` is a 64-bit dHash in hexadecimal, for "similar to this one".
- `palette` is sorted by weight; the first color is the dominant one.
- `kind` is the family: `imagen`, `raw`, `video`, `audio`, `documento`,
  `tipografia` or `modelo`. It comes from the extension; it is written down so
  readers do not have to know which is which.
- `adult` (only when `true`) marks what safe mode blurs.
- `rotation` (0, 90, 180 or 270) is the rotation it is viewed with. The
  original is not rotated.
- `orden` stores the manual order: for each folder (its id, or `""` for
  «All»), the position of the item. They are numbers with gaps between them,
  so one can be inserted in the middle without renumbering.
- `origin.inode` makes it possible to find an original in `ref` mode again if
  someone has moved it.
- `durationMs` and `pages` are present on things that have a duration or pages.
- `pila` and `receta` are present on versions (see «Version stacks» below).
- `triangles` and `sizeMm` are present on 3D models. `sizeMm` is width, depth
  and height in millimeters, and it is only there if the format states its
  units: STL (which is written in millimeters by convention), 3MF, glTF and
  whatever comes out of Blender. OBJ and PLY do not state them, and in that
  case nothing is made up.
- Empty fields are omitted when writing. A reader must tolerate them missing.

### Version stacks

The same image at different file sizes (an export for the web, the copy someone
sent recompressed) is stored as a **stack**: a main item and its versions. Each
version is a complete item, with its own folder in `items/`, its original and
its thumbnail; the only thing that sets it apart is that it points to its main
item:

```json
{
  "id": "01JKX9…",
  "name": "estudio de iluminación nocturna (jpg 1600 px 82 %)",
  "ext": "jpg",
  "pila": "01JKX7Q2M8V3N4P5R6S7T8",
  "receta": { "formato": "jpg", "calidad": 82, "lado": 1600 }
}
```

- `pila` is the id of the main item. Only versions carry it; the main item has
  no list of versions, so there are never two places that can fall out of
  sync. Stacks are flat: a version is never the main item of another.
- `receta` is only there if Grimorio made the version: `formato` (`jpg`, `png`
  or `webp`), `calidad` (1 to 100; it applies to JPEG and WebP, where 100 is
  lossless) and `lado` (the longest side in pixels; 0 keeps the size). It
  never upscales. The original is not touched.
- `receta.recorte`, if present, says which model removed the background
  (`isnet-general-use`): the version is a PNG of the same size with
  transparency where the background was.
- `receta.tramo`, if present, is a section of a video or a sound cut with the
  editor: `desde_ms` and `hasta_ms` on the original, and `exacto` if it was
  re-encoded to cut on the exact frame (otherwise it was copied without
  re-encoding and the file starts at the previous keyframe, with an edit list
  that makes it start where requested). The format is that of the original. A
  section saved «as a new video» carries its `receta` but no `pila`: it is a
  separate item, with the folders, tags and rotation of the original.
- `receta.fotograma`, if present, is the millisecond of a video this image
  (a PNG) was taken from. Like a separate section, it has no `pila` and it
  carries the folders, tags and rotation of the video.
- Tags, folders, rating and note belong to the main item: that is what is
  shown in the grid. Versions do not appear in the grid or in the counts.
- A version whose main item does not exist or is in the trash is shown on its
  own: pointing at nothing must never hide a file.

To list the stacks without Grimorio:

```sh
find MiBiblioteca.grimorio/items -name item.json -exec cat {} + \
  | jq -c 'select(.pila) | {version: .id, principal: .pila, receta}'
```

Every write is atomic: a temporary file in the same directory and `rename`. A
power cut leaves the previous file intact, never a half-written one.

## The other files at the root

They are all JSON with a `schema` and they are written whole every time,
atomically. None is essential: without them the library still opens, with no
folders, groups, smart folders or watched folders.

- **`folders.json`**: which folders exist: `{"schema", "folders": [{"id",
  "name", "parent", "color", "pos"}]}`. `parent` is missing on root folders
  and `pos` orders siblings. Which folders each item is in is stated by its
  `item.json` (`folders`), not by this file.
- **`etiquetas.json`**: tag groups: `{"schema", "grupos": [{"id",
  "nombre", "color", "etiquetas"}]}`. A tag is in one group or in none; its
  color is the group's.
- **`busquedas.json`**: smart folders: `{"schema", "busquedas":
  [{"id", "nombre", "consulta"}]}`. `consulta` is the search bar line as is
  (`#comida -#picante tipo:video`); it can be empty in a newly created one.
- **`vigiladas.json`**: disk folders that are imported automatically:
  `{"schema", "vigiladas": [{"id", "ruta", "carpeta", "desde_ms"}]}`.
  `carpeta` is the library folder where things land (or `null`) and
  `desde_ms` the time of the last pass.
- **`abierta.lock`**: the app creates it while it has the library open
  (`QLockFile` format: the pid on the first line, the host on the third).
  `grim` checks it before writing. One left by a process that no longer exists
  does not count.

## `thumbs/grid.pack`

An *append-only* file with all the grid thumbnails concatenated. It exists
because with 100,000 items, one file per thumbnail means 100,000 `open()` calls
during a scroll; this way the interface maps one file once and the operating
system takes care of caching.

```
header (16 bytes)
  0..8    "GRIMPK01"
  8..16   reserved

record (repeated)
  0       0xA5   start marker
  1       flags  0 = JPEG
  2..4    u16 LE id length
  4..8    u32 LE data length
  8..     id in UTF-8, no terminator
  …       thumbnail data
```

Each record carries its own id, so the pack can be walked from start to end
without help from anything else. That is what makes it possible to rebuild the
index without generating thumbnails again. If the file is truncated mid-write,
the walk stops cleanly at the last complete record.

A replaced thumbnail is appended at the end; the last entry for an id wins. The
old space is reclaimed when compacting (not implemented yet).

**A single writer.** Whoever opens the pack for writing takes an exclusive lock
on the file. Two processes appending at once would step on each other's offsets
and leave thumbnails mixed up, which is one of the few kinds of damage
`reindex` cannot fix. Readers do not need a lock: the file only grows.

## `index.sqlite`

Derived. It contains `items`, `tags`, `item_tags`, `folders`, `item_folders`,
`meta` and an FTS5 table `items_fts`.

`items.pila` (since version 5 of the index, `PRAGMA user_version`) copies the
field from `item.json` and has a partial index that only holds versions: in a
library with no stacks it is empty, and then queries do not even look at the
column.

A detail that matters: `items_fts` does **not** have an id column. It shares
`rowid` with `items`, so deleting and joining are primary key operations. The
first version did have an `id UNINDEXED` column, and every delete scanned the
whole table: importing became quadratic and the index took ten times longer
than it should.

To rebuild it: `grim reindex`. It walks every `item.json`, reads the thumbnail
pack and puts the whole index back together.

## Recovering a library without Grimorio

```sh
# all metadata, one JSON per line
find MiBiblioteca.grimorio/items -name item.json -exec cat {} + | jq -c .

# the originals, with their real name
find MiBiblioteca.grimorio/items -name 'original.*'
```
