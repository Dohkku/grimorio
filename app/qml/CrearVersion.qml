// Sacar una versión más ligera de una imagen.
//
// Unas pocas recetas hechas —las que se usan de verdad: para la web, para
// mandar, una miniatura— y una a medida. El original no se toca nunca: la
// versión es un archivo nuevo que entra en la pila, debajo, y se ve al abrirla.
import QtQuick
import "textos.js" as Textos

Item {
    id: raiz
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    property string idElemento: ""
    property string nombre: ""
    /// El lado mayor del original, para no ofrecer reducir lo que ya es más
    /// pequeño.
    property int ladoOriginal: 0

    property string formato: "jpg"
    property int calidad: 82
    property int lado: 1600
    /// Lo que pesaría con lo que hay puesto, o nada mientras se calcula. Lo
    /// calcula el núcleo de verdad —decodificar, reducir y codificar—, así
    /// que no es una cuenta aproximada: es lo que va a pesar.
    property var estimado: null
    readonly property bool conCalidad: formato === "jpg" || formato === "webp"

    readonly property var recetas: [
        { titulo: qsTr("para la web"), formato: "jpg", calidad: 82, lado: 1600 },
        { titulo: qsTr("para mandar"), formato: "jpg", calidad: 75, lado: 1024 },
        { titulo: qsTr("miniatura"), formato: "jpg", calidad: 70, lado: 400 },
        { titulo: qsTr("webp ligera"), formato: "webp", calidad: 75, lado: 1600 },
        { titulo: qsTr("png sin pérdida"), formato: "png", calidad: 100, lado: 0 }
    ]

    function abrir(id, nombreElemento, w, h) {
        idElemento = id
        nombre = nombreElemento
        ladoOriginal = Math.max(w, h)
        poner(recetas[0])
        abierto = true
        forceActiveFocus()
        pedirEstimacion()
    }

    function poner(r) {
        formato = r.formato
        calidad = r.calidad
        lado = r.lado
        campoLado.text = r.lado > 0 ? String(r.lado) : ""
        campoCalidad.text = String(r.calidad)
    }

    function crear() {
        const l = parseInt(campoLado.text)
        const q = parseInt(campoCalidad.text)
        nucleo.crearVersion(idElemento, formato,
                            isNaN(q) ? 82 : q,
                            isNaN(l) ? 0 : l)
        abierto = false
    }

    function resumen(r) {
        let s = r.formato
        if (r.lado > 0) s += " · " + r.lado + " px"
        if (r.formato === "jpg") s += " · " + r.calidad + " %"
        return s
    }

    // Se recalcula un rato después del último cambio, no en cada tecla: cada
    // cálculo decodifica el original entero, y teclear «1600» serían cuatro.
    onFormatoChanged: pedirEstimacion()
    onCalidadChanged: pedirEstimacion()
    onLadoChanged: pedirEstimacion()
    function pedirEstimacion() {
        if (!abierto) return
        estimado = null
        espera.restart()
    }
    Timer {
        id: espera
        interval: 350
        onTriggered: nucleo.estimarVersion(raiz.idElemento, raiz.formato, raiz.calidad, raiz.lado)
    }
    Connections {
        target: nucleo
        function onEstimacion(e) {
            if (raiz.abierto && e.id === raiz.idElemento) raiz.estimado = e
        }
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onClicked: raiz.abierto = false
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        opacity: 0.75
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(460, raiz.width * 0.85)
        height: contenido.height + tema.margen * 2
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        MouseArea { anchors.fill: parent }

        Column {
            id: contenido
            anchors.centerIn: parent
            width: parent.width - tema.margen * 2
            spacing: tema.hueco

            Text {
                width: parent.width
                elide: Text.ElideMiddle
                text: qsTr("Nueva versión de «%1»").arg(raiz.nombre)
                color: tema.texto
                font.pixelSize: tema.fuente * 1.1
                font.weight: Font.DemiBold
            }
            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: qsTr("El original no se toca. La versión entra en su pila y se elige al abrirla.")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
            }

            // Las recetas hechas. La elegida se ve encendida mientras los
            // campos de abajo digan lo mismo que ella.
            Flow {
                width: parent.width
                spacing: tema.hueco * 0.5
                Repeater {
                    model: raiz.recetas
                    delegate: Boton {
                        required property var modelData
                        centrado: false
                        texto: modelData.titulo
                        activo: raiz.formato === modelData.formato
                                && raiz.lado === modelData.lado
                                && (modelData.formato === "png" || raiz.calidad === modelData.calidad)
                        onPulsado: raiz.poner(modelData)
                    }
                }
            }

            // A medida.
            Row {
                spacing: tema.hueco * 0.5
                Repeater {
                    model: ["jpg", "png", "webp"]
                    delegate: Boton {
                        required property string modelData
                        texto: modelData
                        activo: raiz.formato === modelData
                        onPulsado: raiz.formato = modelData
                    }
                }
            }

            Row {
                spacing: tema.hueco
                Column {
                    spacing: tema.hueco * 0.3
                    Text {
                        text: qsTr("lado mayor (px, vacío = igual)")
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente * 0.85
                    }
                    Rectangle {
                        width: Math.round(tema.fuente * 7)
                        height: Math.round(tema.fuente * 2.1)
                        radius: tema.radio * 0.6
                        color: tema.fondo
                        border.color: campoLado.activeFocus ? tema.seleccion : tema.borde
                        border.width: 1
                        TextInput {
                            id: campoLado
                            anchors.fill: parent
                            anchors.leftMargin: tema.hueco * 0.6
                            verticalAlignment: TextInput.AlignVCenter
                            color: tema.texto
                            font.pixelSize: tema.fuente
                            inputMethodHints: Qt.ImhDigitsOnly
                            validator: IntValidator { bottom: 0; top: 65535 }
                            onTextEdited: {
                                const v = parseInt(text)
                                raiz.lado = isNaN(v) ? 0 : v
                            }
                            onAccepted: raiz.crear()
                            Keys.onEscapePressed: raiz.abierto = false
                            KeyNavigation.tab: campoCalidad
                        }
                    }
                }
                Column {
                    spacing: tema.hueco * 0.3
                    opacity: raiz.conCalidad ? 1 : 0.4
                    Text {
                        text: qsTr("calidad (1–100)")
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente * 0.85
                    }
                    Rectangle {
                        width: Math.round(tema.fuente * 5)
                        height: Math.round(tema.fuente * 2.1)
                        radius: tema.radio * 0.6
                        color: tema.fondo
                        border.color: campoCalidad.activeFocus ? tema.seleccion : tema.borde
                        border.width: 1
                        TextInput {
                            id: campoCalidad
                            anchors.fill: parent
                            anchors.leftMargin: tema.hueco * 0.6
                            verticalAlignment: TextInput.AlignVCenter
                            enabled: raiz.conCalidad
                            color: tema.texto
                            font.pixelSize: tema.fuente
                            inputMethodHints: Qt.ImhDigitsOnly
                            validator: IntValidator { bottom: 1; top: 100 }
                            onTextEdited: {
                                const v = parseInt(text)
                                if (!isNaN(v)) raiz.calidad = v
                            }
                            onAccepted: raiz.crear()
                            Keys.onEscapePressed: raiz.abierto = false
                            KeyNavigation.tab: campoLado
                        }
                    }
                }
            }

            Text {
                width: parent.width
                wrapMode: Text.Wrap
                visible: raiz.lado > 0 && raiz.ladoOriginal > 0 && raiz.lado >= raiz.ladoOriginal
                text: qsTr("La imagen ya mide %1 px: se queda de ese tamaño, nunca se amplía.")
                      .arg(raiz.ladoOriginal)
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.85
            }
            Text {
                width: parent.width
                wrapMode: Text.Wrap
                visible: raiz.formato !== "jpg"
                text: raiz.formato === "png"
                      ? qsTr("PNG no pierde nada: aligera sobre todo si además se reduce.")
                      : qsTr("WebP conserva la transparencia; a calidad 100 sale sin pérdida.")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.85
            }

            // Lo que va a pesar, antes de crearla.
            Text {
                width: parent.width
                wrapMode: Text.Wrap
                readonly property var e: raiz.estimado
                text: !e ? qsTr("calculando el peso…")
                      : e.peso >= e.original
                        ? qsTr("pesará %1 (%2×%3): más que el original, que pesa %4. Prueba con menos calidad o menos lado.")
                          .arg(Textos.peso(e.peso)).arg(e.ancho).arg(e.alto).arg(Textos.peso(e.original))
                        : qsTr("pesará %1 (%2×%3), un %4 % menos que el original (%5)")
                          .arg(Textos.peso(e.peso)).arg(e.ancho).arg(e.alto)
                          .arg(Math.round(100 - e.peso * 100 / e.original))
                          .arg(Textos.peso(e.original))
                color: e && e.peso >= e.original ? tema.texto : tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
                font.weight: Font.DemiBold
            }

            Row {
                anchors.right: parent.right
                spacing: tema.hueco * 0.6
                Boton {
                    texto: qsTr("cancelar")
                    onPulsado: raiz.abierto = false
                }
                Boton {
                    texto: qsTr("crear versión")
                    activo: true
                    onPulsado: raiz.crear()
                }
            }
        }
    }

    Keys.onEscapePressed: raiz.abierto = false
    Keys.onReturnPressed: raiz.crear()
}
