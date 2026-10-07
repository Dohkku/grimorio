// Las etiquetas de lo que hay elegido: fichas que se quitan y un campo que
// autocompleta.
//
// Funciona igual con uno que con mil elementos. Con varios, la ficha dice
// cuántos la llevan, para que quitar una etiqueta no sea a ciegas.
import QtQuick

Column {
    id: raiz
    spacing: tema.hueco * 0.5

    /// Las etiquetas del elemento en foco.
    property var lista: []
    /// Cuántos elementos hay elegidos, para saber si se está editando en lote.
    property int cuantos: 1

    signal anadir(string etiqueta)
    signal quitar(string etiqueta)
    /// Pinchar una ficha busca por ella: es la forma más corta de llegar a
    /// «enséñame todo lo que tenga esto».
    signal buscar(string etiqueta)

    function enfocar() {
        campo.forceActiveFocus()
    }

    /// Escribe en el campo como si se hubiera tecleado. La usa el guion, que
    /// es como se revisa esta parte sin una persona delante: poner `text` a
    /// secas no dispara `onTextEdited` y el desplegable no se abriría.
    function escribir(t) {
        campo.text = t
        campo.cursorPosition = t.length
        campo.forceActiveFocus()
        sugeridas.elegida = -1
        nucleo.pedirEtiquetas(t.trim())
    }

    // Con varios elegidos, las fichas son las del elemento en foco pero lo que
    // se hace va a todos. Decirlo es la diferencia entre editar en lote y
    // quitarle una etiqueta a nueve elementos sin querer.
    Text {
        width: raiz.width
        wrapMode: Text.Wrap
        visible: raiz.cuantos > 1
        text: qsTr("etiquetas de uno · lo que hagas va a los %1").arg(raiz.cuantos)
        color: tema.textoTenue
        font.pixelSize: tema.fuente * 0.85
    }

    Flow {
        width: raiz.width
        spacing: tema.hueco * 0.5
        visible: raiz.lista.length > 0

        Repeater {
            model: raiz.lista
            delegate: Rectangle {
                id: ficha
                required property string modelData
                /// El color de su grupo (gestor de etiquetas), o "".
                readonly property string colorGrupo: nucleo.grupos.length >= 0
                                                     ? nucleo.colorDeEtiqueta(modelData) : ""
                height: Math.round(tema.fuente * 1.7)
                width: nombre.implicitWidth + cruz.width + tema.hueco * 1.4
                       + (colorGrupo.length > 0 ? punto.width + tema.hueco * 0.4 : 0)
                radius: height / 2
                color: fichaRaton.containsMouse ? tema.borde : tema.fondo
                border.color: colorGrupo.length > 0 ? colorGrupo : tema.borde
                border.width: 1

                Rectangle {
                    id: punto
                    visible: ficha.colorGrupo.length > 0
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: tema.hueco * 0.6
                    width: tema.fuente * 0.55
                    height: width
                    radius: width / 2
                    color: ficha.colorGrupo.length > 0 ? ficha.colorGrupo : "transparent"
                }

                Text {
                    id: nombre
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: punto.visible ? punto.right : parent.left
                    anchors.leftMargin: punto.visible ? tema.hueco * 0.4 : tema.hueco * 0.7
                    text: ficha.modelData
                    color: tema.texto
                    font.pixelSize: tema.fuente * 0.9
                }

                MouseArea {
                    id: fichaRaton
                    anchors.fill: parent
                    anchors.rightMargin: cruz.width
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: raiz.buscar(ficha.modelData)
                }

                Item {
                    id: cruz
                    anchors.right: parent.right
                    height: parent.height
                    width: Math.round(tema.fuente * 1.3)

                    Text {
                        anchors.centerIn: parent
                        text: "×"
                        color: cruzRaton.containsMouse ? tema.seleccion : tema.textoTenue
                        font.pixelSize: tema.fuente
                    }

                    MouseArea {
                        id: cruzRaton
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: raiz.quitar(ficha.modelData)
                    }
                }
            }
        }
    }

    Rectangle {
        id: caja
        width: raiz.width
        height: Math.round(tema.fuente * 2)
        radius: tema.radio
        color: tema.fondo
        border.color: campo.activeFocus ? tema.seleccion : tema.borde
        border.width: 1

        TextInput {
            id: campo
            anchors.fill: parent
            anchors.leftMargin: tema.hueco * 0.8
            anchors.rightMargin: tema.hueco * 0.8
            verticalAlignment: TextInput.AlignVCenter
            clip: true
            color: tema.texto
            selectionColor: tema.seleccion
            selectedTextColor: tema.fondo
            font.pixelSize: tema.fuente

            // Las sugerencias se piden al núcleo, no se filtran aquí: la lista
            // completa de etiquetas de una biblioteca grande no tiene por qué
            // caber en la ventana, y quien sabe ordenarlas por uso es el índice.
            onTextEdited: {
                // Nada preseleccionado a propósito: si la primera sugerencia
                // quedara marcada, escribir una etiqueta nueva que empieza
                // como otra que ya existe —«rótul» con «rótulo» dentro— sería
                // imposible, porque Intro pondría la de la lista. Las flechas
                // sí eligen; el teclado a secas escribe lo que escribe.
                sugeridas.elegida = -1
                nucleo.pedirEtiquetas(campo.text.trim())
            }
            onActiveFocusChanged: if (activeFocus) nucleo.pedirEtiquetas(campo.text.trim())

            onAccepted: raiz.aceptar()

            Keys.onDownPressed: sugeridas.mover(1)
            Keys.onUpPressed: sugeridas.mover(-1)
            Keys.onEscapePressed: { campo.text = ""; campo.focus = false }
            Keys.onTabPressed: function (evento) {
                // Tab completa con la primera sugerencia. Es lo que hace
                // cualquier consola y lo que la mano espera aquí.
                if (sugeridas.visible && sugeridas.model.length > 0) {
                    campo.text = sugeridas.model[Math.max(0, sugeridas.elegida)].nombre
                    campo.cursorPosition = campo.text.length
                    evento.accepted = true
                } else {
                    evento.accepted = false
                }
            }
        }

        Text {
            anchors.fill: campo
            anchors.leftMargin: campo.anchors.leftMargin
            verticalAlignment: Text.AlignVCenter
            visible: campo.text.length === 0 && !campo.activeFocus
            text: qsTr("añadir etiqueta…")
            color: tema.textoTenue
            font.pixelSize: tema.fuente
        }
    }

    function aceptar() {
        var t = campo.text.trim()
        // Si hay una sugerencia marcada con las flechas, manda esa: quien
        // baja con el teclado espera que Intro elija lo marcado.
        if (sugeridas.visible && sugeridas.elegida >= 0 && sugeridas.model.length > sugeridas.elegida) {
            t = sugeridas.model[sugeridas.elegida].nombre
        }
        if (t.length === 0) return
        // Varias de una vez separadas por comas: escribir «nocturno, asfalto»
        // y pulsar Intro es lo natural.
        var partes = t.split(",")
        for (var i = 0; i < partes.length; ++i) {
            var p = partes[i].trim()
            if (p.length > 0) raiz.anadir(p)
        }
        campo.text = ""
        sugeridas.elegida = -1
    }

    // El desplegable de sugerencias. Va dentro de la columna y no flotando por
    // encima porque el panel es estrecho: un menú flotante aquí taparía justo
    // las etiquetas que se están mirando.
    Column {
        id: sugeridas
        width: raiz.width
        spacing: 1
        property int elegida: -1
        property var model: nucleo.etiquetas
        visible: campo.activeFocus && model.length > 0

        function mover(d) {
            if (!visible) return
            elegida = Math.max(0, Math.min(model.length - 1, elegida + d))
        }

        Repeater {
            model: sugeridas.visible ? sugeridas.model.slice(0, 6) : []
            delegate: Rectangle {
                id: sug
                required property var modelData
                required property int index
                width: sugeridas.width
                height: Math.round(tema.fuente * 1.8)
                radius: tema.radio
                color: index === sugeridas.elegida || sugRaton.containsMouse
                       ? tema.borde : "transparent"

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: tema.hueco * 0.7
                    text: sug.modelData.nombre
                    color: tema.texto
                    font.pixelSize: tema.fuente * 0.95
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: tema.hueco * 0.7
                    text: sug.modelData.n
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.85
                }

                MouseArea {
                    id: sugRaton
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        raiz.anadir(sug.modelData.nombre)
                        campo.text = ""
                        sugeridas.elegida = -1
                    }
                }
            }
        }
    }
}
