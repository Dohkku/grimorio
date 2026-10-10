// Qué hay de nuevo: los apartados de NOVEDADES.md.
//
// Sale solo una vez, al abrir una versión nueva después de actualizar, con lo
// de las versiones que no se habían visto; «entendido» lo da por visto. Desde
// Ajustes → acerca de → «novedades» se ven todas, cuando se quiera. El texto
// es el mismo que el de la release de GitHub y el de la web (ver novedades.h).
import QtQuick

Item {
    id: panel
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    /// Todas las versiones, o solo las que no se habían visto.
    property bool todas: false
    property var apartados: []

    function abrir(verTodas) {
        todas = verTodas === true
        apartados = novedades.cambios(todas)
        if (apartados.length === 0) return
        hoja.contentY = 0
        abierto = true
    }
    function cerrar() {
        abierto = false
        novedades.cambiosVistos()
    }

    readonly property real f: tema.fuente

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: panel.cerrar()
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        opacity: 0.75
    }

    Rectangle {
        id: tarjeta
        anchors.centerIn: parent
        width: Math.min(panel.f * 46, panel.width * 0.9)
        height: Math.min(cabecera.height + hoja.contentHeight + pie.height + tema.margen * 4,
                         panel.height * 0.86)
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        clip: true

        MouseArea { anchors.fill: parent }

        Column {
            id: cabecera
            anchors { top: parent.top; left: parent.left; right: parent.right }
            anchors.margins: tema.margen
            anchors.rightMargin: tema.margen * 3
            spacing: tema.hueco * 0.3

            Text {
                text: panel.todas ? qsTr("Novedades de Grimorio")
                                  : qsTr("Qué hay de nuevo en Grimorio %1").arg(novedades.actual)
                color: tema.texto
                font.pixelSize: panel.f * 1.35
                font.weight: Font.DemiBold
            }
            Text {
                visible: !panel.todas
                text: qsTr("Se ha actualizado. Esto es lo que cambia:")
                color: tema.textoTenue
                font.pixelSize: panel.f * 0.9
            }
        }

        BotonIcono {
            centrado: false
            anchors { top: parent.top; right: parent.right; margins: tema.hueco }
            icono: "cerrar"
            pista: qsTr("cerrar (Esc)")
            onPulsado: panel.cerrar()
        }

        Flickable {
            id: hoja
            anchors {
                top: cabecera.bottom; bottom: pie.top
                left: parent.left; right: parent.right
                topMargin: tema.margen; bottomMargin: tema.margen
                leftMargin: tema.margen; rightMargin: tema.margen
            }
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            contentWidth: width
            contentHeight: columna.height

            Column {
                id: columna
                width: hoja.width
                spacing: tema.margen

                Repeater {
                    model: panel.apartados
                    delegate: Column {
                        required property var modelData
                        width: columna.width
                        spacing: tema.hueco * 0.5

                        // La versión, como un rótulo: con varias, se ve dónde
                        // empieza cada una.
                        Rectangle {
                            visible: panel.apartados.length > 1
                            width: rotulo.implicitWidth + tema.hueco * 1.6
                            height: rotulo.implicitHeight + tema.hueco * 0.5
                            radius: height / 2
                            color: modelData.version === novedades.actual ? tema.seleccion : tema.borde
                            Text {
                                id: rotulo
                                anchors.centerIn: parent
                                text: modelData.version
                                color: modelData.version === novedades.actual ? tema.fondo : tema.texto
                                font.pixelSize: panel.f * 0.85
                                font.weight: Font.DemiBold
                            }
                        }
                        Text {
                            width: parent.width
                            wrapMode: Text.WordWrap
                            textFormat: Text.MarkdownText
                            text: modelData.texto
                            color: tema.texto
                            linkColor: tema.seleccion
                            font.pixelSize: panel.f * 0.95
                            lineHeight: 1.15
                            onLinkActivated: function (enlace) { Qt.openUrlExternally(enlace) }
                        }
                    }
                }
            }
        }

        Item {
            id: pie
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            anchors.margins: tema.margen
            height: entendido.height

            Text {
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: parent.left
                text: panel.todas ? qsTr("también en la web") : qsTr("todas las novedades")
                color: sobreWeb.containsMouse ? tema.seleccion : tema.textoTenue
                font.pixelSize: panel.f * 0.9
                font.underline: sobreWeb.containsMouse
                MouseArea {
                    id: sobreWeb
                    anchors.fill: parent
                    anchors.margins: -tema.hueco * 0.3
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (panel.todas) Qt.openUrlExternally("https://grimorio.frederickandrade.com/novedades/")
                        else panel.abrir(true)
                    }
                }
            }
            Boton {
                id: entendido
                anchors.right: parent.right
                principal: true
                texto: qsTr("entendido")
                onPulsado: panel.cerrar()
            }
        }
    }
}
