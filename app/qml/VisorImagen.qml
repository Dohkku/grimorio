// Una imagen en el visor, con zoom hasta ver los píxeles.
//
//   rueda             acercar o alejar alrededor del ratón
//   arrastrar         moverse por la imagen acercada
//   doble clic        ajustar ⇄ 100 % en ese punto
//   + / −  ·  Z       acercar, alejar · ajustar ⇄ 100 %
//   C                 copiar el color del píxel bajo el ratón
//   R  ·  Mayús+R     girar a la derecha · a la izquierda
//
// Ajustada a la ventana se ve como antes: la previsualización de 1024 px, centrada. En
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
    /// Si se mira un archivo que no está en la malla —una versión de una
    /// pila—, su imagen por ruta (`nucleo.imagenDeRuta`). La previa y el
    /// original salen de ella: por id, el proveedor no lo encontraría.
    property string fuenteArchivo: ""
    property bool conOriginal: true
    /// Las medidas del original según la ficha, si se saben. Sirven para que
    /// la geometría no salte al llegar el original: hasta entonces se acerca
    /// la previa, pero ya a la escala de lo que va a venir.
    property real anchoPista: 0
    property real altoPista: 0

    signal siguiente()
    signal cerrar()
    /// Clic derecho en (x, y) de la ventana: el menú de lo que se ve.
    signal menu(real x, real y)

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

    /// Cuánto está girada, en grados: 0, 90, 180 o 270. Es solo cómo se
    /// mira; el archivo no se toca. Lo pone el visor, con el giro guardado
    /// del elemento.
    property int giro: 0
    readonly property bool girada: giro % 180 !== 0
    // Girada ya no cabe donde cabía.
    onGiroChanged: encajar()
    /// Girar es pedirlo: el visor lo guarda en el elemento y vuelve por `giro`.
    signal pedirGiro(int grados)

    /// Lo que deja libre la interfaz del visor alrededor: la foto se ajusta
    /// y se mueve dentro de lo que queda, y así las estrellas, las versiones
    /// y el nombre no se le montan encima. Con todo a cero, de borde a borde.
    property real margenArriba: tema.margen * 3
    property real margenAbajo: tema.margen * 3
    property real margenLados: tema.margen * 2
    /// La interfaz del visor se ha apartado: los datos y el giro, también.
    property bool interfazFuera: false

    Item {
        id: hueco
        anchors.fill: parent
        anchors.topMargin: vi.margenArriba
        anchors.bottomMargin: vi.margenAbajo
        anchors.leftMargin: vi.margenLados
        anchors.rightMargin: vi.margenLados
    }

    /// Ajustada llena el hueco. En las medidas del marco que gira: girada un
    /// cuarto, el ancho de la foto va a lo alto.
    readonly property real vistaW: girada ? hueco.height : hueco.width
    readonly property real vistaH: girada ? hueco.width : hueco.height
    /// Encajar no agranda: una imagen pequeña se ve a su tamaño, no borrosa.
    readonly property real encaje: hayMedidas ? Math.max(0.01, Math.min(1, vistaW / natW, vistaH / natH)) : 1
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
    readonly property real posX: (marco.width - dibW) / 2 + (ajustada ? 0 : tx)
    readonly property real posY: (marco.height - dibH) / 2 + (ajustada ? 0 : ty)

    function limitar() {
        const mx = Math.max(0, (dibW - marco.width) / 2)
        const my = Math.max(0, (dibH - marco.height) / 2)
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
        const cx = marco.width / 2, cy = marco.height / 2
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
        zoomA(escalaReal * factor, raton.containsMouse ? raton.mouseX : marco.width / 2,
              raton.containsMouse ? raton.mouseY : marco.height / 2)
    }

    /// Ajustar ⇄ 100 % en el centro, para el teclado.
    function alternarCentro() {
        alternar(marco.width / 2, marco.height / 2)
    }

    /// Un cuarto de vuelta: 1 a la derecha, -1 a la izquierda. Se vuelve a
    /// ajustar, porque girada ya no cabe donde cabía.
    function girar(sentido) {
        pedirGiro((giro + 90 * sentido + 360) % 360)
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

    // Lo que gira. La imagen, su rejilla y el ratón van dentro de un marco
    // que rota entero: así acercar alrededor del cursor, arrastrar y el
    // cuentagotas siguen haciendo sus cuentas en el sentido de la foto, sin
    // saber que está girada. Girado un cuarto, el marco tiene las medidas del
    // visor cambiadas: su ancho es el alto de la pantalla.
    Item {
        id: marco
        anchors.centerIn: hueco
        width: vi.girada ? hueco.height : hueco.width
        height: vi.girada ? hueco.width : hueco.height
        rotation: vi.giro

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
            source: vi.fuenteArchivo.length > 0 ? vi.fuenteArchivo
                    : vi.idElemento.length > 0 ? "image://grim/previa/" + vi.idElemento : ""
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
            source: !vi.pedirAlta || !vi.conOriginal || vi.idElemento.length === 0 ? ""
                    : vi.fuenteArchivo.length > 0 ? vi.fuenteArchivo
                    : "image://grim/original/" + vi.idElemento
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
            width: Math.min(marco.width, vi.posX + vi.dibW) - x
            height: Math.min(marco.height, vi.posY + vi.dibH) - y
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
                if (e.button === Qt.RightButton) {
                    const p = raton.mapToItem(null, e.x, e.y)
                    vi.menu(p.x, p.y)
                    return
                }
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
        id: datos
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: tema.margen
        width: fila.implicitWidth + tema.hueco * 1.6
        height: fila.implicitHeight + tema.hueco * 0.8
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        opacity: vi.interfazFuera ? 0 : vi.ajustada && !vi.dentro ? 0.55 : 0.95
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }

        Row {
            id: fila
            anchors.centerIn: parent
            spacing: tema.hueco

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: Math.round(vi.escalaReal * 100) + " %"
                      + (vi.ajustada ? "  ·  " + qsTr("ajustada") : "")
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

    // Girar, al lado de lo que se sabe: es de cómo se mira, como el zoom.
    Row {
        anchors.left: datos.right
        anchors.leftMargin: tema.hueco * 0.5
        anchors.verticalCenter: datos.verticalCenter
        spacing: tema.hueco * 0.3
        opacity: datos.opacity
        BotonIcono {
            icono: "girarIzq"
            pista: qsTr("girar a la izquierda (Mayús+R)")
            onPulsado: vi.girar(-1)
        }
        BotonIcono {
            icono: "girarDer"
            pista: qsTr("girar a la derecha (R)")
            onPulsado: vi.girar(1)
        }
    }

    Timer {
        id: copiado
        interval: 1200
    }
}
