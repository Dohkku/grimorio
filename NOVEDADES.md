# Novedades de Grimorio

Lo que cambia en cada versión, de la más nueva a la más vieja.

Este archivo es la única fuente: Grimorio lo lleva dentro y lo enseña al abrir
una versión nueva después de actualizar (y en Ajustes → acerca de →
«novedades»); la publicación copia el apartado de cada versión en su release de
GitHub, y la web lo enseña en su página de novedades. Cada versión es un
apartado `## X.Y.Z`; lo que va debajo, en Markdown sencillo.

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
