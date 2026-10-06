// Cinco estrellas. Pulsar la que ya está puesta las quita: es la forma más
// rápida de dejar algo a cero sin buscar un botón de "quitar".
import QtQuick

Row {
    id: fila
    property int valor: 0
    signal elegido(int n)

    spacing: 2
    height: Math.round(tema.fuente * 1.6)

    Repeater {
        model: 5
        delegate: Item {
            id: hueco
            // Con id propio y no `parent.index`: dentro de un delegado, `parent`
            // no siempre es lo que parece y una estrella de más no se nota hasta
            // que alguien cuenta.
            required property int index
            readonly property bool llena: hueco.index < fila.valor

            width: Math.round(tema.fuente * 1.5)
            height: fila.height

            Text {
                anchors.centerIn: parent
                text: hueco.llena ? "★" : "☆"
                color: hueco.llena ? tema.seleccion : tema.textoTenue
                font.pixelSize: tema.fuente * 1.3
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: fila.elegido(hueco.index + 1 === fila.valor ? 0 : hueco.index + 1)
            }
        }
    }
}
