// Los títulos de las columnas de la vista en lista.
//
// Las columnas son las de `ventana.columnasLista`, igual que las de cada fila
// (ver `Celda.qml`): la cabecera no sabe nada que las filas no sepan, así que
// no se pueden desalinear. Pinchar un título ordena por esa columna; otra vez,
// vuelve al orden de cada uno.
import QtQuick
import "consulta.js" as Consulta

Rectangle {
    id: cabecera
    implicitHeight: Math.round(tema.fuente * 2.3)
    color: tema.fondo
    clip: true

    /// El orden que pone cada columna, en el lenguaje del buscador.
    readonly property var ordenDe: ({ "nombre": "nombre", "peso": "peso",
                                      "estrellas": "estrellas", "fecha": "recientes" })
    readonly property string ordenPuesto: Consulta.leer(ventana.textoBusqueda, "orden")

    Rectangle {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: 1
        color: tema.borde
    }

    component Titulo: Text {
        id: titulo
        property string clave: ""
        readonly property string orden: cabecera.ordenDe[clave] || ""
        readonly property bool puesto: orden.length > 0 && cabecera.ordenPuesto === orden
        anchors.verticalCenter: parent ? parent.verticalCenter : undefined
        elide: Text.ElideRight
        color: puesto ? tema.texto : tema.textoTenue
        font.pixelSize: tema.fuente * 0.85
        font.weight: puesto ? Font.DemiBold : Font.Medium

        MouseArea {
            anchors.fill: parent
            enabled: titulo.orden.length > 0
            cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: ventana.ponerFiltro("orden", titulo.puesto ? "" : titulo.orden)
        }
    }

    // El sitio de la miniatura, y el nombre a continuación hasta las columnas.
    Titulo {
        // Las mismas cuentas que la fila (Celda.qml): sangría, miniatura y aire.
        x: tema.margen + Math.round(tema.hueco * 0.4) + Math.round(ventana.ladoMiniaturaLista * 1.3)
           + Math.round(tema.hueco * 1.2)
        width: columnas.x - x - tema.hueco
        clave: "nombre"
        text: qsTr("nombre")
    }

    Row {
        id: columnas
        anchors.right: parent.right
        anchors.rightMargin: tema.margen + tema.hueco
        height: parent.height

        Repeater {
            model: ventana.columnasLista
            delegate: Item {
                id: hueco
                required property var modelData
                width: modelData.ancho
                height: columnas.height
                Titulo {
                    width: parent.width - tema.hueco
                    clave: hueco.modelData.clave
                    text: hueco.modelData.titulo
                }
            }
        }
    }
}
