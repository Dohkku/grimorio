// Lo que viaja en un arrastre de dentro del programa. No se ve.
//
// Dos cosas que parecen detalles y no lo son, y que estaban mal en los dos
// sitios donde esto vivía copiado:
//
//   1. La carga **no** va aquí. El sitio evidente es `Drag.mimeData`, pero en
//      un arrastre interno de Qt Quick ese mapa no llega al otro lado: quien
//      recibe la caída ve las claves y la fuente, y `formats` vacío, así que
//      `getDataAsString` devuelve siempre cadena vacía. Lo único que cruza es
//      la clave, que dice **qué** se arrastra; el **cuál** lo guarda la ventana.
//
//   2. El bulto tiene que ir pegado al cursor, y hay que ponerlo a mano. Qt
//      busca la zona de soltar donde está el bulto, no donde está el ratón, y
//      por su cuenta el bulto nace en la esquina del que lo suelta y además se
//      queda atrás el umbral de arrastre, porque `MouseArea` lo descuenta para
//      que lo arrastrado no pegue un salto al empezar. Con filas de dos dedos
//      de alto eso es media banda de error: se pedía «dentro» y salía «detrás».
import QtQuick

Item {
    id: bulto

    /// El ratón que lo lleva. De él sale si el arrastre está en marcha.
    property MouseArea gesto: null
    /// Qué se arrastra. Es lo único que Qt hace llegar al otro lado.
    property string clave: ""

    width: 1
    height: 1
    Drag.active: gesto !== null && gesto.drag.active
    Drag.keys: [clave]

    /// Se llama al apretar y en cada movimiento, con las coordenadas del ratón.
    ///
    /// De paso le dice a la ventana dónde está, que es lo que necesita para
    /// pegar el fantasma al cursor. Se avisa desde aquí y no desde cada uno de
    /// los que arrastran porque aquí es donde ya se sabe: este es el que va
    /// pegado al cursor. Solo con el arrastre en marcha: apretar sin mover es
    /// un clic, y un clic no tiene que enseñar nada.
    function seguir(x, y) {
        bulto.x = x
        bulto.y = y
        if (Drag.active) ventana.moverArrastre(bulto.mapToItem(null, 0, 0))
    }
}
