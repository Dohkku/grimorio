// La pista de un botón o un dato, en una capa por encima de todo.
//
// Antes cada botón llevaba la suya dentro, con `z: 10`. Pero `z` solo ordena
// entre hermanos: la pista de un botón de la barra de arriba seguía siendo
// hija de la barra, y la malla —que se dibuja después— la tapaba. Se veía la
// mitad de «modo seguro: lo +18 sale difuminado» asomando por debajo de las
// fotos. Aquí vive en la ventana, encima de la malla y de los paneles, y
// quien la quiere solo dice de qué cosa cuelga y qué pone.
import QtQuick

Item {
    id: capa
    anchors.fill: parent

    /// De quién es la pista que se ve, o null.
    property Item dueno: null
    property string texto: ""
    /// Encima de su dueño en vez de debajo: para lo que está al pie de algo.
    property bool arriba: false

    function mostrar(item, t, encima) {
        dueno = item
        texto = t
        arriba = encima === true
        colocar()
    }

    /// Solo la quita si sigue siendo de quien lo pide: al pasar rápido de un
    /// botón a otro, la salida del primero llega después de la entrada del
    /// segundo y borraba la pista buena.
    function ocultar(item) {
        if (dueno === item) dueno = null
    }

    function colocar() {
        if (!dueno) return
        const p = dueno.mapToItem(capa, 0, 0)
        const sep = tema.hueco * 0.4
        // Alineada a la derecha de su dueño, que no se salga por ningún lado.
        const x = p.x + dueno.width - caja.width
        caja.x = Math.max(tema.hueco * 0.5, Math.min(x, capa.width - caja.width - tema.hueco * 0.5))
        const abajo = p.y + dueno.height + sep
        const encima = p.y - caja.height - sep
        caja.y = arriba ? (encima >= 0 ? encima : abajo)
                        : (abajo + caja.height <= capa.height ? abajo : encima)
    }

    Rectangle {
        id: caja
        visible: capa.dueno !== null && capa.texto.length > 0
        width: Math.min(rotulo.implicitWidth + tema.hueco, capa.width * 0.5)
        height: rotulo.implicitHeight + tema.hueco * 0.6
        onWidthChanged: capa.colocar()
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        Text {
            id: rotulo
            anchors.centerIn: parent
            width: Math.min(implicitWidth, parent.width - tema.hueco)
            elide: Text.ElideMiddle
            text: capa.texto
            color: tema.texto
            font.pixelSize: tema.fuente * 0.85
        }
    }
}
