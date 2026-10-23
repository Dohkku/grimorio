// El editor: lo que se le hace a un elemento sin salir de Grimorio.
//
// Por ahora recorta vídeos y sonidos. Se abre con el vídeo entero elegido;
// se arrastran las asas de los extremos sobre la línea de tiempo, con los
// fotogramas del vídeo o la onda del sonido debajo —o el tramo entero, que
// se desliza sin cambiar lo que dura—, y se guarda: como versión en la pila
// del original, que no se toca, o como un vídeo nuevo aparte.
//
//   Espacio      reproducir el tramo, en bucle
//   I · O        inicio y fin donde está el cabezal
//   ← →          un fotograma (una décima en un sonido)
//   S            guardar el fotograma del cabezal como imagen aparte
//   Intro        guardar como versión
//   Mayús+Intro  guardar como vídeo nuevo
//   Esc          cerrar
//
// El imán: con él puesto, el inicio y el fin se pegan a tiempos redondos,
// múltiplos de `ajustes.pasoIman` (medio segundo si no se cambia): un corte
// en 8,04 queda en 8,0. La rejilla se ve como marcas bajo la línea.
//
// Recodificar o no lo decide dónde cae el inicio, no el imán: un corte sin
// recodificar solo empieza limpio en un fotograma clave, y el inicio rara
// vez cae en uno. Desde el principio, o justo en una clave, se copia sin
// tocar nada (ver `cortar_tramo` en derivado.rs); si no, se recodifica para
// que empiece justo donde se pidió, y se avisa antes de guardar.
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
    /// Los extremos se pegan a tiempos redondos. Se recuerda entre vídeos.
    property bool iman: true
    /// La rejilla del imán, en ms.
    readonly property real paso: Math.max(50, ajustes.pasoIman * 1000)
    /// Los fotogramas clave del vídeo, en ms, o vacío si aún no se saben.
    property var claves: []
    property string claveClaves: ""
    /// Si hay que recodificar: el inicio no es el principio ni cae en una
    /// clave. Sin saber las claves (sin ffprobe), se recodifica por si acaso:
    /// lo que se pidió es un corte justo.
    readonly property bool exacto: esVideo && entrada > 0
        && (claves.length === 0 || !enClave(entrada))
    /// Alguien arrastra un asa: la rejilla se enseña más.
    property bool arrastrando: false
    /// El fin sigue al final del vídeo hasta que alguien lo mueve: la
    /// duración llega tarde, y a veces no cambia —el mismo vídeo otra vez—.
    property bool finAlFinal: true
    property bool cortando: false
    property bool suelto: false
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
        finAlFinal = true
        cortando = false
        error = ""
        tira = ({})
        onda = ""
        claveTira = ""
        claves = []
        saltoPendiente = -1
        claveOnda = esVideo ? "" : derivados.pedir("onda", idElemento, original)
        claveClaves = esVideo ? derivados.pedir("claves", idElemento, original) : ""
        yaAbierto = true
        abierto = true
        forceActiveFocus()
        // El mismo archivo que la última vez: el reproductor no vuelve a
        // cargar nada y la duración no cambia, así que nadie avisaría. Era lo
        // que dejaba el tramo en un solo fotograma, en bucle.
        if (rep && String(rep.fuenteCargada) === String(original)) Qt.callLater(prepararTramo)
    }

    /// Con la duración ya sabida: el tramo, el vídeo entero; y la tira.
    function prepararTramo() {
        if (dur <= 0) return
        if (finAlFinal || salida > dur) salida = dur
        if (esVideo && claveTira === "")
            claveTira = derivados.pedir("tira", idElemento, original, { "duracion": dur / 1000 })
    }

    /// Si `ms` cae en un fotograma clave (a un milisegundo).
    function enClave(ms) {
        const i = claveCercana(ms)
        return i >= 0 && Math.abs(claves[i] - ms) <= 1
    }

    /// El índice de la clave más cercana a `ms`, o -1 sin claves.
    function claveCercana(ms) {
        const c = claves
        if (c.length === 0) return -1
        let lo = 0, hi = c.length - 1
        while (lo < hi) {
            const m = (lo + hi) >> 1
            if (c[m] < ms) lo = m + 1
            else hi = m
        }
        if (lo > 0 && Math.abs(c[lo - 1] - ms) <= Math.abs(c[lo] - ms)) return lo - 1
        return lo
    }

    /// Dónde queda un extremo pedido en `ms`: con el imán, en el múltiplo de
    /// `paso` más cercano —o en el final, si está más cerca—; sin él, donde
    /// se pidió.
    function pegar(ms) {
        if (!iman) return ms
        const r = Math.round(ms / paso) * paso
        return dur > 0 && Math.abs(dur - ms) < Math.abs(r - ms) ? dur : r
    }

    /// El tramo más corto que se deja: un paso con imán, una décima sin él.
    readonly property real minimo: iman ? Math.min(paso, dur) : 100

    function ponerEntrada(ms) {
        let v = pegar(Math.max(0, Math.min(ms, dur)))
        if (v > salida - minimo) v = iman ? Math.floor((salida - minimo) / paso) * paso
                                          : salida - minimo
        entrada = Math.max(0, v)
    }
    function ponerSalida(ms) {
        let v = pegar(Math.max(0, Math.min(ms, dur)))
        if (v < entrada + minimo) v = iman ? Math.ceil((entrada + minimo) / paso) * paso
                                           : entrada + minimo
        salida = Math.min(dur, v)
        finAlFinal = salida >= dur
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

    /// Lleva el tramo entero a empezar en `ms`, sin cambiar lo que dura.
    /// Con el imán, el inicio se pega a la rejilla; al llegar a un borde del
    /// vídeo se queda ahí.
    function moverTramo(ms, largo) {
        let v = iman ? pegar(ms) : ms
        v = Math.max(0, Math.min(v, dur - largo))
        entrada = v
        salida = v + largo
        finAlFinal = salida >= dur - 0.5
    }

    function marcarEntrada() { ponerEntrada(cabezal) }
    function marcarSalida() { ponerSalida(cabezal) }

    /// Corta el tramo; `aparte` lo guarda como un vídeo nuevo en vez de como
    /// versión.
    function guardar(aparte) {
        if (cortando || salida - entrada < 100) return
        error = ""
        suelto = aparte === true
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

    onDurChanged: prepararTramo()

    Connections {
        target: derivados
        function onListo(clave, r) {
            if (clave === editor.claveClaves) {
                editor.claves = r.ok ? r.claves : []
            } else if (clave === editor.claveTira) {
                editor.tira = r
            } else if (clave === editor.claveOnda && r.ok) {
                editor.onda = r.ruta
            } else if (clave === editor.claveCorte) {
                editor.cortando = false
                if (r.ok) {
                    nucleo.anadirTramo(editor.idElemento, r.ruta, editor.entrada, editor.salida,
                                       editor.exacto, editor.suelto)
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
        sequence: "S"
        enabled: editor.abierto && editor.esVideo && !ventana.escribiendo
        onActivated: ventana.sacarFotograma(editor.idElemento, editor.original, editor.idElemento, editor.cabezal)
    }
    Shortcut {
        sequences: ["Return", "Enter"]
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: editor.guardar(false)
    }
    Shortcut {
        sequences: ["Shift+Return", "Shift+Enter"]
        enabled: editor.abierto && !ventana.escribiendo
        onActivated: editor.guardar(true)
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
        height: alturaRotulo + tema.hueco * 0.9 + linea.height + tema.hueco * 0.6
                + mandos.height + tema.hueco * 1.2
        /// Lo que miden los rótulos de tiempo encima de las asas.
        readonly property real alturaRotulo: Math.round(tema.fuente * 1.7)

        // La línea de tiempo: el tramo elegido en claro, lo de fuera apagado.
        // Sin recortar: las asas sobresalen arriba y abajo, que es lo que las
        // hace ver como algo que se agarra.
        Item {
            id: linea
            anchors { left: parent.left; right: parent.right; top: parent.top }
            anchors.topMargin: abajo.alturaRotulo + tema.hueco * 0.9
            anchors.leftMargin: asaIzq.width
            anchors.rightMargin: asaIzq.width
            height: Math.round(tema.fuente * 5)
            readonly property real k: editor.dur > 0 ? width / editor.dur : 0

            Rectangle {
                anchors.fill: parent
                radius: tema.radio
                color: tema.panel
            }

            // Fotogramas de la tira, tantos como quepan.
            Item {
                anchors.fill: parent
                clip: true
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
            }

            // La rejilla del imán, como marcas debajo de la línea: es a donde
            // se pegan los extremos. Las de segundo entero, más largas. Si el
            // paso es tan fino que no se distinguirían, se pinta una de cada
            // tantas; el imán sigue pegando a todas.
            Repeater {
                id: rejilla
                readonly property int cada: editor.paso * linea.k >= 6 ? 1
                    : Math.ceil(6 / Math.max(0.001, editor.paso * linea.k))
                model: editor.iman && editor.dur > 0
                       ? Math.floor(editor.dur / (editor.paso * cada)) + 1 : 0
                delegate: Rectangle {
                    required property int index
                    readonly property real ms: index * rejilla.cada * editor.paso
                    readonly property bool segundo: Math.abs(ms / 1000 - Math.round(ms / 1000)) < 0.001
                    x: ms * linea.k - width / 2
                    y: linea.height + tema.hueco * 0.25
                    width: segundo ? 2 : 1
                    height: Math.round(tema.hueco * (segundo ? 0.8 : 0.45))
                    color: editor.arrastrando ? tema.seleccion : tema.textoTenue
                    opacity: editor.arrastrando ? 0.95 : 0.6
                }
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

            // El tramo entero se agarra y se desliza por el vídeo: la ventana
            // de corte se mueve y lo que dura no cambia. Un clic sin arrastrar
            // sigue llevando el cabezal ahí, como en el resto de la línea.
            MouseArea {
                id: bloque
                x: editor.entrada * linea.k
                width: Math.max(0, (editor.salida - editor.entrada) * linea.k)
                height: parent.height
                // Con el tramo entero elegido no hay a dónde moverlo.
                enabled: editor.salida - editor.entrada < editor.dur - 1
                hoverEnabled: true
                cursorShape: pressed && movido ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                property real desde: 0
                property real entradaDesde: 0
                property real largo: 0
                property bool movido: false
                onPressed: function (e) {
                    desde = mapToItem(linea, e.x, e.y).x
                    entradaDesde = editor.entrada
                    largo = editor.salida - editor.entrada
                    movido = false
                }
                onPositionChanged: function (e) {
                    if (!pressed || linea.k <= 0) return
                    const dx = mapToItem(linea, e.x, e.y).x - desde
                    if (!movido && Math.abs(dx) < 4) return
                    if (!movido) { movido = true; editor.arrastrando = true }
                    const antes = editor.entrada
                    editor.moverTramo(entradaDesde + dx / linea.k, largo)
                    if (editor.entrada !== antes) editor.ir(editor.entrada)
                }
                onReleased: function (e) {
                    editor.arrastrando = false
                    if (!movido && linea.k > 0)
                        editor.ir(Math.max(0, Math.min(editor.dur, mapToItem(linea, e.x, e.y).x / linea.k)))
                }
                onCanceled: editor.arrastrando = false
            }
            // Un velo del color del tramo al pasar por encima: «esto se agarra».
            Rectangle {
                x: bloque.x
                width: bloque.width
                height: parent.height
                color: tema.seleccion
                opacity: bloque.enabled && (bloque.containsMouse || bloque.pressed) ? 0.16 : 0
                Behavior on opacity { NumberAnimation { duration: 90 } }
            }

            // El cabezal.
            Rectangle {
                x: editor.cabezal * linea.k - width / 2
                width: 2
                height: parent.height
                color: tema.texto
            }

            // Las dos asas: corchetes del color del tramo que sobresalen de la
            // línea, con su tiempo encima. Se arrastran; el cabezal las sigue
            // para ver dónde cae el corte.
            component Asa: Item {
                id: asa
                property bool esSalida: false
                readonly property real ms: esSalida ? editor.salida : editor.entrada
                /// Con el imán puesto: se dice con el imán en su rótulo.
                readonly property bool pegada: editor.iman
                readonly property bool viva: agarre.containsMouse || agarre.pressed
                width: Math.round(tema.fuente * 1.25)
                height: linea.height + tema.hueco * 1.2
                y: -tema.hueco * 0.6
                x: ms * linea.k - (esSalida ? 0 : width)

                Rectangle {
                    anchors.fill: parent
                    radius: tema.radio
                    color: tema.seleccion
                    scale: asa.viva ? 1.08 : 1
                    Behavior on scale { NumberAnimation { duration: 90 } }
                    // El agarre: tres rayas, como en cualquier asa.
                    Column {
                        anchors.centerIn: parent
                        spacing: 3
                        Repeater {
                            model: 3
                            delegate: Rectangle {
                                width: Math.round(asa.width * 0.45)
                                height: 2
                                radius: 1
                                color: tema.fondo
                            }
                        }
                    }
                }

                // El tiempo del extremo, encima.
                Rectangle {
                    anchors.bottom: parent.top
                    anchors.bottomMargin: tema.hueco * 0.3
                    // Hacia dentro del tramo: hacia fuera se salía de la
                    // ventana con el inicio al principio o el fin al final.
                    x: asa.esSalida ? asa.width - width : 0
                    width: filaRotulo.implicitWidth + tema.hueco
                    height: abajo.alturaRotulo
                    radius: tema.radio
                    color: asa.viva ? tema.seleccion : tema.panel
                    border.color: tema.seleccion
                    border.width: 1
                    Row {
                        id: filaRotulo
                        anchors.centerIn: parent
                        spacing: tema.hueco * 0.3
                        Icono {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: asa.pegada
                            nombre: "iman"
                            width: Math.round(tema.fuente * 1.05)
                            height: width
                            color: asa.viva ? tema.fondo : tema.seleccion
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: (asa.esSalida ? qsTr("fin") : qsTr("inicio")) + "  " + editor.reloj(asa.ms)
                            color: asa.viva ? tema.fondo : tema.texto
                            font.pixelSize: tema.fuente * 0.85
                            font.family: "monospace"
                        }
                    }
                }

                MouseArea {
                    id: agarre
                    anchors.fill: parent
                    anchors.margins: -tema.hueco * 0.4
                    hoverEnabled: true
                    cursorShape: Qt.SizeHorCursor
                    property real desde: 0
                    property real msDesde: 0
                    onPressed: function (e) {
                        desde = mapToItem(linea, e.x, e.y).x
                        msDesde = asa.ms
                        editor.arrastrando = true
                    }
                    onReleased: editor.arrastrando = false
                    onCanceled: editor.arrastrando = false
                    onPositionChanged: function (e) {
                        if (!pressed || linea.k <= 0) return
                        const antes = asa.ms
                        const nuevo = msDesde + (mapToItem(linea, e.x, e.y).x - desde) / linea.k
                        if (asa.esSalida) editor.ponerSalida(nuevo)
                        else editor.ponerEntrada(nuevo)
                        // El cabezal sigue al asa, pero solo cuando cambia: con
                        // el imán el asa salta de paso en paso, y pedir un salto
                        // del vídeo en cada píxel lo atascaba.
                        if (asa.ms !== antes) editor.ir(asa.ms)
                    }
                }
            }
            Asa { id: asaIzq; esSalida: false }
            Asa { esSalida: true }
        }

        Row {
            id: mandos
            anchors { left: parent.left; top: linea.bottom; topMargin: tema.hueco * 1.2 }
            spacing: tema.hueco * 0.6

            Boton {
                texto: editor.rep && editor.rep.enMarcha ? "❚❚" : "▶"
                pista: qsTr("reproducir el tramo (Espacio)")
                onPulsado: editor.reproducir()
            }
            Boton {
                visible: editor.esVideo
                icono: "fotograma"
                texto: qsTr("guardar fotograma")
                pista: qsTr("el fotograma del cabezal, como imagen aparte en tu biblioteca (S)")
                onPulsado: ventana.sacarFotograma(editor.idElemento, editor.original,
                                                  editor.idElemento, editor.cabezal)
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                leftPadding: tema.hueco * 0.5
                text: qsTr("dura %1").arg(editor.reloj(editor.salida - editor.entrada))
                      + (editor.entrada <= 0 && editor.finAlFinal ? "  ·  " + qsTr("el vídeo entero") : "")
                color: tema.texto
                font.pixelSize: tema.fuente
            }
        }

        Row {
            anchors { right: parent.right; verticalCenter: mandos.verticalCenter }
            spacing: tema.hueco * 0.6
            // Qué va a pasar al guardar, dicho antes: instantáneo o recodificar.
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: editor.esVideo || editor.error.length > 0 || editor.cortando
                text: editor.cortando ? (editor.exacto ? qsTr("recodificando…") : qsTr("cortando…"))
                      : editor.error.length > 0 ? editor.error
                      : editor.exacto ? qsTr("corte justo: se recodifica, tarda un poco más")
                      : qsTr("corte instantáneo, sin recodificar")
                color: editor.error.length > 0 && !editor.cortando ? tema.seleccion : tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
            }
            Boton {
                icono: "iman"
                texto: qsTr("imán %1 s").arg(String(ajustes.pasoIman).replace(".", ","))
                activo: editor.iman
                pista: editor.iman
                       ? qsTr("el inicio y el fin se pegan a múltiplos de %1 s (las marcas de debajo); el paso se cambia en ajustes, «editor». Quítalo para dejarlos donde quieras").arg(String(ajustes.pasoIman).replace(".", ","))
                       : qsTr("el inicio y el fin van donde los sueltes. Ponlo para que se peguen a tiempos redondos")
                onPulsado: {
                    editor.iman = !editor.iman
                    if (!editor.iman) return
                    editor.ponerEntrada(editor.entrada)
                    if (!editor.finAlFinal) editor.ponerSalida(editor.salida)
                }
            }
            Boton {
                icono: "mas"
                texto: editor.esVideo ? qsTr("como vídeo nuevo") : qsTr("como sonido nuevo")
                enabled: !editor.cortando && editor.salida - editor.entrada >= 100
                pista: qsTr("un elemento aparte, en las mismas carpetas y con las mismas etiquetas (Mayús+Intro)")
                onPulsado: editor.guardar(true)
            }
            Boton {
                icono: "pila"
                texto: qsTr("como versión")
                principal: true
                enabled: !editor.cortando && editor.salida - editor.entrada >= 100
                pista: qsTr("entra en la pila del original, que no se toca (Intro)")
                onPulsado: editor.guardar(false)
            }
        }
    }
}
