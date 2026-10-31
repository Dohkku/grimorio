English · [Español](WINDOWS.es.md)

# Grimorio on Windows

Grimorio was born on Linux and is developed there, but the same code compiles
on Windows with MSVC: the Rust core as is, and the Qt 6 app with a few
`#ifdef Q_OS_WIN` branches where the system does things differently. This
document explains how to build it by hand, what works differently there and how
a version with an installer is published.

## Building by hand

You need, once:

1. **Visual Studio 2022 Build Tools** (or Visual Studio Community) with the
   «Desktop development with C++» workload. It brings the compiler (`cl`), the
   linker, the resource compiler, CMake and Ninja.
2. **Rust** with [rustup](https://rustup.rs). With MSVC already installed,
   choose only the `x86_64-pc-windows-msvc` target. The exact version is
   downloaded automatically the first time `cargo` is called, because it is
   pinned in `rust-toolchain.toml`.
3. **Qt 6.8 for MSVC 2022, 64-bit**, with the **Qt Multimedia** module. Two
   ways:
   - Qt's online installer (free account), ticking
     «Qt 6.8.x > MSVC 2022 64-bit» and «Additional Libraries > Qt Multimedia»;
   - or without an account, with [aqtinstall](https://github.com/miurahr/aqtinstall):

     ```powershell
     pip install aqtinstall
     aqt install-qt windows desktop 6.8.3 win64_msvc2022_64 -m qtmultimedia -O C:\Qt
     ```
4. For the installer, **[Inno Setup 6](https://jrsoftware.org/isinfo.php)**
   (optional: only if you want to generate the installation `.exe`).

Then, from «x64 Native Tools Command Prompt for VS 2022» (or a PowerShell where
the Visual Studio environment has been loaded), at the root of the repository:

```powershell
cargo test --release                      # core, command line and bridge
cargo build --release -p grimorio-cli     # grim.exe

cmake -S app -B app/build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=C:\Qt\6.8.3\msvc2022_64
cmake --build app/build
ctest --test-dir app/build -E "^qml_"
```

CMake calls `cargo` by itself to build the bridge (`grimorio_puente.lib`) and
links it statically. For debugging, use `RelWithDebInfo` rather than `Debug`:
the Rust bridge always uses the release C runtime (`/MD`), and with the debug
one (`/MDd`) the linker warns about two runtimes.

To run it from the build folder, Qt has to be on the `PATH`
(`C:\Qt\6.8.3\msvc2022_64\bin`) or `windeployqt` has to have been run on the
`.exe`:

```powershell
.\app\build\grimorio.exe C:\Users\yo\Documents\Referencias.grimorio
```

Without a path it opens the last library that was opened, and the first time it
creates one in `Documents\Grimorio.grimorio`. That is what the Start menu
shortcut does.

### Packaging by hand

The same thing the automatic release does (see below), step by step:

```powershell
mkdir dist\Grimorio\licencias
copy app\build\grimorio.exe, target\release\grim.exe dist\Grimorio\
windeployqt --release --qmldir app\qml --no-translations dist\Grimorio\grimorio.exe
copy "$env:VCToolsRedistDir\x64\Microsoft.VC143.CRT\*.dll" dist\Grimorio\
copy LICENSE.md, installer\AVISOS-TERCEROS.txt dist\Grimorio\
copy installer\licencias\*.txt dist\Grimorio\licencias\
iscc /DVersion=0.1.0 installer\windows\grimorio.iss
```

The installer ends up in `dist\Grimorio-0.1.0-windows-x64-instalador.exe`.

## What changes on Windows

| | Linux | Windows |
|---|---|---|
| Open file and folder dialogs | desktop portal (or zenity) | the system ones, through `QFileDialog` |
| Screen capture (`Ctrl+Shift+X`) | the portal lets you pick an area, a window or everything | **the whole screen** the mouse is on; picking an area is not there yet |
| Open with the system program | `xdg-open` | `ShellExecuteW`, the same as a double click |
| Settings | `~/.config/Grimorio/Grimorio.conf` | `%APPDATA%\Grimorio\Grimorio.ini` |
| Rendering | OpenGL | OpenGL too (the 3D viewer needs it); with no driver, `opengl32sw.dll` in software |
| Watched folders | new files by change time | the same, with the creation time (Windows has no «status change» time); files hidden by attribute are skipped too |
| Memory in the benchmark (`--bench`) | from `/proc` | reported as zero |

The Windows dialogs are QFileDialog rather than those from QtQuick.Dialogs
because whoever asks for them are C++ objects with a callback, and from QML all
of them would have to be turned around. QFileDialog belongs to QtWidgets and
needs QApplication, so on Windows the app starts with QApplication instead of
QGuiApplication; nothing changes for the QML window, and on Linux QtWidgets is
not linked.

The program is a windowed app with no console: startup errors (library open in
another window, a path that is not a library) appear in a dialog box as well as
on standard error.

File names created by Grimorio (when dragging an item out, when saving a
download) are cleaned up so they are valid on Windows even if they are created
on Linux: no `\ : * ? " < > |`, no trailing dot or space, and no reserved names
such as `CON` or `NUL`. A library can live on an external drive that later gets
plugged into the other system.

## Optional tools

As on Linux, none is required: without them files are still imported, just
without a thumbnail or a viewer. Grimorio looks for them on the `PATH`; after
installing them you have to close and reopen Grimorio (and the terminal, if you
launch it from one). With [winget](https://learn.microsoft.com/windows/package-manager/):

| For | Program | Install |
|---|---|---|
| video (thumbnail, duration, waveform, H.264 copy) | ffmpeg and ffprobe | `winget install Gyan.FFmpeg` |
| PDF and `.ai` | pdftoppm and pdfinfo (Poppler) | `winget install oschwartz10612.Poppler` (or download Poppler for Windows and add its `Library\bin` to the `PATH`) |
| PSD, XCF, font samples | ImageMagick (`magick`) | `winget install ImageMagick.ImageMagick` |
| EPS | Ghostscript (`gswin64c`) | `winget install ArtifexSoftware.GhostScript` (add its `bin` to the `PATH`) |
| `.blend` and `.fbx` | Blender | `winget install BlenderFoundation.Blender` |

Blender does not need to be on the `PATH`: it is also looked for in
`C:\Program Files\Blender Foundation\Blender X.Y\`, highest version first. For
another location, set the `GRIMORIO_BLENDER` variable to the path of
`blender.exe`. The fallback thumbnail stored inside a `.blend` is not extracted
on Windows: Blender does not ship `blender-thumbnailer` there.

On Windows ImageMagick is always called as `magick` and never as `convert`:
that name is `C:\Windows\System32\convert.exe`, the tool that converts drives
from FAT to NTFS. `.kra`, `.ora` and `.sketch` files no longer need `unzip`:
Grimorio reads the zip on its own.

None of these tools opens a console window while working: they are launched
with `CREATE_NO_WINDOW`.

## Publishing a version

Publishing is automatic, on GitHub Actions (`.github/workflows/release.yml`). A
tag is all it takes:

```sh
git tag v0.1.0
git push --tags
```

The version comes from the tag (without the `v`) and is written into
`Cargo.toml`, into CMake (settings, «about» and the `.exe` properties) and into
the installer. A tag with a hyphen (`v0.2.0-beta.1`) is published as a
pre-release.

Once the release is published and the files can be downloaded, what is left is
**telling people who already have it installed**: on the website
(`grimorio-web/public/version.json`) `version` is changed to the new one and
deployed, together with the fixed `/descargar/` URLs in `vercel.json`. Installed
copies of Grimorio check that file once a day and, if it is newer than theirs,
say so in the bottom bar (see `app/src/novedades.h`). Doing it too early sends
people to a download that does not exist yet. Pre-releases are not listed
there.

On Windows, the workflow builds, runs the tests, prepares the folder with
`windeployqt` (Qt DLLs, plugins, QML modules, `opengl32sw.dll`), copies the
Visual C++ runtime into the folder instead of bundling its installer (so it
works the same for the portable `.zip` and for installing without administrator
rights), starts the program once to check nothing is missing and generates:

- `Grimorio-X.Y.Z-windows-x64-instalador.exe`: Inno Setup, with a Start menu
  shortcut, an optional desktop shortcut and an uninstaller. At the start it
  asks whether to install for everyone (with administrator rights, in Program
  Files) or only for the person installing (no rights needed, in
  `%LOCALAPPDATA%\Programs`). It does not associate any extension. When
  uninstalling, libraries and settings stay.
- `Grimorio-X.Y.Z-windows-x64-portable.zip`: the same folder, to unzip and use,
  but tidied up for whoever opens it: at the top only `Grimorio.exe`,
  `LEEME.txt` and the `programa` folder, which is the installer's folder as is.
  Seeing two hundred DLLs together when opening the zip was scary, and it was
  not clear which one to open. Windows only looks for DLLs next to the .exe, so
  the top-level `Grimorio.exe` is a launcher of a few lines of C
  (`installer/windows/portable/lanzador.c`) that starts
  `programa\grimorio.exe` with the same arguments and exits. The release
  workflow tests it unzipped, starting through the launcher.

Both include `LICENSE.md`, `AVISOS-TERCEROS.txt` and the `licencias` folder
with the LGPL and the GPL (in the portable one, inside `programa`, and the LEEME
says where): Qt is linked dynamically and unmodified, and its DLLs can be
replaced, as the LGPL requires (see `docs/DEPENDENCIAS.md`).

The installer is not signed: Windows SmartScreen will warn about an «unknown
publisher» until it is signed with a code signing certificate.

### Linux

The same release includes `Grimorio-X.Y.Z-linux-x64.tar.gz`, with `grimorio`
and `grim` built on Ubuntu 24.04. Qt is not included: the packages listed in
`app/README.md` («Módulos que hacen falta en Ubuntu 24.04») have to be
installed. It is a first step; an AppImage with Qt inside (linuxdeploy and its
Qt plugin) is still pending, and with it Grimorio would work on other
distributions without installing anything.

## Continuous integration

`.github/workflows/ci.yml` builds and tests on every push and every pull
request, on Ubuntu 24.04 and on Windows. On Windows `qmllint` does not run: the
Qt there is 6.8 and the one used for development on Linux is 6.4, and their
warnings do not match; QML is checked on Linux, which is the same for both.

## Pending or to be checked on Windows

- Picking an area for the screen capture (right now it is the whole screen).
- Signing the installer.

On Windows file locks are mandatory, rather than advisory. That is why the
writer of the thumbnail pack locks `grid.pack.lock`, a file next to it, and
never the pack itself: the interface keeps reading it while importing.
