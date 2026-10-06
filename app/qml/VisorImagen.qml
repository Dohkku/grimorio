// Una imagen en el visor, con zoom hasta ver los píxeles.
//
//   rueda             acercar o alejar alrededor del ratón
//   arrastrar         moverse por la imagen acercada
//   doble clic        encajar ⇄ 100 % en ese punto
//   + / −  ·  Z       acercar, alejar · encajar ⇄ 100 %
//   C                 copiar el color del píxel bajo el ratón
//
// Encajada se ve como antes: la previsualización de 1024 px, centrada. En
// cuanto se acerca más allá de lo que esa previsualización da de sí, se carga
// el original a resolución completa, y de 2× en adelante se pinta sin suavizar
// —el vecino más cercano—: un píxel es un cuadrado con bordes, que es lo que
// se quiere ver al llegar ahí. Desde 12× se dibuja además la rejilla.
//
// Lo que dice el número de abajo es la escala sobre el original: 100 % es un
// píxel de la foto por píxel de la pantalla, se esté viendo la previa o no.
import QtQuick

Item {
    id: vi
    property string idElemento: ""
    /// El archivo original, para el cuentagotas. De un RAW lo que hay es su
    /// vista previa interna, que suele ser mayor que la de 1024.
    property url original
    property bool conOriginal: true
    /// Las medidas del original según la ficha, si se saben. Sirven para que
    /// la geometría no salte al llegar el original: hasta entonces se acerca
    /// la previa, pero ya a la escala de lo que va a venir.
    property real anchoPista: 0
    property real altoPista: 0

    signal siguiente()
    signal cerrar()

    // -------------------------------------------------------------- medidas
    // Las medidas de cada imagen se apuntan al cargar, con el id del que son,
    // y no se atan ni se borran.
    //
    // Atadas a `implicitWidth` o a `sourceSize`, Qt las recalculaba al cambiar
    // el tamaño de dibujo, que sale de ellas. Y borrarlas al cambiar de
    // elemento las tocaba en mitad de calcular `natW` —que lee la ficha, que
    // lee el id que acaba de cambiar—. Los dos eran bucles de ataduras. Con el
    // id al lado, unas medidas viejas simplemente no cuentan.
    property real previaW: 0
    property real previaH: 0
    property string previaDe: ""
    property real altaW: 0
    property real altaH: 0
    property string altaDe: ""

    function medir() {
        if (previa.status === Image.Ready) {
            previaW = previa.implicitWidth
            previaH = previa.implicitHeight
            previaDe = idElemento
        }
        if (alta.status === Image.Ready) {
            altaW = alta.implicitWidth
            altaH = alta.implicitHeight
            altaDe = idElemento
        }
    }

    readonly property bool hayAlta: altaDe === idElemento && altaW > 0
    readonly property bool hayPrevia: previaDe === idElemento && previaW > 0
    readonly property real natW: hayAlta ? altaW : (anchoPista > 0 ? anchoPista : (hayPrevia ? previaW : 0))
    readonly property real natH: hayAlta ? altaH : (altoPista > 0 ? altoPista : (hayPrevia ? previaH : 0))
    readonly property bool hayMedidas: natW > 0 && natH > 0

    /// Encajada usa el mismo hueco que el visor de siempre.
    readonly property real vistaW: width - tema.margen * 4
    readonly property real vistaH: height - tema.margen * 6
    /// Encajar no agranda: una imagen pequeña se ve a su tamaño, no borrosa.
    readonly property real encaje: hayMedidas ? Math.min(1, vistaW / natW, vistaH / natH) : 1
    readonly property real maxEscala: 64

    /// Si sigue encajada. Mientras lo está, cambiar el tamaño de la ventana la
    /// vuelve a encajar sola.
    property bool ajustada: true
    property real escala: 1
    property real tx: 0
    property real ty: 0
    readonly property real escalaReal: ajustada ? encaje : escala

    readonly property real dibW: natW * escalaReal
    readonly property real dibH: natH * escalaReal
    readonly property real posX: (width - dibW) / 2 + (ajustada ? 0 : tx)
    readonly property real posY: (height - dibH) / 2 + (ajustada ? 0 : ty)

    function limitar() {
        const mx = Math.max(0, (dibW - width) / 2)
        const my = Math.max(0, (dibH - height) / 2)
        tx = Math.max(-mx, Math.min(mx, tx))
        ty = Math.max(-my, Math.min(my, ty))
    }

    /// Lleva la escala a `nueva` dejando quieto lo que hay bajo (px, py).
    function zoomA(nueva, px, py) {
        if (!hayMedidas) return
        const vieja = escalaReal
        nueva = Math.max(encaje, Math.min(maxEscala, nueva))
        if (nueva <= encaje * 1.0001) {
            encajar()
            return
        }
        const cx = width / 2, cy = height / 2
        const baseX = ajustada ? 0 : tx, baseY = ajustada ? 0 : ty
        const k = nueva / vieja
        tx = (px - cx) - (px - cx - baseX) * k
        ty = (py - cy) - (py - cy - baseY) * k
        escala = nueva
        ajustada = false
        limitar()
        pedirAlta = true
    }

    function encajar() {
        ajustada = true
        tx = 0
        ty = 0
    }

    function acercar(factor) {
        zoomA(escalaReal * factor, raton.containsMouse ? raton.mouseX : width / 2,
              raton.containsMouse ? raton.mouseY : height / 2)
    }

    /// Encajar ⇄ 100 %. Si la imagen cabe entera a 100 %, el otro extremo es
    /// 4×: ir a «100 %» no cambiaría nada.
    function alternar(px, py) {
        if (!ajustada) { encajar(); return }
        zoomA(encaje >= 1 ? 4 : 1, px, py)
    }

    // ---------------------------------------------------------- el original
    /// Se pide al acercar, o tras un momento quieto en la misma imagen: pasar
    /// con las flechas no decodifica originales de 50 megapíxeles por tecla.
    property bool pedirAlta: false
    Timer {
        id: quieto
        interval: 400
        onTriggered: vi.pedirAlta = true
    }

    // Al volver al bucle y no aquí mismo: este aviso llega en mitad de otras
    // ataduras que leen la escala, y tocarla desde dentro era un bucle.
    onIdElementoChanged: empezarLuego.restart()
    // Temporizadores a cero y no `Qt.callLater`: hacen lo mismo —esperar a
    // la vuelta del bucle— y qmllint no se queja de ellos.
    Timer { id: empezarLuego; interval: 0; onTriggered: vi.empezar() }
    Timer { id: medirLuego; interval: 0; onTriggered: vi.medir() }
    function empezar() {
        encajar()
        pedirAlta = false
        quieto.restart()
        // Una previa que ya estaba en caché no pasa por «cargando»: sigue
        // lista y no avisa.
        medir()
    }

    onPedirAltaChanged: if (pedirAlta && conOriginal) pixeles.cargar(original)

    Image {
        id: previa
        // A su tamaño natural y escalada, no estirada con `width`: cambiarle
        // el tamaño hace que Qt la vuelva a pedir, y si está en caché llega
        // lista en el acto, en mitad de calcular el propio tamaño —un bucle de
        // ataduras—.
        x: vi.posX
        y: vi.posY
        transformOrigin: Item.TopLeft
        scale: vi.hayPrevia ? vi.dibW / vi.previaW : 1
        source: vi.idElemento.length > 0 ? "image://grim/previa/" + vi.idElemento : ""
        asynchronous: true
        cache: true
        visible: !vi.hayAlta
        smooth: vi.dibW < vi.previaW * 2
        mipmap: true
        onStatusChanged: medirLuego.restart()
    }

    Image {
        id: alta
        x: vi.posX
        y: vi.posY
        transformOrigin: Item.TopLeft
        scale: vi.hayAlta ? vi.dibW / vi.altaW : 1
        // Por el proveedor y no con la ruta: Qt no sabe abrir WebP en esta
        // máquina, y el proveedor cae al núcleo cuando Qt no sabe.
        source: vi.pedirAlta && vi.conOriginal && vi.idElemento.length > 0
                ? "image://grim/original/" + vi.idElemento : ""
        asynchronous: true
        // Un original de varios megas por imagen vista no puede quedarse en el
        // caché de Qt: se lleva la memoria y no se vuelve a mirar.
        cache: false
        smooth: vi.escalaReal < 2
        mipmap: vi.escalaReal < 1
        onStatusChanged: medirLuego.restart()
    }

    // La rejilla de píxeles, desde 12×: a esa escala un píxel son doce de
    // pantalla y sin raya entre ellos dos iguales seguidos son un borrón.
    Item {
        id: rejilla
        readonly property bool toca: vi.escalaReal >= 12 && vi.hayAlta
        visible: toca
        x: Math.max(0, vi.posX)
        y: Math.max(0, vi.posY)
        width: Math.min(vi.width, vi.posX + vi.dibW) - x
        height: Math.min(vi.height, vi.posY + vi.dibH) - y
        clip: true
        readonly property int primeraX: Math.max(0, Math.floor(-vi.posX / vi.escalaReal))
        readonly property int primeraY: Math.max(0, Math.floor(-vi.posY / vi.escalaReal))

        Repeater {
            model: rejilla.toca ? Math.ceil(rejilla.width / vi.escalaReal) + 2 : 0
            Rectangle {
                required property int index
                x: vi.posX + (rejilla.primeraX + index) * vi.escalaReal - rejilla.x
                width: 1
                height: rejilla.height
                color: tema.fondo
                opacity: 0.35
            }
        }
        Repeater {
            model: rejilla.toca ? Math.ceil(rejilla.height / vi.escalaReal) + 2 : 0
            Rectangle {
                required property int index
                y: vi.posY + (rejilla.primeraY + index) * vi.escalaReal - rejilla.y
                height: 1
                width: rejilla.width
                color: tema.fondo
                opacity: 0.35
            }
        }
    }

    // --------------------------------------------------------------- ratón
    /// El píxel del original bajo el ratón, o -1.
    readonly property int pixX: raton.containsMouse && hayMedidas
                                ? Math.floor((raton.mouseX - posX) / escalaReal) : -1
    readonly property int pixY: raton.containsMouse && hayMedidas
                                ? Math.floor((raton.mouseY - posY) / escalaReal) : -1
    readonly property bool dentro: pixX >= 0 && pixY >= 0 && pixX < natW && pixY < natH
    readonly property string colorBajo: dentro && pixeles.listo && pixeles.fuente === original
                                        && pixeles.ancho === natW
                                        ? pixeles.hex(pixX, pixY) : ""

    function copiarColor() {
        if (colorBajo.length === 0) return false
        portapapeles.copiarTexto(colorBajo)
        copiado.restart()
        return true
    }

    MouseArea {
        id: raton
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: vi.ajustada ? Qt.ArrowCursor
                     : (pressed ? Qt.ClosedHandCursor
                                : (vi.escalaReal >= 8 ? Qt.CrossCursor : Qt.OpenHandCursor))
        property real x0: 0
        property real y0: 0
        property real tx0: 0
        property real ty0: 0
        property bool movido: false

        onPressed: function (e) {
            x0 = e.x; y0 = e.y; tx0 = vi.tx; ty0 = vi.ty
            movido = false
        }
        onPositionChanged: function (e) {
            if (!pressed) return
            if (Math.abs(e.x - x0) + Math.abs(e.y - y0) > 4) movido = true
            if (!vi.ajustada) {
                vi.tx = tx0 + e.x - x0
                vi.ty = ty0 + e.y - y0
                vi.limitar()
            }
        }
        onClicked: function (e) {
            if (movido) return
            if (e.button === Qt.RightButton) { vi.cerrar(); return }
            // El clic suelto espera un momento por si es doble: si no, el
            // primer clic del doble pasaba a la siguiente foto y se acercaba
            // a esa.
            clicSolo.restart()
        }
        onDoubleClicked: function (e) {
            clicSolo.stop()
            vi.alternar(e.x, e.y)
        }
        onWheel: function (r) {
            const d = r.angleDelta.y !== 0 ? r.angleDelta.y : r.angleDelta.x
            vi.zoomA(vi.escalaReal * Math.pow(1.0015, d), r.x, r.y)
        }
    }

    // Encajada, un clic pasa a la siguiente como siempre. Acercada no: ahí el
    // clic es para agarrar y mover, y soltar sin moverse no tiene que cambiar
    // de foto por sorpresa.
    Timer {
        id: clicSolo
        interval: 260
        onTriggered: if (vi.ajustada) vi.siguiente()
    }

    // ----------------------------------------------------- lo que se sabe
    Rectangle {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: tema.margen
        width: fila.implicitWidth + tema.hueco * 1.6
        height: fila.implicitHeight + tema.hueco * 0.8
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        opacity: vi.ajustada && !vi.dentro ? 0.55 : 0.95

        Row {
            id: fila
            anchors.centerIn: parent
            spacing: tema.hueco

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: Math.round(vi.escalaReal * 100) + " %"
                      + (vi.ajustada ? "  ·  " + qsTr("encajada") : "")
                color: tema.texto
                font.pixelSize: tema.fuente * 0.9
                font.family: "monospace"
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: vi.dentro
                text: vi.pixX + ", " + vi.pixY
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
                font.family: "monospace"
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                visible: vi.colorBajo.length > 0
                width: tema.fuente
                height: tema.fuente
                radius: tema.radio * 0.4
                color: vi.colorBajo.length > 0 ? vi.colorBajo : "transparent"
                border.color: tema.borde
                border.width: 1
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: vi.colorBajo.length > 0
                text: copiado.running ? qsTr("copiado") : vi.colorBajo + "  " + qsTr("(C copia)")
                color: copiado.running ? tema.seleccion : tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
                font.family: "monospace"
            }
        }
    }

    Timer {
        id: copiado
        interval: 1200
    }
}
