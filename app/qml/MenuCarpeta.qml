// El menú de una carpeta: crear dentro, renombrar, borrar.
//
// Vive en la ventana y no dentro de la fila que lo abre. Dentro de la fila
// tendría la altura de una fila y el ancho del panel —que se puede estrechar
// hasta que no quepa una palabra—, y encima el árbol recorta lo que se sale.
// Aquí puede desbordar el panel, que es lo que un menú tiene que poder hacer.
//
// Crear y renombrar son la misma pantalla con otro rótulo: las dos piden un
// nombre y ninguna necesita más. Borrar no pregunta, porque se deshace como
// todo lo demás; lo que no se deshace es lo único que pregunta en este
// programa, y eso es vaciar la papelera.
import QtQuick
import "consulta.js" as Consulta

Item {
    id: menu
    visible: false

    /// La carpeta sobre la que se abrió, o "" si se abrió sobre el hueco del
    /// panel: entonces lo único que se puede hacer es crear en la raíz.
    property string idCarpeta: ""
    property string nombreCarpeta: ""
    /// "acciones" mientras se elige qué hacer, "nueva" o "renombrar" cuando ya
    /// se está escribiendo el nombre, "color" mientras se elige su color.
    property string modo: "acciones"

    function abrir(px, py, id, nombre) {
        idCarpeta = id
        nombreCarpeta = nombre
        modo = id.length > 0 ? "acciones" : "nueva"
        // Que no se salga por abajo ni por la derecha. Un menú al que hay que
        // mover la ventana para leerlo no es un menú.
        cuadro.x = Math.min(px, menu.width - cuadro.width - tema.hueco)
        cuadro.y = Math.min(py, menu.height - cuadro.height - tema.hueco)
        visible = true
        if (modo === "nueva") empezarAEscribir("")
    }

    function cerrar() {
        visible = false
        modo = "acciones"
        campo.focus = false
    }

    function empezarAEscribir(texto) {
        campo.text = texto
        campo.forceActiveFocus()
        campo.selectAll()
    }

    function confirmar() {
        const nombre = campo.text.trim()
        if (nombre.length > 0) {
            if (menu.modo === "nueva") nucleo.crearCarpeta(nombre, menu.idCarpeta)
            else if (nombre !== menu.nombreCarpeta)
                nucleo.renombrarCarpeta(menu.idCarpeta, nombre)
        }
        menu.cerrar()
    }

    // Pinchar fuera cierra. Va antes que el cuadro para quedar por debajo.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: menu.cerrar()
    }

    Rectangle {
        id: cuadro
        width: Math.max(tema.fuente * 12, tema.lateral)
        height: columna.height + tema.hueco
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        radius: tema.radio

        Column {
            id: columna
            y: tema.hueco * 0.5
            width: parent.width

            // El rótulo dice sobre qué se está actuando. Sin él, dos carpetas
            // con el nombre a medio leer en un panel estrecho son la misma.
            Text {
                visible: menu.modo !== "acciones"
                width: parent.width
                leftPadding: tema.hueco
                rightPadding: tema.hueco
                bottomPadding: tema.hueco * 0.4
                elide: Text.ElideMiddle
                text: menu.modo === "renombrar"
                      ? qsTr("renombrar «%1»").arg(menu.nombreCarpeta)
                      : menu.modo === "color"
                      ? qsTr("color de «%1»").arg(menu.nombreCarpeta)
                      : (menu.idCarpeta.length > 0
                         ? qsTr("nueva dentro de «%1»").arg(menu.nombreCarpeta)
                         : qsTr("nueva carpeta"))
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.9
            }

            // Las muestras son las mismas que las del filtro de color: datos que
            // se guardan en `folders.json`, no colores de la interfaz. La
            // primera, vacía, le quita el color.
            Flow {
                visible: menu.modo === "color"
                x: tema.hueco
                width: parent.width - tema.hueco * 2
                bottomPadding: tema.hueco * 0.4
                spacing: tema.hueco * 0.5

                Repeater {
                    model: Consulta.muestrasConNinguna()
                    delegate: Rectangle {
                        id: muestra
                        required property string modelData
                        width: Math.round(tema.fuente * 1.6)
                        height: width
                        radius: width / 2
                        color: modelData.length > 0 ? modelData : "transparent"
                        border.color: sobreMuestra.containsMouse ? tema.texto : tema.borde
                        border.width: modelData.length > 0 ? 1 : 1.5

                        // La vacía lleva una raya: «ninguno», no «blanco».
                        Rectangle {
                            visible: muestra.modelData.length === 0
                            anchors.centerIn: parent
                            width: parent.width * 0.7
                            height: 1.5
                            rotation: -45
                            color: tema.textoTenue
                        }
                        MouseArea {
                            id: sobreMuestra
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                nucleo.colorCarpeta(menu.idCarpeta, muestra.modelData)
                                menu.cerrar()
                            }
                        }
                    }
                }
            }

            Rectangle {
                visible: menu.modo === "nueva" || menu.modo === "renombrar"
                x: tema.hueco
                width: parent.width - tema.hueco * 2
                height: Math.round(tema.fuente * 2)
                radius: tema.radio
                color: tema.fondo
                border.color: campo.activeFocus ? tema.seleccion : tema.borde
                border.width: 1

                TextInput {
                    id: campo
                    anchors.fill: parent
                    anchors.leftMargin: tema.hueco * 0.6
                    anchors.rightMargin: tema.hueco * 0.6
                    verticalAlignment: TextInput.AlignVCenter
                    clip: true
                    color: tema.texto
                    selectionColor: tema.seleccion
                    selectedTextColor: tema.fondo
                    font.pixelSize: tema.fuente
                    onAccepted: menu.confirmar()
                    Keys.onEscapePressed: menu.cerrar()
                }
            }

            Repeater {
                model: menu.modo === "acciones"
                       ? [{ "clave": "nueva", "texto": qsTr("+ carpeta dentro") },
                          { "clave": "renombrar", "texto": qsTr("renombrar") },
                          { "clave": "color", "texto": qsTr("color…") },
                          nucleo.vigiladaDe(menu.idCarpeta).length > 0
                          ? { "clave": "novigilar", "texto": qsTr("desvincular %1").arg(nucleo.vigiladaDe(menu.idCarpeta)) }
                          : { "clave": "vigilar", "texto": qsTr("vincular una carpeta del disco…") },
                          { "clave": "borrar", "texto": qsTr("borrar") }]
                       : []

                delegate: Rectangle {
                    id: opcion
                    required property var modelData
                    width: columna.width
                    height: Math.round(tema.fuente * 2.2)
                    color: sobre.containsMouse ? tema.borde : "transparent"

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: tema.hueco
                        anchors.right: parent.right
                        anchors.rightMargin: tema.hueco
                        elide: Text.ElideRight
                        text: opcion.modelData.texto
                        color: tema.texto
                        font.pixelSize: tema.fuente
                    }

                    MouseArea {
                        id: sobre
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            const clave = opcion.modelData.clave
                            if (clave === "borrar") {
                                nucleo.borrarCarpeta(menu.idCarpeta)
                                // La vista puede estar enseñando justo esa
                                // carpeta: hay que sacarla de ahí o se queda
                                // mirando algo que ya no existe.
                                if (ventana.carpetaActual === menu.idCarpeta)
                                    ventana.irACarpeta("")
                                menu.cerrar()
                                return
                            }
                            if (clave === "vigilar") {
                                const destino = menu.idCarpeta
                                menu.cerrar()
                                vigilancia.vigilar(destino)
                                return
                            }
                            if (clave === "novigilar") {
                                const vs = nucleo.vigiladas
                                for (let i = 0; i < vs.length; i++)
                                    if (vs[i].carpeta === menu.idCarpeta) nucleo.dejarDeVigilar(vs[i].id)
                                menu.cerrar()
                                return
                            }
                            // Crear dentro ya no se escribe aquí: se escribe
                            // en el árbol, donde va a quedar.
                            if (clave === "nueva") {
                                const madre = menu.idCarpeta
                                menu.cerrar()
                                ventana.nuevaCarpeta(madre)
                                return
                            }
                            if (clave === "color") {
                                menu.modo = "color"
                                return
                            }
                            menu.modo = clave
                            menu.empezarAEscribir(clave === "renombrar"
                                                  ? menu.nombreCarpeta : "")
                        }
                    }
                }
            }
        }
    }
}
