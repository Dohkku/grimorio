[English](PUBLICAR.md) · Español

# Publicar una versión

La lista entera, en orden. Lo de compilar y empaquetar lo hace GitHub Actions
(ver «Publicar una versión» en [WINDOWS.es.md](WINDOWS.es.md)); esto es lo que queda
a mano antes y después.

## Antes: la documentación

La documentación se queda atrás sola si nadie mira. Antes de cada versión:

1. **`CHANGELOG.md` y `NOVEDADES.md`**: un apartado `## X.Y.Z` arriba del
   todo en cada uno, el mismo texto en inglés y en español, con lo que nota
   quien la usa (no cómo está hecho por dentro). Es lo que la aplicación
   enseña al actualizar, en su idioma, y el texto de la release (primero en
   inglés y debajo, plegado, en español). Breve y llano.
2. **`README.md`**: si hay algo nuevo que se vea, una línea en «What it does»
   (y lo mismo en `README.es.md`, «Qué hace»).
3. **Traducciones**: si la interfaz gana o cambia textos, poner al día
   `app/traducciones/` (ver su `LEEME.es.md`).
4. **`docs/FORMATO.md`**: si cambia algo de lo que se guarda en disco (un
   campo de `item.json`, un archivo nuevo en la biblioteca, la caché).
5. **El número**: `version` en `Cargo.toml` y `project(... VERSION ...)` en
   `app/CMakeLists.txt`. El CI lo vuelve a escribir desde la etiqueta, pero
   en local tiene que decir lo mismo.

Las pruebas vigilan lo que se puede vigilar (`crates/grimorio-core/tests/documentacion.rs`):
fallan si `item.json` lleva un campo que `FORMATO.md` no cuenta, si la
biblioteca escribe un archivo que no está documentado o si la versión del
`Cargo.toml` no tiene su apartado en `NOVEDADES.md` o en `CHANGELOG.md`. Lo
demás (que el README cuente lo nuevo, que las novedades se entiendan) es esta
lista.

## Publicar

1. `cargo test --release` y `ctest` en `app/build`: todo en verde.
2. El commit público es uno por versión, «Grimorio X.Y.Z», encima del
   anterior de `main`.
3. Etiqueta `vX.Y.Z` y empujar: el CI compila Linux y Windows, pasa las
   pruebas en los dos y crea la release con los apartados de `CHANGELOG.md`
   y `NOVEDADES.md`.
4. Si el CI falla antes de crear la release: se arregla con un commit
   «Grimorio X.Y.Z: lo que sea» encima y se mueve la etiqueta. Sin release
   creada no hay nada que romper.

La release `modelo-isnet-1` tiene que existir: de ahí baja el CI el modelo
de quitar fondos.

## Después: avisar

Solo cuando los archivos de la release ya se descargan: en la web,
`public/version.json` (versión, URL y sha256 de Linux y Windows) y las URLs
de `/descargar/` del `vercel.json`, y desplegar. Los Grimorio instalados lo
miran una vez al día y avisan en la barra de abajo.
