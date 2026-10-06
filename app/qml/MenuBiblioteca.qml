// El menú del nombre de la biblioteca: cambiar a otra, abrir, crear.
//
// Vive en la ventana, como el de las carpetas, para poder desbordar la barra.
// Cambiar de biblioteca relanza el programa (el porqué, en bibliotecas.h), así
// que lo que se elige aquí cierra esta ventana y abre otra.
import QtQuick

Item {
    id: menu
    visible: false

    function abrir(px, py) {
        cuadro.x = Math.min(px, menu.width - cuadro.width - tema.hueco)
        cuadro.y = py
        visible = true
    }

    function cerrar() { visible = false }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: menu.cerrar()
    }

    Rectangle {
        id: cuadro
        width: Math.max(tema.fuente * 20, tema.lateral)
        height: columna.height + tema.hueco
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        radius: tema.radio

        Column {
            id: columna
            y: tema.hueco * 0.5
            width: parent.width

            Text {
                width: parent.width
                leftPadding: tema.hueco
                bottomPadding: tema.hueco * 0.4
                text: qsTr("bibliotecas")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.85
            }

            Repeater {
                model: bibliotecas.recientes
                delegate: Rectangle {
                    id: fila
                    required property string modelData
                    readonly property bool esActual: modelData === bibliotecas.actual
                    width: columna.width
                    height: Math.round(tema.fuente * 2.6)
                    color: sobre.containsMouse && !esActual ? tema.borde : "transparent"

                    Text {
                        id: marca
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: tema.hueco
                        text: fila.esActual ? "●" : ""
                        width: tema.fuente
                        color: tema.seleccion
                        font.pixelSize: tema.fuente * 0.7
                    }
                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: marca.right
                        anchors.leftMargin: tema.hueco * 0.4
                        anchors.right: quitar.left
                        Text {
                            width: parent.width
                            elide: Text.ElideRight
                            text: bibliotecas.nombreDe(fila.modelData)
                            color: tema.texto
                            font.pixelSize: tema.fuente
                            font.weight: fila.esActual ? Font.DemiBold : Font.Normal
                        }
                        // La ruta entera, porque dos bibliotecas pueden tener el
                        // mismo nombre en sitios distintos.
                        Text {
                            width: parent.width
                            elide: Text.ElideMiddle
                            text: fila.modelData
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.75
                        }
                    }
                    MouseArea {
                        id: sobre
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: fila.esActual ? Qt.ArrowCursor : Qt.PointingHandCursor
                        onClicked: {
                            if (fila.esActual) return
                            menu.cerrar()
                            bibliotecas.abrir(fila.modelData)
                        }
                    }
                    BotonIcono {
                        id: quitar
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.right: parent.right
                        anchors.rightMargin: tema.hueco * 0.4
                        visible: !fila.esActual && sobre.containsMouse || encima
                        icono: "cerrar"
                        pista: qsTr("quitar de la lista (no se borra nada)")
                        onPulsado: bibliotecas.olvidar(fila.modelData)
                    }
                }
            }

            Rectangle {
                width: parent.width
                height: 1
                color: tema.borde
            }

            // Las carpetas del disco que entran solas en esta biblioteca.
            Text {
                visible: nucleo.vigiladas.length > 0
                width: parent.width
                leftPadding: tema.hueco
                topPadding: tema.hueco * 0.4
                bottomPadding: tema.hueco * 0.2
                text: qsTr("vigiladas")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.85
            }
            Repeater {
                model: nucleo.vigiladas
                delegate: Item {
                    id: vig
                    required property var modelData
                    width: columna.width
                    height: Math.round(tema.fuente * 2.4)
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: tema.hueco
                        anchors.right: dejar.left
                        elide: Text.ElideMiddle
                        text: "◉ " + vig.modelData.ruta + "  →  "
                              + (vig.modelData.carpeta ? ventana.nombreDeCarpeta(vig.modelData.carpeta)
                                                       : qsTr("Todo"))
                        color: tema.texto
                        font.pixelSize: tema.fuente * 0.85
                    }
                    BotonIcono {
                        id: dejar
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.right: parent.right
                        anchors.rightMargin: tema.hueco * 0.4
                        icono: "cerrar"
                        pista: qsTr("dejar de vigilar (lo ya importado se queda)")
                        onPulsado: nucleo.dejarDeVigilar(vig.modelData.id)
                    }
                }
            }
            Rectangle {
                visible: nucleo.vigiladas.length > 0
                width: parent.width
                height: 1
                color: tema.borde
            }

            Repeater {
                model: [{ "nueva": false, "texto": qsTr("abrir otra…") },
                        { "nueva": true, "texto": qsTr("nueva biblioteca…") },
                        { "vigilar": true, "texto": qsTr("vigilar una carpeta del disco…") }]
                delegate: Rectangle {
                    id: accion
                    required property var modelData
                    width: columna.width
                    height: Math.round(tema.fuente * 2.2)
                    color: sobreAccion.containsMouse ? tema.borde : "transparent"
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: tema.hueco
                        text: accion.modelData.texto
                        color: tema.texto
                        font.pixelSize: tema.fuente
                    }
                    MouseArea {
                        id: sobreAccion
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            menu.cerrar()
                            if (accion.modelData.vigilar) vigilancia.vigilar("")
                            else bibliotecas.elegir(accion.modelData.nueva)
                        }
                    }
                }
            }
        }
    }
}
