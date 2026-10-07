// Un icono de `iconos.js`, pintado con un color del tema.
//
// Trazos y no imágenes: el color sale del tema y cambia con él, y a cualquier
// tamaño se ve nítido. El suavizado va en una capa del tamaño del icono —no
// del dibujo de 24×24— para que al escalar no salga borroso.
import QtQuick
import QtQuick.Shapes
import "iconos.js" as Iconos

Item {
    id: icono
    property string nombre: ""
    property color color: tema.texto
    property real grosor: 1.7

    implicitWidth: Math.round(tema.fuente * 1.35)
    implicitHeight: implicitWidth

    layer.enabled: true
    layer.samples: 4
    layer.smooth: true

    Shape {
        width: 24
        height: 24
        transformOrigin: Item.TopLeft
        scale: Math.min(icono.width, icono.height) / 24
        x: (icono.width - 24 * scale) / 2
        y: (icono.height - 24 * scale) / 2

        ShapePath {
            strokeColor: icono.color
            strokeWidth: icono.grosor
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: Iconos.trazo(icono.nombre) }
        }
        ShapePath {
            strokeColor: "transparent"
            fillColor: icono.color
            PathSvg { path: Iconos.relleno(icono.nombre) }
        }
    }
}
