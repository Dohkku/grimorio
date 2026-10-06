// Un botón sin QtQuick.Controls: son cuatro propiedades y así el estilo entero
// sale del tema en vez de pelearse con el de Controls.
//
// Tres estados que se tienen que distinguir de un vistazo:
//
//   normal   fondo de panel y borde suave: se ve que es un botón, pero no grita
//   encima   un velo claro y el borde se aviva: «esto responde»
//   activo   tinte del color de selección, borde y texto del mismo color
//
// El activo era un bloque macizo del color de selección con el texto en negro,
// y en una barra con dos o tres activos a la vez se comía la vista: era lo más
// llamativo de la pantalla, más que las propias imágenes. Con el tinte se lee
// igual de «encendido» y deja de competir.
//
// Los velos son el color del tema con opacidad, no colores hechos aquí: la
// prueba `sin_colores_a_mano` no deja construir colores en el QML, y hace bien.
import QtQuick

Item {
    id: boton
    property string texto: ""
    /// Un icono de `iconos.js`. Solo, el botón es cuadrado; con texto, va
    /// delante.
    property string icono: ""
    property bool activo: false
    /// Sin borde ni fondo hasta pasar por encima: para lo que va dentro de
    /// otra cosa y no tiene que pesar (el «+» de las carpetas, por ejemplo).
    property bool discreto: false
    /// Relleno macizo del color de selección: para la acción que manda en un
    /// sitio y solo para esa —el play de los mandos—. Uno por zona; dos ya no
    /// mandan.
    property bool principal: false
    signal pulsado()

    width: Math.max(altura, contenido.implicitWidth + (texto.length > 0 ? tema.hueco * 2 : 0))
    height: altura
    property real altura: Math.round(tema.fuente * 2.1)
    /// Dentro de un `Row` los elementos se apoyan arriba, así que el botón se
    /// centra solo. Dentro de un `Flow` no puede: `Flow` coloca a sus hijos y
    /// un ancla se lo impide (y avisa por consola). Por eso se puede apagar.
    property bool centrado: true
    /// Con un ancho puesto desde fuera, el texto que no cabe se corta con
    /// «…» en vez de salirse del botón. Solo así: con el ancho por defecto,
    /// que sale del texto, cortarlo sería un bucle.
    property bool recortar: false
    anchors.verticalCenter: centrado && parent ? parent.verticalCenter : undefined

    readonly property bool encima: raton.containsMouse
    readonly property bool apretado: raton.pressed

    Item {
        id: cara
        anchors.fill: parent
        // Hundirse un poco al apretar: sin esto, el clic no se siente hasta
        // que pasa lo que el botón hace, y eso a veces tarda.
        scale: boton.apretado ? 0.97 : 1
        Behavior on scale { NumberAnimation { duration: 70 } }

        Rectangle {
            id: fondo
            anchors.fill: parent
            radius: tema.radio
            color: boton.principal ? tema.seleccion
                   : (boton.discreto && !boton.activo ? "transparent" : tema.panel)
            border.width: boton.discreto && !boton.encima && !boton.activo ? 0 : 1
            border.color: boton.activo || boton.principal ? tema.seleccion
                          : (boton.encima ? tema.textoTenue : tema.borde)
            Behavior on border.color { ColorAnimation { duration: 90 } }
        }

        // El tinte del activo.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: tema.radio - 1
            color: tema.seleccion
            opacity: boton.activo && !boton.principal ? (boton.encima ? 0.24 : 0.16) : 0
            Behavior on opacity { NumberAnimation { duration: 90 } }
        }

        // El velo de encima y de apretar.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: tema.radio - 1
            color: boton.principal ? tema.fondo : tema.texto
            opacity: boton.activo && !boton.principal ? 0
                     : (boton.apretado ? tema.realce * 2 : (boton.encima ? tema.realce : 0))
            Behavior on opacity { NumberAnimation { duration: 90 } }
        }

        Row {
            id: contenido
            anchors.centerIn: parent
            spacing: tema.hueco * 0.45
            readonly property color tinta: boton.principal ? tema.fondo
                                           : (boton.activo ? tema.seleccion : tema.texto)
            Icono {
                anchors.verticalCenter: parent.verticalCenter
                visible: boton.icono.length > 0
                nombre: boton.icono
                color: contenido.tinta
                width: Math.round(boton.altura * 0.6)
                height: width
            }
            Text {
                id: etiqueta
                anchors.verticalCenter: parent.verticalCenter
                visible: boton.texto.length > 0
                width: boton.recortar
                       ? Math.min(implicitWidth, boton.width - tema.hueco * 2
                                  - (boton.icono.length > 0 ? Math.round(boton.altura * 0.6) + contenido.spacing : 0))
                       : implicitWidth
                elide: Text.ElideRight
                text: boton.texto
                color: contenido.tinta
                font.pixelSize: tema.fuente
                font.weight: Font.Medium
                font.letterSpacing: tema.fuente * 0.01
            }
        }
    }

    MouseArea {
        id: raton
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: boton.pulsado()
    }
}
