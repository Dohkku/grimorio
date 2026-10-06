// La pregunta que hay que hacer antes de algo que no tiene vuelta.
//
// No es un diálogo del sistema: es una tarjeta en el centro de la ventana. Un
// diálogo aparte se puede quedar detrás, tarda en abrirse y en Wayland a veces
// aparece en otro sitio. Y el fondo se oscurece porque una pregunta que no se
// distingue del resto se contesta sin leerla.
import QtQuick

Item {
    id: raiz
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    property string titulo: ""
    property string detalle: ""
    property string confirmar: qsTr("sí")
    signal aceptado()

    function preguntar(t, d, c) {
        titulo = t
        detalle = d
        confirmar = c
        abierto = true
    }

    // Tapa lo de debajo, también para el ratón: mientras hay una pregunta en
    // pantalla no se puede seguir tocando lo de detrás.
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
        width: Math.min(420, raiz.width * 0.8)
        height: contenido.height + tema.margen * 2
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        // Se traga los clics para que no lleguen a la capa de cerrar.
        MouseArea { anchors.fill: parent }

        Column {
            id: contenido
            anchors.centerIn: parent
            width: parent.width - tema.margen * 2
            spacing: tema.hueco

            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: raiz.titulo
                color: tema.texto
                font.pixelSize: tema.fuente * 1.1
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                wrapMode: Text.Wrap
                visible: raiz.detalle.length > 0
                text: raiz.detalle
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.95
            }

            Row {
                anchors.right: parent.right
                spacing: tema.hueco * 0.6

                Boton {
                    texto: qsTr("cancelar")
                    onPulsado: raiz.abierto = false
                }
                Boton {
                    texto: raiz.confirmar
                    activo: true
                    onPulsado: {
                        raiz.abierto = false
                        raiz.aceptado()
                    }
                }
            }
        }
    }

    Keys.onEscapePressed: raiz.abierto = false
}
