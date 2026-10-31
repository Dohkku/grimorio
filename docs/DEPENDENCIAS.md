English · [Español](DEPENDENCIAS.es.md)

# Dependency policy

## The rule

A dependency gets in only if it meets all three:

1. It is actively maintained and someone serious uses it in production.
2. Writing it ourselves would take more than two weeks.
3. Its license is compatible with Grimorio's AGPL-3.0 (see below).

Otherwise, it is written in-house. We have already written: ULIDs, the RFC 3339
date format, the thumbnail pack, palette extraction in Oklab, the perceptual
hash, directory walking and the query engine.

The full dependency tree must fit in an afternoon's review. It is audited at
every milestone with `cargo tree --duplicates`, and with `cargo deny` once it is
added.

## Project license and what can be linked

Grimorio started as a closed, free-of-charge binary (the Obsidian model), and
that is where the caution in this list comes from. Since the first public
version it has been free software under the **AGPL-3.0**. That widens what can
be used, but the earlier rules stay because they are still good ones (few
dependencies, dynamic Qt) and because they keep the door open to offering
Grimorio under another license later on, something only whoever holds the
copyright for all of the code can do.

| License | Usable? | Condition |
|---|---|---|
| MIT / Apache-2 / BSD / public domain | yes | none |
| LGPL 2.1 / 3 | yes | **dynamic linking only**, with notice and the ability to replace the library |
| MPL-2.0 | yes | changes to its files are published |
| GPL-3.0 / AGPL-3.0 | yes, with care | they are compatible with the AGPL, but they tie any future license to them; only if there is no alternative |

**Hard rule of the project: never link Qt or any LGPL library statically.**
Static linking of Qt requires a commercial license. In the packages (`.deb`,
Flatpak, AppImage) the LGPL libraries ship as separate `.so` files and their
license texts are included.

## Current state

Direct dependencies of the core and the CLI:

| Crate | What for | License | Note |
|---|---|---|---|
| `rusqlite` (+ `bundled`) | index and search | MIT | compiles SQLite inside; SQLite is public domain |
| `serde` / `serde_json` | `item.json`, `library.json`, themes | MIT/Apache-2 | |
| `image` | decode and scale JPEG/PNG/GIF/WebP/BMP | MIT/Apache-2 | with `default-features = false` |
| `blake3` | content identity | CC0/Apache-2 | |
| `rayon` | parallel import | MIT/Apache-2 | |
| `memmap2` | mapping the thumbnail pack | MIT/Apache-2 | |
| `thiserror` | core errors | MIT/Apache-2 | |
| `miniz_oxide` | inflating the zips of 3MF, and of `.kra`, `.ora` and `.sketch` (without `unzip`, which Windows does not have) | MIT/Apache-2/Zlib | it was already compiled in for `image`'s PNG support; naming it adds no code |
| `clap` | CLI interface | MIT/Apache-2 | only in the `grim` binary |
| `webp` | lossy WebP for lighter versions | MIT/Apache-2 | compiles libwebp (BSD-3) inside; `image` only writes lossless WebP, and a VP8 encoder of our own would take months |
| `tract-onnx`, `tract-linalg` | running the background removal model | MIT/Apache-2 | **only in `grim-recorte`**, a separate binary: the core does not link it. Pure Rust, with no onnxruntime to package per system |
| `tempfile` | tests | MIT/Apache-2 | only `dev-dependencies` |

The `grim-recorte` model (ISNet, Apache-2.0) **ships with the program**, in
`modelos/` next to the executable. Its weights are stored in f16 (91 MB instead
of 179) with the same computation: `herramientas/modelo/preparar.py` rebuilds it
from the original and checks both hashes. The packages take it from the
project's own `modelo-isnet-1` release, never from another project's. A build
from source without it downloads it from there, asking first.

Only in the spikes (they are not part of the product):

- `spikes/grid-wgpu`: `wgpu`, `winit`, `pollster`, `bytemuck`, all MIT/Apache-2.
- `spikes/grid-qml`: Qt 6 Core/Gui/Quick/Qml, LGPLv3, **linked dynamically**
  from day one, as the hard rule above requires. The bridge to the core
  (`grimorio-puente`) is linked statically, but it is our own code.

### What Qt 6 requires in practice, measured

Installing Qt 6 on Ubuntu 24.04 to build and run the QML spike leaves three
warnings to keep in mind before choosing it for the interface:

1. `qt6-base-dev` + `qt6-declarative-dev` + `cmake` are enough to **build**.
2. To **run**, seven more `qml6-module-*` packages are needed that Debian does
   not declare as dependencies. Without them, `import QtQuick` fails at runtime
   without saying what to install. Any package we make (`.deb`, Flatpak,
   AppImage) will have to list them by hand.
3. There is no `qsb` and no QtShaderTools in the base installation: without an
   extra package no custom `ShaderEffect` can be compiled.

## Pending decisions for v1

| Area | Choice | License | Why |
|---|---|---|---|
| Fast thumbnails | **libvips** | LGPL-2.1+ | downscaling while loading: thumbnailing a 24 MP JPEG without decoding all of it |
| RAW | **LibRaw** | LGPL-2.1 or CDDL | de facto standard, in an isolated subprocess |
| PDF | **pdfium** | BSD-3 | MuPDF would be compatible (AGPL), but it would tie the license |
| Video | **ffmpeg as an external binary** | LGPL-2.1+ | separate process: we neither link it nor inherit its failure surface; never a build with GPL flags |
| Fonts | pure Rust | MIT/Apache-2 | it only needs to rasterize a sample |
| Interface | **Qt 6 / QML** | LGPLv3 | dynamic linking required |

## Outside tools, optional

They are called as programs, in isolated processes and with a time limit. None
is required: without them files are still imported, just missing something.

| Program | What for | Without it |
|---|---|---|
| `ffmpeg` / `ffprobe` | frame and duration of videos; H.264 copy of what Qt cannot open; waveform and spectrum | video with no thumbnail, sound with no waveform |
| `pdftoppm` / `pdfinfo` | first page and the viewer's pages | PDF with no thumbnail or viewer |
| `blender` | opening `.blend` and `.fbx` (`blender -b`, with no window) | imported blind |
| ImageMagick (`magick`, or `convert` in version 6) | PSD, XCF and font samples | imported blind |
| `gs` (Ghostscript; `gswin64c` on Windows) | EPS | imported blind |

On Windows they are looked for the same way, on the `PATH` (and Blender also in
its installation folder), and on Windows ImageMagick is always called as
`magick`: there `convert` is the system tool that converts drives from FAT to
NTFS. How to install them is in `docs/WINDOWS.md`. The Windows installer also
includes the Qt DLLs, with their licenses and the third-party notices
(`installer/AVISOS-TERCEROS.txt`).

A program called by absolute path is not probed by launching it with
`-version`: Blender does not understand that option and opened its whole
window, one for every check.

## What stays out, and why

- **Qt Quick 3D**: it comes under the GPL or a commercial license, not the
  LGPL. The model viewer is hand-written OpenGL on Qt Gui (`Qt6::OpenGL`,
  LGPL), which for a mesh without textures is one buffer and two shaders.
- **QtPdf**: it would be LGPL and would qualify, but `pdftoppm` was already
  there for thumbnails, in a separate process, and a broken PDF must not be
  able to crash the app.

- **Third-party JS runtimes for plugins**: Qt already ships its JS engine.
  Bringing another one means tens of megabytes and a new security surface.
- **Web UI frameworks**: the whole point of the project is not having a browser
  inside.
- **Single-function utility crates** (`itertools`, `lazy_static` and the like):
  Rust's standard library is enough.
