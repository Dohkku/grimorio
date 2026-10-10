// Las carpetas vinculadas, como nodos: de dónde lee esta biblioteca.
//
// A la izquierda, la biblioteca y la carpeta del disco donde vive. A la
// derecha, cada carpeta del disco vinculada, con una línea hasta ella. Por la
// línea corren unos trazos de la carpeta hacia la biblioteca, que es hacia
// donde va lo que aparece allí; cuando de verdad entra algo, la línea late.
// Si la carpeta no está (un disco sin conectar, una carpeta movida), la línea
// queda punteada y quieta, y el nodo lo dice: el vínculo no se pierde, vuelve
// a funcionar cuando la carpeta vuelve.
//
// Desde aquí se conecta una carpeta nueva (el último nodo, «+») y se
// desconecta una que ya está (la × de su nodo). Desconectar no borra nada: lo
// que ya entró se queda en la biblioteca.
//
// En la interfaz es «vincular»; por dentro sigue siendo `vigiladas` (ver
// vigilancia.h).
import QtQuick
import QtQuick.Shapes
import "textos.js" as Textos

Item {
    id: panel
    anchors.fill: parent
    visible: abierto

    property bool abierto: false

    function abrir() {
        recuento++
        abierto = true
    }
    function cerrar() {
        abierto = false
    }

    /// Sube cada vez que puede haber cambiado qué carpetas están: al abrir y
    /// cuando cambia la lista. `vigilancia.existe` no avisa sola.
    property int recuento: 0
    Connections {
        target: nucleo
        function onVigiladasCambiaron() { panel.recuento++ }
    }

    readonly property real f: tema.fuente

    function ultimaParte(ruta) {
        const r = ruta.replace(/[\\/]+$/, "")
        const trozos = r.split(/[\\/]/)
        return trozos[trozos.length - 1] || r
    }
    function abrirEnElDisco(ruta) {
        const r = ruta.replace(/\\/g, "/")
        Qt.openUrlExternally(r.startsWith("/") ? "file://" + r : "file:///" + r)
    }

    // Una ruta que se puede pulsar para abrirla en el explorador de archivos.
    component Ruta: Text {
        id: ruta
        property string valor: ""
        text: valor
        elide: Text.ElideMiddle
        color: sobreRuta.containsMouse ? tema.texto : tema.textoTenue
        font.pixelSize: panel.f * 0.8
        font.underline: sobreRuta.containsMouse
        MouseArea {
            id: sobreRuta
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: panel.abrirEnElDisco(ruta.valor)
        }
    }

    // Una línea de una carpeta a la biblioteca. `x1, y1` es la biblioteca.
    component Conexion: Shape {
        id: con
        property real x1: 0
        property real y1: 0
        property real x2: 0
        property real y2: 0
        /// La carpeta está y lo nuevo puede entrar: corre y tiene color.
        property bool viva: true
        /// Solo la del «+»: tenue y quieta.
        property bool fantasma: false
        /// La última vez que entró algo por aquí; cuando cambia, late.
        property real desde: 0
        onDesdeChanged: latido.restart()

        antialiasing: true
        layer.enabled: true
        layer.samples: 4

        readonly property real medio: (x2 - x1) * 0.5

        ShapePath {
            fillColor: "transparent"
            strokeColor: con.viva && !con.fantasma ? tema.borde : tema.textoTenue
            strokeWidth: Math.max(1, panel.f * 0.14)
            strokeStyle: con.viva && !con.fantasma ? ShapePath.SolidLine : ShapePath.DashLine
            dashPattern: [3, 3]
            capStyle: ShapePath.RoundCap
            startX: con.x1; startY: con.y1
            PathCubic {
                x: con.x2; y: con.y2
                control1X: con.x1 + con.medio; control1Y: con.y1
                control2X: con.x2 - con.medio; control2Y: con.y2
            }
        }
        // Lo que corre por encima: trazos cortos de la carpeta a la biblioteca.
        ShapePath {
            id: flujo
            fillColor: "transparent"
            strokeColor: con.viva && !con.fantasma ? tema.seleccion : "transparent"
            strokeWidth: Math.max(1.5, panel.f * 0.2) * (1 + con.extra)
            strokeStyle: ShapePath.DashLine
            dashPattern: [1.2, 3.5]
            capStyle: ShapePath.RoundCap
            startX: con.x1; startY: con.y1
            PathCubic {
                x: con.x2; y: con.y2
                control1X: con.x1 + con.medio; control1Y: con.y1
                control2X: con.x2 - con.medio; control2Y: con.y2
            }
        }
        // Subir el desplazamiento del trazo lo mueve hacia el principio de la
        // línea, que es la biblioteca.
        NumberAnimation {
            target: flujo
            property: "dashOffset"
            from: 0
            to: 4.7
            duration: 700
            loops: Animation.Infinite
            running: panel.abierto && con.viva && !con.fantasma
        }
        property real extra: 0
        SequentialAnimation {
            id: latido
            NumberAnimation { target: con; property: "extra"; to: 1.2; duration: 140; easing.type: Easing.OutQuad }
            NumberAnimation { target: con; property: "extra"; to: 0; duration: 900; easing.type: Easing.InOutQuad }
        }
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: panel.cerrar()
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        opacity: 0.75
    }

    Rectangle {
        id: tarjeta
        anchors.centerIn: parent
        width: Math.min(panel.f * 74, panel.width * 0.92)
        height: Math.min(panel.f * 48, panel.height * 0.9)
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        clip: true

        MouseArea { anchors.fill: parent }

        Column {
            id: cabecera
            anchors { top: parent.top; left: parent.left; right: parent.right }
            anchors.margins: tema.margen
            anchors.rightMargin: tema.margen * 3
            spacing: tema.hueco * 0.4

            Row {
                spacing: tema.hueco * 0.6
                Icono {
                    anchors.verticalCenter: parent.verticalCenter
                    nombre: "nodos"
                    color: tema.seleccion
                    width: Math.round(panel.f * 1.4)
                    height: width
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Carpetas vinculadas")
                    color: tema.texto
                    font.pixelSize: panel.f * 1.3
                    font.weight: Font.DemiBold
                }
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: qsTr("Lo que aparece en una carpeta vinculada entra solo en la biblioteca. Desconectarla no borra nada: lo que ya entró se queda.")
                color: tema.textoTenue
                font.pixelSize: panel.f * 0.9
            }
        }

        BotonIcono {
            centrado: false
            anchors { top: parent.top; right: parent.right; margins: tema.hueco }
            icono: "cerrar"
            pista: qsTr("cerrar (Esc)")
            onPulsado: panel.cerrar()
        }

        Flickable {
            id: lienzo
            anchors {
                top: cabecera.bottom; bottom: parent.bottom
                left: parent.left; right: parent.right
                topMargin: tema.margen; bottomMargin: tema.margen
                leftMargin: tema.margen * 1.5; rightMargin: tema.margen * 1.5
            }
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            contentWidth: width
            contentHeight: Math.max(height, alto)

            readonly property var lista: nucleo.vigiladas
            // Una de más: el nodo de conectar.
            readonly property int cuantos: lista.length + 1
            readonly property real anchoNodo: Math.min(panel.f * 26, width * 0.48)
            readonly property real altoNodo: Math.round(panel.f * 5.6)
            readonly property real hueco: Math.round(panel.f * 1.1)
            readonly property real alto: cuantos * altoNodo + (cuantos - 1) * hueco
            readonly property real arriba: (contentHeight - alto) / 2
            readonly property real xCarpetas: width - anchoNodo
            function yDe(i) { return arriba + i * (altoNodo + hueco) }

            // Las líneas, debajo de los nodos.
            Repeater {
                model: lienzo.lista
                delegate: Conexion {
                    required property var modelData
                    required property int index
                    width: lienzo.contentWidth
                    height: lienzo.contentHeight
                    x1: nodoBiblio.x + nodoBiblio.width
                    y1: nodoBiblio.y + nodoBiblio.height / 2
                    x2: lienzo.xCarpetas
                    y2: lienzo.yDe(index) + lienzo.altoNodo / 2
                    viva: panel.recuento >= 0 && vigilancia.existe(modelData.ruta)
                    desde: modelData.desde_ms
                }
            }
            Conexion {
                width: lienzo.contentWidth
                height: lienzo.contentHeight
                fantasma: true
                x1: nodoBiblio.x + nodoBiblio.width
                y1: nodoBiblio.y + nodoBiblio.height / 2
                x2: lienzo.xCarpetas
                y2: lienzo.yDe(lienzo.lista.length) + lienzo.altoNodo / 2
            }

            // ---------------------------------------------- la biblioteca
            Rectangle {
                id: nodoBiblio
                x: 0
                y: (lienzo.contentHeight - height) / 2
                width: Math.min(panel.f * 19, lienzo.width * 0.36)
                height: columnaBiblio.height + tema.hueco * 2.4
                radius: tema.radio * 1.5
                color: tema.fondo
                border.color: tema.seleccion
                border.width: Math.max(1, Math.round(panel.f * 0.14))

                Column {
                    id: columnaBiblio
                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter }
                    anchors.margins: tema.hueco * 1.2
                    spacing: tema.hueco * 0.3

                    Row {
                        spacing: tema.hueco * 0.5
                        width: parent.width
                        Icono {
                            id: iconoBiblio
                            anchors.verticalCenter: parent.verticalCenter
                            nombre: "biblioteca"
                            color: tema.seleccion
                            width: Math.round(panel.f * 1.3)
                            height: width
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - iconoBiblio.width - parent.spacing
                            elide: Text.ElideRight
                            text: nucleo.nombre
                            color: tema.texto
                            font.pixelSize: panel.f * 1.15
                            font.weight: Font.DemiBold
                        }
                    }
                    Text {
                        text: qsTr("%1 elementos").arg(Textos.numero(nucleo.total))
                        color: tema.textoTenue
                        font.pixelSize: panel.f * 0.85
                    }
                    Text {
                        topPadding: tema.hueco * 0.4
                        text: qsTr("vive en")
                        color: tema.textoTenue
                        font.pixelSize: panel.f * 0.75
                        font.capitalization: Font.AllUppercase
                        font.letterSpacing: panel.f * 0.06
                    }
                    Ruta {
                        width: parent.width
                        valor: nucleo.raiz
                    }
                }

                // El enchufe de las líneas.
                Rectangle {
                    anchors { horizontalCenter: parent.right; verticalCenter: parent.verticalCenter }
                    width: Math.round(panel.f * 0.7)
                    height: width
                    radius: width / 2
                    color: tema.seleccion
                }
            }

            // ------------------------------------------------ las carpetas
            Repeater {
                model: lienzo.lista
                delegate: Rectangle {
                    id: nodo
                    required property var modelData
                    required property int index
                    readonly property bool viva: panel.recuento >= 0 && vigilancia.existe(modelData.ruta)
                    x: lienzo.xCarpetas
                    y: lienzo.yDe(index)
                    width: lienzo.anchoNodo
                    height: lienzo.altoNodo
                    radius: tema.radio * 1.5
                    color: tema.fondo
                    border.color: tema.borde
                    border.width: 1
                    // Una que no se encuentra no es la que importa ahora: más
                    // apagada que las demás, y no más brillante.
                    opacity: viva ? 1 : 0.7

                    Rectangle {
                        anchors { horizontalCenter: parent.left; verticalCenter: parent.verticalCenter }
                        width: Math.round(panel.f * 0.7)
                        height: width
                        radius: width / 2
                        color: nodo.viva ? tema.seleccion : tema.textoTenue
                    }

                    Icono {
                        id: iconoCarpeta
                        anchors { left: parent.left; leftMargin: tema.hueco * 1.3; verticalCenter: parent.verticalCenter }
                        nombre: "carpeta"
                        color: nodo.viva ? tema.seleccion : tema.textoTenue
                        width: Math.round(panel.f * 1.4)
                        height: width
                    }
                    Column {
                        anchors {
                            left: iconoCarpeta.right; leftMargin: tema.hueco * 0.8
                            right: desconectar.left; rightMargin: tema.hueco * 0.4
                            verticalCenter: parent.verticalCenter
                        }
                        spacing: tema.hueco * 0.15
                        Text {
                            width: parent.width
                            elide: Text.ElideRight
                            text: panel.ultimaParte(nodo.modelData.ruta)
                            color: nodo.viva ? tema.texto : tema.textoTenue
                            font.pixelSize: panel.f
                            font.weight: Font.DemiBold
                        }
                        Ruta {
                            width: parent.width
                            valor: nodo.modelData.ruta
                        }
                        Text {
                            width: parent.width
                            elide: Text.ElideRight
                            text: nodo.viva
                                  ? qsTr("entra en «%1»").arg(nodo.modelData.carpeta
                                                              ? ventana.nombreDeCarpeta(nodo.modelData.carpeta)
                                                              : qsTr("Todo"))
                                  : qsTr("no se encuentra: ¿un disco sin conectar?")
                            color: nodo.viva ? tema.textoTenue : tema.seleccion
                            font.pixelSize: panel.f * 0.8
                        }
                    }
                    BotonIcono {
                        id: desconectar
                        anchors { right: parent.right; rightMargin: tema.hueco * 0.6; verticalCenter: parent.verticalCenter }
                        icono: "cerrar"
                        pista: qsTr("desconectar (lo que ya entró se queda)")
                        onPulsado: nucleo.dejarDeVigilar(nodo.modelData.id)
                    }
                }
            }

            // ----------------------------------------- conectar una nueva
            Rectangle {
                x: lienzo.xCarpetas
                y: lienzo.yDe(lienzo.lista.length)
                width: lienzo.anchoNodo
                height: lienzo.altoNodo
                radius: tema.radio * 1.5
                color: sobreConectar.containsMouse ? tema.borde : "transparent"
                border.color: sobreConectar.containsMouse ? tema.seleccion : tema.borde
                border.width: 1

                Rectangle {
                    anchors { horizontalCenter: parent.left; verticalCenter: parent.verticalCenter }
                    width: Math.round(panel.f * 0.7)
                    height: width
                    radius: width / 2
                    color: tema.panel
                    border.color: tema.textoTenue
                    border.width: 1
                }
                Row {
                    anchors.centerIn: parent
                    spacing: tema.hueco * 0.6
                    Icono {
                        anchors.verticalCenter: parent.verticalCenter
                        nombre: "mas"
                        color: sobreConectar.containsMouse ? tema.seleccion : tema.texto
                        width: Math.round(panel.f * 1.2)
                        height: width
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("conectar una carpeta del disco…")
                        color: sobreConectar.containsMouse ? tema.seleccion : tema.texto
                        font.pixelSize: panel.f
                    }
                }
                MouseArea {
                    id: sobreConectar
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: vigilancia.vigilar("")
                }
            }
        }
    }
}
