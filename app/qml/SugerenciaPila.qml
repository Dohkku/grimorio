// «Esto ya lo tenías en otro peso: ¿lo apilo?»
//
// Sale después de importar algo que es casi la misma imagen que una que ya
// estaba. Es una tarjeta en una esquina y no una pregunta en medio: importar
// no se para a esperar, y si no se contesta no pasa nada —lo importado se
// queda suelto, como siempre—.
//
// Al apilar, la principal es la que más pesa de las dos: es la que más se
// parece al original, y la que uno querría ver en la malla.
import QtQuick
import "textos.js" as Textos

Rectangle {
    id: raiz
    property var pares: []
    readonly property bool abierta: pares.length > 0

    visible: abierta
    width: Math.min(420, parent ? parent.width * 0.4 : 420)
    height: contenido.height + tema.margen * 1.4
    radius: tema.radio
    color: tema.panel
    border.color: tema.borde
    border.width: 1

    function mostrar(lista) {
        pares = lista
    }

    function apilar() {
        let grupos = []
        for (let i = 0; i < pares.length; i++) {
            const p = pares[i]
            const nuevoManda = p.peso_nuevo > p.peso_existente
            grupos.push({
                principal: nuevoManda ? p.nuevo : p.existente,
                ids: [nuevoManda ? p.existente : p.nuevo]
            })
        }
        nucleo.apilarGrupos(grupos)
        pares = []
    }

    // Se traga los clics: debajo está la malla.
    MouseArea { anchors.fill: parent }

    Column {
        id: contenido
        anchors.centerIn: parent
        width: parent.width - tema.margen * 1.4
        spacing: tema.hueco * 0.7

        Row {
            spacing: tema.hueco * 0.5
            Icono {
                anchors.verticalCenter: parent.verticalCenter
                nombre: "pila"
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: raiz.pares.length === 1
                      ? qsTr("Ya la tenías en otro peso")
                      : qsTr("%1 ya las tenías en otro peso").arg(raiz.pares.length)
                color: tema.texto
                font.pixelSize: tema.fuente
                font.weight: Font.DemiBold
            }
        }

        Text {
            width: parent.width
            wrapMode: Text.Wrap
            visible: raiz.pares.length === 1
            readonly property var p: raiz.pares.length === 1 ? raiz.pares[0] : null
            text: p ? qsTr("«%1» (%2) se parece a «%3» (%4). ¿Apilarlas como versiones de la misma imagen?")
                      .arg(p.nombre_nuevo).arg(Textos.peso(p.peso_nuevo))
                      .arg(p.nombre_existente).arg(Textos.peso(p.peso_existente))
                    : ""
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.9
        }
        Text {
            width: parent.width
            wrapMode: Text.Wrap
            visible: raiz.pares.length > 1
            text: qsTr("Lo que acabas de importar se parece a imágenes que ya estaban. Al apilar, la más pesada de cada pareja queda como principal.")
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.9
        }

        Row {
            anchors.right: parent.right
            spacing: tema.hueco * 0.6
            Boton {
                texto: qsTr("dejarlas sueltas")
                onPulsado: raiz.pares = []
            }
            Boton {
                texto: raiz.pares.length > 1 ? qsTr("apilar todas") : qsTr("apilar")
                activo: true
                onPulsado: raiz.apilar()
            }
        }
    }
}
