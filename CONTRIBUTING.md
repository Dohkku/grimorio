# Contribuir a Grimorio

Gracias por pasarte. Grimorio es un proyecto pequeño y con opiniones fuertes;
esta página cuenta cuáles son para que tu tiempo no se pierda.

## Antes de escribir código

- **Abre un issue primero** para cualquier cosa que no sea un arreglo
  pequeño. Una función nueva puede chocar con alguna de las cinco decisiones
  del [README](README.md) (datos legibles, nada de webview, núcleo sin
  ventana, estilo como dato, sin IA), y es mejor saberlo antes.
- Los fallos, con pasos para reproducirlos, la versión (`Ctrl+,` → acerca de)
  y el sistema. Si es con un archivo concreto, `grim mirar ARCHIVO` dice qué
  ve el núcleo en él.

## Cómo está organizado

| Carpeta | Qué hay |
|---|---|
| `crates/grimorio-core` | el núcleo en Rust: biblioteca, índice, importación, miniaturas, consultas |
| `crates/grimorio-cli` | `grim`, la línea de órdenes |
| `app/` | la aplicación Qt 6 / QML y el puente con el núcleo (`app/puente`) |
| `temas/` | los temas en JSON; se recargan al guardarlos |
| `docs/` | formato de datos, plan, dependencias, Windows |
| `herramientas/` | biblioteca de demostración y recorrido de pruebas de usuario |

## Compilar y probar

```sh
cargo test --release                              # núcleo, CLI y puente
cmake -S app -B app/build -DCMAKE_BUILD_TYPE=Release
cmake --build app/build -j && (cd app/build && ctest)
```

Las pruebas de interfaz manejan la app con ratón y teclado simulados en una
pantalla virtual (hace falta `Xvfb` y `ffmpeg`):

```sh
python3 herramientas/demo/generar.py /tmp/Demo.grimorio --grim target/release/grim
python3 herramientas/recorrido/recorrido.py /tmp/Demo.grimorio --app app/build/grimorio
```

## Estilo

- Código, comentarios, documentación y mensajes de commit **en español**. Los
  comentarios cuentan el porqué, no el qué; mira los que hay alrededor.
- En el QML no se escribe ni un color ni una medida a mano: todo sale de
  `tema` (lo vigila la prueba `sin_colores_a_mano`).
- Rendimiento con presupuesto: `grim bench query` tiene que seguir por debajo
  de 30 ms de p95 con 100.000 elementos.

## Licencia de las contribuciones

Grimorio se publica bajo la licencia que indica [`LICENSE.md`](LICENSE.md).
Para poder ofrecer en el futuro un servicio en línea sin quedar atado a otra
licencia, cada contribución necesita un acuerdo de cesión de derechos (CLA)
sencillo. El primer pull request te lo pedirá; basta con aceptarlo una vez.
