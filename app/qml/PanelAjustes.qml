// Los ajustes de quien está delante, y quién hizo esto.
//
// Una tarjeta en medio, como la de las etiquetas, con dos páginas a la
// izquierda. Todo se aplica al tocarlo —no hay «guardar»—: lo que se mueve
// aquí se ve detrás en el acto, que es la mejor vista previa que hay. Se
// guarda en `~/.config/Grimorio/Grimorio.conf` (ver `ajustes.h`), no en el
// tema: el tema es dato del proyecto y esto es de cada uno.
//
//   Ctrl+,   abrir y cerrar
import QtQuick

Item {
    id: panel
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    /// "general" o "acerca".
    property string pagina: "general"

    // Esc lo cierra desde el atajo de la ventana, que es quien se lleva la
    // tecla antes que nadie.
    function abrir(p) {
        pagina = p
        abierto = true
    }
    function cerrar() {
        abierto = false
    }

    /// Los pasos del tamaño de la interfaz. Sueltos y no un deslizador: entre
    /// 100 y 103 % no se ve diferencia y el deslizador invita a buscarla.
    readonly property var escalas: [0.8, 0.9, 1.0, 1.1, 1.2, 1.35, 1.5]
    function pasoEscala(dir) {
        const e = ajustes.escala
        let i = 0
        for (let k = 0; k < escalas.length; k++)
            if (Math.abs(escalas[k] - e) < Math.abs(escalas[i] - e)) i = k
        i = Math.max(0, Math.min(escalas.length - 1, i + dir))
        ajustes.escala = escalas[i]
    }

    // Un interruptor con su rótulo y su explicación debajo.
    component Interruptor: Item {
        id: inter
        property string titulo: ""
        property string detalle: ""
        property bool puesto: false
        signal cambiado(bool v)

        width: parent ? parent.width : 0
        height: Math.max(textos.height, llave.height) + tema.hueco * 0.8

        Column {
            id: textos
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - llave.width - tema.hueco
            spacing: tema.hueco * 0.15
            Text {
                width: parent.width
                text: inter.titulo
                color: tema.texto
                font.pixelSize: tema.fuente
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                visible: inter.detalle.length > 0
                text: inter.detalle
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.85
                wrapMode: Text.Wrap
            }
        }

        Rectangle {
            id: llave
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: Math.round(tema.fuente * 2.6)
            height: Math.round(tema.fuente * 1.45)
            radius: height / 2
            color: inter.puesto ? tema.seleccion : tema.fondo
            border.color: inter.puesto ? tema.seleccion : tema.borde
            border.width: 1
            Behavior on color { ColorAnimation { duration: 120 } }

            Rectangle {
                width: parent.height - 6
                height: width
                radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                x: inter.puesto ? parent.width - width - 3 : 3
                color: inter.puesto ? tema.fondo : tema.textoTenue
                Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: inter.cambiado(!inter.puesto)
        }
    }

    // Un rótulo de sección, pequeño y en mayúsculas, como los del panel.
    component Seccion: Text {
        width: parent ? parent.width : 0
        topPadding: tema.hueco
        bottomPadding: tema.hueco * 0.3
        color: tema.textoTenue
        font.pixelSize: tema.fuente * 0.78
        font.letterSpacing: tema.fuente * 0.08
        font.capitalization: Font.AllUppercase
    }

    // Una fila «qué · valor» de la página de acerca de.
    component Dato: Item {
        property string que: ""
        property string valor: ""
        width: parent ? parent.width : 0
        height: valorTexto.height + tema.hueco * 0.5
        Text {
            width: Math.round(tema.fuente * 8)
            text: parent.que
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.9
        }
        Text {
            id: valorTexto
            x: Math.round(tema.fuente * 8)
            width: parent.width - x
            text: parent.valor
            color: tema.texto
            font.pixelSize: tema.fuente * 0.9
            wrapMode: Text.Wrap
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
        width: Math.min(tema.fuente * 54, panel.width * 0.9)
        height: Math.min(tema.fuente * 40, panel.height * 0.88)
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        clip: true

        MouseArea { anchors.fill: parent }

        // ------------------------------------------------------ la columna
        Rectangle {
            id: indice
            anchors { top: parent.top; bottom: parent.bottom; left: parent.left }
            anchors.margins: 1
            width: Math.round(tema.fuente * 13)
            color: tema.fondo
            radius: tema.radio

            Column {
                anchors.fill: parent
                anchors.margins: tema.hueco
                spacing: tema.hueco * 0.3

                Text {
                    leftPadding: tema.hueco * 0.5
                    bottomPadding: tema.hueco
                    topPadding: tema.hueco * 0.4
                    text: qsTr("Ajustes")
                    color: tema.texto
                    font.pixelSize: tema.fuente * 1.15
                    font.weight: Font.DemiBold
                }

                Repeater {
                    model: [
                        { clave: "general", texto: qsTr("general"), icono: "engranaje" },
                        { clave: "acerca", texto: qsTr("acerca de"), icono: "info" }
                    ]
                    delegate: Boton {
                        required property var modelData
                        centrado: false
                        recortar: true
                        width: parent.width
                        discreto: panel.pagina !== modelData.clave
                        activo: panel.pagina === modelData.clave
                        icono: modelData.icono
                        texto: modelData.texto
                        onPulsado: panel.pagina = modelData.clave
                    }
                }
            }

            Text {
                anchors { left: parent.left; bottom: parent.bottom; margins: tema.hueco * 1.5 }
                text: "Ctrl+,"
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.8
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
            id: hoja
            anchors {
                top: parent.top; bottom: parent.bottom
                left: indice.right; right: parent.right
                topMargin: tema.margen; bottomMargin: tema.margen
                leftMargin: tema.margen * 1.2; rightMargin: tema.margen * 1.2
            }
            contentHeight: panel.pagina === "general" ? general.height : acerca.height
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            // -------------------------------------------------- general
            Column {
                id: general
                visible: panel.pagina === "general"
                width: hoja.width

                // La versión nueva, lo primero de todo y con el color de la
                // selección: abajo del todo, como una fila más de «versiones»,
                // no la veía nadie.
                // Aire arriba: la × de cerrar el panel cae en esa esquina.
                Item {
                    width: 1
                    height: tarjetaVersion.visible ? Math.round(tema.hueco * 1.4) : 0
                }
                Rectangle {
                    id: tarjetaVersion
                    visible: novedades.nueva.length > 0 || novedades.fase.length > 0
                    width: parent.width
                    height: visible ? contenidoVersion.height + tema.hueco * 2.4 : 0
                    radius: tema.radio * 1.5
                    color: "transparent"
                    border.color: tema.seleccion
                    border.width: Math.max(1, Math.round(tema.fuente * 0.12))

                    readonly property bool enMarcha: novedades.fase.length > 0

                    Rectangle {
                        anchors.fill: parent
                        radius: parent.radius
                        color: tema.seleccion
                        opacity: tema.realce * 1.5
                    }

                    Column {
                        id: contenidoVersion
                        anchors { left: parent.left; right: parent.right; top: parent.top }
                        anchors.margins: tema.hueco * 1.2
                        spacing: tema.hueco * 0.6

                        Row {
                            spacing: tema.hueco * 0.6
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
                                font.pixelSize: tema.fuente * 1.3
                                font.weight: Font.DemiBold
                            }
                        }
                        Text {
                            width: parent.width
                            wrapMode: Text.WordWrap
                            text: tarjetaVersion.enMarcha
                                  ? (novedades.fase === "bajando"
                                     ? (novedades.progreso >= 0
                                        ? qsTr("bajando… %1 %").arg(Math.round(novedades.progreso * 100))
                                        : qsTr("bajando…"))
                                     : qsTr("instalando; Grimorio se cierra y se vuelve a abrir solo"))
                                  : novedades.instalable
                                    ? qsTr("tienes la %1. Se baja de las publicadas, se comprueba que es la misma y se instala; Grimorio se vuelve a abrir solo, con esta biblioteca.").arg(novedades.actual)
                                    : qsTr("tienes la %1. Se abre la página de descargas para bajar la nueva.").arg(novedades.actual)
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.95
                        }

                        // Si el intento anterior falló, aquí y no solo abajo: la
                        // barra de estado queda tapada por este panel.
                        Text {
                            visible: novedades.error.length > 0 && !tarjetaVersion.enMarcha
                            width: parent.width
                            wrapMode: Text.WordWrap
                            text: qsTr("no se ha podido actualizar: %1").arg(novedades.error)
                            color: tema.seleccion
                            font.pixelSize: tema.fuente * 0.95
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
                            spacing: tema.hueco * 0.6
                            visible: !tarjetaVersion.enMarcha
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

                Seccion { topPadding: tarjetaVersion.visible ? tema.hueco * 1.5 : 0; text: qsTr("apariencia") }

                Item {
                    width: parent.width
                    height: filaTema.height + tema.hueco * 0.8
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("tema")
                        color: tema.texto
                        font.pixelSize: tema.fuente
                    }
                    Row {
                        id: filaTema
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                        spacing: tema.hueco * 0.4
                        Repeater {
                            model: [
                                { clave: "oscuro", texto: qsTr("noche") },
                                { clave: "papel", texto: qsTr("papel") }
                            ]
                            delegate: Boton {
                                required property var modelData
                                texto: modelData.texto
                                activo: ajustes.tema === modelData.clave
                                onPulsado: ajustes.tema = modelData.clave
                            }
                        }
                    }
                }

                Item {
                    width: parent.width
                    height: filaEscala.height + tema.hueco * 0.8
                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - filaEscala.width - tema.hueco
                        spacing: tema.hueco * 0.15
                        Text {
                            text: qsTr("tamaño de la interfaz")
                            color: tema.texto
                            font.pixelSize: tema.fuente
                        }
                        Text {
                            width: parent.width
                            text: qsTr("letra, iconos y paneles; las miniaturas van aparte, con Ctrl+±")
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.85
                            wrapMode: Text.Wrap
                        }
                    }
                    Row {
                        id: filaEscala
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                        spacing: tema.hueco * 0.4
                        Boton {
                            icono: "menos"
                            onPulsado: panel.pasoEscala(-1)
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            width: Math.round(tema.fuente * 3.6)
                            horizontalAlignment: Text.AlignHCenter
                            text: Math.round(ajustes.escala * 100) + " %"
                            color: tema.texto
                            font.pixelSize: tema.fuente
                        }
                        Boton {
                            icono: "mas"
                            onPulsado: panel.pasoEscala(1)
                        }
                        Boton {
                            discreto: true
                            visible: Math.abs(ajustes.escala - 1) > 0.001
                            texto: qsTr("100 %")
                            onPulsado: ajustes.escala = 1
                        }
                    }
                }

                Seccion { text: qsTr("galería") }

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
                    titulo: qsTr("modo seguro")
                    detalle: qsTr("lo marcado como +18 sale difuminado, también al arrastrarlo")
                    puesto: ajustes.modoSeguro
                    onCambiado: function (v) { ajustes.modoSeguro = v }
                }

                Seccion { text: qsTr("paneles") }

                Interruptor {
                    titulo: qsTr("panel de detalle")
                    detalle: qsTr("también con Ctrl+I")
                    puesto: ajustes.verInspector
                    onCambiado: function (v) { ajustes.verInspector = v }
                }
                Item {
                    width: parent.width
                    height: restablecer.height + tema.hueco * 0.8
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("anchos de los paneles")
                        color: tema.texto
                        font.pixelSize: tema.fuente
                    }
                    Boton {
                        id: restablecer
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                        texto: qsTr("los del tema")
                        enabled: ajustes.anchoLateral > 0 || ajustes.anchoInspector > 0
                        opacity: enabled ? 1 : 0.4
                        onPulsado: {
                            ajustes.anchoLateral = 0
                            ajustes.anchoInspector = 0
                        }
                    }
                }

                Seccion { text: qsTr("versiones") }

                Interruptor {
                    titulo: qsTr("avisar de versiones nuevas")
                    detalle: qsTr("una vez al día pregunta a grimorio.frederickandrade.com cuál es la última; no envía nada tuyo, y no baja ni instala nada si no pulsas «actualizar»")
                    puesto: ajustes.buscarVersiones
                    onCambiado: function (v) { ajustes.buscarVersiones = v }
                }
                Item {
                    width: parent.width
                    height: comprobarAhora.height + tema.hueco * 0.8
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - comprobarAhora.width - tema.hueco
                        elide: Text.ElideRight
                        text: novedades.preguntando
                              ? qsTr("preguntando…")
                              : novedades.ultimaVez.getTime() > 0
                                ? qsTr("última comprobación: %1").arg(
                                      novedades.ultimaVez.toLocaleString(Qt.locale(), Locale.ShortFormat))
                                : qsTr("todavía no se ha comprobado")
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente * 0.9
                    }
                    Boton {
                        id: comprobarAhora
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                        texto: qsTr("comprobar ahora")
                        enabled: !novedades.preguntando
                        opacity: enabled ? 1 : 0.4
                        onPulsado: novedades.comprobar()
                    }
                }
            }

            // -------------------------------------------------- acerca de
            Column {
                id: acerca
                visible: panel.pagina === "acerca"
                width: hoja.width
                spacing: tema.hueco * 0.4

                Row {
                    spacing: tema.hueco
                    Rectangle {
                        width: Math.round(tema.fuente * 3.4)
                        height: width
                        radius: tema.radio * 1.5
                        color: tema.fondo
                        border.color: tema.borde
                        Icono {
                            anchors.centerIn: parent
                            nombre: "biblioteca"
                            color: tema.seleccion
                            width: Math.round(parent.width * 0.55)
                            height: width
                        }
                    }
                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: tema.hueco * 0.2
                        Text {
                            text: "Grimorio"
                            color: tema.texto
                            font.pixelSize: tema.fuente * 1.6
                            font.weight: Font.DemiBold
                        }
                        Text {
                            text: qsTr("biblioteca visual de referencias")
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.95
                        }
                    }
                }

                Seccion { text: qsTr("versión") }
                Dato { que: qsTr("Grimorio"); valor: versionGrimorio }
                Boton {
                    centrado: false
                    icono: "info"
                    texto: qsTr("novedades: qué trae cada versión")
                    onPulsado: ventana.abrirNovedades(true)
                }
                Dato { que: qsTr("Qt"); valor: versionQt }
                Dato { que: qsTr("biblioteca"); valor: nucleo.nombre }

                Seccion { text: qsTr("quién") }
                Dato { que: qsTr("creado por"); valor: "Frederick Andrade Pérez · dohkku" }
                Dato { que: qsTr("desde"); valor: qsTr("España") }
                Dato { que: qsTr("licencia"); valor: qsTr("© 2026 · AGPL-3.0, software libre") }

                Seccion { text: qsTr("cinco promesas") }
                Repeater {
                    model: [
                        qsTr("Tus datos son tuyos y se leen: un item.json por elemento."),
                        qsTr("Nada de webview: ni Electron, ni Tauri."),
                        qsTr("El núcleo no sabe qué es una ventana."),
                        qsTr("El estilo es dato: todo sale de un tema."),
                        qsTr("Sin funciones de IA.")
                    ]
                    delegate: Row {
                        required property string modelData
                        required property int index
                        width: acerca.width
                        spacing: tema.hueco * 0.6
                        Text {
                            text: (index + 1) + "."
                            color: tema.seleccion
                            font.pixelSize: tema.fuente * 0.9
                        }
                        Text {
                            width: parent.width - tema.fuente * 2
                            text: modelData
                            color: tema.texto
                            font.pixelSize: tema.fuente * 0.9
                            wrapMode: Text.Wrap
                        }
                    }
                }
            }
        }
    }
}
