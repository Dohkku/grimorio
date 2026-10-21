# Novedades de Grimorio

Lo que cambia en cada versión, de la más nueva a la más vieja.

Este archivo es la única fuente: Grimorio lo lleva dentro y lo enseña al abrir
una versión nueva después de actualizar (y en Ajustes → acerca de →
«novedades»); la publicación copia el apartado de cada versión en su release de
GitHub, y la web lo enseña en su página de novedades. Cada versión es un
apartado `## X.Y.Z`; lo que va debajo, en Markdown sencillo.

## 0.1.15

- **Arreglado:** al cambiar la salida de sonido del ordenador (de cascos a
  altavoces, por ejemplo) con Grimorio abierto, los vídeos y sonidos seguían
  sonando por la de antes. Ahora siguen a la que elijas.

## 0.1.14

- **Vídeos largos sin tirones.** Al abrir un vídeo largo, preparar la vista
  previa de la barra ocupaba el ordenador entero unos segundos (más cuanto
  más largo), justo mientras se pinchaba la barra. Ahora es unas treinta
  veces más ligero. Y pinchar o arrastrar la barra deprisa ya no amontona
  saltos.
- **Arreglado:** después de abrir algo con Intro y cerrarlo con Esc, las
  flechas dejaban de moverse por la galería.

## 0.1.13

- **Recortar vídeos y sonidos.** Con un vídeo o un sonido elegido, pulsa E (o
  clic derecho, «abrir en el editor»): marca la entrada y la salida sobre los
  fotogramas o la onda, con I y O, y guarda. El trozo sale como versión en su
  pila, sin tocar el original, y al momento: no se recodifica, así que tarda
  lo mismo con un vídeo de una hora. «Exacto» recodifica para cortar en el
  fotograma justo.
- **Dejar fuera en el buscador.** Un menos delante quita lo que diga:
  `#comida -#picante`, `-tipo:video`, `-borrador`. Y una barra vale cualquiera
  de las dos: `#comida|postre`.
- **Carpetas dinámicas que se editan.** El + crea la carpeta al momento,
  aunque esté vacía, y lo que filtres con ella abierta se queda como sus
  filtros. «Guardar como carpeta» le pone de nombre sus filtros («comida ·
  sin picante»); doble clic para cambiarlo.
- **Etiquetas sugeridas.** Debajo de las etiquetas de lo elegido salen
  propuestas: por su nombre, por las imágenes que se le parecen, por las que
  suelen ir con las que ya tiene y por su carpeta. También nuevas (en
  cursiva), sacadas del nombre del archivo o de la carpeta de la que vino. Un
  clic y se pone.
- **Modelos 3D con sus texturas.** GLB, glTF, .blend y .fbx se ven con sus
  colores y texturas en la galería y en el visor. Los que ya tenías se ponen
  al día con `grim miniaturas` (con Grimorio cerrado).
- **Retoques en el clic derecho.** Girar, crear versión, quitar el fondo y el
  editor, juntos.
- **Arreglado:** al elegir una etiqueta del buscador y pulsar la cruz, se
  quitaba el filtro pero el texto se quedaba escrito.

## 0.1.12

- **Las versiones de un vídeo, a la vista.** En el visor, con un vídeo que
  tiene versiones apiladas, la fila para elegir versión quedaba tapada por el
  propio vídeo. Ahora va siempre por encima, y las estrellas también a
  pantalla completa.

## 0.1.11

- **Dos bibliotecas a la vez.** En el menú de la biblioteca, cada una tiene un
  botón para abrirla en una ventana nueva (o Ctrl + clic), y hay «abrir otra en
  una ventana nueva…». La que tenías abierta se queda.
- **Copiar de una biblioteca a otra arrastrando.** Arrastra desde la galería de
  una ventana a la de otra biblioteca: se copia con su nombre, etiquetas,
  estrellas, nota, fuente y giro. La de origen no pierde nada.
- **Desplazarse con la rueda, sin resbalar.** Cada muesca baja lo mismo y la
  galería para donde tiene que parar; con el panel táctil sigue al dedo.
- **Vídeos repetidos en una pila.** En el visor se ven sus versiones también
  con vídeos, y se reproduce la que elijas. V pasa a la siguiente versión y
  Mayús + V a la anterior, también con fotos.
- **Ordenar por «antiguos»**, lo primero que entró arriba.
- **Arreglado:** en las carpetas vinculadas, lo que mandabas a la papelera
  podía volver a entrar solo. Y si una importación de una carpeta vinculada
  fallaba, esa carpeta dejaba de traer cosas hasta reiniciar.

## 0.1.10

- **Girar fotos y vídeos.** En el visor, R gira a la derecha y Mayús + R a la
  izquierda, o con sus botones. Se queda girado en la galería, el panel y el
  visor, y se deshace con Ctrl + Z. El archivo no se toca.
- **Vídeo a pantalla completa.** F, doble clic o su botón. Con el ratón quieto,
  los mandos se apartan y solo queda el vídeo.
- **Acercar un vídeo sin pararlo.** La rueda lo acerca hasta ver los píxeles,
  como en las fotos; Z lo devuelve. Los saltos con Mayús + flecha son ahora de
  10 segundos.
- **Tamaño de la interfaz con el teclado.** Ctrl + «+» y Ctrl + «−», como en
  un navegador; Ctrl + 0 la deja al 100 %. El tamaño de las miniaturas pasa a
  Ctrl + rueda.
- **El vídeo del panel de detalle, callado.** Tiene su botón de sonido y un
  ajuste para que suene siempre. Espacio lo para y lo sigue, y doble clic lo
  abre en grande.
- **Los botones dicen su atajo** al dejar el ratón encima.
- **Arrastrar una imagen a otra aplicación** funciona aunque Grimorio ocupe la
  pantalla entera.
- **Arreglado:** con el visor abierto, Supr dejaba el visor sin nada y las
  flechas volvían al principio. Ahora pasa a la siguiente.

## 0.1.9

- **Reordenar con Mayús.** Para cambiar de sitio una imagen en la biblioteca,
  arrástrala con Mayús apretada. Arrastrar sin más ya no mueve la galería, así
  que sacar una imagen a otra aplicación es más cómodo. Mayús + clic sigue
  eligiendo un tramo.

## 0.1.8

- **Arreglado:** un vídeo podía sonar dos veces a la vez. Pasaba al elegirlo
  y al abrirlo en grande con doble clic: seguía sonando también debajo.

## 0.1.7

- **Borrar para siempre.** En la papelera, Supr o el botón borran del disco
  lo que elijas, después de preguntar. Antes lo devolvían a la biblioteca.
- **Recuperar.** En la papelera, el botón de la ficha recupera la imagen.
- **Enter confirma** las preguntas, como «vaciar papelera».
- **Arreglado:** importar justo después de otra importación podía decir
  «otro proceso está escribiendo» sin que lo hubiera.

## 0.1.6

- **Versiones de una imagen.** La misma imagen en varios pesos se apila, y en
  la galería se ve una sola. Al abrirla eliges cuál ver.
- **Versiones más ligeras.** Clic derecho, «crear versión…». En JPEG, PNG o
  WebP, y ves cuánto pesará antes de crearla. El original no se toca.
- **Quitar el fondo.** Clic derecho, «quitar el fondo». Se hace en tu
  ordenador y queda como otra versión de la imagen.
- **Imágenes repetidas.** Si importas una que ya tenías en otro tamaño,
  Grimorio te ofrece apilarlas.
- **Actualizar en Linux trae todo.** Si vienes de la 0.1.5, Grimorio termina
  de instalarse solo la primera vez que lo abras.

## 0.1.5

- **Carpetas vinculadas, como nodos.** El menú de la biblioteca abre una vista
  nueva: tu biblioteca a la izquierda, con la carpeta del disco donde vive, y
  cada carpeta vinculada a la derecha, unidas por una línea por la que corre lo
  que entra. Cuando llega algo nuevo, su línea late. Desde ahí conectas una
  carpeta nueva (el nodo «+») o desconectas una (su ×); lo que ya entró se
  queda. Una carpeta que no está, como la de un disco sin conectar, se ve
  apagada y vuelve a funcionar sola cuando vuelve.
- **«Vincular» en lugar de «vigilar».** Es lo mismo de antes —lo que aparece
  en esa carpeta entra solo en la biblioteca— con un nombre que dice mejor lo
  que hace. En el árbol, la carpeta vinculada lleva un eslabón; pulsarlo abre
  la vista de nodos.
- **Renombrar la biblioteca.** «Renombrar esta biblioteca…» en el menú de su
  nombre. Cambia el nombre que ves, no la carpeta del disco, así que nada de lo
  que apunta a ella se rompe.
- **Novedades al actualizar.** Al abrir una versión nueva por primera vez,
  Grimorio enseña esto mismo. Para volver a verlo: Ajustes → acerca de →
  «novedades».
- **La versión portable de Windows, más clara.** Al abrir el zip solo se ven
  `Grimorio.exe`, un `LEEME.txt` y la carpeta `programa`, en lugar de
  doscientos archivos sueltos.

## 0.1.4

- **Actualizar desde dentro.** «Actualizar ahora» baja la versión nueva,
  comprueba que es exactamente la publicada y la instala; Grimorio se cierra y
  se vuelve a abrir solo, con tu biblioteca. En Windows con el instalador, y en
  Linux si la carpeta del programa es tuya; si no, abre la página de descargas.
- **Un aviso de versión nueva que se ve.** Una píldora de color en la barra de
  abajo, un punto en el engranaje y una tarjeta arriba del todo en Ajustes.
- **Los ajustes, arriba a la izquierda.** El engranaje está ahora junto al
  nombre de la biblioteca, y algo más grande.

## 0.1.3

- **Una biblioteca que no está ya no deja sin ventana.** Si la biblioteca que
  se abría no se encuentra (un disco sin montar, una carpeta movida), Grimorio
  abre la última que usaste, o crea una en Documentos, y te dice cuál no
  encontró. Antes se cerraba sin decir nada.

## 0.1.2

- Ajustes en el panel «acerca de».

## 0.1.1

- **Aviso de versión nueva.** Una vez al día Grimorio pregunta a su web cuál es
  la última versión, sin mandar nada tuyo, y te avisa si hay una nueva. Se
  apaga en Ajustes.

## 0.1.0

- La primera versión publicada: biblioteca visual para Linux y Windows con
  imágenes, vídeo, sonido, PDF y modelos 3D en una carpeta tuya; etiquetas,
  carpetas, carpetas dinámicas, búsqueda por texto, color y fecha, y un visor
  para cada cosa.
