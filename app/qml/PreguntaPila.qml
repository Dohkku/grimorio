// Qué pasa con las versiones al tirar una principal.
//
// Las dos respuestas son razonables y ninguna es la de siempre: quien tira una
// exportación vieja quiere quedarse con el original que hay debajo, y quien
// limpia quiere que se vaya todo. Por eso se pregunta, y se puede dejar de
// preguntar marcando «recordar»; la respuesta guardada se cambia en ajustes.
//
// Las dos cosas van a la papelera y se deshacen con Ctrl+Z: esto no es la
// pregunta de algo sin vuelta, y por eso no se parece a la de vaciar.
import QtQuick

Item {
    id: raiz
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    /// Lo que se va a tirar, tomado al preguntar: la selección puede cambiar
    /// mientras la tarjeta está abierta y la respuesta es sobre esto.
    property var ids: []
    property bool recordar: false
    signal elegido(var ids, bool conVersiones, bool recordar)

    function preguntar(lista) {
        ids = lista
        recordar = false
        abierto = true
        forceActiveFocus()
    }

    function responder(conVersiones) {
        abierto = false
        elegido(ids, conVersiones, recordar)
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
                wrapMode: Text.Wrap
                text: raiz.ids.length > 1
                      ? qsTr("Algunas de estas tienen versiones debajo")
                      : qsTr("Esta imagen tiene versiones debajo")
                color: tema.texto
                font.pixelSize: tema.fuente * 1.1
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: qsTr("Si va sola, la versión más pesada ocupa su sitio y se queda con sus etiquetas y carpetas. Las dos cosas se pueden deshacer.")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.95
            }

            // «Recordar», como casilla y no como tercer botón: es una forma de
            // contestar, no otra respuesta.
            Item {
                width: casilla.implicitWidth
                height: casilla.implicitHeight

                Row {
                    id: casilla
                    spacing: tema.hueco * 0.5
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.round(tema.fuente * 1.1)
                        height: width
                        radius: tema.radio * 0.4
                        color: raiz.recordar ? tema.seleccion : "transparent"
                        border.color: raiz.recordar ? tema.seleccion : tema.borde
                        border.width: 1
                        Text {
                            anchors.centerIn: parent
                            visible: raiz.recordar
                            text: "✓"
                            color: tema.fondo
                            font.pixelSize: tema.fuente * 0.8
                        }
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("recordar mi elección")
                        color: tema.texto
                        font.pixelSize: tema.fuente * 0.95
                    }
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: raiz.recordar = !raiz.recordar
                }
            }

            Row {
                anchors.right: parent.right
                spacing: tema.hueco * 0.6

                Boton {
                    texto: qsTr("cancelar")
                    onPulsado: raiz.abierto = false
                }
                Boton {
                    texto: qsTr("toda la pila")
                    onPulsado: raiz.responder(true)
                }
                Boton {
                    texto: qsTr("solo la principal")
                    activo: true
                    onPulsado: raiz.responder(false)
                }
            }
        }
    }

    Keys.onEscapePressed: raiz.abierto = false
    Keys.onReturnPressed: raiz.responder(false)
}
