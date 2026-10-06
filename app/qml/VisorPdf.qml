// Un PDF entero, página tras página.
//
// Las páginas las dibuja `pdftoppm` en el núcleo, una a una y solo las que se
// ven (y un par alrededor), y se guardan: volver a una página ya vista es leer
// un JPEG. Antes de dibujar nada se piden las medidas de todas, para que el
// documento tenga su largo de verdad desde el principio y la barra de
// desplazamiento no salte a medida que llegan.
//
//   rueda              bajar y subir
//   Ctrl + rueda, + −  acercar y alejar
//   Re Pág / Av Pág    página anterior y siguiente (las lleva el visor)
import QtQuick
import "textos.js" as Textos

Item {
    id: pdf
    property string idElemento: ""
    property url original
    /// 1 es «la página cabe a lo ancho»; más, más cerca.
    property real escala: 1
    property var medidas: []
    property string error: ""
    property string claveMedidas: ""
    readonly property int paginaActual: lista.count > 0
        ? Math.max(0, lista.indexAt(lista.width / 2, lista.contentY + lista.height / 3)) : 0

    /// Ancho base de una página: el del visor, sin pasarse de lo cómodo para
    /// leer. Un A4 a todo lo ancho de un monitor de 27 pulgadas no se lee.
    readonly property real anchoBase: Math.min(width - tema.margen * 2, height * 0.9)

    function pagina(d) {
        lista.positionViewAtIndex(Math.max(0, Math.min(lista.count - 1, paginaActual + d)),
                                  ListView.Beginning)
    }
    function zoom(f) { escala = Math.max(0.3, Math.min(5, escala * f)) }

    onIdElementoChanged: {
        medidas = []
        error = ""
        escala = 1
        if (idElemento.length > 0)
            claveMedidas = derivados.pedir("pdf_medidas", idElemento, original)
    }

    Connections {
        target: derivados
        function onListo(clave, r) {
            if (clave !== pdf.claveMedidas) return
            if (r.ok) pdf.medidas = r.paginas
            else pdf.error = r.error
        }
    }

    ListView {
        id: lista
        anchors.fill: parent
        clip: true
        spacing: tema.hueco
        model: pdf.medidas.length
        cacheBuffer: pdf.height
        boundsBehavior: Flickable.StopAtBounds
        // Acercar con Ctrl + rueda; sin Ctrl, la rueda es de la lista.
        WheelHandler {
            acceptedModifiers: Qt.ControlModifier
            onWheel: function (e) { pdf.zoom(e.angleDelta.y > 0 ? 1.15 : 1 / 1.15) }
        }
        header: Item { width: 1; height: tema.margen }
        footer: Item { width: 1; height: tema.margen }

        delegate: Item {
            id: hoja
            readonly property var m: pdf.medidas[index] || [595, 842]
            readonly property real w: pdf.anchoBase * pdf.escala * (m[0] / Math.max(m[0], m[1] * 0.75))
            width: lista.width
            height: w * m[1] / m[0]

            /// El ancho al que se pide al núcleo, redondeado a tramos de 256 px
            /// de pantalla real: así acercar un poco no dibuja la página otra
            /// vez, y la caché no se llena de variantes casi iguales.
            readonly property int anchoPedido: Math.max(512, Math.ceil(w * Screen.devicePixelRatio / 256) * 256)
            property string clave: ""
            property url imagen

            function pedir() {
                clave = derivados.pedir("pdf_pagina", pdf.idElemento, pdf.original,
                                        { pagina: index + 1, ancho: anchoPedido })
            }
            Component.onCompleted: pedir()
            onAnchoPedidoChanged: pedir()

            Connections {
                target: derivados
                function onListo(clave, r) {
                    if (clave === hoja.clave && r.ok) hoja.imagen = derivados.url(r.ruta)
                }
            }

            Rectangle {
                id: papel
                anchors.horizontalCenter: parent.horizontalCenter
                width: hoja.w
                height: parent.height
                color: tema.panel
                border.color: tema.borde

                Image {
                    anchors.fill: parent
                    source: hoja.imagen
                    asynchronous: true
                    cache: true
                    smooth: true
                    mipmap: true
                    fillMode: Image.PreserveAspectFit
                }
                Text {
                    anchors.centerIn: parent
                    visible: String(hoja.imagen).length === 0
                    text: String(index + 1)
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 2
                }
            }
        }
    }

    // Página y zoom, abajo a la derecha.
    Row {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: tema.hueco
        spacing: tema.hueco / 2
        visible: pdf.medidas.length > 0

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: indicador.implicitWidth + tema.hueco * 2
            height: Math.round(tema.fuente * 2)
            radius: tema.radio
            color: tema.panel
            Text {
                id: indicador
                anchors.centerIn: parent
                text: qsTr("página %1 de %2").arg(pdf.paginaActual + 1).arg(pdf.medidas.length)
                color: tema.texto
                font.pixelSize: tema.fuente
            }
        }
        Boton { texto: "−"; onPulsado: pdf.zoom(1 / 1.25) }
        Boton { texto: Math.round(pdf.escala * 100) + " %"; onPulsado: pdf.escala = 1 }
        Boton { texto: "+"; onPulsado: pdf.zoom(1.25) }
    }

    Text {
        anchors.centerIn: parent
        visible: pdf.medidas.length === 0
        width: parent.width * 0.7
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        text: pdf.error.length > 0 ? qsTr("no pude abrir el PDF: %1").arg(pdf.error)
                                   : qsTr("abriendo el documento…")
        color: tema.textoTenue
        font.pixelSize: tema.fuente
    }
}
