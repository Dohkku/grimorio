// La fila en la que se escribe el nombre de una carpeta nueva, en su sitio.
//
// Antes, crear una carpeta abría un cuadro flotante sobre el panel: se escribía
// en el aire y la carpeta aparecía después en otro sitio. Aquí la fila sale
// donde va a quedar —en la raíz, o debajo de su madre con la sangría de una
// hija—, así que lo que se ve mientras se escribe es lo que habrá al acabar.
//
//   Enter    crea y deja la fila abierta para otra
//   Esc      cancela
//   salir    crea si hay algo escrito; si no, cancela
//
// Crear varias seguidas es lo normal al montar un árbol —«referencias»,
// «paletas», «tipografía»—, y por eso Enter no cierra: cierra Esc o pinchar
// fuera.
import QtQuick

Rectangle {
    id: nueva
    /// De quién colgará, o "" para la raíz.
    property string padre: ""
    property int nivel: 0
    /// Lo que se ve en el campo vacío.
    property string pista: padre.length > 0 ? qsTr("nombre de la subcarpeta…")
                                            : qsTr("nombre de la carpeta…")
    /// Si Enter deja la fila abierta para otra. Para carpetas sí —se crean
    /// varias seguidas—; para una búsqueda guardada no tiene sentido.
    property bool seguir: true
    /// Se ha escrito un nombre y se ha dado Intro (o se ha salido con algo
    /// escrito). Quien la usa decide qué se crea con él.
    signal nombrado(string nombre)
    signal acabada()

    height: Math.round(tema.fuente * 2.6)
    color: "transparent"

    function empezar() {
        campo.text = ""
        campo.forceActiveFocus()
    }

    function crear() {
        const nombre = campo.text.trim()
        campo.text = ""
        if (nombre.length > 0) nueva.nombrado(nombre)
        return nombre.length > 0
    }

    Rectangle {
        anchors.fill: parent
        anchors.leftMargin: tema.hueco * 0.5 + nueva.nivel * tema.hueco * 1.4
        anchors.rightMargin: tema.hueco * 0.5
        anchors.topMargin: tema.hueco * 0.3
        anchors.bottomMargin: tema.hueco * 0.3
        radius: tema.radio
        color: tema.fondo
        border.color: tema.seleccion
        border.width: 1

        TextInput {
            id: campo
            anchors.fill: parent
            anchors.leftMargin: tema.hueco * 0.5
            anchors.rightMargin: tema.hueco * 0.5
            verticalAlignment: TextInput.AlignVCenter
            clip: true
            color: tema.texto
            selectionColor: tema.seleccion
            selectedTextColor: tema.fondo
            font.pixelSize: tema.fuente
            // Al acabar se suelta el foco: escondido y con foco, el campo hacía
            // creer a la ventana que se seguía escribiendo y apagaba los atajos.
            onAccepted: if (nueva.crear() && !nueva.seguir) focus = false
            Keys.onEscapePressed: {
                text = ""
                focus = false
            }
            onActiveFocusChanged: {
                if (activeFocus) return
                nueva.crear()
                nueva.acabada()
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: campo.text.length === 0
                text: nueva.pista
                color: tema.textoTenue
                font.pixelSize: tema.fuente
            }
        }
    }
}
