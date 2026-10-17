// El editor: lo que se le hace a un elemento sin salir de Grimorio.
//
// Por ahora recorta vídeos y sonidos. Se marca la entrada y la salida sobre
// la línea de tiempo —con los fotogramas del vídeo o la onda del sonido
// debajo— y se guarda: el tramo entra como versión en la pila del original,
// que no se toca.
//
//   Espacio   reproducir el tramo, en bucle
//   I · O     entrada y salida donde está el cabezal
//   ← →       un fotograma (una décima en un sonido)
//   Intro     guardar el recorte
//   Esc       cerrar
//
// Rápido de verdad: el corte copia los paquetes sin recodificar, así que
// tarda lo que tarda leerlos aunque el vídeo pese gigas (ver `cortar_tramo`
// en derivado.rs). «Exacto» recodifica y corta en el fotograma justo.
//
// El reproductor no se destruye al cerrar: montar y desmontar la tubería de
// multimedia una y otra vez tira el programa (app/README.md). Se queda,
// parado y fuera de pantalla, hasta que se vuelve a abrir.
import QtQuick
import Grimorio
import "textos.js" as Textos

Item {
    id: editor
    visible: abierto

    property bool abierto: false
    property bool yaAbierto: false
    property string idElemento: ""
    property url original
    property string nombre: ""
    property int familia: -1
    readonly property bool esVideo: familia === Textos.VIDEO

    /// El tramo, en milisegundos.
    property real entrada: 0
    property real salida: 0
    property bool exacto: false
    property bool cortando: false
    property string error: ""

    property var tira: ({})
    property string onda: ""
    property string claveTira: ""
    property string claveOnda: ""
    property string claveCorte: ""

    readonly property var rep: reproductor.item
    readonly property real dur: rep ? rep.duracion : 0
    readonly property real pos: rep ? rep.posicion : 0
    readonly property bool sePuedeSaltar: rep ? rep.sePuedeSaltar : false

    /// Saltar a `ms`. Recién abierto, el reproductor aún no deja saltar y el
    /// salto se perdía: se guarda y se hace en cuanto deja.
    property real saltoPendiente: -1
    function ir(ms) {
        if (!rep) return
        if (sePuedeSaltar) rep.irA(ms)
        else saltoPendiente = ms
    }
    onSePuedeSaltarChanged: if (sePuedeSaltar && saltoPendiente >= 0) { rep.irA(saltoPendiente); saltoPendiente = -1 }
    /// Dónde está el cabezal, o a dónde va a ir.
    readonly property real cabezal: saltoPendiente >= 0 ? saltoPendiente : pos

    /// Si `i` se puede abrir aquí.
    function sabe(i) {
        const f = modelo.familiaDe(i)
        return f === Textos.VIDEO || f === Textos.AUDIO
    }

    function abrir(i) {
        if (i < 0 || !sabe(i)) return
        idElemento = modelo.idDe(i)
        original = modelo.urlDe(i)
        nombre = modelo.nombreDe(i)
        familia = modelo.familiaDe(i)
        entrada = 0
        salida = 0
        exacto = false
        cortando = false
        error = ""
        tira = ({})
        onda = ""
        claveTira = ""
        saltoPendiente = -1
        claveOnda = esVideo ? "" : derivados.pedir("onda", idElemento, original)
        yaAbierto = true
        abierto = true
        forceActiveFocus()
    }

    function cerrar() {
        if (rep && rep.enMarcha) rep.alternar()
        abierto = false
    }

    function reproducir() {
        if (!rep) return
        if (!rep.enMarcha && (pos < entrada || pos >= salida - 50)) rep.irA(entrada)
        rep.alternar()
    }

    function marcarEntrada() {
        entrada = Math.max(0, Math.min(cabezal, salida - 100))
    }
    function marcarSalida() {
        salida = Math.min(dur, Math.max(cabezal, entrada + 100))
    }

    function guardar() {
        if (cortando || salida - entrada < 100) return
        error = ""
        cortando = true
        claveCorte = derivados.pedir("tramo", idElemento, original, {
            "desde_ms": Math.round(entrada),
            "hasta_ms": Math.round(salida),
            "exacto": exacto
        })
    }

    /// `75300` → `1:15,3`, como se lee un tramo.
    function reloj(ms) {
        const s = Math.floor(ms / 1000)
        const d = Math.floor((ms % 1000) / 100)
        const m = Math.floor(s / 60)
        return m + ":" + String(s % 60).padStart(2, "0") + "," + d
    }

    onDurChanged: {
        if (dur <= 0) return
        if (salida <= 0 || salida > dur) salida = dur
        if (esVideo && claveTira === "")
            claveTira = derivados.pedir("tira", idElemento, original, { "duracion": dur / 1000 })
    }

    Connections {
        target: derivados
        function onListo(clave, r) {
            if (clave === editor.claveTira) {
                editor.tira = r
            } else if (clave === editor.claveOnda && r.ok) {
                editor.onda = r.ruta
            } else if (clave === editor.claveCorte) {
                editor.cortando = false
                if (r.ok) {
                    nucleo.anadirTramo(editor.idElemento, r.ruta, editor.entrada, editor.salida, editor.exacto)
                    editor.cerrar()
                } else {
                    editor.error = r.error
                }
            }
        }
    }

    // En bucle dentro del tramo: al llegar a la salida, vuelta a la entrada.
    Timer {
        interval: 30
        repeat: true
        running: editor.abierto && editor.rep !== null && editor.rep !== undefined && editor.rep.enMarcha
        onTriggered: if (editor.pos >= editor.salida || editor.pos < editor.entrada - 250) editor.rep.irA(editor.entrada)
    }

    Shortcut {
        sequence: "Space"
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: editor.reproducir()
    }
    Shortcut {
        sequence: "I"
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: editor.marcarEntrada()
    }
    Shortcut {
        sequence: "O"
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: editor.marcarSalida()
    }
    Shortcut {
        sequence: "Left"
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: if (editor.rep) editor.esVideo ? editor.rep.paso(-1) : editor.ir(editor.pos - 100)
    }
    Shortcut {
        sequence: "Right"
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: if (editor.rep) editor.esVideo ? editor.rep.paso(1) : editor.ir(editor.pos + 100)
    }
    Shortcut {
        sequences: ["Return", "Enter"]
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: editor.guardar()
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
    }
    // Se traga los clics: debajo está la galería.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        onWheel: function (r) { r.accepted = true }
    }

    // ------------------------------------------------------------- arriba
    Item {
        id: cabeza
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: ventana.altoBarra
        Text {
            anchors { left: parent.left; leftMargin: tema.margen; verticalCenter: parent.verticalCenter }
            text: qsTr("editar")
            color: tema.textoTenue
            font.pixelSize: tema.fuente
        }
        Text {
            anchors.centerIn: parent
            width: parent.width * 0.6
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideMiddle
            text: editor.nombre
            color: tema.texto
            font.pixelSize: tema.fuente * 1.1
            font.weight: Font.DemiBold
        }
        BotonIcono {
            anchors { right: parent.right; rightMargin: tema.margen; verticalCenter: parent.verticalCenter }
            icono: "cerrar"
            pista: qsTr("cerrar sin guardar (Esc)")
            onPulsado: editor.cerrar()
        }
        Rectangle {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: 1
            color: tema.borde
        }
    }

    // ------------------------------------------------------------- en medio
    Loader {
        id: reproductor
        anchors {
            left: parent.left; right: parent.right; top: cabeza.bottom; bottom: abajo.top
            margins: tema.margen
        }
        active: editor.yaAbierto
        visible: editor.esVideo
        source: "qrc:/qml/Reproductor.qml"
    }
    Binding { target: reproductor.item; property: "idElemento"; value: editor.idElemento; when: reproductor.status === Loader.Ready }
    Binding { target: reproductor.item; property: "fuente"; value: editor.original; when: reproductor.status === Loader.Ready }
    Binding { target: reproductor.item; property: "soloSonido"; value: !editor.esVideo; when: reproductor.status === Loader.Ready }
    Binding { target: reproductor.item; property: "conMandos"; value: false; when: reproductor.status === Loader.Ready }
    Binding { target: reproductor.item; property: "enPantalla"; value: editor.abierto; when: reproductor.status === Loader.Ready }
    Binding { target: reproductor.item; property: "bucle"; value: false; when: reproductor.status === Loader.Ready }

    // Un sonido no tiene imagen: en su sitio, la onda en grande.
    Onda {
        visible: !editor.esVideo
        anchors.fill: reproductor
        archivo: editor.onda
        modo: 0
        desde: 0
        hasta: 1
        colorFondo: tema.fondo
        colorOnda: tema.textoTenue
        colorMedio: tema.texto
        colorAlto: tema.seleccion
    }

    // ------------------------------------------------------------- abajo
    Item {
        id: abajo
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: tema.margen }
        height: linea.height + mandos.height + tema.hueco * 1.5

        // La línea de tiempo: el tramo elegido en claro, lo de fuera apagado.
        Item {
            id: linea
            anchors { left: parent.left; right: parent.right; top: parent.top }
            height: Math.round(tema.fuente * 5)
            readonly property real k: editor.dur > 0 ? width / editor.dur : 0
            clip: true

            Rectangle {
                anchors.fill: parent
                radius: tema.radio
                color: tema.panel
            }

            // Fotogramas de la tira, tantos como quepan.
            Row {
                id: fotos
                anchors.fill: parent
                readonly property var t: editor.tira
                readonly property bool lista: editor.esVideo && t.ok === true && tiraEntera.status === Image.Ready
                readonly property real fotoAncho: lista ? tiraEntera.sourceSize.width / t.columnas : 1
                readonly property real fotoAlto: lista ? tiraEntera.sourceSize.height / t.filas : 1
                readonly property real celda: height * fotoAncho / Math.max(1, fotoAlto)
                readonly property int cuantas: lista ? Math.max(1, Math.ceil(width / celda)) : 0
                Image {
                    id: tiraEntera
                    visible: false
                    source: fotos.t.ok === true ? derivados.url(fotos.t.ruta) : ""
                    asynchronous: true
                    cache: true
                }
                Repeater {
                    model: fotos.cuantas
                    delegate: Item {
                        required property int index
                        readonly property real segundos: (index + 0.5) * editor.dur / 1000 / fotos.cuantas
                        readonly property int n: Math.min(fotos.t.n - 1, Math.floor(segundos / fotos.t.cadaS))
                        readonly property real escala: fotos.height / fotos.fotoAlto
                        width: fotos.celda
                        height: fotos.height
                        clip: true
                        Image {
                            source: tiraEntera.source
                            asynchronous: true
                            cache: true
                            width: tiraEntera.sourceSize.width * parent.escala
                            height: tiraEntera.sourceSize.height * parent.escala
                            x: -(parent.n % fotos.t.columnas) * fotos.fotoAncho * parent.escala
                            y: -Math.floor(parent.n / fotos.t.columnas) * fotos.fotoAlto * parent.escala
                        }
                    }
                }
            }
            Onda {
                anchors.fill: parent
                visible: !editor.esVideo
                archivo: editor.onda
                modo: 0
                desde: 0
                hasta: 1
                colorFondo: tema.panel
                colorOnda: tema.textoTenue
                colorMedio: tema.texto
                colorAlto: tema.seleccion
            }

            // Fuera del tramo, apagado.
            Rectangle {
                x: 0
                width: editor.entrada * linea.k
                height: parent.height
                color: tema.fondo
                opacity: 0.72
            }
            Rectangle {
                x: editor.salida * linea.k
                width: Math.max(0, linea.width - x)
                height: parent.height
                color: tema.fondo
                opacity: 0.72
            }
            Rectangle {
                x: editor.entrada * linea.k
                width: Math.max(2, (editor.salida - editor.entrada) * linea.k)
                height: parent.height
                color: "transparent"
                border.color: tema.seleccion
                border.width: 2
                radius: tema.radio
            }

            // Pinchar o arrastrar sobre la línea mueve el cabezal.
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                function ir(x) {
                    if (linea.k > 0) editor.ir(Math.max(0, Math.min(editor.dur, x / linea.k)))
                }
                onPressed: function (e) { ir(e.x) }
                onPositionChanged: function (e) { if (pressed) ir(e.x) }
            }

            // El cabezal.
            Rectangle {
                x: editor.cabezal * linea.k - width / 2
                width: 2
                height: parent.height
                color: tema.texto
            }

            // Las dos asas, que se arrastran.
            component Asa: Rectangle {
                id: asa
                property bool esSalida: false
                readonly property real ms: esSalida ? editor.salida : editor.entrada
                width: Math.round(tema.fuente * 0.8)
                height: linea.height
                x: ms * linea.k - (esSalida ? 0 : width)
                radius: tema.radio * 0.5
                color: tema.seleccion
                Rectangle {
                    anchors.centerIn: parent
                    width: 2
                    height: parent.height * 0.35
                    radius: 1
                    color: tema.fondo
                }
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -tema.hueco * 0.4
                    cursorShape: Qt.SizeHorCursor
                    property real desde: 0
                    onPressed: function (e) { desde = e.x }
                    onPositionChanged: function (e) {
                        if (!pressed || linea.k <= 0) return
                        const nuevo = asa.ms + (e.x - desde) / linea.k
                        if (asa.esSalida)
                            editor.salida = Math.min(editor.dur, Math.max(editor.entrada + 100, nuevo))
                        else
                            editor.entrada = Math.max(0, Math.min(editor.salida - 100, nuevo))
                        // El cabezal sigue al asa, para ver dónde cae el corte.
                        editor.ir(asa.esSalida ? editor.salida : editor.entrada)
                    }
                }
            }
            Asa { esSalida: false }
            Asa { esSalida: true }
        }

        Row {
            id: mandos
            anchors { left: parent.left; top: linea.bottom; topMargin: tema.hueco }
            spacing: tema.hueco * 0.6

            Boton {
                texto: editor.rep && editor.rep.enMarcha ? "❚❚" : "▶"
                pista: qsTr("reproducir el tramo (Espacio)")
                onPulsado: editor.reproducir()
            }
            Boton {
                texto: qsTr("entrada aquí")
                pista: qsTr("la entrada donde está el cabezal (I)")
                onPulsado: editor.marcarEntrada()
            }
            Boton {
                texto: qsTr("salida aquí")
                pista: qsTr("la salida donde está el cabezal (O)")
                onPulsado: editor.marcarSalida()
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                leftPadding: tema.hueco * 0.5
                text: editor.reloj(editor.entrada) + " – " + editor.reloj(editor.salida)
                      + "   ·   " + qsTr("dura %1").arg(editor.reloj(editor.salida - editor.entrada))
                color: tema.texto
                font.pixelSize: tema.fuente
            }
        }

        Row {
            anchors { right: parent.right; top: linea.bottom; topMargin: tema.hueco }
            spacing: tema.hueco * 0.6
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: editor.error.length > 0 || editor.cortando
                text: editor.cortando ? (editor.exacto ? qsTr("recodificando…") : qsTr("cortando…"))
                                      : editor.error
                color: editor.cortando ? tema.textoTenue : tema.seleccion
                font.pixelSize: tema.fuente * 0.9
            }
            Boton {
                texto: qsTr("exacto")
                activo: editor.exacto
                pista: editor.exacto
                       ? qsTr("corta en el fotograma justo, recodificando: tarda más")
                       : qsTr("ahora es inmediato y empieza en el fotograma clave de antes; pulsa para cortar justo, recodificando")
                onPulsado: editor.exacto = !editor.exacto
            }
            Boton {
                texto: qsTr("guardar recorte")
                principal: true
                enabled: !editor.cortando && editor.salida - editor.entrada >= 100
                pista: qsTr("entra como versión en su pila; el original no se toca (Intro)")
                onPulsado: editor.guardar()
            }
        }
    }
}
