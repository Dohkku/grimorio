// La escena mínima para probar el arrastre del árbol: dos filas y un bulto.
//
// Se cargan las `FilaCarpeta` de verdad, con su ratón, sus bandas y su zona de
// soltar. Lo único de mentira son `ventana`, `nucleo` y `tema`, que los pone la
// prueba en C++: `nucleo` apunta lo que le piden en vez de tocar la biblioteca.
//
// El que arrastra elementos sí está imitado aquí, porque una `Celda` de verdad
// pide un modelo, un proveedor de miniaturas y una malla que la coloque. Lo que
// se copia de ella es exactamente lo que importa: el `Bulto`, que es el que
// lleva la clave y el que va pegado al cursor.
import QtQuick
import "../qml"

Item {
    id: escena
    width: 240
    height: 240

    FilaCarpeta {
        id: arriba
        objectName: "arriba"
        width: parent.width
        y: 0
        idCarpeta: "a"
        nombre: "a"
    }

    FilaCarpeta {
        id: abajo
        objectName: "abajo"
        width: parent.width
        y: arriba.height
        idCarpeta: "b"
        nombre: "b"
    }

    // Unos cuantos elementos de la malla, arrastrándose a la vez.
    Rectangle {
        id: elementos
        objectName: "elementos"
        y: arriba.height * 2
        width: parent.width
        height: arriba.height * 2
        color: "transparent"

        MouseArea {
            id: raton
            anchors.fill: parent
            drag.target: fantasma
            onPressed: function (evento) {
                ventana.arrastrarElementos(["uno", "dos"], "vieja")
                fantasma.seguir(evento.x, evento.y)
            }
            onPositionChanged: function (evento) {
                if (raton.drag.active) fantasma.seguir(evento.x, evento.y)
            }
            onReleased: {
                fantasma.Drag.drop()
                ventana.acabarArrastre()
            }
        }

        Bulto {
            id: fantasma
            gesto: raton
            clave: "application/x-grimorio-ids"
        }
    }
}
