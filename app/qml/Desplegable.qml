// Un menú que se abre debajo de lo que se pulsa —los chips de filtro, el menú
// de importar— o donde se hace clic derecho, con `abrirEn`.
//
// Vive en la ventana y no en quien lo abre, por lo de siempre: colgando de la
// barra quedaría debajo de la malla, que se dibuja después. Quien lo abre le
// pasa qué enseñar y qué hacer al elegir; el desplegable no sabe de filtros.
//
//   modo "lista"   opciones con icono, atajo y marca en la elegida
//   modo "color"   las muestras de color y un campo para un hex
//   modo "texto"   un campo, para lo que se escribe (una dirección web)
import QtQuick
import "consulta.js" as Consulta

Item {
    id: menu
    anchors.fill: parent
    visible: false

    property string modo: "lista"
    /// `[{texto, valor, icono?, atajo?}]`, y `{separador: true}` para una raya
    /// entre grupos.
    property var opciones: []
    /// Un rótulo pequeño arriba, que dice sobre qué se actúa. Vacío, nada.
    property string titulo: ""
    /// El valor elegido ahora, para marcarlo.
    property string actual: ""
    property string pista: ""
    property var alElegir: null
    /// Dónde se pidió. El cuadro se coloca con bindings y no a mano porque su
    /// alto se sabe un instante después de poner las opciones.
    property real pedidoX: 0
    property real pedidoY: 0

    function abrir(anclaje, def) {
        const p = anclaje.mapToItem(menu, 0, anclaje.height)
        abrirEn(p.x, p.y + tema.hueco * 0.4, def)
    }

    /// Abre con la esquina de arriba a la izquierda en (px, py), en
    /// coordenadas de la ventana. Si no cabe debajo, sube; si no cabe a la
    /// derecha, se va a la izquierda: un menú al que hay que mover la ventana
    /// para leerlo no es un menú.
    function abrirEn(px, py, def) {
        modo = def.modo || "lista"
        opciones = def.opciones || []
        actual = def.actual || ""
        pista = def.pista || ""
        titulo = def.titulo || ""
        alElegir = def.alElegir || null
        pedidoX = px
        pedidoY = py
        visible = true
        if (modo === "texto") {
            campo.text = ""
            campo.forceActiveFocus()
        } else if (modo === "color") {
            hex.text = Consulta.MUESTRAS.indexOf(actual) < 0 ? actual : ""
        }
    }

    function cerrar() {
        campo.focus = false
        hex.focus = false
        visible = false
    }

    function elegir(valor) {
        const f = alElegir
        cerrar()
        if (f) f(valor)
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: menu.cerrar()
    }

    Rectangle {
        id: cuadro
        x: Math.max(tema.hueco, Math.min(menu.pedidoX, menu.width - width - tema.hueco))
        y: menu.pedidoY + height + tema.hueco <= menu.height
           ? menu.pedidoY : Math.max(tema.hueco, menu.height - height - tema.hueco)
        // Ancho fijo: atarlo al de las filas, que se atan al suyo, era un bucle
        // de maquetación. Cabe lo más largo que hay («vigilar una carpeta del
        // disco…» con su atajo).
        width: menu.modo === "lista" ? tema.fuente * 21 : tema.fuente * 20
        height: contenido.height + tema.hueco
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        MouseArea { anchors.fill: parent }

        Column {
            id: contenido
            y: tema.hueco * 0.5
            width: parent.width

            Text {
                visible: menu.titulo.length > 0 && menu.modo === "lista"
                width: parent.width
                leftPadding: tema.hueco
                rightPadding: tema.hueco
                topPadding: tema.hueco * 0.2
                bottomPadding: tema.hueco * 0.5
                elide: Text.ElideMiddle
                text: menu.titulo
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.8
                font.letterSpacing: tema.fuente * 0.06
                font.capitalization: Font.AllUppercase
            }

            // --------------------------------------------------- lista
            Column {
                id: lista
                visible: menu.modo === "lista"
                width: parent.width
                Repeater {
                    model: menu.modo === "lista" ? menu.opciones : []
                    delegate: Rectangle {
                        id: op
                        required property var modelData
                        readonly property bool raya: modelData.separador === true
                        readonly property bool puesta: !raya && (modelData.valor || "") === menu.actual
                                                       && modelData.valor !== undefined
                        width: lista.width
                        height: raya ? tema.hueco : Math.round(tema.fuente * 2.3)
                        color: !raya && sobre.containsMouse ? tema.borde : "transparent"

                        Rectangle {
                            visible: op.raya
                            anchors.verticalCenter: parent.verticalCenter
                            x: tema.hueco
                            width: parent.width - tema.hueco * 2
                            height: 1
                            color: tema.borde
                        }
                        Row {
                            visible: !op.raya
                            anchors.verticalCenter: parent.verticalCenter
                            x: tema.hueco
                            spacing: tema.hueco * 0.6
                            Icono {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: (op.modelData.icono || "").length > 0
                                nombre: op.modelData.icono || ""
                                color: op.puesta ? tema.seleccion : tema.textoTenue
                                width: Math.round(tema.fuente * 1.2)
                                height: width
                            }
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: op.modelData.texto || ""
                                color: op.puesta ? tema.seleccion : tema.texto
                                font.pixelSize: tema.fuente
                            }
                        }
                        Text {
                            visible: !op.raya
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.right: parent.right
                            anchors.rightMargin: tema.hueco
                            text: op.puesta ? "✓" : (op.modelData.atajo || "")
                            color: op.puesta ? tema.seleccion : tema.textoTenue
                            font.pixelSize: tema.fuente * (op.puesta ? 1 : 0.85)
                        }
                        MouseArea {
                            id: sobre
                            anchors.fill: parent
                            enabled: !op.raya
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: menu.elegir(op.modelData.valor)
                        }
                    }
                }
            }

            // --------------------------------------------------- color
            Column {
                visible: menu.modo === "color"
                width: parent.width
                padding: tema.hueco
                spacing: tema.hueco * 0.6
                Text {
                    text: qsTr("ordena por cercanía a ese color")
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.85
                }
                Flow {
                    width: parent.width - tema.hueco * 2
                    spacing: tema.hueco * 0.45
                    Repeater {
                        model: Consulta.MUESTRAS
                        delegate: Rectangle {
                            required property string modelData
                            readonly property bool puesta: menu.actual.toLowerCase() === modelData
                            width: Math.round(tema.fuente * 1.9)
                            height: width
                            radius: tema.radio
                            color: modelData
                            border.width: puesta ? 2 : 1
                            border.color: puesta ? tema.texto : tema.borde
                            scale: sobreM.containsMouse ? 1.1 : 1
                            Behavior on scale { NumberAnimation { duration: 80 } }
                            MouseArea {
                                id: sobreM
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: menu.elegir(parent.modelData)
                            }
                        }
                    }
                }
                Row {
                    spacing: tema.hueco * 0.5
                    Rectangle {
                        width: tema.fuente * 7
                        height: Math.round(tema.fuente * 2)
                        radius: tema.radio
                        color: tema.fondo
                        border.color: hex.activeFocus ? tema.seleccion : tema.borde
                        border.width: 1
                        TextInput {
                            id: hex
                            anchors.fill: parent
                            anchors.leftMargin: tema.hueco * 0.5
                            verticalAlignment: TextInput.AlignVCenter
                            color: tema.texto
                            font.pixelSize: tema.fuente * 0.9
                            font.family: "monospace"
                            maximumLength: 7
                            onAccepted: {
                                let t = text.trim()
                                if (t.length > 0 && t.charAt(0) !== "#") t = "#" + t
                                menu.elegir(t)
                            }
                            Keys.onEscapePressed: menu.cerrar()
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: hex.text.length === 0 && !hex.activeFocus
                                text: "#hex"
                                color: tema.textoTenue
                                font: hex.font
                            }
                        }
                    }
                    Boton {
                        centrado: false
                        altura: Math.round(tema.fuente * 2)
                        visible: menu.actual.length > 0
                        texto: qsTr("quitar")
                        onPulsado: menu.elegir("")
                    }
                }
            }

            // --------------------------------------------------- texto
            Rectangle {
                visible: menu.modo === "texto"
                x: tema.hueco
                width: parent.width - tema.hueco * 2
                height: Math.round(tema.fuente * 2.2)
                radius: tema.radio
                color: tema.fondo
                border.color: tema.seleccion
                border.width: 1
                TextInput {
                    id: campo
                    anchors.fill: parent
                    anchors.leftMargin: tema.hueco * 0.6
                    anchors.rightMargin: tema.hueco * 0.6
                    verticalAlignment: TextInput.AlignVCenter
                    clip: true
                    color: tema.texto
                    font.pixelSize: tema.fuente
                    onAccepted: if (text.trim().length > 0) menu.elegir(text.trim())
                    Keys.onEscapePressed: menu.cerrar()
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: campo.text.length === 0
                        text: menu.pista
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente
                    }
                }
            }
        }
    }
}
