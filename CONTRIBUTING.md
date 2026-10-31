English · [Español](CONTRIBUTING.es.md)

# Contributing to Grimorio

Thanks for stopping by. Grimorio is a small project with strong opinions;
this page explains what they are so your time is not wasted.

## Before writing code

- **Open an issue first** for anything that is not a small fix. A new feature
  may clash with one of the four decisions in the [README](README.md)
  (readable data, no webview, a core without windows, style as data), and it
  is better to know that early.
- Bug reports: include steps to reproduce, the version (`Ctrl+,` → About)
  and your system. If it happens with a specific file, `grim mirar ARCHIVO`
  tells you what the core sees in it.

## How it is organized

| Folder | Contents |
|---|---|
| `crates/grimorio-core` | the Rust core: library, index, import, thumbnails, queries |
| `crates/grimorio-cli` | `grim`, the command line |
| `app/` | the Qt 6 / QML application and the bridge to the core (`app/puente`) |
| `temas/` | the JSON themes; they reload when saved |
| `docs/` | data format, plan, dependencies, Windows |
| `herramientas/` | demo library and the user test walkthrough |

## Build and test

```sh
cargo test --release                              # core, CLI and bridge
cmake -S app -B app/build -DCMAKE_BUILD_TYPE=Release
cmake --build app/build -j && (cd app/build && ctest)
```

The interface tests drive the app with simulated mouse and keyboard on a
virtual display (you need `Xvfb` and `ffmpeg`):

```sh
python3 herramientas/demo/generar.py /tmp/Demo.grimorio --grim target/release/grim
python3 herramientas/recorrido/recorrido.py /tmp/Demo.grimorio --app app/build/grimorio
```

## Style

- Code, comments, documentation and commit messages are **in Spanish**.
  Comments explain the why, not the what; look at the ones around you.
- No color or size is ever hardcoded in the QML: everything comes from
  `tema` (the `sin_colores_a_mano` test checks this).
- Performance has a budget: `grim bench query` has to stay under 30 ms at
  p95 with 100,000 items.

## Licensing of contributions

Grimorio is released under the license stated in [`LICENSE.md`](LICENSE.md).
To be able to offer an online service in the future without being tied to
another license, each contribution needs a simple contributor license
agreement (CLA). Your first pull request will ask you for it; accepting it
once is enough.
