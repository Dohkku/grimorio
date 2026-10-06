// Un filtro de la fila de filtros: «tipo ▾», o «tipo: vídeos ×» si está puesto.
//
// Lee y escribe la misma línea del buscador que se teclea a mano (consulta.js):
// no hay estado propio, así que el chip y el texto no pueden discrepar.
import QtQuick
import "consulta.js" as Consulta

Rectangle {
    id: chip
    property string icono: ""
    property string titulo: ""
    /// El campo del lenguaje de búsqueda: `tipo`, `fecha`, `color`…
    property string campo: ""
    /// `[{texto, valor, icono?}]`. El de valor "" es «sin filtro».
    property var opciones: []
    /// "lista" o "color".
    property string modo: "lista"
    /// Un chip que no abre nada: se enciende y se apaga (repetidos).
    property string interruptor: ""

    readonly property string valor: Consulta.leer(ventana.textoBusqueda, campo)
    readonly property bool puesto: valor.length > 0
    readonly property string valorTexto: {
        for (let i = 0; i < opciones.length; i++)
            if (opciones[i].valor === valor) return opciones[i].texto
        return valor
    }

    height: Math.round(tema.fuente * 2)
    width: fila.implicitWidth + tema.hueco * 1.6
    radius: height / 2
    color: puesto ? "transparent" : (sobre.containsMouse ? tema.borde : "transparent")
    border.color: puesto ? tema.seleccion : (sobre.containsMouse ? tema.textoTenue : tema.borde)
    border.width: 1
    Behavior on border.color { ColorAnimation { duration: 90 } }

    // El tinte de puesto, como los botones activos.
    Rectangle {
        anchors.fill: parent
        radius: parent.radius
        color: tema.seleccion
        opacity: chip.puesto ? 0.14 : 0
    }

    Row {
        id: fila
        anchors.centerIn: parent
        spacing: tema.hueco * 0.4
        Icono {
            anchors.verticalCenter: parent.verticalCenter
            visible: chip.modo !== "color" || !chip.puesto
            nombre: chip.icono
            color: chip.puesto ? tema.seleccion : tema.textoTenue
            width: Math.round(tema.fuente * 1.1)
            height: width
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            visible: chip.modo === "color" && chip.puesto
            width: Math.round(tema.fuente * 0.95)
            height: width
            radius: width / 2
            color: chip.modo === "color" && chip.puesto ? chip.valor : "transparent"
            border.color: tema.borde
            border.width: 1
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: chip.puesto && chip.interruptor.length === 0 && chip.modo !== "color"
                  ? chip.titulo + ": " + chip.valorTexto : chip.titulo
            color: chip.puesto ? tema.seleccion : tema.texto
            font.pixelSize: tema.fuente * 0.92
        }
        Icono {
            anchors.verticalCenter: parent.verticalCenter
            visible: !chip.puesto && chip.interruptor.length === 0
            nombre: "abajo"
            color: tema.textoTenue
            width: Math.round(tema.fuente * 0.9)
            height: width
        }
        // Quitar el filtro sin abrir el menú.
        Icono {
            id: quitar
            anchors.verticalCenter: parent.verticalCenter
            visible: chip.puesto
            nombre: "cerrar"
            color: sobreQuitar.containsMouse ? tema.texto : tema.seleccion
            width: Math.round(tema.fuente * 0.9)
            height: width
            MouseArea {
                id: sobreQuitar
                anchors.fill: parent
                anchors.margins: -tema.hueco * 0.3
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: ventana.ponerFiltro(chip.campo, "")
            }
        }
    }

    MouseArea {
        id: sobre
        anchors.fill: parent
        anchors.rightMargin: quitar.visible ? quitar.width + tema.hueco * 0.8 : 0
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            if (chip.interruptor.length > 0) {
                ventana.ponerFiltro(chip.campo, chip.puesto ? "" : chip.interruptor)
                return
            }
            ventana.abrirDesplegable(chip, {
                modo: chip.modo,
                opciones: chip.opciones,
                actual: chip.valor,
                alElegir: function (v) { ventana.ponerFiltro(chip.campo, v) }
            })
        }
    }
}
