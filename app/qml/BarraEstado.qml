// Una línea abajo: lo último que ha pasado y el progreso de lo que esté en marcha.
import QtQuick
import "textos.js" as Textos

Rectangle {
    id: estado
    height: Math.round(tema.fuente * 2.2)
    color: tema.fondo

    property string mensaje: ""
    property bool esError: false
    property int hechos: 0
    property int deCuantos: 0
    /// Qué está en marcha, con el nombre que le puso quien lo mandó: «etiquetar
    /// como rótulo», «vaciar la papelera». Sin esto, una barra que avanza no
    /// dice de qué.
    property string que: ""

    Rectangle {
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 1
        color: tema.borde
    }

    // Barra de progreso: el fondo entero, no una barrita perdida en un rincón.
    Rectangle {
        anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
        width: estado.deCuantos > 0 ? parent.width * (estado.hechos / estado.deCuantos) : 0
        color: tema.seleccion
        opacity: tema.realce * 3
        visible: width > 0
        Behavior on width { NumberAnimation { duration: 120 } }
    }

    Text {
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: tema.hueco
        anchors.right: derecha.left
        elide: Text.ElideRight
        text: estado.deCuantos > 0
              ? Textos.enMarcha(estado.que, estado.hechos, estado.deCuantos)
              : estado.mensaje
        color: estado.esError ? tema.seleccion : tema.textoTenue
        font.pixelSize: tema.fuente * 0.92
    }

    Row {
        id: derecha
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: parent.right
        anchors.rightMargin: tema.hueco
        spacing: tema.hueco

        // La versión nueva, si la hay. Se queda puesta (no se borra a los
        // seis segundos como los avisos) hasta que se descarga o se ignora:
        // es la única forma que tiene Grimorio de decirlo. Ver novedades.h.
        Row {
            anchors.verticalCenter: parent.verticalCenter
            visible: novedades.nueva.length > 0
            spacing: tema.hueco * 0.5

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("versión %1 disponible").arg(novedades.nueva)
                color: tema.seleccion
                font.pixelSize: tema.fuente * 0.92
                font.underline: nuevaRaton.containsMouse

                MouseArea {
                    id: nuevaRaton
                    anchors.fill: parent
                    anchors.margins: -tema.hueco * 0.3
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: novedades.abrir()
                }
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "×"
                color: ignorarRaton.containsMouse ? tema.texto : tema.textoTenue
                font.pixelSize: tema.fuente

                MouseArea {
                    id: ignorarRaton
                    anchors.fill: parent
                    anchors.margins: -tema.hueco * 0.3
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: novedades.ignorar()
                }
            }
        }

        // Deshacer, con el nombre de lo que va a deshacer. Va aquí y no arriba
        // porque esta barra ya cuenta lo que acaba de pasar, y porque un botón
        // que dice «deshacer poner 4 estrellas» enseña el atajo mejor que
        // cualquier ayuda: se lee justo después de haberlo hecho.
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: nucleo.queSeDeshace.length > 0
            // Con tope y elipsis: una etiqueta larga en el título empujaba la
            // cuenta de elegidos fuera de la ventana.
            width: Math.min(implicitWidth, estado.width * 0.3)
            elide: Text.ElideRight
            // Con la palabra delante. Sin ella se lee «poner 3 estrellas» y
            // parece un botón que las pone, que es lo contrario de lo que hace.
            text: qsTr("↶ deshacer %1").arg(nucleo.queSeDeshace)
            color: deshacerRaton.containsMouse ? tema.texto : tema.textoTenue
            font.pixelSize: tema.fuente * 0.92

            MouseArea {
                id: deshacerRaton
                anchors.fill: parent
                anchors.margins: -tema.hueco * 0.3
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: nucleo.deshacer()
            }
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: nucleo.queSeRehace.length > 0
            text: qsTr("↷ rehacer")
            color: rehacerRaton.containsMouse ? tema.texto : tema.textoTenue
            font.pixelSize: tema.fuente * 0.92

            MouseArea {
                id: rehacerRaton
                anchors.fill: parent
                anchors.margins: -tema.hueco * 0.3
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: nucleo.rehacer()
            }
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: modelo.elegidos > 0 ? Textos.elegidos(modelo.elegidos) : ""
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.92
        }
    }

    Timer {
        id: olvido
        interval: 6000
        onTriggered: estado.mensaje = ""
    }

    /// Un aviso que no viene del núcleo: lo que pasa solo en la ventana
    /// —copiar, pegar—, que también tiene que decir que ha pasado.
    function avisar(mensaje, error) {
        estado.mensaje = mensaje
        estado.esError = error === true
        olvido.restart()
    }

    Connections {
        target: traer
        function onAviso(mensaje, error) { estado.avisar(mensaje, error) }
    }

    Connections {
        target: captura
        function onAviso(mensaje, error) { estado.avisar(mensaje, error) }
    }

    Connections {
        target: bibliotecas
        function onAviso(mensaje, error) { estado.avisar(mensaje, error) }
    }

    Connections {
        target: exportar
        function onAviso(mensaje, error) { estado.avisar(mensaje, error) }
    }

    Connections {
        target: novedades
        function onAviso(mensaje, error) { estado.avisar(mensaje, error) }
    }

    Connections {
        target: vigilancia
        function onAviso(mensaje, error) { estado.avisar(mensaje, error) }
    }

    Connections {
        target: nucleo
        function onAviso(mensaje, error) {
            estado.mensaje = mensaje
            estado.esError = error
            olvido.restart()
        }
        function onProgreso(que, hechos, total) {
            estado.hechos = hechos
            estado.deCuantos = hechos >= total ? 0 : total
            estado.que = que
        }
    }
}
