// Un sonido como se trabaja con él: la onda en el tiempo arriba, el espectro
// en frecuencia abajo, los dos con el mismo tramo y el mismo cabezal.
//
//   clic o arrastrar   ir a ese momento
//   rueda              acercar o alejar alrededor del ratón
//   Mayús + rueda      moverse por el tiempo
//   doble clic         verlo entero
//
// El análisis lo hace el núcleo la primera vez que se abre y lo guarda; lo de
// aquí solo pinta (ver `Onda` en src/onda.h).
import QtQuick
import Grimorio
import "textos.js" as Textos

Item {
    id: sonido
    /// El reproductor (`Reproductor.qml`).
    property var rep: null
    /// En el panel de la derecha: sin cabecera, sin botones y sin la escala
    /// de frecuencias, que ahí no caben. La rueda y el clic siguen valiendo.
    property bool compacto: false
    readonly property bool hayRep: rep !== null && rep !== undefined
    readonly property real dur: hayRep ? rep.duracion : 0
    readonly property real avance: dur > 0 && hayRep ? rep.posicion / dur : 0

    /// Tramo visible, de 0 a 1.
    property real desde: 0
    property real hasta: 1
    readonly property real ancho: hasta - desde
    /// Lo más que se deja acercar: un pico de la onda son 256 muestras, y más
    /// allá de eso no hay detalle que enseñar.
    readonly property real anchoMinimo: dur > 0 ? Math.min(1, 50 / dur) : 1

    function aFraccion(x, w) { return desde + ancho * Math.max(0, Math.min(1, x / w)) }
    function aX(f, w) { return (f - desde) / ancho * w }

    function acercar(factor, centro) {
        const nuevo = Math.max(anchoMinimo, Math.min(1, ancho * factor))
        let d = centro - (centro - desde) * nuevo / ancho
        d = Math.max(0, Math.min(1 - nuevo, d))
        desde = d
        hasta = d + nuevo
    }
    function mover(f) {
        const d = Math.max(0, Math.min(1 - ancho, desde + f))
        hasta = d + ancho
        desde = d
    }
    function entero() { desde = 0; hasta = 1 }

    // Acercado y en marcha, la vista sigue al cabezal: si se sale por la
    // derecha, pasa de página. Sin esto, acercarse y darle a reproducir deja
    // de verse lo que suena en cuanto pasan unos segundos.
    onAvanceChanged: {
        if (ancho < 1 && hayRep && rep.enMarcha && !raton.pressed
                && (avance > hasta || avance < desde))
            mover(avance - desde - ancho * 0.05)
    }

    Connections {
        target: sonido.hayRep ? sonido.rep : null
        function onFuenteChanged() { sonido.entero() }
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
    }

    // --------------------------------------------------------- la cabecera
    Row {
        id: cabecera
        anchors { left: parent.left; top: parent.top; right: parent.right }
        anchors.margins: tema.hueco
        height: sonido.compacto ? 0 : Math.round(tema.fuente * 4.5)
        visible: !sonido.compacto
        spacing: tema.hueco

        Image {
            width: parent.height
            height: parent.height
            source: sonido.hayRep ? sonido.rep.caratula : ""
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            visible: status === Image.Ready
        }

        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                text: raton.containsMouse
                      ? Textos.tiempoFino(sonido.aFraccion(raton.mouseX, raton.width) * sonido.dur / 1000)
                        + (raton.enEspectro ? "   ·   " + Textos.hercios(espectro.frecuenciaEn(
                              1 - (raton.mouseY - espectro.y) / espectro.height)) : "")
                      : Textos.tiempoFino(sonido.avance * sonido.dur / 1000)
                color: tema.texto
                font.pixelSize: tema.fuente * 1.3
                font.family: "monospace"
            }
            Text {
                text: sonido.ancho < 1
                      ? qsTr("viendo %1 de %2  ·  doble clic para verlo entero")
                            .arg(Textos.tiempoFino(sonido.ancho * sonido.dur / 1000))
                            .arg(Textos.tiempoFino(sonido.dur / 1000))
                      : qsTr("rueda para acercar  ·  Mayús + rueda para moverse  ·  clic para ir")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
            }
        }
    }

    Row {
        anchors.right: parent.right
        anchors.rightMargin: tema.hueco
        anchors.verticalCenter: cabecera.verticalCenter
        visible: !sonido.compacto
        spacing: tema.hueco / 2
        Boton { texto: "−"; onPulsado: sonido.acercar(2, sonido.desde + sonido.ancho / 2) }
        Boton { texto: "+"; onPulsado: sonido.acercar(0.5, sonido.ancho < 1 ? sonido.desde + sonido.ancho / 2 : sonido.avance) }
        Boton { texto: qsTr("entero"); activo: sonido.ancho >= 1; onPulsado: sonido.entero() }
    }

    // ------------------------------------------------- la onda y el espectro
    Item {
        id: zona
        anchors { left: parent.left; right: parent.right; top: cabecera.bottom; bottom: parent.bottom }
        anchors.margins: tema.hueco
        anchors.topMargin: sonido.compacto ? 0 : tema.hueco
        anchors.leftMargin: tema.hueco + (escala.visible ? escala.width : 0)

        Onda {
            id: onda
            anchors { left: parent.left; right: parent.right; top: parent.top }
            height: parent.height * 0.38
            archivo: sonido.hayRep ? sonido.rep.onda : ""
            modo: 0
            desde: sonido.desde
            hasta: sonido.hasta
            colorFondo: tema.panel
            colorOnda: tema.seleccion
            colorMedio: tema.seleccion
            colorAlto: tema.texto
        }

        Onda {
            id: espectro
            anchors { left: parent.left; right: parent.right; top: onda.bottom; bottom: parent.bottom }
            anchors.topMargin: tema.hueco / 2
            archivo: onda.archivo
            modo: 1
            desde: sonido.desde
            hasta: sonido.hasta
            colorFondo: tema.fondo
            colorOnda: tema.seleccion
            colorMedio: tema.seleccion
            colorAlto: tema.texto
        }

        // El tramo A-B, sobre los dos.
        Rectangle {
            readonly property real a: sonido.hayRep && sonido.dur > 0 ? sonido.rep.puntoA / sonido.dur : -1
            readonly property real b: sonido.hayRep && sonido.dur > 0 ? sonido.rep.puntoB / sonido.dur : -1
            visible: a >= 0
            x: sonido.aX(a, zona.width)
            width: b > a ? sonido.aX(b, zona.width) - x : 2
            height: parent.height
            color: tema.seleccion
            opacity: 0.18
        }

        // Donde está el ratón.
        Rectangle {
            visible: raton.containsMouse
            x: raton.mouseX
            width: 1
            height: parent.height
            color: tema.textoTenue
        }

        // El cabezal.
        Rectangle {
            visible: sonido.avance >= sonido.desde && sonido.avance <= sonido.hasta
            x: sonido.aX(sonido.avance, zona.width) - 1
            width: 2
            height: parent.height
            color: tema.texto
        }

        MouseArea {
            id: raton
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.IBeamCursor
            readonly property bool enEspectro: mouseY >= espectro.y
            function ir(x) {
                if (sonido.hayRep) sonido.rep.irA(sonido.aFraccion(x, width) * sonido.dur)
            }
            onPressed: function (e) { ir(e.x) }
            onPositionChanged: function (e) { if (pressed) ir(e.x) }
            onDoubleClicked: sonido.entero()
            onWheel: function (r) {
                const d = r.angleDelta.y !== 0 ? r.angleDelta.y : r.angleDelta.x
                if ((r.modifiers & Qt.ShiftModifier) || r.angleDelta.x !== 0)
                    sonido.mover(-d / 1200 * sonido.ancho)
                else
                    sonido.acercar(d > 0 ? 0.8 : 1.25, sonido.aFraccion(r.x, width))
            }
        }

        Text {
            anchors.centerIn: parent
            visible: !onda.listo
            text: sonido.hayRep && sonido.rep.errorOnda.length > 0
                  ? qsTr("no pude analizar el sonido: %1").arg(sonido.rep.errorOnda)
                  : qsTr("analizando el sonido…")
            color: tema.textoTenue
            font.pixelSize: tema.fuente
        }
    }

    // Las frecuencias a la izquierda del espectro, en escala logarítmica, que
    // es como se oyen: de 100 a 1000 Hz hay tanto sitio como de 1 a 10 kHz.
    Item {
        id: escala
        anchors { left: parent.left; leftMargin: tema.hueco / 2 }
        y: zona.y + espectro.y
        height: espectro.height
        width: tema.fuente * 4
        visible: espectro.listo && !sonido.compacto
        Repeater {
            model: [100, 1000, 5000, 10000]
            Text {
                readonly property real h: espectro.fMax > espectro.fMin
                    ? (Math.log(modelData) - Math.log(espectro.fMin))
                      / (Math.log(espectro.fMax) - Math.log(espectro.fMin)) : -1
                visible: h > 0 && h < 1
                y: escala.height * (1 - h) - height / 2
                text: Textos.hercios(modelData)
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.8
            }
        }
    }
}
