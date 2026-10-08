# Grimorio en Windows

Grimorio nació en Linux y allí se desarrolla, pero el mismo código compila en
Windows con MSVC: el núcleo en Rust tal cual, y la aplicación en Qt 6 con unas
pocas ramas `#ifdef Q_OS_WIN` donde el sistema hace las cosas de otra manera.
Este documento cuenta cómo compilarlo a mano, qué funciona distinto allí y
cómo se publica una versión con instalador.

## Compilar a mano

Hace falta, una sola vez:

1. **Visual Studio 2022 Build Tools** (o Visual Studio Community) con la
   carga de trabajo «Desarrollo para el escritorio con C++». Trae el
   compilador (`cl`), el enlazador, el compilador de recursos, CMake y Ninja.
2. **Rust** con [rustup](https://rustup.rs). Con MSVC ya instalado elige solo
   el destino `x86_64-pc-windows-msvc`. La versión exacta la descarga sola la
   primera vez que se llama a `cargo`, porque está fijada en
   `rust-toolchain.toml`.
3. **Qt 6.8 para MSVC 2022, 64 bits**, con el módulo **Qt Multimedia**. Dos
   formas:
   - el instalador en línea de Qt (cuenta gratuita), marcando
     «Qt 6.8.x > MSVC 2022 64-bit» y «Additional Libraries > Qt Multimedia»;
   - o sin cuenta, con [aqtinstall](https://github.com/miurahr/aqtinstall):

     ```powershell
     pip install aqtinstall
     aqt install-qt windows desktop 6.8.3 win64_msvc2022_64 -m qtmultimedia -O C:\Qt
     ```
4. Para el instalador, **[Inno Setup 6](https://jrsoftware.org/isinfo.php)**
   (opcional: solo si se quiere generar el `.exe` de instalación).

Después, desde «x64 Native Tools Command Prompt for VS 2022» (o un PowerShell
donde se haya cargado el entorno de Visual Studio), en la raíz del repositorio:

```powershell
cargo test --release                      # núcleo, línea de órdenes y puente
cargo build --release -p grimorio-cli     # grim.exe

cmake -S app -B app/build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=C:\Qt\6.8.3\msvc2022_64
cmake --build app/build
ctest --test-dir app/build -E "^qml_"
```

CMake llama a `cargo` él solo para compilar el puente (`grimorio_puente.lib`)
y lo enlaza estático. Para depurar conviene `RelWithDebInfo` y no `Debug`: el
puente de Rust va siempre con el runtime de C de publicación (`/MD`), y con el
de depuración (`/MDd`) el enlazador avisa de dos runtimes.

Para ejecutarlo desde la carpeta de compilación, Qt tiene que estar en el
`PATH` (`C:\Qt\6.8.3\msvc2022_64\bin`) o haber pasado `windeployqt` por el
`.exe`:

```powershell
.\app\build\grimorio.exe C:\Users\yo\Documents\Referencias.grimorio
```

Sin ruta abre la última biblioteca que se abrió, y la primera vez crea una en
`Documentos\Grimorio.grimorio`. Es lo que hace el acceso directo del menú
de inicio.

### Empaquetar a mano

Lo mismo que hace la publicación automática (ver más abajo), paso a paso:

```powershell
mkdir dist\Grimorio\licencias
copy app\build\grimorio.exe, target\release\grim.exe dist\Grimorio\
windeployqt --release --qmldir app\qml --no-translations dist\Grimorio\grimorio.exe
copy "$env:VCToolsRedistDir\x64\Microsoft.VC143.CRT\*.dll" dist\Grimorio\
copy LICENSE.md, installer\AVISOS-TERCEROS.txt dist\Grimorio\
copy installer\licencias\*.txt dist\Grimorio\licencias\
iscc /DVersion=0.1.0 installer\windows\grimorio.iss
```

El instalador sale en `dist\Grimorio-0.1.0-windows-x64-instalador.exe`.

## Qué cambia en Windows

| | Linux | Windows |
|---|---|---|
| Diálogos de abrir archivos y carpetas | portal del escritorio (o zenity) | los del sistema, vía `QFileDialog` |
| Capturar pantalla (`Ctrl+Mayús+X`) | el portal deja elegir zona, ventana o todo | **la pantalla entera** donde está el ratón; elegir zona aún no está |
| Abrir con el programa del sistema | `xdg-open` | `ShellExecuteW`, lo mismo que un doble clic |
| Ajustes | `~/.config/Grimorio/Grimorio.conf` | `%APPDATA%\Grimorio\Grimorio.ini` |
| Pintar | OpenGL | OpenGL también (el visor 3D lo necesita); sin controlador, `opengl32sw.dll` por software |
| Vigilar carpetas | archivos nuevos por fecha de cambio | igual, con la fecha de creación (Windows no tiene «cambio de estado»); los ocultos por atributo también se saltan |
| Memoria en el banco de pruebas (`--bench`) | de `/proc` | sale a cero |

Los diálogos de Windows son QFileDialog y no los de QtQuick.Dialogs porque
quien los pide son objetos de C++ con una función de vuelta, y desde QML habría
que darles la vuelta a todos. QFileDialog es de QtWidgets y necesita
QApplication, así que en Windows la aplicación arranca con QApplication en vez
de QGuiApplication; para la ventana de QML no cambia nada, y en Linux no se
enlaza QtWidgets.

El programa es de ventana y no tiene consola: los errores de arranque
(biblioteca abierta en otra ventana, ruta que no es una biblioteca) salen en un
cuadro de diálogo además de por la salida de error.

Los nombres de archivo que crea Grimorio (al arrastrar un elemento fuera, al
guardar una descarga) se limpian para que valgan en Windows aunque se creen en
Linux: sin `\ : * ? " < > |`, sin punto o espacio al final y sin nombres
reservados como `CON` o `NUL`. Una biblioteca puede vivir en un disco externo
que luego se conecta al otro sistema.

## Herramientas opcionales

Como en Linux, ninguna es obligatoria: sin ellas los archivos entran igual,
sin miniatura o sin visor. Grimorio las busca en el `PATH`; tras instalarlas
hay que cerrar y volver a abrir Grimorio (y la terminal, si se lanza desde
una). Con [winget](https://learn.microsoft.com/windows/package-manager/):

| Para | Programa | Instalar |
|---|---|---|
| vídeo (miniatura, duración, onda, copia en H.264) | ffmpeg y ffprobe | `winget install Gyan.FFmpeg` |
| PDF y `.ai` | pdftoppm y pdfinfo (Poppler) | `winget install oschwartz10612.Poppler` (o descargar Poppler para Windows y añadir su `Library\bin` al `PATH`) |
| PSD, XCF, muestras de tipografías | ImageMagick (`magick`) | `winget install ImageMagick.ImageMagick` |
| EPS | Ghostscript (`gswin64c`) | `winget install ArtifexSoftware.GhostScript` (añadir su `bin` al `PATH`) |
| `.blend` y `.fbx` | Blender | `winget install BlenderFoundation.Blender` |

Blender no hace falta en el `PATH`: se busca también en
`C:\Program Files\Blender Foundation\Blender X.Y\`, la versión más alta. Para
otra ubicación, la variable `GRIMORIO_BLENDER` con la ruta a `blender.exe`.
La miniatura de reserva que el `.blend` trae dentro no se saca en Windows: allí
Blender no trae `blender-thumbnailer`.

En Windows ImageMagick se llama siempre como `magick` y nunca como `convert`:
ese nombre es `C:\Windows\System32\convert.exe`, la herramienta que convierte
discos de FAT a NTFS. Los `.kra`, `.ora` y `.sketch` ya no necesitan `unzip`:
Grimorio lee el zip por su cuenta.

Ninguna de estas herramientas abre una ventana de consola al trabajar: se
lanzan con `CREATE_NO_WINDOW`.

## Publicar una versión

La publicación es automática, en GitHub Actions
(`.github/workflows/release.yml`). Basta con una etiqueta:

```sh
git tag v0.1.0
git push --tags
```

La versión sale de la etiqueta (sin la `v`) y se escribe en el `Cargo.toml`, en
CMake (ajustes, «acerca de» y las propiedades del `.exe`) y en el instalador.
Una etiqueta con guion (`v0.2.0-beta.1`) sale como versión previa.

Cuando la release está publicada y los archivos se descargan, falta **avisar a
quien ya lo tiene instalado**: en la web (`grimorio-web/public/version.json`)
se cambia `version` por la nueva y se despliega, junto con las URLs fijas de
`/descargar/` del `vercel.json`. Los Grimorio instalados preguntan por ese
archivo una vez al día y, si es más nueva que la suya, lo dicen en la barra de
abajo (ver `app/src/novedades.h`). Hacerlo antes de tiempo manda a la gente a
una descarga que aún no existe. Las versiones previas no se ponen ahí.

En Windows, el flujo compila, pasa las pruebas, prepara la carpeta con
`windeployqt` (DLL de Qt, complementos, módulos de QML, `opengl32sw.dll`),
copia el runtime de Visual C++ dentro de la carpeta en vez de meter su
instalador (así vale igual para el `.zip` portable y para instalar sin
permisos de administrador), arranca el programa una vez para ver que no falta
nada y genera:

- `Grimorio-X.Y.Z-windows-x64-instalador.exe`: Inno Setup, con acceso en el
  menú de inicio, acceso en el escritorio opcional y desinstalador. Pregunta
  al empezar si instalar para todos (con permisos de administrador, en
  Archivos de programa) o solo para quien instala (sin permisos, en
  `%LOCALAPPDATA%\Programs`). No asocia ninguna extensión. Al desinstalar,
  las bibliotecas y los ajustes se quedan.
- `Grimorio-X.Y.Z-windows-x64-portable.zip`: la misma carpeta, para
  descomprimir y usar.

Los dos llevan `LICENSE.md`, `AVISOS-TERCEROS.txt` y la carpeta `licencias`
con la LGPL y la GPL: Qt va enlazado de forma dinámica y sin modificar, y sus
DLL se pueden sustituir, como exige la LGPL (ver `docs/DEPENDENCIAS.md`).

El instalador no va firmado: Windows SmartScreen avisará de «editor
desconocido» hasta que se firme con un certificado de firma de código.

### Linux

En la misma publicación sale `Grimorio-X.Y.Z-linux-x64.tar.gz`, con
`grimorio` y `grim` compilados en Ubuntu 24.04. Qt no va dentro: hay que tener
instalados los paquetes de `app/README.md` («Módulos que hacen falta en
Ubuntu 24.04»). Es un primer paso; un AppImage con Qt dentro (linuxdeploy y su
complemento de Qt) queda pendiente, y con él funcionaría en otras
distribuciones sin instalar nada.

## Integración continua

`.github/workflows/ci.yml` compila y prueba en cada push y en cada pull
request, en Ubuntu 24.04 y en Windows. En Windows no corre `qmllint`: el Qt
de allí es el 6.8 y el de desarrollo en Linux el 6.4, y sus avisos no
coinciden; el QML se revisa en Linux, que es igual para los dos.

## Pendiente o por comprobar en Windows

- Elegir una zona al capturar la pantalla (ahora es la pantalla entera).
- Firmar el instalador.

En Windows los cerrojos de archivo son obligatorios, no de aviso. Por eso el
escritor del pack de miniaturas bloquea `grid.pack.lock`, un archivo al lado,
y nunca el pack: la interfaz lo sigue leyendo mientras se importa.
