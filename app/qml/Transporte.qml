// Los mandos de vídeo y sonido: la barra y la fila de botones.
//
// No importa QtMultimedia: manda a través de `rep`, el reproductor, que es el
// que sabe de tuberías. Así este archivo se puede revisar con qmllint sin el
// módulo de multimedia, y el reproductor sigue siendo el único sitio donde un
// import que falta puede romper algo.
//
// Todo con rectángulos, texto y `MouseArea`, sin `Controls`: el estilo sale
// entero del tema, como en el resto de la aplicación.
import QtQuick
import "textos.js" as Textos

Rectangle {
    id: transporte
    /// El reproductor (`Reproductor.qml`).
    property var rep: null
    /// Solo parar, tiempo y barra: para el panel de la derecha.
    property bool compacto: false

    readonly property bool hayRep: rep !== null && rep !== undefined
    readonly property real dur: hayRep ? rep.duracion : 0
    readonly property real pos: hayRep ? rep.posicion : 0
    readonly property bool esVideo: hayRep && !rep.soloSonido
    readonly property real fila: Math.round(tema.fuente * 2.2)

    height: barra.height + fila + tema.hueco * 1.5
    color: tema.panel
    opacity: raton.containsMouse || barraRaton.containsMouse || !(hayRep && rep.enMarcha)
             ? 0.96 : 0.55
    Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }

    MouseArea {
        id: raton
        anchors.fill: parent
        hoverEnabled: true
        // Se traga el clic: aquí abajo se maneja, no se pausa.
        acceptedButtons: Qt.LeftButton | Qt.RightButton
    }

    // ---------------------------------------------------------------- la barra
    Item {
        id: barra
        anchors { left: parent.left; right: parent.right; top: parent.top }
        anchors.leftMargin: tema.hueco
        anchors.rightMargin: tema.hueco
        anchors.topMargin: tema.hueco / 2
        height: Math.round(tema.fuente * 1.6)

        readonly property real avance: transporte.dur > 0 ? transporte.pos / transporte.dur : 0
        readonly property real grosor: Math.max(3, Math.round(tema.fuente * 0.28))

        Rectangle {
            id: carril
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width
            height: barraRaton.containsMouse ? barra.grosor * 1.8 : barra.grosor
            radius: height / 2
            color: tema.marcador
            Behavior on height { NumberAnimation { duration: 90 } }
        }
        // El tramo A-B, encima del carril y debajo del avance.
        Rectangle {
            visible: transporte.hayRep && transporte.dur > 0 && transporte.rep.puntoA >= 0
            x: transporte.hayRep ? barra.width * transporte.rep.puntoA / Math.max(1, transporte.dur) : 0
            width: transporte.hayRep && transporte.rep.puntoB > transporte.rep.puntoA
                   ? barra.width * (transporte.rep.puntoB - transporte.rep.puntoA) / Math.max(1, transporte.dur)
                   : Math.max(2, barra.grosor / 2)
            anchors.verticalCenter: parent.verticalCenter
            height: carril.height + barra.grosor * 2
            radius: tema.radio / 2
            color: tema.seleccion
            opacity: 0.3
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width * barra.avance
            height: carril.height
            radius: height / 2
            color: tema.seleccion
        }
        Rectangle {
            x: parent.width * barra.avance - width / 2
            anchors.verticalCenter: parent.verticalCenter
            width: tema.fuente * 0.9
            height: width
            radius: width / 2
            color: tema.texto
            border.color: tema.seleccion
            border.width: 2
            visible: transporte.hayRep && transporte.rep.sePuedeSaltar
        }

        MouseArea {
            id: barraRaton
            anchors.fill: parent
            anchors.topMargin: -tema.hueco / 2
            anchors.bottomMargin: -tema.hueco / 2
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            // Atado a `mouseX` y no apuntado al moverse: al entrar sin
            // moverse, la vista previa salía en el principio y con 0:00.
            readonly property real sobre: Math.max(0, Math.min(1, mouseX / barra.width))
            // Arrastrar es buscar: el vídeo sigue al ratón punto a punto, y
            // soltar deja la reproducción como estaba.
            function llevar(x) {
                if (transporte.hayRep && transporte.dur > 0)
                    transporte.rep.irA(Math.max(0, Math.min(1, x / barra.width)) * transporte.dur)
            }
            onPressed: function (e) { llevar(e.x) }
            onPositionChanged: function (e) { if (pressed) llevar(e.x) }
        }

        // La vista previa: el tiempo exacto, y en un vídeo el fotograma de la
        // tira más cercano. Recortar de la tira no decodifica nada mientras
        // el ratón se mueve: la imagen entera ya está cargada.
        Rectangle {
            id: previa
            visible: barraRaton.containsMouse && transporte.dur > 0
            readonly property var t: transporte.hayRep ? transporte.rep.tira : ({})
            readonly property bool conFoto: transporte.esVideo && t.ok === true && tiraImagen.status === Image.Ready
            readonly property real fotoAncho: conFoto ? tiraImagen.sourceSize.width / t.columnas : 0
            readonly property real fotoAlto: conFoto ? tiraImagen.sourceSize.height / t.filas : 0
            readonly property real segundos: barraRaton.sobre * transporte.dur / 1000
            readonly property int indice: conFoto ? Math.min(t.n - 1, Math.floor(segundos / t.cadaS)) : 0

            width: Math.max(hora.implicitWidth + tema.hueco * 2, conFoto ? fotoAncho + 4 : 0)
            height: (conFoto ? fotoAlto + 4 : 0) + hora.implicitHeight + tema.hueco / 2
            x: Math.max(0, Math.min(barra.width - width, barraRaton.sobre * barra.width - width / 2))
            y: -height - tema.hueco / 2
            radius: tema.radio
            color: tema.fondo
            border.color: tema.borde

            Item {
                x: 2; y: 2
                width: previa.fotoAncho
                height: previa.fotoAlto
                clip: true
                visible: previa.conFoto
                Image {
                    id: tiraImagen
                    source: previa.t.ok === true ? derivados.url(previa.t.ruta) : ""
                    asynchronous: true
                    cache: true
                    x: -(previa.indice % Math.max(1, previa.t.columnas || 1)) * previa.fotoAncho
                    y: -Math.floor(previa.indice / Math.max(1, previa.t.columnas || 1)) * previa.fotoAlto
                }
            }
            Text {
                id: hora
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: tema.hueco / 4
                text: Textos.tiempoFino(previa.segundos)
                color: tema.texto
                font.pixelSize: tema.fuente * 0.9
            }
        }
    }

    // ------------------------------------------------------------- los botones
    Item {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        anchors.leftMargin: tema.hueco
        anchors.rightMargin: tema.hueco
        anchors.bottomMargin: tema.hueco / 2
        height: transporte.fila

        Row {
            id: izquierda
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            spacing: tema.hueco / 2

            Boton {
                texto: "−5 s"
                visible: !transporte.compacto
                onPulsado: transporte.rep.irA(transporte.pos - 5000)
            }
            Boton {
                visible: !transporte.compacto
                // Un fotograma atrás. En el sonido no hay fotogramas: una décima.
                texto: transporte.esVideo ? "‹ 1" : "‹ 0,1 s"
                onPulsado: transporte.esVideo ? transporte.rep.paso(-1) : transporte.rep.irA(transporte.pos - 100)
            }
            Boton {
                texto: transporte.hayRep && transporte.rep.enMarcha ? "❚❚" : "▶"
                principal: true
                onPulsado: transporte.rep.alternar()
            }
            Boton {
                visible: !transporte.compacto
                texto: transporte.esVideo ? "1 ›" : "0,1 s ›"
                onPulsado: transporte.esVideo ? transporte.rep.paso(1) : transporte.rep.irA(transporte.pos + 100)
            }
            Boton {
                visible: !transporte.compacto
                texto: "+5 s"
                onPulsado: transporte.rep.irA(transporte.pos + 5000)
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                leftPadding: tema.hueco / 2
                text: Textos.tiempoFino(transporte.pos / 1000) + "  /  "
                      + Textos.tiempoFino(transporte.dur / 1000)
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.95
                font.family: "monospace"
            }
        }

        Row {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: tema.hueco / 2
            visible: !transporte.compacto

            // La velocidad: clic sube un escalón, clic derecho o rueda abajo
            // lo baja. Es un número que se lee, no un menú que se abre.
            Boton {
                id: botonVelocidad
                readonly property real v: transporte.hayRep ? transporte.rep.velocidad : 1
                texto: String(v).replace(".", ",") + "×"
                activo: v !== 1
                onPulsado: transporte.rep.cambiarVelocidad(1)
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.RightButton | Qt.MiddleButton
                    onClicked: function (e) {
                        if (e.button === Qt.MiddleButton) transporte.rep.velocidad = 1
                        else transporte.rep.cambiarVelocidad(-1)
                    }
                    onWheel: function (r) { transporte.rep.cambiarVelocidad(r.angleDelta.y > 0 ? 1 : -1) }
                }
            }

            Item { width: tema.hueco / 2; height: 1 }

            Boton {
                texto: "A"
                activo: transporte.hayRep && transporte.rep.puntoA >= 0
                onPulsado: transporte.rep.marcar("a")
            }
            Boton {
                texto: "B"
                activo: transporte.hayRep && transporte.rep.puntoB >= 0
                onPulsado: transporte.rep.marcar("b")
            }
            Boton {
                texto: "×"
                visible: transporte.hayRep && (transporte.rep.puntoA >= 0 || transporte.rep.puntoB >= 0)
                onPulsado: transporte.rep.quitarTramo()
            }
            Boton {
                texto: qsTr("bucle")
                activo: transporte.hayRep && transporte.rep.bucle
                onPulsado: transporte.rep.bucle = !transporte.rep.bucle
            }

            Item { width: tema.hueco / 2; height: 1 }

            Boton {
                texto: transporte.hayRep && (transporte.rep.silencio || transporte.rep.volumen === 0)
                       ? "🔇" : "🔊"
                onPulsado: transporte.rep.silencio = !transporte.rep.silencio
            }
            // El volumen: arrastrar, o la rueda encima.
            Item {
                id: volumen
                anchors.verticalCenter: parent.verticalCenter
                width: tema.fuente * 6
                height: transporte.fila
                readonly property real v: transporte.hayRep && !transporte.rep.silencio ? transporte.rep.volumen : 0
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width
                    height: barra.grosor
                    radius: height / 2
                    color: tema.marcador
                }
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width * volumen.v
                    height: barra.grosor
                    radius: height / 2
                    color: tema.textoTenue
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    function poner(x) {
                        transporte.rep.silencio = false
                        transporte.rep.volumen = Math.max(0, Math.min(1, x / volumen.width))
                    }
                    onPressed: function (e) { poner(e.x) }
                    onPositionChanged: function (e) { if (pressed) poner(e.x) }
                    onWheel: function (r) {
                        transporte.rep.silencio = false
                        transporte.rep.volumen = Math.max(0, Math.min(1,
                            transporte.rep.volumen + (r.angleDelta.y > 0 ? 0.05 : -0.05)))
                    }
                }
            }
        }
    }
}
