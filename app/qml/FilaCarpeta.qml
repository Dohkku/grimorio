// Una fila del árbol: nombre, cuenta y las tres zonas de soltar.
//
// Soltar una carpeta sobre otra fila quiere decir dos cosas distintas según
// dónde se suelte, y por eso la fila se parte en bandas:
//
//   ─── borde de arriba ──►  queda **delante** de esta, como hermana
//      centro              ►  queda **dentro** de esta, la última de sus hijas
//   ─── borde de abajo ───►  queda **detrás** de esta, como hermana
//
// Sin las bandas, el árbol solo sabía colgar de otra y el orden entre hermanas
// no se podía tocar. Con ellas, arrastrar hace lo que hace en cualquier árbol.
// Los elementos —fotos, vídeos— no tienen bandas: caigan donde caigan dentro de
// la fila, entran en la carpeta. Un elemento no tiene sitio entre dos carpetas.
import QtQuick
import "textos.js" as Textos

Rectangle {
    id: fila
    property string idCarpeta: ""
    /// De quién cuelga. Hace falta para las bandas: colocarse delante de esta
    /// es colgar de su padre, no de ella.
    property string padre: ""
    property string nombre: ""
    property int cuenta: 0
    property int nivel: 0
    /// El color que le ha puesto quien la usa, en "#rrggbb", o "" sin color.
    property string colorCarpeta: ""
    property bool elegida: false
    signal pulsada()
    /// El «+» de la fila: crear una carpeta dentro de esta.
    signal nuevaDentro()

    /// A qué altura de la fila está lo que se arrastra, para elegir banda.
    property real alturaEncima: 0

    /// La carpeta que se está arrastrando encima ahora mismo, o "".
    ///
    /// Sale de la ventana, no de la caída: en un arrastre interno el `mimeData`
    /// no viaja y preguntárselo devolvía siempre cadena vacía. Ver el comentario
    /// del arrastre en `Ventana.qml`.
    readonly property string carpetaEncima:
        soltar.containsDrag ? ventana.arrastreCarpeta : ""

    /// -1 delante, 0 dentro, 1 detrás. Sin carpeta encima siempre es dentro:
    /// un elemento no tiene sitio entre dos carpetas.
    function bandaEn(y) {
        if (ventana.arrastreCarpeta.length === 0) return 0
        if (y < height * 0.3) return -1
        if (y > height * 0.7) return 1
        return 0
    }

    /// De quién colgaría lo que se suelte a esa altura. Colocarse delante o
    /// detrás de esta es colgar de su padre, no de ella.
    function destinoEn(y) { return bandaEn(y) === 0 ? idCarpeta : padre }

    readonly property int banda: soltar.containsDrag ? bandaEn(alturaEncima) : 0
    readonly property string destino: destinoEn(alturaEncima)

    /// Si lo que hay encima se puede soltar donde apunta.
    readonly property bool admite: puedeIr(carpetaEncima, destino)

    /// Una carpeta no puede acabar dentro de sí misma ni de una de sus nietas.
    /// La cadena vacía en `id` es «no es una carpeta lo que se arrastra», que
    /// siempre vale; en `bajo` es la raíz, que siempre acepta.
    function puedeIr(id, bajo) {
        return id.length === 0 || (id !== bajo && !nucleo.cuelgaDe(bajo, id))
    }

    height: Math.round(tema.fuente * 2.6)
    radius: tema.radio
    // Elegida, en el tono de «marcado» y en negrita; no del color de selección.
    // Ese color es el de lo elegido en la malla, y con los dos sitios pintados
    // igual no se sabía cuál de los dos tenía el foco.
    color: (soltar.containsDrag && admite && banda === 0) ? tema.borde
           : (elegida ? tema.marcador : (raton.containsMouse ? tema.borde : "transparent"))
    Behavior on color { ColorAnimation { duration: 90 } }

    // La raya dice dónde va a caer: pegada al borde de arriba o al de abajo
    // según la banda. Sin ella, soltar sobre un árbol es adivinar.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: fila.banda === -1 ? parent.top : undefined
        anchors.bottom: fila.banda === 1 ? parent.bottom : undefined
        height: Math.max(1, Math.round(tema.fuente * 0.14))
        color: tema.seleccion
        visible: soltar.containsDrag && fila.admite && fila.banda !== 0
                 && fila.carpetaEncima.length > 0
    }

    // El punto de color. Sin color puesto es un aro: dice «aquí va un color»
    // sin inventarse uno.
    Rectangle {
        id: punto
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        // La sangría es lo único que dice de quién cuelga una carpeta: sin
        // ella, un árbol de tres niveles se lee como una lista plana.
        anchors.leftMargin: tema.hueco * 0.9 + fila.nivel * tema.hueco * 1.4
        width: Math.round(tema.fuente * 0.62)
        height: width
        radius: width / 2
        color: fila.colorCarpeta.length > 0 ? fila.colorCarpeta : "transparent"
        border.color: fila.colorCarpeta.length > 0 ? "transparent" : tema.textoTenue
        border.width: 1
    }

    Text {
        id: etiqueta
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: punto.right
        anchors.leftMargin: tema.hueco * 0.8
        anchors.right: ojo.visible ? ojo.left : numero.left
        anchors.rightMargin: tema.hueco * 0.5
        elide: Text.ElideMiddle
        text: fila.nombre
        color: tema.texto
        font.pixelSize: tema.fuente
        font.weight: fila.elegida ? Font.DemiBold : Font.Normal
    }

    // Vinculada: lo que aparezca en esa carpeta del disco entra aquí solo.
    // Pulsarlo abre los vínculos como nodos.
    Icono {
        id: ojo
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: numero.left
        anchors.rightMargin: tema.hueco * 0.4
        readonly property string ruta: nucleo.vigiladas.length >= 0 ? nucleo.vigiladaDe(fila.idCarpeta) : ""
        visible: ruta.length > 0 && !mas.visible
        nombre: "vincular"
        color: tema.seleccion
        width: Math.round(tema.fuente * 0.95)
        height: width
        MouseArea {
            anchors.fill: parent
            anchors.margins: -tema.hueco * 0.3
            cursorShape: Qt.PointingHandCursor
            onClicked: ventana.abrirVinculos()
        }
    }

    Text {
        id: numero
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: parent.right
        anchors.rightMargin: tema.hueco * 0.7
        text: fila.cuenta > 0 ? Textos.numero(fila.cuenta) : ""
        color: tema.textoTenue
        font.pixelSize: tema.fuente * 0.88
        // Al pasar por encima deja su sitio al «+».
        opacity: mas.visible ? 0 : 1
    }

    MouseArea {
        id: raton
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: raton.drag.active ? Qt.ClosedHandCursor : Qt.PointingHandCursor
        drag.target: fila.idCarpeta.length > 0 ? bulto : null
        drag.threshold: tema.fuente
        onPressed: function (evento) {
            ventana.arrastrarCarpeta(fila.idCarpeta)
            bulto.seguir(evento.x, evento.y)
        }
        onPositionChanged: function (evento) {
            if (raton.drag.active) bulto.seguir(evento.x, evento.y)
        }
        onClicked: function (evento) {
            // Tocar el panel se lleva el foco de la malla: si no, las teclas
            // siguen hablándole a las fotos mientras se está en las carpetas.
            fila.forceActiveFocus()
            if (evento.button === Qt.RightButton) {
                const p = mapToItem(ventana.contentItem, evento.x, evento.y)
                ventana.abrirMenuCarpeta(p.x, p.y, fila.idCarpeta, fila.nombre)
                return
            }
            fila.pulsada()
        }
        // Primero se entrega la caída y después se limpia: quien la recibe lee
        // de `ventana` lo que se estaba arrastrando.
        onReleased: {
            if (bulto.Drag.active) bulto.Drag.drop()
            ventana.acabarArrastre()
        }
    }

    // Crear dentro sin pasar por un menú: asoma al pasar por la fila, en el
    // sitio del número. Va después del `MouseArea` de la fila para quedar
    // encima y llevarse el clic.
    BotonIcono {
        id: mas
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: parent.right
        anchors.rightMargin: tema.hueco * 0.4
        visible: (raton.containsMouse || encima) && !raton.drag.active
                 && ventana.arrastreCarpeta.length === 0
        icono: "mas"
        pista: qsTr("carpeta dentro de «%1»").arg(fila.nombre)
        onPulsado: fila.nuevaDentro()
    }

    // Lo que viaja al arrastrar la carpeta. Lo que se ve es la fila de destino
    // iluminándose o la raya de dónde va a caer.
    Bulto {
        id: bulto
        gesto: raton
        clave: "application/x-grimorio-carpeta"
    }

    // Aquí se sueltan dos cosas distintas y no hacen lo mismo.
    //
    // Una carpeta encima cambia de sitio en el árbol —de quién cuelga, o entre
    // qué hermanas—. Elementos encima entran en esta carpeta, y **salen de
    // aquella de la que vienen** si vienen de una: arrastrar de una carpeta a
    // otra es mover, que es lo que espera cualquiera. Desde una búsqueda o sin
    // carpeta no hay de dónde sacarlos, así que solo se añaden.
    DropArea {
        id: soltar
        anchors.fill: parent
        enabled: fila.idCarpeta.length > 0
        keys: ["application/x-grimorio-ids", "application/x-grimorio-carpeta"]
        // Entrar se acepta siempre, aunque ahí no se pueda soltar. Rechazar la
        // entrada deja la fila sorda el resto del arrastre —Qt no vuelve a
        // avisarla— y con eso una carpeta arrastrada sobre sí misma perdía
        // también sus dos bandas, que son justo donde sí valía soltarla. Lo que
        // no se puede hacer no se ilumina, y al soltar no pasa nada.
        onEntered: function (caida) { fila.alturaEncima = caida.y }
        onPositionChanged: function (caida) { fila.alturaEncima = caida.y }
        onDropped: function (caida) {
            // La banda se lee antes de tocar nada: mira dónde está lo que se
            // arrastra, y en cuanto el arrastre acaba deja de decir la verdad.
            const donde = fila.bandaEn(caida.y)
            const bajo = fila.destinoEn(caida.y)

            const arrastrada = ventana.arrastreCarpeta
            if (arrastrada.length > 0) {
                if (!fila.puedeIr(arrastrada, bajo)) return
                if (donde === 0) nucleo.moverCarpeta(arrastrada, bajo)
                else if (donde === -1) nucleo.moverCarpeta(arrastrada, bajo, fila.idCarpeta)
                else nucleo.moverCarpeta(arrastrada, bajo,
                                         ventana.hermanaSiguiente(fila.idCarpeta))
                caida.accept()
                return
            }

            if (ventana.arrastreIds.length === 0) return
            nucleo.moverEntreCarpetas(ventana.arrastreIds, ventana.arrastreOrigen,
                                      fila.idCarpeta)
            caida.accept()
        }
    }
}
