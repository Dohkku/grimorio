// Renombrar varios de una vez con un patrón.
//
//   {nombre}  el que tenía   ·   {n}  contador   ·   {n:3}  con ceros   ·   {fecha}
//
// Una tarjeta en medio, como la de confirmar, con la vista previa de los
// primeros: un patrón se equivoca fácil y lo que se ve antes de aceptar es lo
// que evita tener que deshacer. Deshacer, igual, se puede.
import QtQuick

Item {
    id: raiz
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    property var ids: []
    property var nombres: []

    function abrir() {
        ids = modelo.idsElegidos()
        const idx = modelo.indicesElegidos()
        let ns = []
        for (let i = 0; i < Math.min(idx.length, 4); i++) ns.push(modelo.nombreDe(idx[i]))
        nombres = ns
        patron.text = "{nombre}"
        inicio.text = "1"
        abierto = true
        patron.forceActiveFocus()
        patron.selectAll()
    }

    // Soltar el foco al cerrar: un campo escondido que se lo queda hace creer
    // a la ventana que se sigue escribiendo, y apaga los atajos (Ctrl+E, F2…).
    function cerrar() {
        patron.focus = false
        inicio.focus = false
        abierto = false
    }

    function aceptar() {
        if (patron.text.trim().length === 0) return
        nucleo.renombrarEnLote(ids, patron.text, Math.max(0, parseInt(inicio.text) || 0))
        cerrar()
    }

    /// El mismo patrón que aplica el núcleo (grimorio_core::patron), para
    /// la vista previa. La fecha no la sabe la vista: se enseña la forma.
    function aplicar(p, nombre, n) {
        let out = p.replace(/\{nombre\}/g, nombre)
                   .replace(/\{fecha\}/g, "aaaa-mm-dd")
                   .replace(/\{n:(\d{1,2})\}/g, function (m, ancho) {
                       let s = String(n)
                       while (s.length < Number(ancho)) s = "0" + s
                       return s
                   })
                   .replace(/\{n\}/g, String(n))
        out = out.trim().replace(/\//g, "-")
        return out.length > 0 ? out : nombre
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onClicked: raiz.cerrar()
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
                text: qsTr("renombrar %1").arg(raiz.ids.length)
                color: tema.texto
                font.pixelSize: tema.fuente * 1.15
                font.weight: Font.DemiBold
            }

            Row {
                width: parent.width
                spacing: tema.hueco

                Rectangle {
                    width: parent.width - numero.width - tema.hueco
                    height: Math.round(tema.fuente * 2.2)
                    radius: tema.radio
                    color: tema.fondo
                    border.color: patron.activeFocus ? tema.seleccion : tema.borde
                    border.width: 1
                    TextInput {
                        id: patron
                        anchors.fill: parent
                        anchors.leftMargin: tema.hueco * 0.6
                        anchors.rightMargin: tema.hueco * 0.6
                        verticalAlignment: TextInput.AlignVCenter
                        clip: true
                        color: tema.texto
                        selectionColor: tema.seleccion
                        selectedTextColor: tema.fondo
                        font.pixelSize: tema.fuente
                        onAccepted: raiz.aceptar()
                        Keys.onEscapePressed: raiz.cerrar()
                        KeyNavigation.tab: inicio
                    }
                }

                // Desde qué número cuenta {n}.
                Rectangle {
                    id: numero
                    width: tema.fuente * 5
                    height: Math.round(tema.fuente * 2.2)
                    radius: tema.radio
                    color: tema.fondo
                    border.color: inicio.activeFocus ? tema.seleccion : tema.borde
                    border.width: 1
                    TextInput {
                        id: inicio
                        anchors.fill: parent
                        anchors.leftMargin: tema.hueco * 0.6
                        verticalAlignment: TextInput.AlignVCenter
                        color: tema.texto
                        font.pixelSize: tema.fuente
                        inputMethodHints: Qt.ImhDigitsOnly
                        validator: IntValidator { bottom: 0; top: 999999 }
                        onAccepted: raiz.aceptar()
                        Keys.onEscapePressed: raiz.cerrar()
                        KeyNavigation.tab: patron
                    }
                }
            }

            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: qsTr("{nombre} el que tenía · {n} contador (desde el número de la derecha) · {n:3} con ceros · {fecha} el día que entró")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.8
            }

            // La vista previa: los primeros, en el orden en que se numeran.
            Column {
                width: parent.width
                spacing: tema.hueco * 0.3
                Repeater {
                    model: raiz.nombres
                    delegate: Row {
                        required property string modelData
                        required property int index
                        spacing: tema.hueco * 0.6
                        Text {
                            text: modelData
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.9
                            width: contenido.width * 0.42
                            elide: Text.ElideMiddle
                        }
                        Text {
                            text: "→"
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.9
                        }
                        Text {
                            text: raiz.aplicar(patron.text, modelData, (parseInt(inicio.text) || 0) + index)
                            color: tema.texto
                            font.pixelSize: tema.fuente * 0.9
                            width: contenido.width * 0.48
                            elide: Text.ElideMiddle
                        }
                    }
                }
                Text {
                    visible: raiz.ids.length > raiz.nombres.length
                    text: qsTr("… y %1 más").arg(raiz.ids.length - raiz.nombres.length)
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.85
                }
            }

            Row {
                anchors.right: parent.right
                spacing: tema.hueco * 0.6
                Boton {
                    centrado: false
                    texto: qsTr("cancelar")
                    onPulsado: raiz.cerrar()
                }
                Boton {
                    centrado: false
                    principal: true
                    texto: qsTr("renombrar")
                    onPulsado: raiz.aceptar()
                }
            }
        }
    }
}
