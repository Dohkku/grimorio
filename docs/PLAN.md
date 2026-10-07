# Plan de Grimorio

## Qué es

Una biblioteca visual para Linux al nivel de Eagle: capturar, organizar,
encontrar y previsualizar referencias sin que el programa se interponga. Eagle
solo existe en Windows y macOS, corre sobre Chromium 107 y no da su código.

En Linux hay cosas parecidas —digiKam es de fotógrafo y su interfaz es de otra
época, TagStudio tiene la filosofía correcta pero no el rendimiento, Hydrus es
de archivista— pero ninguna cubre el hueco: **biblioteca visual multiformato,
con visor instantáneo, cuidada, extensible y rápida con cientos de miles de
elementos**.

## Decisiones tomadas

| Tema | Decisión |
|---|---|
| Interfaz | Núcleo en Rust + capa QML (Qt 6) + pegamento fino en C++. Confirmado con números en M0 (ver docs/M0.md) |
| Archivos | Copiar dentro de la biblioteca por defecto, como Eagle. `mover` y `referenciar` existen en el núcleo y se exponen como ajuste en M3 |
| Licencia | Software libre bajo AGPL-3.0 desde la primera versión pública. Formato de biblioteca documentado en público |
| IA | Fuera del núcleo. Si algún día, como plugin |

## Alcance

**MVP (0.1):** biblioteca portable, importación (arrastrar, carpeta, URL),
carpetas jerárquicas con pertenencia múltiple, etiquetas, malla virtualizada,
visor instantáneo, inspector, búsqueda con filtros, valoración y notas.

**v1:** carpetas inteligentes, búsqueda por color, vigilancia de carpetas,
vídeo/audio/PDF/tipografías/RAW, duplicados, plugins.

**Después:** extensión de navegador, comparación y presentación, formatos de
diseño (PSD, AI, Sketch), sincronización.

**Fuera:** cualquier función de IA.

## Arquitectura

Tres capas y una costura estable:

```
Capa 3 · Shell de UI      malla, visor, inspector, atajos, temas, plugins
   ▲ comandos · consultas · suscripciones ▼
Capa 2 · Bus de comandos  validación, permisos, deshacer, eventos
   ▲ API interna de Rust ▼
Capa 1 · Núcleo           almacén, índice, búsqueda, importación, miniaturas
```

- El deshacer se implementa una vez, en el bus, y cubre toda operación.
- Los plugins reciben el mismo bus con permisos recortados: no hay puerta
  trasera distinta a la de la propia aplicación.
- El CLI (`grim`) es el primer cliente del bus. Existe desde el día uno y sirve
  para pruebas y mediciones sin abrir una ventana.
- Si el toolkit resulta un error, se tira la capa 3 y no las otras dos.

Estado: las tres capas en pie. El bus explícito y el deshacer llegaron en M2:
toda edición pasa por `Library::editar`, que devuelve el camino de vuelta.

## Presupuestos de rendimiento

Con 100.000 elementos, en portátil de gama media. Se miden en cada hito; una
regresión es un fallo bloqueante.

| Métrica | Objetivo | Medido en M0 |
|---|---|---|
| Arranque a malla usable | < 400 ms | 324 ms |
| Fotogramas perdidos al hacer scroll | 0 | 0,09 % sobre 16,6 ms |
| Búsqueda de texto (p95) | < 30 ms | 11-18 ms |
| Etiquetar 10.000 | sin congelar | 344 ms, 2 fotogramas |
| Abrir el visor | < 120 ms | 6,2 ms (p95) |
| Memoria en reposo | < 400 MB | 124 MB |
| Importación (JPEG de 12 MP) | > 20/s | 63,7/s a 6,75 MP |

## Hitos

- **M0 · Spike y esqueleto** — núcleo por CLI, biblioteca sintética de 100.000,
  banco de pruebas de la malla. *Puerta: decisión de toolkit con números.*
  Estado: **cerrado**. Qt Quick gana; wgpu queda como listón y plan B.
- **M1 · Ver** — ventana, malla, visor a pantalla completa, árbol de carpetas,
  arrastrar y soltar, tokens de tema con recarga en caliente.
  *Puerta: scroll sin saltos con 100.000 y visor por debajo de 120 ms.*
  Estado: **cerrado**. 131 fps en cuadrícula y 94 en justificado con 100.000
  elementos; visor con p95 de 7 ms. Ver docs/M1.md.
- **M2 · Organizar** — etiquetas con autocompletado, estrellas, notas, edición
  en lote, inspector completo, buscador con filtros, deshacer global, papelera.
  *Puerta: etiquetar 10.000 elementos sin congelar la interfaz.*
  Estado: **cerrado**. 344 ms de extremo a extremo con la malla a 133 fps y dos
  fotogramas perdidos. Ver docs/M2.md.
- **M3 · Tragar de todo** — vídeo, audio, PDF, tipografías, RAW en procesos
  aislados; vigilancia de carpetas; modo referencia; carpetas inteligentes;
  búsqueda por color; duplicados; integración de escritorio.
  *Puerta: importar 50 GB heterogéneos sin perder un elemento.*
  Estado: **en marcha**. Familias de archivo; RAW y carátulas de audio sin
  dependencias; vídeo y PDF en procesos aislados con tope de tiempo; duplicados
  exactos y perceptuales (100.000 en medio segundo); reproducción de vídeo y
  sonido en el visor, con QtMultimedia opcional; modelos 3D (STL, OBJ, PLY,
  glTF/GLB, 3MF, y `.blend`/`.fbx` vía Blender) con miniatura y visor propio;
  onda y espectrograma del sonido; PDF página a página; copia reproducible de
  los vídeos que Qt no abre. Quedan las tipografías, las carpetas
  inteligentes, la vigilancia de carpetas y la integración de escritorio.
- **M4 · MVP 0.1** — `.deb`, AppImage, Flatpak; atajos; español e inglés;
  importador desde bibliotecas de Eagle; copia de seguridad.
  *Puerta: dos semanas de uso diario propio sin pérdida de datos.*

Estimación a MVP: 12 a 17 semanas de trabajo enfocado.

## Cola de trabajo

Lo que hay pedido y sin hacer, en el orden en que se está haciendo. Se vacía
por arriba; lo cerrado sale de aquí y entra en el hito que le toque.

| | Qué | Estado |
|---|---|---|
| 1 | Estrellas del panel al día con la malla | hecho |
| 2 | Paneles laterales de ancho ajustable | hecho |
| 3 | Carpeta nueva junto al árbol, y arrastrar carpetas para cambiar la jerarquía | hecho |
| 4 | Arrastrar elementos de una carpeta a otra: mover, no solo añadir | hecho |
| 5 | Vídeo que se reproduce al pasar por encima en la malla, en bucle | hecho |
| 6 | Vídeo reproducible en el panel de la derecha | hecho |
| 7 | Interruptor: reproducir a la vez todo el vídeo que se ve | hecho |
| 8 | Marcar como +18, con modo seguro que difumina y se levanta al entrar | hecho |
| 9 | Que un vídeo ProRes no diga «Internal data stream error» | hecho: copia en H.264 |
| 10 | Ver el contenido de un PDF, no solo su portada | hecho |
| 11 | Modelos 3D: STL para imprimir, `.blend`, y los de intercambio | hecho |
| 12 | Sonido: onda en el tiempo y en frecuencia, ir a un momento con el ratón, cámara lenta | hecho |
| 13 | Mandos de vídeo más ricos | hecho: fotograma a fotograma, velocidad, A–B, volumen, vista previa en la barra |

Sobre el 8, lo pedido con detalle: el modo seguro difumina lo marcado; abrir un
elemento marcado lo enseña **solo mientras está abierto** y al volver a la malla
sigue difuminado; apagar el modo seguro es lo único que lo levanta del todo.

Sobre el 7 hay un límite físico que conviene tener escrito: cada vídeo en
marcha es una tubería de decodificación. Se reproducen los que se ven, con tope,
y los reproductores se reciclan en vez de crearse y destruirse — montar y
desmontar la tubería muchas veces seguidas tira el programa (ver app/README.md).

## Riesgos

| Riesgo | Mitigación |
|---|---|
| El toolkit no aguanta la malla | Spike medido antes de escribir interfaz; capa 3 desechable por diseño |
| Dos cadenas de construcción (Cargo + CMake) | Integración e CI desde el primer commit |
| Escritura masiva de JSON en operaciones en lote | Índice inmediato, volcado diferido, registro de operaciones |
| Arrastrar y soltar en Wayland | Probar pronto en GNOME y KDE, usar portals |
| Alcance que se estira solo | La tabla de alcance es contrato: añadir implica quitar |
| Proyecto de una sola persona | Núcleo con pruebas de verdad y CLI completo: sobrevive a un parón |
