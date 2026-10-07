// La raya que separa dos paneles y sirve para moverla.
//
// La zona sensible es más ancha que la raya: una línea de un píxel se puede
// dibujar, pero no se puede agarrar. Lo que se ve es fino; lo que se pilla,
// cómodo.
import QtQuick

Item {
    id: divisor

    /// Cuánto se ha arrastrado desde la última vez, en píxeles. El positivo va
    /// hacia la derecha siempre; quien lo use decide el signo que le conviene.
    signal arrastrado(real dx)
    /// Doble clic: volver a lo que diga el tema.
    signal restablecido()

    width: Math.max(1, Math.round(tema.hueco * 0.3))

    Rectangle {
        anchors.fill: parent
        color: raton.containsMouse || raton.drag.active ? tema.seleccion : tema.borde
        opacity: raton.containsMouse || raton.drag.active ? 1 : 0.7
        Behavior on color { ColorAnimation { duration: 90 } }
    }

    MouseArea {
        id: raton
        anchors.fill: parent
        // A cada lado, para que se pueda agarrar sin apuntar.
        anchors.leftMargin: -tema.hueco * 0.5
        anchors.rightMargin: -tema.hueco * 0.5
        hoverEnabled: true
        cursorShape: Qt.SplitHCursor
        // Se arrastra un bulto que no se ve y se va contando el camino: mover
        // el panel de verdad mientras se arrastra haría que el ratón y el
        // panel se persiguieran, porque el panel se topa con sus límites y el
        // ratón no.
        drag.target: bulto
        drag.axis: Drag.XAxis
        drag.threshold: 0
        onPressed: bulto.x = 0
        onDoubleClicked: divisor.restablecido()
    }

    Item {
        id: bulto
        width: 1
        height: 1
        onXChanged: if (raton.drag.active) {
            divisor.arrastrado(x)
            x = 0
        }
    }
}
