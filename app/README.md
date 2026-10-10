# app — Grimorio

La aplicación. El núcleo está en `crates/grimorio-core`; entre los dos hay un
puente en C ABI (`app/puente`) con hilo propio, bus de comandos en JSON y vistas
inmutables. La explicación está en `docs/M1.md` (la aplicación) y `docs/M2.md`
(deshacer, papelera y edición en lote).

## Compilar

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j
```

`cargo` se invoca solo para construir el puente.

## Ejecutar

```sh
./build/grimorio ~/Referencias.grimorio
./build/grimorio ~/Referencias.grimorio --tema ../temas/papel.json
```

El tema se recarga al guardarlo, con el programa abierto. Sin `--tema` se usa
el elegido en los ajustes (`Ctrl+,`): «noche» o «papel», que van dentro del
binario. Ahí también está el tamaño de la interfaz —letra, iconos, márgenes y
paneles, del 80 al 150 %—, que se aplica multiplicando los tokens del tema y
no con `QT_SCALE_FACTOR`, para que las miniaturas sigan nítidas.

Para una biblioteca de pruebas: `grim synth ~/bench.grimorio --items 100000`.

## Teclas

| | |
|---|---|
| `Ctrl+F` | buscar |
| `Ctrl+A` | elegir todo |
| `Ctrl+Z` / `Ctrl+Mayús+Z` | deshacer y rehacer |
| `Supr` | a la papelera (dentro de la papelera, recuperar) |
| `Ctrl+K` | poner etiquetas a lo elegido |
| `F2` | renombrar (con varios elegidos, renombrar con patrón) |
| `Ctrl+C` | copiar lo elegido (archivos, y la imagen si es una) |
| `Ctrl+V` | pegar: imagen, archivos o una dirección web |
| `Ctrl+E` | exportar lo elegido a una carpeta |
| `Ctrl+Mayús+N` | carpeta nueva |
| `Ctrl+Mayús+X` | capturar una zona de la pantalla |
| `Ctrl+L` | pasar por las tres vistas: justificado, cuadrícula y lista |
| `Ctrl+I` | plegar o abrir el panel de detalle |
| `Ctrl+,` | ajustes: tema, tamaño de la interfaz, malla; y «acerca de» |
| `Ctrl+±` | tamaño de celda |
| `0`…`5` | estrellas a lo elegido |
| `Intro` | abrir el visor |
| `Espacio` | en el visor: siguiente, o parar y seguir si se reproduce |
| rueda · `+` `−` · `Z` | en el visor, con una foto: acercar y alejar hasta el píxel; encajar ⇄ 100 % |
| `C` | en el visor, con una foto: copiar el color del píxel bajo el ratón |
| `Mayús+←/→` | en el visor: cinco segundos atrás o adelante |
| `,` / `.` | en el visor: un fotograma atrás o adelante (se para primero) |
| `[` / `]` | en el visor: más lento o más rápido, de 0,1× a 2× |
| `I` / `O` | en el visor: principio y final del tramo que se repite |
| `X` | en el visor: quitar el tramo |
| `L` / `M` | en el visor: bucle y silencio |
| `Re Pág` / `Av Pág` | en un PDF: página anterior y siguiente |
| flechas | moverse por la malla, con `Mayús` para ir extendiendo la selección |
| `Esc` | cerrar lo que esté abierto, o soltar la selección |

Las flechas son de la malla, así que **tocar la malla le devuelve el foco**:
pinchar una carpeta o escribir en el buscador se lo lleva —tiene que
llevárselo—, y sin esa vuelta las flechas se quedaban hablándole al panel el
resto de la sesión. Se elegía una foto con el ratón y moverse con el teclado no
hacía nada. El hueco entre celdas cuenta como malla y también lo devuelve.

Mientras haya un campo de texto con el foco, los atajos que compiten con
escribir se apagan: `Intro`, `Esc`, `Supr`, `F2`, `Ctrl+Z` y los números. Un
`Shortcut` de ventana se lleva la tecla **antes** de que llegue al campo, y sin
esto poner nombre a una carpeta con una imagen elegida abría el visor en vez de
crear la carpeta.

Los dos paneles se ensanchan arrastrando la raya que los separa, y vuelven a lo
que diga el tema con doble clic en esa misma raya. El ancho se guarda en
`~/.config/Grimorio/Grimorio.conf`, no en el tema: el tema es dato del proyecto
y esto es de quien está delante.

Clic para elegir, `Ctrl+clic` para sumar, `Mayús+clic` para un tramo. Clic
derecho sobre una celda abre sus acciones (ver, abrir fuera, copiar, exportar,
mover, renombrar, +18, papelera); son las mismas que los botones del panel. Arrastrar
archivos a la ventana los importa.

## La ventana

Tres columnas, como un estudio: **de dónde se mira** a la izquierda, **lo que se
mira** en el centro y **lo elegido** a la derecha. Las dos de los lados van de
arriba abajo; la barra de arriba y la de estado son solo del centro.

* **Izquierda.** El nombre de la biblioteca (pinchándolo, las demás), las
  vistas fijas —«Todo», «Sin etiquetar», «Recientes», «Papelera»—, las
  carpetas inteligentes y el árbol de carpetas, cada una con su punto de
  color. Al pie, cuántos elementos hay y cuánto pesan, con el reparto por tipo
  en una raya (al pasar por cada trozo dice de qué es).
* **Centro.** Arriba, el nombre de lo que se mira y cuántos tiene; a la
  derecha, el buscador, las tres vistas (justificado, cuadrícula, lista), el
  orden y «filtros», que despliega la fila de filtros y dice cuántos hay
  puestos aunque esté plegada. En esa fila están también «nombres» (el nombre
  bajo cada celda) y el tamaño de celda.
* **Derecha.** El panel de detalle, siempre puesto aunque no haya nada elegido:
  así la malla no cambia de ancho cada vez que se elige algo. Vista grande (con
  varios elegidos, un mosaico de los primeros), nombre —se cambia pinchándolo—,
  estrellas, paleta —pinchar un color busca lo que se le parece—, etiquetas,
  nota, ficha técnica con carpetas y origen, y las acciones. Se pliega con
  `Ctrl+I`.

Las vistas fijas son búsquedas como cualquier otra, escritas en el buscador:
«Sin etiquetar» es `etiquetado:no` y «Recientes» es `fecha:7d`. El buscador
sigue siendo lo único que decide qué se ve.

En **lista**, cada elemento es una fila con su miniatura, nombre, tipo, medidas,
peso, estrellas y fecha. Pinchar el título de una columna ordena por ella. Es la
misma celda que en las otras dos vistas, así que elegir, arrastrar a una
carpeta, reordenar y moverse con las flechas funcionan igual.

La vista, los nombres y el panel de detalle se recuerdan en
`~/.config/Grimorio/Grimorio.conf`.

## Carpetas

Una carpeta es una etiqueta con jerarquía, no un sitio donde vive el archivo: un
elemento puede estar en varias.

«Todo» es *no tener ninguna elegida*: además de su fila, se vuelve pinchando
otra vez la carpeta elegida o el hueco de debajo del árbol. Las vistas fijas de
arriba no son carpetas y no se renombran ni se arrastran.

| Se hace | Pasa |
|---|---|
| clic en una carpeta | se mira esa carpeta |
| clic en la que ya está elegida | se vuelve a «Todo» |
| clic en el hueco de debajo del árbol | se vuelve a «Todo» |
| clic derecho en una carpeta | crear dentro, renombrar, color, vincular una carpeta del disco, borrar |
| clic derecho en el hueco | crear en la raíz |

Tocar el panel se lleva el foco de la malla: seguir escribiendo después de pinchar
aquí ya no le habla a las fotos.

Arrastrar tiene el sentido que espera quien arrastra:

| Se arrastra | Desde | Hace |
|---|---|---|
| la selección | una carpeta | la **mueve**: entra en la de destino y sale de la de origen, en una sola operación que se deshace de una vez |
| la selección | «Todo» o una búsqueda | la **añade**, sin sacarla de nada |
| la selección | cualquier sitio, al hueco de debajo del árbol | la saca de la carpeta en la que se estaba mirando |
| una carpeta | el hueco de debajo del árbol | vuelve a la raíz |

Una carpeta soltada sobre otra fila hace una cosa u otra según **dónde** de la
fila se suelte, y la raya que aparece lo dice antes de soltar:

```
─── borde de arriba ──►  queda delante de esa, como hermana
    centro             ►  queda dentro de esa, la última de sus hijas
─── borde de abajo ───►  queda detrás de esa, como hermana
```

Sin las bandas, el árbol solo sabía colgar de otra y el orden entre hermanas no
se podía tocar. Los elementos no tienen bandas: caigan donde caigan dentro de la
fila, entran en la carpeta, porque un elemento no tiene sitio entre dos carpetas.

Colgar y colocar son la **misma** operación en el núcleo —`recolocar`—, no dos:
arrastrar una fila es un solo gesto, y dos operaciones dejarían dos entradas en
el deshacer para algo que se vio pasar una vez. Al colocar se renumeran los
`pos` del grupo de diez en diez; si dos hermanas empataran, el desempate
alfabético mandaría sobre lo que acaba de decidir quien arrastró.

Una carpeta no se puede soltar dentro de sí misma ni de una de sus nietas: la
fila deja de iluminarse antes de soltar, no después. Lo que no se puede hacer se
apaga, pero la fila **no rechaza** la entrada: Qt no vuelve a avisar a una zona
de soltar que ha rechazado una vez, así que rechazar dejaba la fila sorda el
resto del arrastre y le quitaba también los dos bordes, que es justo donde sí se
podía soltar.

### Lo que viaja en un arrastre de dentro

Dos trampas de Qt Quick, las dos invisibles: se veía todo bien y no funcionaba
nada. Están sujetas por la prueba `arrastre`, que carga las filas de verdad y
mueve el ratón.

**La carga no va en el `Drag.mimeData`.** En un arrastre interno ese mapa no
llega al otro lado: quien recibe la caída ve las claves y la fuente, y `formats`
vacío, así que `getDataAsString` devuelve siempre cadena vacía. La fila de
destino se iluminaba —eso solo mira las claves— y al soltar no pasaba nada,
porque el identificador que había que mover llegaba en blanco. Lo que cruza es
la clave, que dice **qué** se arrastra; el **cuál** lo guarda la ventana
mientras dura el gesto. Las claves siguen sirviendo para lo suyo: decidir qué
zona acepta qué. Por eso la zona de importar de la ventana entera pide
`text/uri-list`, que es lo que trae un archivo de fuera: sin esa clave se
quedaba con cualquier arrastre de dentro y encendía el velo de «suelta aquí para
importar» al mover una foto de carpeta.

**Se ve qué se arrastra desde el primer píxel.** Un fantasma pegado al cursor
—la miniatura de lo elegido con cuántos van, o el nombre de la carpeta— y el
ratón en mano cerrada. Sin eso, arrastrar era un acto de fe: la única señal
llegaba al final del camino, cuando la fila de destino se encendía. La posición
la manda el propio bulto, que es el que ya la sabe.

**El bulto va pegado al cursor a mano** (`Bulto.qml`). Qt busca la zona de soltar
donde está el bulto, no donde está el ratón, y por su cuenta el bulto nace en la
esquina del que lo suelta y encima se queda atrás el umbral de arrastre, porque
`MouseArea` lo descuenta para que lo arrastrado no pegue un salto al empezar. Con
filas de dos dedos de alto eso es media banda de error: se pedía «dentro» y salía
«detrás».

### Cuando un vídeo de la malla se queda quieto

Qt puede escribir «Failed to start video surface due to main thread blocked» al
arrancar: el motor de multimedia le pide el relevo al hilo de interfaz para
montar la salida, se cansa de esperar y no vuelve a pedirlo. Pasa cuando la
malla está decodificando la primera pantalla. El reproductor sigue contando
posición y la celda se queda siendo una foto quieta para siempre.

`Reproductor.qml` cuenta los fotogramas que llegan a la salida y, si en tres
segundos no llega ninguno, vuelve a arrancar. Tres veces y para. Ni la posición
ni el tamaño sirven para darse cuenta: la posición avanza igual —lo que falla es
enseñar, no decodificar— y el tamaño se rellena en cuanto se conoce el formato.
Medido: con ocho de esos avisos, los cinco reproductores decían tener tamaño y
posición.

Un puñado de fotogramas por vuelta tampoco es estar roto: con la pantalla
apagada el compositor deja de pedir dibujado y la ventana entera baja a un
fotograma por segundo. Por eso vale cualquier fotograma, no un mínimo.

### Los reproductores de la malla no se destruyen, y hay que comprobarlo

Está escrito arriba del todo de `MallaVideos.qml` y aun así no se cumplía. El
goteo que crea los reproductores de uno en uno usaba `model: <un número>`, y
`Repeater` **rehace todos** sus hijos cada vez que ese número cambia: contando
nacimientos y muertes, cinco reproductores nacían quince veces y morían diez.
Justo lo que la cabecera dice que tira el programa. Ahora el modelo es una lista
a la que solo se añade, y son cinco nacimientos y cero muertes.

Tres cosas más que se pagaban en el hilo de interfaz y ya no:

* **Al cambiar de consulta el reparto se vacía en el acto.** Lleva índices, y un
  índice de la vista de antes apunta en la nueva a otra cosa —muchas veces a una
  foto, que el reproductor decodifica como un vídeo de un solo fotograma y pinta
  donde estaba el vídeo viejo y con su tamaño—. Eso era el cuadro que aparecía
  flotando fuera de sitio al entrar en una carpeta.
* **Quedarse sin sitio no suelta la fuente.** La fuente se pone a mano en vez de
  atarse al reparto: sin sitio el reproductor se calla y se esconde, pero se
  queda con lo que tenía. Atada, se quedaba en blanco y eso desmonta la tubería
  entera. Medido en un cambio de carpeta con vídeo automático: **2,3 s** hasta
  ver la carpeta, contra 0,6 s con el vídeo automático apagado. Sin desmontar,
  0,7 s con el vídeo puesto.
* **Salir de pantalla pausa, no para**, y volver a entrar arranca. `stop` suelta
  la tubería; `pause` no decodifica nada y vuelve al instante. Y hacía falta
  arrancar al volver: parar era fácil de pedir y arrancar no tenía quién lo
  pidiera, así que un vídeo que perdía su sitio y recuperaba el mismo se quedaba
  en su portada para siempre. Era lo que pasaba al entrar en una carpeta cuyos
  vídeos ya estaban en marcha.
* **Pausado tres segundos, se suelta.** Pausar y ya está sale caro: una tubería
  en pausa no decodifica pero se queda con todo lo suyo, y en una máquina con
  decodificador de vídeo por GPU cada una arrastra su contexto. Medido rodando
  la malla arriba y abajo cinco veces, con cinco vídeos: pausando y nada más, de
  0,93 a 1,33 GB; soltando a los tres segundos, de 0,82 a 0,86. El plazo es lo
  que hace que las dos cosas convivan: volver a la carpeta de la que se acaba de
  salir no cuesta nada, y lo que no vuelve no se queda ocupando.

Estas tres se arreglaron mientras existía el modo que movía todos los vídeos que
se veían. Ese modo ya no está —las cifras que lo mataron están en
«Reproducir»— pero los arreglos siguen valiendo: el reproductor de la malla es
uno y va y viene igual, y el visor y el panel montan tuberías por su cuenta.

## Ni un fotograma a JavaScript

El rescate de la malla cuenta los fotogramas que llegan a la salida, y los
contaba con un `Connections` a `videoFrameChanged`. Cada aviso entrega el
fotograma a JavaScript como un valor, y ese valor sujeta la textura del
decodificador hasta que pasa el recolector, que no ve esa memoria y no tiene
prisa. Con el vídeo por la tarjeta, una textura 1080p por fotograma: la memoria
de vídeo subía **250 MB por segundo** con un solo vídeo en bucle, y con un
manejador vacío igual. Pinchar entre dos vídeos lo multiplicaba —panel y malla
reproduciendo a la vez—: en cuatro minutos el programa tenía 13,5 GB de la
tarjeta y 14 GB de RAM, iba a tirones y moría.

Ahora se cuentan en C++ (`src/fotogramas.cpp`), con la conexión por el nombre
de la señal para no enlazar QtMultimedia, y el fotograma no sale de ahí. Medido
con el mismo vídeo en bucle: de 353 a 2580 MiB en nueve segundos antes; 284 MiB
planos durante veintiuno después. La regla: **ninguna señal que lleve un
`QVideoFrame` se escucha desde QML.**

## Cambiar de vídeo: pausar, y soltar después

Pinchar en otro vídeo congelaba la interfaz casi un segundo. No era abrir el
nuevo —eso va en segundo plano y vuelve en un milisegundo—, era soltar el viejo:
Qt 6.4 baja la tubería a NULL y espera, en el hilo de interfaz, y una tubería en
marcha tarda mucho más en bajar que una en pausa. Ahora el reproductor pausa al
momento, esconde la salida y cambia de fuente 100 ms después. Medido cambiando
entre dos vídeos cada segundo y medio: de **804 ms** de mediana (1,3 s de
máximo) a **179 ms** (251). Con 50 ms de espera todavía no había asentado la
pausa; con 200 no se gana más. Una ráfaga de cambios paga solo el último.

## El anillo del foco

Elegir algo abre el panel de detalle, y abrirlo estrecha la malla: la fila
entera se rehace y lo que se acaba de elegir cambia de sitio, a veces de fila.
Con el borde de la propia celda y nada más, hay que volver a buscarlo con los
ojos cada vez.

Los saltos se quedan —la malla justificada es así, y evitarlos costaría reservar
el hueco del panel siempre—, pero se puede no perderle la pista. Un anillo con
halo marca lo que enseña el panel y **se desliza** de donde estaba a donde va,
en 180 ms, con un golpe de luz que se apaga al llegar. Se sigue con la mirada en
vez de buscarlo.

Tres decisiones dentro:

* **Uno solo para toda la malla**, por encima de las celdas y de los vídeos, no
  uno por celda. Preguntar en cada una de las trescientas ochenta y cuatro si es
  ella la elegida son trescientas ochenta y cuatro ataduras a `modelo.actual`, y
  esto pinta lo mismo con un nodo. Es la misma razón por la que los vídeos van
  por encima y no dentro.
* **La celda avisa de dónde estaba al pincharla**, antes de tocar la selección.
  Sin eso el anillo no tiene de dónde salir: para cuando alguien se entera de
  cuál es la elegida, la malla ya se rehizo y el sitio viejo no existe. Y el
  aviso sale al **apretar**, no al soltar, porque elegir pasa al apretar.
* **El halo son tres bandas**, cada una más ancha y más tenue, y son bordes
  anchos y no rectángulos rellenos: relleno tiñe la foto de cian, y lo que hay
  que resaltar es la foto, no taparla. Sin módulo de efectos —el Qt de Ubuntu
  24.04 no trae `QtQuick.Effects`— un degradado es esto.

## Moverse con las flechas, y lo que costaba

Cada flecha cambia el foco, y cambiar el foco movía media aplicación. Tres cosas
que se pagaban en cada tecla y ya no:

* **El aviso del modelo se traduce a ranuras.** El modelo avisa por elemento y
  la malla sirve por ranura; el atajo que había ignoraba de qué elemento se
  avisaba y repintaba las cuatrocientas ranuras con los ocho papeles. Elegir
  manda dos avisos —el que se deja y el que se toma—, así que cada tecla lo
  hacía dos veces.
* **El visor cerrado ya no carga nada.** Su previsualización a tamaño completo
  colgaba de `modelo.actual` sin mirar si el visor estaba abierto: cada tecla
  descodificaba una imagen grande para nadie.
* **El panel espera a que pares** para darle la fuente al reproductor. Sin eso,
  pasar con las flechas por encima de los vídeos monta y desmonta una tubería de
  multimedia por tecla, en el hilo de la interfaz.

Y el anillo del foco **salta en vez de viajar cuando las teclas vienen
seguidas**: el viaje y el brillo están para no perderle la pista a un salto, y
yendo deprisa no se mira ninguno. Cada uno repintaba las tres bandas del halo
sesenta veces por segundo durante medio segundo, y con una tecla cada sesenta
milisegundos eso no paraba nunca.

**Aviso sobre medir esto:** en pantalla virtual (`Xvfb`) no hay tarjeta, dibuja
`llvmpipe` por software, y en un paseo con flechas **el 97 % del tiempo de CPU
se lo llevan sus hilos**. Ahí cualquier medida dice «lo que cuesta es repintar»
y tapa todo lo demás. Las cifras de arriba salen de comparar con la pieza
quitada, no de un perfil; y el número absoluto de una pantalla virtual no vale
para decidir nada sobre una máquina con GPU.

## Cambiar rápido de carpeta

Dos fallos que se veían igual —celdas con la foto de otro elemento— y que solo
salían alternando deprisa entre dos carpetas. Los dos quedan escritos porque
no se ven en una captura tranquila: hace falta la ráfaga.

**Las ranuras no se enteraban de que cambiaba su contenido.** La malla reparte
los elementos en ranuras y `repartir` solo avisa de las que cambian de sitio o
de medida. Si en la vista nueva el elemento de una ranura tenía la misma forma
y caía en el mismo sitio que el de la vista anterior, la ranura se quedaba con
el id viejo. Ahora, al reiniciarse el modelo, se refresca el contenido de todas.

**El proveedor de imágenes buscaba en una vista y decodificaba de otra.** Corre
en el hilo de carga de imágenes: buscaba la posición del id en la vista
publicada y luego tomaba otra referencia para decodificar. Si entre las dos
cosas el hilo de la interfaz cambiaba de vista, se leía la posición de la vieja
en la nueva, y como el caché de Qt guarda la imagen con la URL del id pedido,
la foto equivocada se quedaba pegada. Ahora toma la vista una vez y hace todo
con esa. Y cambiar de vista y clonarla van bajo el mismo cerrojo: antes, clonar
la vieja justo cuando se soltaba era tocar memoria ya liberada.

## Renombrar la biblioteca y sus carpetas vinculadas

«Renombrar esta biblioteca…», en el menú del nombre de arriba a la izquierda,
cambia el nombre que se enseña y nada más: la carpeta `.grimorio` se queda
donde y como está, porque su ruta vive en las recientes, en los accesos
directos y en lo que otros programas tengan apuntado. En `library.json` se toca
solo `name`; lo demás del archivo, aunque sea de una versión más nueva, sigue
ahí. Las recientes enseñan ese nombre y no el de la carpeta.

Las **carpetas vinculadas** son las del disco cuyo contenido nuevo entra solo
en la biblioteca. Antes se decía «vigilar», que en castellano suena a
vigilancia; por dentro sigue siendo `vigiladas.json` y los mismos comandos,
para no cambiar el formato. El menú de la biblioteca abre su vista de nodos
(`PanelVinculos.qml`): la biblioteca a la izquierda, con la carpeta donde vive;
cada carpeta vinculada a la derecha, con adónde entra lo suyo; una línea entre
las dos por la que corren trazos hacia la biblioteca, y que late cuando de
verdad entra algo. Una carpeta que no está (un disco sin conectar) se ve
apagada, con la línea punteada y quieta, y vuelve a funcionar cuando vuelve.
Desde ahí se conecta una nueva (el nodo «+») y se desconecta una (su ×); lo que
ya entró se queda. Las rutas se pulsan para abrirlas en el explorador, y el
eslabón de una carpeta vinculada en el árbol abre la misma vista.

## Una ventana por biblioteca

Abrirla dos veces sobre la misma carpeta no da un aviso: da dos programas
escribiendo en `folders.json` y en el índice, y el segundo en guardar pisa al
primero. Con el vídeo automático puesto además se pelean por el decodificador de
la tarjeta —cada vídeo es una tubería, y dos programas son el doble—: la tarjeta
se queda sin sesiones y la reproducción se cae con «No decoder available».

Así que la segunda no arranca y lo dice:

```
error: «~/Referencias.grimorio» ya está abierta (proceso 2274646)
```

El cerrojo es `abierta.lock` dentro de la biblioteca y guarda el pid de quien la
tiene. Un cierre a lo bruto no la deja condenada: la siguiente ve que ese proceso
ya no está y se lo queda. `grim` no pasa por aquí: la línea de órdenes sigue
funcionando con la ventana abierta.

## Qué biblioteca se abre

Por orden: la ruta que se pase (o `GRIMORIO_LIB`), la última que se abrió, y si
no hay ninguna, una nueva en `Documentos/Grimorio.grimorio`. Una ruta que es un
archivo de dentro de la biblioteca abre esa biblioteca.

Una ruta que no lleva a ninguna —un disco sin montar, una carpeta movida, un
`GRIMORIO_LIB` viejo— **no deja sin ventana**: se abre la siguiente de la lista
y la barra de estado dice cuál no se encontró. Antes el programa se cerraba, y
encima con «ya está abierta», porque el cerrojo no se puede crear en una carpeta
que no existe. Las pasadas automáticas (`--bench`, `--captura`, `--guion`…) sí
fallan: medir otra biblioteca sin decirlo daría números que no son.

## Buscar

El campo de arriba entiende filtros además de palabras, y el botón «filtros»
escribe en ese mismo campo:

```
tipografía etiqueta:rótulo estrellas:>=4 peso:>2mb orientacion:vertical
```

Campos: `etiqueta:` (o `#etiqueta`), `tipo:`, `ext:`, `estrellas:`, `ancho:`,
`alto:`, `peso:`, `orientacion:`, `color:#rrggbb`, `orden:`, `papelera:`,
`etiquetado:` (`etiquetado:no` es lo que no tiene ninguna etiqueta; es un campo
aparte y no `etiqueta:no`, que buscaría la etiqueta «no»).
Aceptan `>=`, `>`, `<=`, `<`; un número a secas quiere decir «exactamente ese».
Lo que no se entiende se busca como texto.

`tipo:` elige solo entre familia y extensión: `tipo:video` son todos los vídeos
y `tipo:png` son los PNG. `adulto:` y `+18:` son lo mismo.

El `grim search` del CLI entiende esta misma línea: es el mismo analizador, no
uno parecido.

## Modo seguro

Un elemento se marca como +18 desde el panel de la derecha, o desde el CLI con
`grim adult ID`. La marca vive en el `item.json`, se deshace como todo lo demás
y se busca con `adulto:si` (o `+18:si`).

Marcar no esconde nada por sí solo: quien decide es el **modo seguro**, el botón
de arriba, que solo aparece si hay algo marcado o si está puesto. Con él:

- en la malla y en el panel, lo marcado sale difuminado y con un `+18` encima;
- un vídeo marcado no se reproduce solo al pasar por encima;
- **el visor lo enseña entero**: entrar en un elemento es pedir verlo. Al volver
  a la malla sigue difuminado. Lo único que lo levanta del todo es apagar el
  modo.

El difuminado es la propia miniatura pedida diminuta y estirada hasta la celda.
No hace falta ningún módulo de efectos —el Qt de Ubuntu 24.04 no trae
`QtQuick.Effects` ni el compilador de sombreadores— y, lo que importa más, los
píxeles de verdad no llegan a estar en pantalla: lo que se decodifica son unas
decenas de píxeles, no la imagen tapada con algo encima.

## Versiones nuevas

Grimorio avisa de las versiones nuevas, y se actualiza solo **si se le pide**.
Una vez al día (y ocho segundos después de arrancar, para no competir con la
primera pantalla de miniaturas) pide
`https://grimorio.frederickandrade.com/version.json`:

```json
{
  "version": "0.2.0",
  "descargas": "https://grimorio.frederickandrade.com/",
  "paquetes": {
    "linux":   { "url": "https://github.com/Dohkku/grimorio/releases/download/v0.2.0/Grimorio-0.2.0-linux-x64.tar.gz",
                 "sha256": "…" },
    "windows": { "url": "https://github.com/Dohkku/grimorio/releases/download/v0.2.0/Grimorio-0.2.0-windows-x64-instalador.exe",
                 "sha256": "…" }
  }
}
```

Si es más nueva que la suya, la barra de estado enseña una píldora del color de
la selección, «Grimorio 0.2.0 disponible · actualizar», con una × que la ignora
hasta la siguiente; el engranaje de arriba a la izquierda lleva un punto, y
Ajustes abre con una tarjeta arriba del todo: «actualizar ahora», «ver la
página» y «ahora no». Antes era un texto del tamaño de los avisos en una
esquina, y no se veía. «Avisar de versiones nuevas» lo apaga del todo y
«comprobar ahora» pregunta en el acto (y vuelve a enseñar una versión
ignorada).

**Actualizar ahora** baja el paquete de su sistema, comprueba que su SHA-256 es
el del `version.json` y lo instala:

- **Windows**: lanza el instalador con `/SILENT /actualizar=1`, en el mismo
  modo en que se instaló (`/ALLUSERS` si está en Archivos de programa,
  `/CURRENTUSER` si no: si no, quedarían dos copias) y se cierra. El `.iss` lo
  vuelve a abrir al acabar, como quien lo usa y no como administrador.
- **Linux**: abre el `.tar.gz`, copia `grimorio` y `grim` al lado como
  `.nuevo`, y cambia cada uno con un renombrado; el de antes queda como
  `.anterior`. Se vuelve a abrir con la misma biblioteca, por un `sh` que
  espera a que el proceso viejo suelte el cerrojo.

Solo lo hacen los **paquetes publicados** (`-DGRIMORIO_PAQUETE=ON`, que pone
`release.yml`), instalados donde se puedan cambiar: con el instalador en
Windows (no el zip portable) y en una carpeta de quien lo usa en Linux. Un
build de desarrollo, el portable o un `/opt` de root abren la página de
descargas, como antes. `paquetes` es nuevo en la 0.1.4: las versiones de antes
no lo miran y siguen abriendo la página.

- **La web y no la API de GitHub.** El archivo es nuestro: si las descargas se
  mueven, se cambia el enlace y los programas ya instalados siguen sirviendo.
  GitHub además limita las consultas sin cuenta.
- **No manda nada.** Un GET sin cookies y con el agente `Grimorio` a secas, sin
  la versión. La página de privacidad de la web lo cuenta. Bajar el paquete es
  otro GET igual, y solo al pulsar.
- **Solo de las releases, y con huella.** Un paquete vale si es `https`, de
  `github.com/Dohkku/grimorio/releases/download/` y trae un SHA-256; lo bajado
  que no coincide se borra sin instalar, y la tarjeta dice por qué. Lo que se
  ejecuta tiene que ser lo publicado, bit a bit.
- **Lo último que se supo se guarda** (`novedades/*` en los ajustes): el aviso
  sale al arrancar aunque ese día no toque preguntar o no haya red.
- **Solo un enlace https.** El aviso abre el navegador con lo que diga el
  archivo; cualquier otra cosa se cambia por la portada.
- **Las pasadas automáticas no preguntan** (`--bench`, `--captura`, `--visor`,
  `--lote`, `--guion`), ni nada que arranque con `GRIMORIO_SIN_RED=1`, como el
  recorrido de pruebas: una prueba no puede depender de la red ni sacar un
  aviso en lo que captura.
- **Para probarlo**, `GRIMORIO_NOVEDADES_URL` cambia de dónde se lee el
  `version.json` (un `python3 -m http.server` con uno propio basta).

La comparación y la lectura del archivo están en `src/version.h`, sin red, y
las prueba `version`. Cómo se publica el número nuevo: `docs/WINDOWS.md`.

### Qué hay de nuevo

`NOVEDADES.md`, en la raíz, es la única fuente de lo que trae cada versión: un
apartado `## X.Y.Z` por versión. Va dentro del programa como recurso, y la
primera vez que se abre una versión después de actualizar, la ventana enseña
los apartados de las versiones que no se habían visto (`novedades/vistos` en
los ajustes); «entendido» lo da por visto. Quien lo instala por primera vez no
ve nada: si no había ajustes de antes (`bibliotecas/recientes`), no hay nada
«nuevo». Quien viene de una versión anterior a esto ve solo lo de la actual,
porque no se sabe de cuál viene. Ajustes → acerca de → «novedades» las enseña
todas, cuando se quiera. No dependen de la red; solo las pasadas automáticas
no las enseñan.

La publicación (`release.yml`) copia el apartado de su versión como texto de la
release de GitHub, y falla al principio si no existe: no se puede publicar una
versión sin decir qué trae. La web lo enseña en `/novedades/`.

## Medir y revisar

```sh
./build/grimorio BIBLIOTECA --bench 12       # recorrido automático y fotogramas
./build/grimorio BIBLIOTECA --visor 24       # cuánto tarda el visor en verse
./build/grimorio BIBLIOTECA --bench 12 --lote 10000   # editar en lote con la malla en marcha
./build/grimorio BIBLIOTECA --guion /tmp/x-  # recorre la interfaz y captura cada paso
./build/grimorio BIBLIOTECA --captura a.png  # un fotograma y salir
./build/grimorio BIBLIOTECA --filtro "tipo:video" --captura a.png   # con el buscador puesto
```

El modo guion etiqueta en lote, autocompleta, filtra por una etiqueta, escribe
una nota, crea una carpeta, abre el visor, tira a la papelera y pregunta antes de
vaciarla, dejando una captura de cada paso. Al final **se limpia lo suyo con el
propio deshacer**: si la pila se equivocara de orden, la biblioteca no volvería a
quedarse como estaba.

Los dos bancos que editan (`--lote` y `grim bench batch`) deshacen lo que hacen
al terminar, para no cambiar la biblioteca que están midiendo.

## Pruebas

```sh
ctest --test-dir build --output-on-failure   # geometría, arrastre, colores, qmllint
cargo test -p grimorio-puente                 # el bus de comandos y la frontera C
```

- `filas`: la geometría de la malla, sin Qt Quick y sin modelo. Incluye un
  presupuesto de tiempo: rehacer la disposición de cien mil elementos tiene que
  caber en un fotograma.
- `version`: cuándo sale el aviso de versión nueva. «0.10.0» va por delante
  de «0.9.0», un archivo roto no avisa de nada y un enlace que no es https no
  se abre.
- `sin_colores_a_mano`: ni un color ni una medida escritos a mano en el QML. Si
  el estilo no sale entero del tema, la promesa de que los visuales se pueden
  cambiar se rompe en silencio.
- `arrastre`: mueve el ratón de verdad sobre las filas del árbol de verdad, sin
  pantalla. Es lo único que caza que la carga del arrastre llegue al otro lado y
  que las bandas caigan donde apunta el cursor: las dos cosas se ven bien en una
  captura y no funcionaban.
- `qml_sin_avisos`: `qmllint` limpio.
- `qml_tipos_propios`: `qmllint` sobre los dos archivos que usan `Onda` y
  `Visor3D`. Esos tipos se registran desde C++ al arrancar y qmllint no los ve;
  metidos con los demás, «Failed to import Grimorio» saltaba la prueba entera y
  dejaba sin revisar todo lo demás.

## Formatos

Imágenes y negativos digitales van en Rust puro: de un RAW se saca el JPEG de
vista previa que la cámara guarda dentro, sin `libraw`.

En la malla, un sonido se ve como su onda en el tiempo: una celda gris que dice
«audio» no se distingue de la de al lado, y la forma de un golpe seco, una voz
o una canción sí. Si trae carátula —`APIC` de ID3v2 o el bloque `PICTURE` de
FLAC, leídos sin dependencias— va en un cuadrado a la izquierda de la onda: la
carátula de un disco es la misma para doce pistas, y la onda no. Se analiza a
22 kHz: a 8 kHz se perdía todo lo que pasa de 4 kHz y la miniatura mentía.

Lo importado antes de que el núcleo supiera dibujar algo se queda con la cara
vieja hasta que se le pide otra, con la aplicación cerrada:

```sh
grim miniaturas --tipo audio     # o modelo, video… sin --tipo, todo
```

No toca etiquetas, estrellas, notas, carpetas ni nombre.

Los modelos 3D —STL, OBJ, PLY, glTF/GLB y 3MF— se leen en Rust puro y su
miniatura la dibuja el núcleo sin tarjeta gráfica: un rasterizador con búfer
de profundidad y supermuestreo, en color arcilla, desde arriba a la derecha.
Un STL de 815.000 triángulos sale en 0,3 s. Al importar se guardan también los
triángulos y lo que mide la pieza en milímetros, que es lo primero que se
pregunta antes de imprimirla. `.blend` y `.fbx` los abre Blender sin ventana
(`blender -b`), si está: un guion vuelca la geometría ya evaluada —con los
modificadores aplicados— y el núcleo la dibuja igual que las demás.

Ese guion no pasa por el exportador de glTF de Blender a propósito: con él,
una escena de 438.000 triángulos tardaba 26 s y una de 124.000, 41. Volcando
con `foreach_get`, 1,5 y 1,9 s, casi todo arrancar Blender.

La miniatura que el `.blend` lleva dentro es de reserva y no la primera
opción: Blender la guarda a 128 px y con la luz de la escena, y en una celda
de 320 sale borrosa y casi siempre a oscuras.

Vídeo y PDF necesitan dos herramientas del sistema, y **no son obligatorias**:
sin ellas los archivos entran igual en la biblioteca, con sus datos, y se quedan
sin miniatura hasta que se instalen.

```sh
sudo apt install ffmpeg poppler-utils
```

## Reproducir

El visor reproduce vídeo y sonido si está el módulo de multimedia de Qt, que
tampoco es obligatorio:

```sh
sudo apt install qml6-module-qtmultimedia
```

Sin él, un vídeo en el visor se queda en su fotograma de portada y se ofrece
abrirlo con el reproductor del sistema. El `import` vive en `Reproductor.qml`,
él solo: un import que falla se lleva por delante el archivo que lo escribe, y
puesto en `Visor.qml` no habría visor tampoco para las fotos.

El vídeo también se mueve fuera del visor:

| Dónde | Cuándo |
|---|---|
| en la celda de la malla | al pasar el ratón por encima, y solo ese |
| en el panel de la derecha | siempre que haya un vídeo elegido y el visor esté cerrado |
| en el visor | al abrirlo |

En la malla va en bucle y sin sonido —una malla que suena sola no se puede
mirar—; en el panel y en el visor, con sonido y con mandos.

**Uno, no los que se vean.** Hubo un modo que movía todos los visibles con un
tope de ocho y se quitó midiendo, en la máquina de desarrollo —NVIDIA,
decodificación por GPU, 61 elementos con 5 vídeos—:

| | memoria | CPU |
|---|---|---|
| cinco vídeos moviéndose solos | 0,88 GB | 33 % de un núcleo |
| en reposo, sin ratón encima | 0,28 GB | 0 % |
| con el ratón sobre un vídeo | 0,41 GB | 7 % |

Cada vídeo en marcha es una tubería de decodificación entera y, en una máquina
con decodificador por GPU, un contexto de la tarjeta. Ocho a la vez no era un
tope: era una promesa de que el ventilador se iba a oír.

El botón de arriba tiene dos estados: `vídeo: al pasar` y `vídeo: quieto`.

Dos cosas del reproductor no son adorno y conviene no deshacerlas:

- **`play()` se pide dos veces, y la segunda se pregunta en vez de recordarse.**
  Sobre un medio que todavía no ha cargado no hace nada y no deja aviso, y nadie
  vuelve a intentarlo: el vídeo del panel de detalle se quedaba en su portada
  con el triángulo puesto, esperando un clic que no hacía falta. Se vuelve a
  pedir cuando el medio avisa de que ya está.
  Recordar «tenía uno pendiente» no basta: cambiar de fuente dispara dos cosas
  —el manejador del que la pone y la recarga del medio— y el orden entre ellas
  no está dicho, así que el apunte se lo gastaba el medio viejo y el nuevo se
  quedaba parado para siempre. En la malla no hay botón de pausa, así que ahí se
  pregunta directamente: si tiene sitio y tiene fuente, tiene que correr. En el
  visor y en el panel sí lo hay y se respeta lo que se haya pedido a mano.
- **Se crea una vez y no se destruye.** Montar y desmontar la tubería de
  multimedia muchas veces seguidas tira el programa —«double free or
  corruption» dentro de GStreamer, con `--visor 8` sale a la primera—. Un solo
  reproductor al que se le cambia la fuente no pasa por ahí.
- **Se crea 120 ms tarde.** Arrancarlo en el mismo fotograma en que se abre el
  visor cuesta el fotograma entero: abrir un vídeo pasaba de 6 ms a 691 ms, y
  lo que se ve mientras es nada. Medido con `--visor`.
- **En la malla van por encima de las celdas, no dentro.** Dentro habría que
  crear uno al entrar el ratón en una celda y destruirlo al salir, que es
  exactamente lo que tira el programa. Por encima hay unos pocos a los que se
  les cambia el sitio y la fuente. Y el reparto se aplaza mientras la malla se
  mueve: reasignarlos en cada fotograma solo tira tuberías a medio montar.

## Lo que no es una foto, en el visor

Cada familia tiene su manera de mirarse, y el visor la elige sola.

Lo que hace falta para enseñarla se saca la primera vez que se abre y se
guarda en `cache/` dentro de la biblioteca: la copia reproducible de un vídeo,
la onda de un sonido, las páginas de un PDF, la malla de un modelo. Es
derivado: se puede borrar entero y se rehace. Se pide desde un reparto de
hilos propio (`Derivados`, `src/derivados.h`) y **no** por el bus del núcleo:
el bus tiene un hilo y es el de editar, y convertir un vídeo de diez minutos no
puede dejar esperando a una estrella.

### Vídeo que el reproductor no abre

ProRes, DNxHD y compañía: ffmpeg los lee, pero el GStreamer que hay debajo de
Qt solo los decodifica con `gstreamer1.0-libav`, que Ubuntu no instala. Sin
él, el visor decía «Internal data stream error» y se quedaba en la portada.

Ahora, al fallar, el reproductor pide al núcleo una copia en H.264 de 8 bits
—lo único que todo decodificador entiende, el de la tarjeta incluido—, avisa
de que la está preparando y cambia a ella al llegar. La primera vez tarda lo
que tarde ffmpeg (unos segundos para un clip corto); después está en la caché.
Vale igual en el visor, en el panel y en la malla.

### Los mandos

Abajo, la barra y una fila de botones: −5 s, un fotograma atrás, parar,
un fotograma adelante, +5 s, el tiempo con décimas, la velocidad, el tramo
A–B, el bucle y el volumen. Al pasar el ratón por la barra se ve el momento
exacto y, en un vídeo, el fotograma de ese punto: sale de una tira de hasta
cien fotogramas que ffmpeg hace una vez, y enseñar uno es recortar la imagen,
que no decodifica nada mientras el ratón se mueve.

La velocidad: clic sube un escalón, clic derecho o rueda abajo lo baja, clic
central vuelve a 1×. A cámara lenta el sonido baja de tono: Qt 6.4 no corrige
el tono al cambiar de ritmo.

El bucle A–B lo vigila un temporizador de 15 ms y no el aviso de posición: la
posición avisa cuando le parece, y un bucle de medio segundo que se pasa de
largo cien milisegundos no es un bucle.

En el panel de la derecha los mandos son los justos —parar, tiempo y barra—:
la fila entera no cabe en 300 px.

### Sonido

La onda en el tiempo arriba y el espectro en frecuencia abajo, con el mismo
tramo y el mismo cabezal. Clic o arrastrar para ir a un momento; la rueda
acerca alrededor del ratón y con Mayús se mueve; doble clic lo enseña entero.
Al pasar el ratón se lee el tiempo y, sobre el espectro, la frecuencia.
Acercado y en marcha, la vista sigue al cabezal.

El análisis es del núcleo (`grimorio-core/src/senal.rs`): ffmpeg decodifica a
`f32` mono de 44,1 kHz y se recorre **por trozos** —una hora de audio son
seiscientos megas en `f32` y no hace falta tenerlos en memoria—. Picos cada
256 muestras y un espectrograma de hasta 4096 columnas por 256 filas en escala
logarítmica, de 30 Hz a 22 kHz, con FFT de 2048 propia. Los colores del
espectro salen del tema: del fondo al color de selección y de ahí al del texto.

Una trampa que costó encontrar: el `MouseArea` del reproductor que para y
sigue al hacer clic está declarado después de la onda, así que quedaba encima
y se quedaba los clics. Elegir un momento paraba la reproducción. Con la onda
delante, ese `MouseArea` se apaga.

### PDF

El documento entero, página tras página. Antes de dibujar nada se piden las
medidas de todas (`pdfinfo`), para que tenga su largo de verdad desde el
principio y la barra de desplazamiento no salte. Cada página la dibuja
`pdftoppm` al ancho que ocupa en pantalla, redondeado a tramos de 256 px: así
acercar un poco no la dibuja otra vez, y la caché no se llena de variantes
casi iguales. Ctrl + rueda o los botones acercan.

### Modelos 3D

Arrastrar gira, botón derecho o central desplaza, la rueda acerca y doble clic
vuelve a la vista de partida, que es la misma que la de la miniatura: abrir no
es un salto. Botones para frente, arriba, lado e iso, el alambre y abrir con el
programa del sistema. Arriba a la izquierda, lo que mide y cuántos triángulos.

El visor es OpenGL a mano en un `QQuickFramebufferObject` (`src/visor3d.h`) y
**no Qt Quick 3D**: Qt Quick 3D va con licencia GPL o comercial, y Grimorio
nació como binario cerrado. Para una malla sin texturas bastan un búfer de
vértices y dos sombreadores, que están en Qt Gui, que es LGPL. Por eso
`main.cpp` fija la escena en OpenGL.

La malla llega del núcleo centrada, a escala unidad y sin normales: el
sombreador las saca de la pendiente de la posición (`dFdx`/`dFdy`), que da
caras planas —lo que se quiere ver en una pieza de impresión— y deja el archivo
en la mitad. La proyección es ortográfica: en una pieza las paralelas se
quieren paralelas. Se encaja por la esfera que la contiene y no por la caja,
que girada asomaba por las esquinas y cortaba una punta.

El alambre va a media luz y no opaco: en una malla de impresión de un millón
de triángulos, opaco tapaba la pieza entera con un borrón de color.

## Módulos que hacen falta en Ubuntu 24.04

Debian no los declara como dependencias de `qml6-module-qtquick`, así que sin
ellos el binario compila pero `import QtQuick` falla al arrancar:

```sh
sudo apt install qt6-base-dev qt6-declarative-dev cmake \
                 qml6-module-qtqml qml6-module-qtqml-workerscript \
                 qml6-module-qtqml-models qml6-module-qtquick \
                 qml6-module-qtquick-window qml6-module-qtquick-templates \
                 qml6-module-qtquick-controls qml6-module-qtquick-layouts \
                 qml6-module-qtquick-shapes
```
