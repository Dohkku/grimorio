// Los ejes de la vista, como el de Blender: arriba a la derecha del visor 3D,
// giran con la pieza y dicen desde dónde se la mira.
//
//   clic en una bola     mirar desde ese eje (X, Y o Z, o sus contrarios)
//   arrastrar encima     girar, igual que sobre la pieza
//
// Los ejes son los del mundo, no los de la pieza: enderezar un modelo lo gira
// a él, y arriba sigue siendo Y. Rojo, verde y azul salen del tema.
import QtQuick

Item {
    id: ejes
    property real giro: 0
    property real inclinacion: 0
    /// Pide mirar con ese giro e inclinación.
    signal elegido(real giro, real inclinacion)
    /// Arrastrar sobre los ejes gira la vista, en píxeles.
    signal arrastrado(real dx, real dy)

    width: Math.round(tema.fuente * 7)
    height: width

    readonly property real radio: width / 2 - bola / 2 - 1
    readonly property real bola: Math.round(tema.fuente * 1.45)

    /// Un eje del mundo en pantalla: la misma rotación que el visor (primero
    /// Y por el giro, luego X por la inclinación). `z` hacia quien mira.
    function proyectar(x, y, z) {
        const g = giro * Math.PI / 180, i = inclinacion * Math.PI / 180
        const x1 = x * Math.cos(g) + z * Math.sin(g)
        const z1 = -x * Math.sin(g) + z * Math.cos(g)
        const y2 = y * Math.cos(i) - z1 * Math.sin(i)
        const z2 = y * Math.sin(i) + z1 * Math.cos(i)
        return { x: width / 2 + x1 * radio, y: height / 2 - y2 * radio, z: z2 }
    }

    readonly property var lista: [
        { nombre: "X", v: [1, 0, 0], color: tema.ejeX, giro: -90, incl: 0, lleno: true },
        { nombre: "Y", v: [0, 1, 0], color: tema.ejeY, giro: 0, incl: 90, lleno: true },
        { nombre: "Z", v: [0, 0, 1], color: tema.ejeZ, giro: 0, incl: 0, lleno: true },
        { nombre: "-X", v: [-1, 0, 0], color: tema.ejeX, giro: 90, incl: 0, lleno: false },
        { nombre: "-Y", v: [0, -1, 0], color: tema.ejeY, giro: 0, incl: -90, lleno: false },
        { nombre: "-Z", v: [0, 0, -1], color: tema.ejeZ, giro: 180, incl: 0, lleno: false }
    ]

    // El círculo de fondo solo al pasar por encima, como en Blender: quieto
    // no tapa la pieza.
    Rectangle {
        anchors.fill: parent
        radius: width / 2
        color: tema.texto
        opacity: zona.containsMouse || zona.pressed ? tema.realce * 1.5 : 0
        Behavior on opacity { NumberAnimation { duration: 120 } }
    }

    MouseArea {
        id: zona
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        property point antes
        onPressed: function (e) { antes = Qt.point(e.x, e.y) }
        onPositionChanged: function (e) {
            if (!pressed) return
            ejes.arrastrado(e.x - antes.x, e.y - antes.y)
            antes = Qt.point(e.x, e.y)
        }
    }

    Repeater {
        model: ejes.lista
        delegate: Item {
            id: eje
            required property var modelData
            readonly property var p: ejes.proyectar(modelData.v[0], modelData.v[1], modelData.v[2])
            anchors.fill: parent
            // Lo de delante, encima: z sale de la profundidad del extremo.
            z: p.z

            // La raya, solo hacia los positivos.
            Rectangle {
                visible: eje.modelData.lleno
                x: ejes.width / 2
                y: ejes.height / 2 - height / 2
                width: Math.hypot(eje.p.x - ejes.width / 2, eje.p.y - ejes.height / 2)
                height: 2
                radius: 1
                color: eje.modelData.color
                transformOrigin: Item.Left
                rotation: Math.atan2(eje.p.y - ejes.height / 2, eje.p.x - ejes.width / 2) * 180 / Math.PI
                opacity: 0.85
            }

            Rectangle {
                id: punta
                width: eje.modelData.lleno ? ejes.bola : Math.round(ejes.bola * 0.8)
                height: width
                radius: width / 2
                x: eje.p.x - width / 2
                y: eje.p.y - height / 2
                color: eje.modelData.lleno ? eje.modelData.color : "transparent"
                border.color: eje.modelData.color
                border.width: eje.modelData.lleno ? 0 : 1.5
                // Los de detrás, un poco apagados: ayudan a leer la
                // profundidad sin tener que pensarla.
                opacity: eje.p.z < -0.05 ? 0.6 : 1
                scale: sobre.containsMouse ? 1.15 : 1
                Behavior on scale { NumberAnimation { duration: 80 } }

                // El de detrás se rellena un poco para que se vea a qué
                // eje pertenece.
                Rectangle {
                    visible: !eje.modelData.lleno
                    anchors.fill: parent
                    anchors.margins: 1.5
                    radius: width / 2
                    color: eje.modelData.color
                    opacity: sobre.containsMouse ? 0.7 : 0.25
                }

                Text {
                    anchors.centerIn: parent
                    visible: eje.modelData.lleno || sobre.containsMouse
                    text: eje.modelData.lleno ? eje.modelData.nombre : eje.modelData.nombre.substring(1)
                    color: tema.fondo
                    font.pixelSize: tema.fuente * 0.8
                    font.weight: Font.Bold
                }

                MouseArea {
                    id: sobre
                    anchors.fill: parent
                    anchors.margins: -2
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: ejes.elegido(eje.modelData.giro, eje.modelData.incl)
                }
            }
        }
    }
}
