// Los ajustes de quien está delante, y quién hizo esto.
//
// Una tarjeta en medio, como la de las etiquetas, con las páginas a la
// izquierda. Todo se aplica al tocarlo —no hay «guardar»—: lo que se mueve
// aquí se ve detrás en el acto, que es la mejor vista previa que hay. Se
// guarda en `~/.config/Grimorio/Grimorio.conf` (ver `ajustes.h`), no en el
// tema: el tema es dato del proyecto y esto es de cada uno.
//
// Antes era una sola página larga con todo en fila, rótulo tras rótulo, y se
// leía como un formulario de hacienda. Ahora cada tema tiene su página, y
// dentro de ella los ajustes van en grupos: una caja con filas separadas por
// una raya fina, el texto a la izquierda y el mando a la derecha. Las opciones
// de «una entre varias» son un selector de piezas pegadas con una marca que se
// desliza hasta la elegida, en vez de botones sueltos que no parecían ir juntos.
//
// Las medidas salen de la letra (`aire`) y no de `tema.hueco`: el tema papel
// lleva un hueco de 4 px para que las celdas vayan pegadas, y con eso las
// filas de aquí quedaban apretadísimas.
//
//   Ctrl+,   abrir y cerrar
import QtQuick
import "textos.js" as Textos

Item {
    id: panel
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    /// "general", "galeria", "edicion", "versiones" o "acerca".
    property string pagina: "general"

    /// El espacio de respirar de todo el panel.
    readonly property real aire: Math.round(tema.fuente * 0.9)
    /// Las esquinas de las cajas: las del tema, un poco más suaves.
    readonly property real esquina: tema.radio + Math.round(tema.fuente * 0.2)
    /// Hay una versión nueva, o se está instalando.
    readonly property bool hayVersion: novedades.nueva.length > 0 || novedades.fase.length > 0

    readonly property var paginas: [
        { clave: "general", texto: qsTr("general"), icono: "engranaje",
          lema: qsTr("cómo se ve Grimorio, en qué idioma y a qué tamaño") },
        { clave: "galeria", texto: qsTr("galería"), icono: "cuadricula",
          lema: qsTr("las celdas, los vídeos y lo que se difumina") },
        { clave: "edicion", texto: qsTr("pilas y editor"), icono: "pila",
          lema: qsTr("qué pasa al tirar versiones y cómo se recorta un vídeo") },
        { clave: "versiones", texto: qsTr("versiones"), icono: "importar",
          lema: qsTr("cuándo te avisa Grimorio de que hay una nueva") },
        { clave: "acerca", texto: qsTr("acerca de"), icono: "info", lema: "" }
    ]
    readonly property var paginaActual: {
        for (let i = 0; i < paginas.length; i++)
            if (paginas[i].clave === pagina) return paginas[i]
        return paginas[0]
    }

    // Esc lo cierra desde el atajo de la ventana, que es quien se lleva la
    // tecla antes que nadie.
    function abrir(p) {
        pagina = p
        abierto = true
    }
    function cerrar() {
        abierto = false
    }

    onPaginaChanged: {
        hoja.contentY = 0
        entrada.restart()
    }
    onAbiertoChanged: if (abierto) {
        hoja.contentY = 0
        entrada.restart()
    }

    // ------------------------------------------------------------ piezas

    // Una caja de ajustes con su rótulo encima. Lo que se mete dentro va en
    // columna, y cada fila pinta su raya de arriba menos la primera.
    component Grupo: Column {
        id: grupo
        property string titulo: ""
        default property alias filas: lista.data
        width: parent ? parent.width : 0
        spacing: Math.round(panel.aire * 0.55)

        Text {
            visible: grupo.titulo.length > 0
            leftPadding: Math.round(panel.aire * 0.25)
            text: grupo.titulo
            color: tema.textoTenue
            font.pixelSize: Math.round(tema.fuente * 0.78)
            font.weight: Font.DemiBold
            font.letterSpacing: tema.fuente * 0.09
            font.capitalization: Font.AllUppercase
        }
        Rectangle {
            width: parent.width
            height: lista.height
            radius: panel.esquina
            color: tema.fondo
            border.color: tema.borde
            border.width: 1
            Column {
                id: lista
                width: parent.width
            }
        }
    }

    // Una fila: el título y su explicación a la izquierda, el mando a la
    // derecha (o debajo, con `debajo`, para los selectores anchos). Con
    // `pulsable`, la fila entera responde: en un interruptor nadie apunta a
    // la llavecita.
    component Fila: Item {
        id: fila
        property string titulo: ""
        property string detalle: ""
        property bool debajo: false
        property bool pulsable: false
        /// El título en tenue y el valor en claro: para los datos de «acerca de».
        property bool dato: false
        default property alias control: lado.data
        signal pulsada()

        width: parent ? parent.width : 0
        height: (debajo ? textosFila.height + Math.round(panel.aire * 0.7) + lado.height
                        : Math.max(textosFila.height, lado.height)) + panel.aire * (dato ? 1.5 : 2)

        Rectangle {
            visible: !fila.Positioner.isFirstItem
            anchors { left: parent.left; right: parent.right; top: parent.top }
            anchors.leftMargin: panel.aire * 1.2
            anchors.rightMargin: panel.aire * 1.2
            height: 1
            color: tema.borde
        }

        // El velo de encima, solo si toda la fila hace algo.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: panel.esquina - 1
            color: tema.texto
            opacity: fila.pulsable && raton.containsMouse ? tema.realce * 0.5 : 0
            Behavior on opacity { NumberAnimation { duration: 90 } }
        }

        Column {
            id: textosFila
            x: panel.aire * 1.2
            y: fila.debajo ? panel.aire
                           : Math.round((fila.height - height) / 2)
            width: fila.debajo ? fila.width - panel.aire * 2.4
                               : fila.width - panel.aire * 3.4 - lado.width
            spacing: Math.round(tema.fuente * 0.2)
            Text {
                width: parent.width
                text: fila.titulo
                color: fila.dato ? tema.textoTenue : tema.texto
                font.pixelSize: tema.fuente
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                visible: fila.detalle.length > 0
                text: fila.detalle
                color: tema.textoTenue
                font.pixelSize: Math.round(tema.fuente * 0.85)
                lineHeight: 1.1
                wrapMode: Text.Wrap
            }
        }

        MouseArea {
            id: raton
            anchors.fill: parent
            enabled: fila.pulsable
            hoverEnabled: true
            cursorShape: fila.pulsable ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: fila.pulsada()
        }

        // Encima de la zona de la fila, para que sus botones reciban el clic.
        Item {
            id: lado
            x: fila.debajo ? panel.aire * 1.2 : fila.width - width - panel.aire * 1.2
            y: fila.debajo ? textosFila.y + textosFila.height + Math.round(panel.aire * 0.7)
                           : Math.round((fila.height - height) / 2)
            width: childrenRect.width
            height: childrenRect.height
        }
    }

    // La llave de encender y apagar.
    component Llave: Rectangle {
        id: llave
        property bool puesto: false
        width: Math.round(tema.fuente * 2.7)
        height: Math.round(tema.fuente * 1.5)
        radius: height / 2
        color: puesto ? tema.seleccion : tema.panel
        border.color: puesto ? tema.seleccion : tema.borde
        border.width: 1
        Behavior on color { ColorAnimation { duration: 140 } }

        Rectangle {
            width: parent.height - 6
            height: width
            radius: width / 2
            anchors.verticalCenter: parent.verticalCenter
            x: llave.puesto ? parent.width - width - 3 : 3
            color: llave.puesto ? tema.fondo : tema.textoTenue
            Behavior on x { NumberAnimation { duration: 160; easing.type: Easing.OutBack; easing.overshoot: 1.2 } }
        }
    }

    // Un interruptor: una fila entera que se pulsa, con su llave.
    component Interruptor: Fila {
        id: inter
        property bool puesto: false
        signal cambiado(bool v)
        pulsable: true
        onPulsada: inter.cambiado(!inter.puesto)
        Llave { puesto: inter.puesto }
    }

    // Una entre varias: piezas pegadas en una sola caja y la marca de la
    // elegida, que se desliza de una a otra.
    component Segmentos: Rectangle {
        id: seg
        /// [{ valor, texto }]
        property var opciones: []
        property var valor
        signal elegido(var v)

        function igual(a, b) {
            return typeof a === "number" && typeof b === "number" ? Math.abs(a - b) < 0.001 : a === b
        }
        readonly property int activo: {
            for (let i = 0; i < opciones.length; i++)
                if (igual(opciones[i].valor, valor)) return i
            return -1
        }
        readonly property Item piezaActiva: repetidor.count > 0 && activo >= 0 ? repetidor.itemAt(activo) : null

        width: filaSeg.width + 6
        height: Math.round(tema.fuente * 2.2)
        radius: panel.esquina
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        Item {
            id: marca
            visible: seg.piezaActiva !== null
            x: 3 + (seg.piezaActiva ? seg.piezaActiva.x : 0)
            y: 3
            width: seg.piezaActiva ? seg.piezaActiva.width : 0
            height: seg.height - 6
            Behavior on x { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
            Behavior on width { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
            Rectangle {
                anchors.fill: parent
                radius: Math.max(0, panel.esquina - 2)
                color: tema.fondo
                border.color: tema.seleccion
                border.width: 1
            }
            Rectangle {
                anchors.fill: parent
                anchors.margins: 1
                radius: Math.max(0, panel.esquina - 3)
                color: tema.seleccion
                opacity: 0.14
            }
        }

        Row {
            id: filaSeg
            x: 3
            y: 3
            Repeater {
                id: repetidor
                model: seg.opciones
                delegate: Item {
                    id: pieza
                    required property var modelData
                    required property int index
                    readonly property bool elegida: seg.activo === index
                    width: Math.max(Math.round(tema.fuente * 2.6), rotulo.implicitWidth + panel.aire * 1.6)
                    height: seg.height - 6

                    Rectangle {
                        anchors.fill: parent
                        radius: Math.max(0, panel.esquina - 2)
                        color: tema.texto
                        opacity: !pieza.elegida && toque.containsMouse ? tema.realce : 0
                        Behavior on opacity { NumberAnimation { duration: 90 } }
                    }
                    Text {
                        id: rotulo
                        anchors.centerIn: parent
                        text: pieza.modelData.texto
                        color: pieza.elegida ? tema.seleccion : tema.texto
                        font.pixelSize: Math.round(tema.fuente * 0.95)
                        font.weight: pieza.elegida ? Font.DemiBold : Font.Normal
                        Behavior on color { ColorAnimation { duration: 140 } }
                    }
                    MouseArea {
                        id: toque
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: seg.elegido(pieza.modelData.valor)
                    }
                }
            }
        }
    }

    // Lo de la versión nueva: arriba del todo, con el color de la selección.
    // Abajo, como una fila más, no la veía nadie.
    component TarjetaVersion: Rectangle {
        id: tv
        readonly property bool enMarcha: novedades.fase.length > 0
        visible: panel.hayVersion
        width: parent ? parent.width : 0
        height: visible ? contenidoVersion.height + panel.aire * 2.6 : 0
        radius: panel.esquina
        color: "transparent"
        border.color: tema.seleccion
        border.width: Math.max(1, Math.round(tema.fuente * 0.12))

        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: tema.seleccion
            opacity: tema.realce * 1.5
        }

        Column {
            id: contenidoVersion
            anchors { left: parent.left; right: parent.right; top: parent.top }
            anchors.margins: panel.aire * 1.3
            spacing: Math.round(panel.aire * 0.7)

            Row {
                spacing: Math.round(panel.aire * 0.6)
                Icono {
                    anchors.verticalCenter: parent.verticalCenter
                    nombre: "importar"
                    color: tema.seleccion
                    width: Math.round(tema.fuente * 1.4)
                    height: width
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Grimorio %1 está disponible").arg(novedades.nueva)
                    color: tema.texto
                    font.pixelSize: Math.round(tema.fuente * 1.25)
                    font.weight: Font.DemiBold
                }
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: tv.enMarcha
                      ? (novedades.fase === "bajando"
                         ? (novedades.progreso >= 0
                            ? qsTr("bajando… %1 %").arg(Math.round(novedades.progreso * 100))
                            : qsTr("bajando…"))
                         : qsTr("instalando; Grimorio se cierra y se vuelve a abrir solo"))
                      : novedades.instalable
                        ? qsTr("tienes la %1. Se baja de las publicadas, se comprueba que es la misma y se instala; Grimorio se vuelve a abrir solo, con esta biblioteca.").arg(novedades.actual)
                        : qsTr("tienes la %1. Se abre la página de descargas para bajar la nueva.").arg(novedades.actual)
                color: tema.textoTenue
                font.pixelSize: Math.round(tema.fuente * 0.95)
            }

            // Si el intento anterior falló, aquí y no solo abajo: la barra de
            // estado queda tapada por este panel.
            Text {
                visible: novedades.error.length > 0 && !tv.enMarcha
                width: parent.width
                wrapMode: Text.WordWrap
                text: qsTr("no se ha podido actualizar: %1").arg(novedades.error)
                color: tema.seleccion
                font.pixelSize: Math.round(tema.fuente * 0.95)
                font.weight: Font.DemiBold
            }

            // Lo bajado.
            Rectangle {
                visible: novedades.fase === "bajando"
                width: parent.width
                height: Math.max(3, Math.round(tema.fuente * 0.35))
                radius: height / 2
                color: tema.borde
                Rectangle {
                    anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
                    radius: parent.radius
                    width: novedades.progreso > 0 ? parent.width * Math.min(1, novedades.progreso) : 0
                    color: tema.seleccion
                    Behavior on width { NumberAnimation { duration: 120 } }
                }
            }

            Row {
                spacing: Math.round(panel.aire * 0.6)
                visible: !tv.enMarcha
                Boton {
                    principal: true
                    altura: Math.round(tema.fuente * 2.4)
                    icono: "importar"
                    texto: novedades.instalable ? qsTr("actualizar ahora") : qsTr("descargar")
                    onPulsado: novedades.actualizar()
                }
                Boton {
                    visible: novedades.instalable
                    altura: Math.round(tema.fuente * 2.4)
                    texto: qsTr("ver la página")
                    onPulsado: novedades.abrir()
                }
                Boton {
                    altura: Math.round(tema.fuente * 2.4)
                    texto: qsTr("ahora no")
                    onPulsado: novedades.ignorar()
                }
            }
        }
    }

    // Una página de la izquierda.
    component Entrada: Item {
        id: entradaIndice
        required property var modelData
        readonly property bool activa: panel.pagina === modelData.clave
        /// El punto de «hay algo nuevo aquí».
        readonly property bool aviso: modelData.clave === "versiones" && panel.hayVersion
        width: parent ? parent.width : 0
        height: Math.round(tema.fuente * 2.5)

        Rectangle {
            anchors.fill: parent
            radius: panel.esquina
            color: entradaIndice.activa ? tema.seleccion : tema.texto
            opacity: entradaIndice.activa ? 0.13
                     : (toqueEntrada.pressed ? tema.realce * 1.6
                        : (toqueEntrada.containsMouse ? tema.realce : 0))
            Behavior on opacity { NumberAnimation { duration: 90 } }
        }
        // La raya de la elegida, en el borde.
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            x: 0
            width: Math.max(2, Math.round(tema.fuente * 0.22))
            height: entradaIndice.activa ? Math.round(parent.height * 0.55) : 0
            radius: width / 2
            color: tema.seleccion
            Behavior on height { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }
        }

        Row {
            anchors.verticalCenter: parent.verticalCenter
            x: Math.round(panel.aire * 0.9)
            spacing: Math.round(panel.aire * 0.65)
            Icono {
                anchors.verticalCenter: parent.verticalCenter
                nombre: entradaIndice.modelData.icono
                color: entradaIndice.activa ? tema.seleccion : tema.textoTenue
                width: Math.round(tema.fuente * 1.25)
                height: width
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: entradaIndice.modelData.texto
                color: entradaIndice.activa ? tema.seleccion : tema.texto
                font.pixelSize: tema.fuente
                font.weight: entradaIndice.activa ? Font.DemiBold : Font.Normal
            }
        }
        Rectangle {
            visible: entradaIndice.aviso
            anchors { right: parent.right; rightMargin: panel.aire; verticalCenter: parent.verticalCenter }
            width: Math.round(tema.fuente * 0.55)
            height: width
            radius: width / 2
            color: tema.seleccion
        }
        MouseArea {
            id: toqueEntrada
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: panel.pagina = entradaIndice.modelData.clave
        }
    }

    // El valor de un dato, a la derecha de su fila.
    component Valor: Text {
        width: Math.min(implicitWidth, Math.round(tema.fuente * 22))
        horizontalAlignment: Text.AlignRight
        elide: Text.ElideMiddle
        color: tema.texto
        font.pixelSize: tema.fuente
    }

    // ------------------------------------------------------------ el panel

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: panel.cerrar()
        // La rueda que sobra al llegar al final de una página no baja la
        // galería de detrás.
        onWheel: function (rueda) { rueda.accepted = true }
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        opacity: 0.75
    }

    Rectangle {
        id: tarjeta
        anchors.centerIn: parent
        width: Math.min(tema.fuente * 56, panel.width * 0.9)
        height: Math.min(tema.fuente * 40, panel.height * 0.88)
        radius: panel.esquina
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        clip: true

        MouseArea { anchors.fill: parent }

        // ------------------------------------------------------ el índice
        Item {
            id: indice
            anchors { top: parent.top; bottom: parent.bottom; left: parent.left }
            width: Math.round(tema.fuente * 14)

            // Un velo apenas más oscuro que la hoja, y una raya que los separa.
            Rectangle {
                anchors.fill: parent
                anchors.margins: 1
                radius: panel.esquina
                color: tema.texto
                opacity: tema.realce * 0.35
            }
            Rectangle {
                anchors { top: parent.top; bottom: parent.bottom; right: parent.right }
                width: 1
                color: tema.borde
            }

            Column {
                anchors { left: parent.left; right: parent.right; top: parent.top }
                anchors.margins: panel.aire
                anchors.topMargin: panel.aire * 1.4
                spacing: Math.round(panel.aire * 0.25)

                Text {
                    leftPadding: Math.round(panel.aire * 0.9)
                    bottomPadding: panel.aire
                    text: qsTr("Ajustes")
                    color: tema.texto
                    font.pixelSize: Math.round(tema.fuente * 1.3)
                    font.weight: Font.DemiBold
                }

                Repeater {
                    model: panel.paginas
                    delegate: Entrada {}
                }
            }

            // Abajo: qué versión es y cómo se vuelve aquí.
            Column {
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                anchors.margins: panel.aire * 1.2
                anchors.leftMargin: panel.aire * 1.9
                spacing: Math.round(panel.aire * 0.5)
                Text {
                    text: "Grimorio " + versionGrimorio
                    color: tema.textoTenue
                    font.pixelSize: Math.round(tema.fuente * 0.85)
                }
                Row {
                    spacing: Math.round(panel.aire * 0.5)
                    Rectangle {
                        width: tecla.implicitWidth + panel.aire * 0.9
                        height: tecla.implicitHeight + panel.aire * 0.35
                        radius: Math.max(2, tema.radio * 0.6)
                        color: tema.fondo
                        border.color: tema.borde
                        Text {
                            id: tecla
                            anchors.centerIn: parent
                            text: "Ctrl+,"
                            color: tema.textoTenue
                            font.pixelSize: Math.round(tema.fuente * 0.78)
                        }
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("abre y cierra")
                        color: tema.textoTenue
                        font.pixelSize: Math.round(tema.fuente * 0.78)
                    }
                }
            }
        }

        // ------------------------------------------------------ la hoja
        Flickable {
            id: hoja
            anchors {
                top: parent.top; bottom: parent.bottom
                left: indice.right; right: parent.right
            }
            contentHeight: contenido.height + panel.aire * 4
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            Column {
                id: contenido
                x: panel.aire * 2.2
                y: panel.aire * 1.8
                width: hoja.width - panel.aire * 4.4
                spacing: Math.round(panel.aire * 1.5)

                // Al cambiar de página, lo nuevo entra subiendo un poco.
                transform: Translate { id: desliz }
                ParallelAnimation {
                    id: entrada
                    NumberAnimation { target: contenido; property: "opacity"; from: 0; to: 1; duration: 160 }
                    NumberAnimation { target: desliz; property: "y"; from: panel.aire * 0.8; to: 0; duration: 220; easing.type: Easing.OutCubic }
                }

                // La cabecera de la página. Se aparta de la × de cerrar.
                Column {
                    visible: panel.pagina !== "acerca"
                    width: parent.width - panel.aire * 2.5
                    spacing: Math.round(tema.fuente * 0.3)
                    Text {
                        // Mayúscula solo la primera: «Pilas y editor».
                        text: panel.paginaActual.texto.charAt(0).toUpperCase()
                              + panel.paginaActual.texto.slice(1)
                        color: tema.texto
                        font.pixelSize: Math.round(tema.fuente * 1.55)
                        font.weight: Font.DemiBold
                    }
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: panel.paginaActual.lema
                        color: tema.textoTenue
                        font.pixelSize: Math.round(tema.fuente * 0.95)
                        wrapMode: Text.Wrap
                    }
                }

                TarjetaVersion {
                    visible: panel.hayVersion && (panel.pagina === "general" || panel.pagina === "versiones")
                }

                // -------------------------------------------- general
                Grupo {
                    visible: panel.pagina === "general"
                    titulo: qsTr("apariencia")

                    Fila {
                        titulo: qsTr("tema")
                        detalle: qsTr("noche, oscuro y con aire; papel, claro y con las celdas juntas")
                        Segmentos {
                            opciones: [
                                { valor: "oscuro", texto: qsTr("noche") },
                                { valor: "papel", texto: qsTr("papel") }
                            ]
                            valor: ajustes.tema
                            onElegido: function (v) { ajustes.tema = v }
                        }
                    }
                    // Los nombres de los idiomas, cada uno en el suyo: quien
                    // no entiende el que está puesto tiene que reconocer el
                    // propio.
                    Fila {
                        titulo: qsTr("idioma")
                        detalle: (ajustes.idioma.length > 0 ? ajustes.idioma : ajustes.idiomaEnUso)
                                 !== ajustes.idiomaEnUso
                                 ? qsTr("se cambia al volver a abrir Grimorio") : ""
                        Segmentos {
                            opciones: [
                                { valor: "", texto: qsTr("el del sistema") },
                                { valor: "es", texto: "español" },
                                { valor: "en", texto: "English" }
                            ]
                            valor: ajustes.idioma
                            onElegido: function (v) { ajustes.idioma = v }
                        }
                    }
                    Fila {
                        titulo: qsTr("tamaño de la interfaz")
                        detalle: qsTr("letra, iconos y paneles; Ctrl y ± desde cualquier sitio; las miniaturas, con Ctrl y la rueda")
                        Row {
                            spacing: Math.round(panel.aire * 0.5)
                            Boton {
                                centrado: false
                                discreto: true
                                visible: Math.abs(ajustes.escala - 1) > 0.001
                                anchors.verticalCenter: parent.verticalCenter
                                texto: qsTr("100 %")
                                pista: qsTr("volver al tamaño normal")
                                onPulsado: ventana.pasoEscala(0)
                            }
                            // Menos, el número y más, en una sola pieza.
                            Rectangle {
                                width: filaEscala.width + 6
                                height: Math.round(tema.fuente * 2.2)
                                radius: panel.esquina
                                color: tema.panel
                                border.color: tema.borde
                                Row {
                                    id: filaEscala
                                    x: 3
                                    anchors.verticalCenter: parent.verticalCenter
                                    Boton {
                                        discreto: true
                                        altura: Math.round(tema.fuente * 2.2) - 6
                                        icono: "menos"
                                        pista: qsTr("más pequeño (Ctrl+−)")
                                        onPulsado: ventana.pasoEscala(-1)
                                    }
                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: Math.round(tema.fuente * 3.8)
                                        horizontalAlignment: Text.AlignHCenter
                                        text: Math.round(ajustes.escala * 100) + " %"
                                        color: tema.texto
                                        font.pixelSize: tema.fuente
                                        font.weight: Font.DemiBold
                                    }
                                    Boton {
                                        discreto: true
                                        altura: Math.round(tema.fuente * 2.2) - 6
                                        icono: "mas"
                                        pista: qsTr("más grande (Ctrl++)")
                                        onPulsado: ventana.pasoEscala(1)
                                    }
                                }
                            }
                        }
                    }
                }

                Grupo {
                    visible: panel.pagina === "general"
                    titulo: qsTr("paneles")

                    Interruptor {
                        titulo: qsTr("panel de detalle")
                        detalle: qsTr("también con Ctrl+I")
                        puesto: ajustes.verInspector
                        onCambiado: function (v) { ajustes.verInspector = v }
                    }
                    Fila {
                        titulo: qsTr("anchos de los paneles")
                        detalle: ajustes.anchoLateral > 0 || ajustes.anchoInspector > 0
                                 ? qsTr("los has movido tú; esto los deja como vienen en el tema")
                                 : qsTr("los que vienen en el tema; se cambian arrastrando su borde")
                        Boton {
                            centrado: false
                            icono: "restablecer"
                            texto: qsTr("los del tema")
                            enabled: ajustes.anchoLateral > 0 || ajustes.anchoInspector > 0
                            opacity: enabled ? 1 : 0.4
                            onPulsado: {
                                ajustes.anchoLateral = 0
                                ajustes.anchoInspector = 0
                            }
                        }
                    }
                }

                // -------------------------------------------- galería
                Grupo {
                    visible: panel.pagina === "galeria"

                    Interruptor {
                        titulo: qsTr("nombres bajo las celdas")
                        puesto: ajustes.verNombres
                        onCambiado: function (v) { ajustes.verNombres = v }
                    }
                    Interruptor {
                        titulo: qsTr("vídeo al pasar el ratón")
                        detalle: qsTr("solo el de debajo del ratón se pone en marcha")
                        puesto: ajustes.videoAlPasar
                        onCambiado: function (v) { ajustes.videoAlPasar = v }
                    }
                    Interruptor {
                        titulo: qsTr("sonido en el panel de detalle")
                        detalle: qsTr("el vídeo que se ve a la derecha, con su sonido o callado")
                        puesto: ajustes.sonidoPanel
                        onCambiado: function (v) { ajustes.sonidoPanel = v }
                    }
                }

                Grupo {
                    visible: panel.pagina === "galeria"
                    titulo: qsTr("privacidad")

                    Interruptor {
                        titulo: qsTr("modo seguro")
                        detalle: qsTr("lo marcado como +18 sale difuminado, también al arrastrarlo")
                        puesto: ajustes.modoSeguro
                        onCambiado: function (v) { ajustes.modoSeguro = v }
                    }
                }

                // -------------------------------------------- pilas y editor
                // Lo que se contestó con «recordar» al tirar una principal con
                // versiones. Aquí se cambia, y «preguntar» lo olvida.
                Grupo {
                    visible: panel.pagina === "edicion"
                    titulo: qsTr("pilas")

                    Fila {
                        debajo: true
                        titulo: qsTr("al tirar una imagen con versiones")
                        detalle: qsTr("sola: la versión más pesada ocupa su sitio. Las dos cosas se deshacen con Ctrl+Z")
                        Segmentos {
                            opciones: [
                                { valor: "", texto: qsTr("preguntar") },
                                { valor: "sola", texto: qsTr("va sola") },
                                { valor: "todas", texto: qsTr("toda la pila") }
                            ]
                            valor: ajustes.borrarPila
                            onElegido: function (v) { ajustes.borrarPila = v }
                        }
                    }
                }

                // El imán del editor de vídeo: a cada cuánto se pegan el
                // inicio y el fin al arrastrarlos.
                Grupo {
                    visible: panel.pagina === "edicion"
                    titulo: qsTr("editor")

                    Fila {
                        debajo: true
                        titulo: qsTr("el imán al recortar")
                        detalle: qsTr("con el imán puesto, el inicio y el fin se pegan a múltiplos de este tiempo: con 0,5 s, un corte en 8,04 queda en 8,0")
                        Segmentos {
                            opciones: [0.1, 0.25, 0.5, 1, 2, 5].map(function (s) {
                                return { valor: s, texto: Textos.decimal(s) + " s" }
                            })
                            valor: ajustes.pasoIman
                            onElegido: function (v) { ajustes.pasoIman = v }
                        }
                    }
                }

                // -------------------------------------------- versiones
                Grupo {
                    visible: panel.pagina === "versiones"

                    Interruptor {
                        titulo: qsTr("avisar de versiones nuevas")
                        detalle: qsTr("una vez al día pregunta a grimorio.frederickandrade.com cuál es la última; no envía nada tuyo, y no baja ni instala nada si no pulsas «actualizar»")
                        puesto: ajustes.buscarVersiones
                        onCambiado: function (v) { ajustes.buscarVersiones = v }
                    }
                    Fila {
                        titulo: qsTr("la tuya es la %1").arg(versionGrimorio)
                        detalle: novedades.preguntando
                                 ? qsTr("preguntando…")
                                 : novedades.ultimaVez.getTime() > 0
                                   ? qsTr("última comprobación: %1").arg(
                                         novedades.ultimaVez.toLocaleString(Qt.locale(), Locale.ShortFormat))
                                   : qsTr("todavía no se ha comprobado")
                        Boton {
                            centrado: false
                            icono: "restablecer"
                            texto: qsTr("comprobar ahora")
                            enabled: !novedades.preguntando
                            opacity: enabled ? 1 : 0.4
                            onPulsado: novedades.comprobar()
                        }
                    }
                    Fila {
                        titulo: qsTr("novedades")
                        detalle: qsTr("qué trae cada versión, de la última a la primera")
                        pulsable: true
                        onPulsada: ventana.abrirNovedades(true)
                        Icono {
                            nombre: "derecha"
                            color: tema.textoTenue
                            width: Math.round(tema.fuente * 1.2)
                            height: width
                        }
                    }
                }

                // -------------------------------------------- acerca de
                Item {
                    visible: panel.pagina === "acerca"
                    width: parent.width
                    height: cabeceraAcerca.height + panel.aire

                    Column {
                        id: cabeceraAcerca
                        anchors.horizontalCenter: parent.horizontalCenter
                        y: panel.aire
                        spacing: Math.round(panel.aire * 0.6)
                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: Math.round(tema.fuente * 4.6)
                            height: width
                            radius: panel.esquina * 1.6
                            color: tema.fondo
                            border.color: tema.borde
                            Rectangle {
                                anchors.fill: parent
                                anchors.margins: 1
                                radius: parent.radius - 1
                                color: tema.seleccion
                                opacity: tema.realce
                            }
                            Icono {
                                anchors.centerIn: parent
                                nombre: "biblioteca"
                                color: tema.seleccion
                                grosor: 1.5
                                width: Math.round(parent.width * 0.56)
                                height: width
                            }
                        }
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "Grimorio"
                            color: tema.texto
                            font.pixelSize: Math.round(tema.fuente * 1.9)
                            font.weight: Font.DemiBold
                        }
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: qsTr("biblioteca visual de referencias")
                            color: tema.textoTenue
                            font.pixelSize: Math.round(tema.fuente * 1.0)
                        }
                    }
                }

                Grupo {
                    visible: panel.pagina === "acerca"
                    titulo: qsTr("versión")
                    Fila {
                        dato: true
                        titulo: qsTr("Grimorio")
                        Valor { text: versionGrimorio }
                    }
                    Fila {
                        dato: true
                        titulo: qsTr("Qt")
                        Valor { text: versionQt }
                    }
                    Fila {
                        dato: true
                        titulo: qsTr("biblioteca")
                        Valor { text: nucleo.nombre }
                    }
                    Fila {
                        titulo: qsTr("novedades: qué trae cada versión")
                        pulsable: true
                        onPulsada: ventana.abrirNovedades(true)
                        Icono {
                            nombre: "derecha"
                            color: tema.textoTenue
                            width: Math.round(tema.fuente * 1.2)
                            height: width
                        }
                    }
                }

                Grupo {
                    visible: panel.pagina === "acerca"
                    titulo: qsTr("quién")
                    Fila {
                        dato: true
                        titulo: qsTr("creado por")
                        Valor { text: "Frederick Andrade Pérez · dohkku" }
                    }
                    Fila {
                        dato: true
                        titulo: qsTr("desde")
                        Valor { text: qsTr("España") }
                    }
                    Fila {
                        dato: true
                        titulo: qsTr("licencia")
                        Valor { text: qsTr("© 2026 · AGPL-3.0, software libre") }
                    }
                }

                Grupo {
                    visible: panel.pagina === "acerca"
                    titulo: qsTr("cuatro promesas")
                    Repeater {
                        model: [
                            qsTr("Tus datos son tuyos y se leen: un item.json por elemento."),
                            qsTr("Nada de webview: ni Electron, ni Tauri."),
                            qsTr("El núcleo no sabe qué es una ventana."),
                            qsTr("El estilo es dato: todo sale de un tema.")
                        ]
                        delegate: Item {
                            id: promesa
                            required property string modelData
                            required property int index
                            width: parent ? parent.width : 0
                            height: Math.max(textoPromesa.height, numero.height) + panel.aire * 2
                            Rectangle {
                                visible: promesa.index > 0
                                anchors { left: parent.left; right: parent.right; top: parent.top }
                                anchors.leftMargin: panel.aire * 1.2
                                anchors.rightMargin: panel.aire * 1.2
                                height: 1
                                color: tema.borde
                            }
                            Rectangle {
                                id: numero
                                x: panel.aire * 1.2
                                anchors.verticalCenter: parent.verticalCenter
                                width: Math.round(tema.fuente * 1.7)
                                height: width
                                radius: width / 2
                                color: "transparent"
                                border.color: tema.seleccion
                                Text {
                                    anchors.centerIn: parent
                                    text: promesa.index + 1
                                    color: tema.seleccion
                                    font.pixelSize: Math.round(tema.fuente * 0.85)
                                    font.weight: Font.DemiBold
                                }
                            }
                            Text {
                                id: textoPromesa
                                anchors.verticalCenter: parent.verticalCenter
                                x: numero.x + numero.width + panel.aire
                                width: parent.width - x - panel.aire * 1.2
                                text: promesa.modelData
                                color: tema.texto
                                font.pixelSize: tema.fuente
                                wrapMode: Text.Wrap
                            }
                        }
                    }
                }
            }
        }

        // La barrita de dónde se está, solo si hay más de lo que cabe.
        Rectangle {
            visible: hoja.contentHeight > hoja.height + 1
            anchors.right: parent.right
            anchors.rightMargin: 3
            y: hoja.visibleArea.yPosition * hoja.height
            width: Math.max(3, Math.round(tema.fuente * 0.25))
            height: hoja.visibleArea.heightRatio * hoja.height
            radius: width / 2
            color: tema.textoTenue
            opacity: hoja.moving ? 0.7 : 0.3
            Behavior on opacity { NumberAnimation { duration: 200 } }
        }

        BotonIcono {
            centrado: false
            anchors { top: parent.top; right: parent.right; margins: panel.aire }
            icono: "cerrar"
            pista: qsTr("cerrar (Esc)")
            onPulsado: panel.cerrar()
        }
    }
}
