// La barra de arriba del centro: qué se está mirando y cómo.
//
// A la izquierda, el nombre de lo que se mira —«Todo», una carpeta, una
// dinámica— y cuántos elementos tiene. A la derecha, en este orden, lo que
// cambia lo que se ve: el buscador, la vista (justificado, cuadrícula, lista),
// el orden y los filtros; y, apartado por una raya, lo que no cambia la vista
// sino lo que entra o cómo se enseña: importar, el modo seguro, el vídeo al
// pasar y el panel de detalle.
//
// La biblioteca ya no está aquí: es de dónde se mira, y eso es la barra
// lateral.
//
// Los filtros van plegados en una segunda fila. Antes estaban siempre a la
// vista, y el razonamiento era bueno —lo que no se ve no se usa—, así que el
// botón que los abre dice cuántos hay puestos: plegados no esconden nada que
// esté actuando. Cada chip escribe en la misma línea del buscador.
import QtQuick
import "textos.js" as Textos
import "consulta.js" as Consulta

Rectangle {
    id: barra
    readonly property real altoArriba: ventana.altoBarra
    readonly property real altoFiltros: Math.round(tema.fuente * 2.9)
    height: altoArriba + (ventana.verFiltros ? altoFiltros : 0)
    Behavior on height { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
    color: tema.fondo
    clip: true

    /// El buscador, para colgar de él el ayudante.
    readonly property Item cajaBusqueda: caja
    readonly property TextInput campoBusqueda: campo

    /// Cuántos filtros de la fila hay puestos, para decirlo en su botón.
    readonly property int filtrosPuestos: {
        const t = ventana.textoBusqueda
        let n = 0
        const campos = ["tipo", "color", "fecha", "estrellas", "orientacion", "repetidos"]
        for (let i = 0; i < campos.length; i++)
            if (Consulta.leer(t, campos[i]).length > 0) n += 1
        return n
    }

    function enfocarBusqueda() {
        campo.forceActiveFocus()
        campo.selectAll()
    }

    Rectangle {
        anchors { left: parent.left; right: parent.right; top: parent.top }
        anchors.topMargin: barra.altoArriba - 1
        height: 1
        color: tema.borde
    }

    // ------------------------------------------------------ qué se mira
    Row {
        id: titulo
        x: tema.margen
        height: barra.altoArriba
        spacing: tema.hueco * 0.8
        // Lo que sobre después de lo de la derecha; el nombre se acorta antes
        // de empujar al buscador.
        readonly property real cabe: Math.max(tema.fuente * 4, derecha.x - x - tema.hueco * 2)

        Text {
            id: rotulo
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, titulo.cabe - cuenta.implicitWidth - titulo.spacing
                            - (guardarFiltro.visible ? guardarFiltro.width + titulo.spacing : 0))
            elide: Text.ElideRight
            text: ventana.tituloVista
            color: tema.texto
            font.pixelSize: tema.fuente * 1.15
            font.weight: Font.DemiBold
        }
        Text {
            id: cuenta
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: tema.fuente * 0.05
            text: Textos.elementos(modelo.total)
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.92
        }
        // Guardar lo que se ve, al lado de su nombre: es donde se mira cuando
        // se acaba de filtrar. El «+» de la barra lateral hace lo mismo, pero
        // nadie lo relacionaba con el filtro que tenía puesto.
        Boton {
            id: guardarFiltro
            anchors.verticalCenter: parent.verticalCenter
            visible: ventana.hayFiltroGuardable
            altura: Math.round(tema.fuente * 1.9)
            icono: "dinamica"
            // Con poco sitio, solo el icono y la frase en su pista: el nombre
            // de lo que se mira no se puede quedar en nada por un botón.
            readonly property bool holgado: titulo.cabe > rotulo.implicitWidth + cuenta.implicitWidth
                                                        + tema.fuente * 14
            texto: holgado ? qsTr("guardar como carpeta") : ""
            onEncimaChanged: encima && !holgado
                             ? ventana.pistas.mostrar(guardarFiltro, qsTr("guardar como carpeta dinámica"))
                             : ventana.pistas.ocultar(guardarFiltro)
            onPulsado: ventana.guardarComoDinamica()
        }
    }

    // ------------------------------------------------------ cómo se mira
    Row {
        id: derecha
        anchors.right: parent.right
        anchors.rightMargin: tema.hueco
        height: barra.altoArriba
        spacing: tema.hueco * 0.6

        // El buscador. Más estrecho que cuando iba centrado en la ventana: el
        // ayudante de debajo sí es ancho, que es donde hace falta leer.
        Rectangle {
            id: caja
            anchors.verticalCenter: parent.verticalCenter
            width: Math.max(tema.fuente * 12, Math.min(tema.fuente * 22, barra.width * 0.3))
            height: Math.round(tema.fuente * 2.3)
            radius: tema.radio
            color: tema.panel
            border.color: campo.activeFocus ? tema.seleccion : (sobreCaja.containsMouse ? tema.textoTenue : tema.borde)
            border.width: 1
            Behavior on border.color { ColorAnimation { duration: 90 } }

            MouseArea {
                id: sobreCaja
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.IBeamCursor
                onClicked: campo.forceActiveFocus()
            }

            Icono {
                id: lupa
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: parent.left
                anchors.leftMargin: tema.hueco * 0.7
                nombre: "buscar"
                color: campo.activeFocus ? tema.seleccion : tema.textoTenue
                width: Math.round(tema.fuente * 1.15)
                height: width
            }

            TextInput {
                id: campo
                // El campo y el filtro son la misma cosa: si se dejan sueltos,
                // se puede acabar con la malla filtrada y la caja vacía.
                text: ventana.textoBusqueda
                anchors.left: lupa.right
                anchors.leftMargin: tema.hueco * 0.5
                anchors.right: vaciar.visible ? vaciar.left : parent.right
                anchors.rightMargin: tema.hueco * 0.5
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                verticalAlignment: TextInput.AlignVCenter
                clip: true
                color: tema.texto
                selectionColor: tema.seleccion
                selectedTextColor: tema.fondo
                font.pixelSize: tema.fuente
                // Buscar mientras se escribe, pero no en cada tecla: con
                // 100.000 elementos, una consulta por pulsación deja la
                // ventana pegajosa sin que se note por qué.
                onTextEdited: rebote.restart()
                // El ayudante de debajo se maneja desde aquí: el foco no sale del
                // buscador mientras se elige una etiqueta.
                Keys.onUpPressed: ventana.ayudante.mover(-1)
                Keys.onDownPressed: ventana.ayudante.mover(1)
                onAccepted: {
                    if (ventana.ayudante.aceptar()) {
                        cursorPosition = text.length
                        return
                    }
                    rebote.stop()
                    ventana.textoBusqueda = campo.text
                    ventana.consultar()
                    ventana.ayudante.cerrado = true
                }
                Keys.onEscapePressed: {
                    // Primero se cierra el ayudante; otro Esc vacía la búsqueda.
                    if (ventana.ayudante.abierto) {
                        ventana.ayudante.cerrado = true
                        return
                    }
                    ventana.textoBusqueda = ""
                    ventana.consultar()
                    focus = false
                }

                Timer {
                    id: rebote
                    interval: 140
                    onTriggered: {
                        ventana.textoBusqueda = campo.text
                        ventana.consultar()
                    }
                }
            }

            Text {
                anchors.fill: campo
                verticalAlignment: Text.AlignVCenter
                visible: campo.text.length === 0 && !campo.activeFocus
                elide: Text.ElideRight
                text: qsTr("buscar, #etiqueta, tipo:vídeo…")
                color: tema.textoTenue
                font.pixelSize: tema.fuente
            }

            BotonIcono {
                id: vaciar
                anchors.verticalCenter: parent.verticalCenter
                anchors.right: parent.right
                anchors.rightMargin: tema.hueco * 0.3
                visible: campo.text.length > 0
                icono: "cerrar"
                pista: qsTr("vaciar la búsqueda (Esc)")
                onPulsado: {
                    ventana.textoBusqueda = ""
                    ventana.consultar()
                }
            }
        }

        // Las tres vistas, juntas en una pieza: se elige una de tres, no se
        // encienden y apagan por separado.
        Rectangle {
            id: vistas
            anchors.verticalCenter: parent.verticalCenter
            width: filaVistas.width + 2
            height: Math.round(tema.fuente * 2.3)
            radius: tema.radio
            color: "transparent"
            border.color: tema.borde
            border.width: 1

            Row {
                id: filaVistas
                anchors.centerIn: parent
                Repeater {
                    model: [
                        { modo: 1, icono: "justificado", pista: qsTr("justificado (Ctrl+L)") },
                        { modo: 0, icono: "cuadricula", pista: qsTr("cuadrícula (Ctrl+L)") },
                        { modo: 2, icono: "lista", pista: qsTr("lista (Ctrl+L)") }
                    ]
                    delegate: BotonIcono {
                        required property var modelData
                        altura: vistas.height - 2
                        width: Math.round(altura * 1.25)
                        icono: modelData.icono
                        pista: modelData.pista
                        activo: disposicion.modo === modelData.modo
                        onPulsado: disposicion.modo = modelData.modo
                    }
                }
            }
        }

        Chip {
            anchors.verticalCenter: parent.verticalCenter
            titulo: qsTr("orden")
            icono: "orden"
            campo: "orden"
            opciones: [
                { texto: qsTr("a mano"), valor: "" },
                { texto: qsTr("recientes"), valor: "recientes" },
                { texto: qsTr("nombre"), valor: "nombre" },
                { texto: qsTr("peso"), valor: "peso" },
                { texto: qsTr("estrellas"), valor: "estrellas" },
                { texto: qsTr("azar"), valor: "azar" }
            ]
        }

        Boton {
            anchors.verticalCenter: parent.verticalCenter
            icono: "ajustes"
            texto: barra.filtrosPuestos > 0 ? qsTr("filtros · %1").arg(barra.filtrosPuestos)
                                            : qsTr("filtros")
            altura: Math.round(tema.fuente * 2)
            discreto: !ventana.verFiltros && barra.filtrosPuestos === 0
            activo: ventana.verFiltros || barra.filtrosPuestos > 0
            onPulsado: ventana.verFiltros = !ventana.verFiltros
        }

        Rectangle { width: 1; height: tema.fuente * 1.4; color: tema.borde; anchors.verticalCenter: parent.verticalCenter }

        BotonIcono {
            id: botonImportar
            anchors.verticalCenter: parent.verticalCenter
            icono: "importar"
            pista: qsTr("importar, pegar, capturar…")
            onPulsado: ventana.abrirMenuImportar(botonImportar)
        }
        // En la papelera, borrar de verdad: el único botón de este programa
        // que no tiene vuelta atrás, y por eso solo aparece donde hace algo.
        Boton {
            anchors.verticalCenter: parent.verticalCenter
            texto: qsTr("vaciar papelera")
            altura: Math.round(tema.fuente * 2)
            visible: ventana.enPapelera && nucleo.tirados > 0
            onPulsado: ventana.pedirVaciarPapelera()
        }
        // El modo seguro, solo si hay algo marcado o si está puesto.
        BotonIcono {
            anchors.verticalCenter: parent.verticalCenter
            visible: ajustes.modoSeguro || nucleo.hayAdultos
            icono: ajustes.modoSeguro ? "escudo" : "ojo"
            activo: ajustes.modoSeguro
            pista: ajustes.modoSeguro ? qsTr("modo seguro: lo +18 sale difuminado")
                                      : qsTr("sin censura: se ve todo")
            onPulsado: ajustes.modoSeguro = !ajustes.modoSeguro
        }
        BotonIcono {
            anchors.verticalCenter: parent.verticalCenter
            icono: "video"
            activo: ajustes.videoAlPasar
            pista: ajustes.videoAlPasar ? qsTr("vídeo al pasar el ratón")
                                        : qsTr("vídeo quieto")
            onPulsado: ajustes.videoAlPasar = !ajustes.videoAlPasar
        }
        BotonIcono {
            anchors.verticalCenter: parent.verticalCenter
            icono: "panel"
            activo: ajustes.verInspector
            pista: ajustes.verInspector ? qsTr("plegar el panel de detalle (Ctrl+I)")
                                        : qsTr("abrir el panel de detalle (Ctrl+I)")
            onPulsado: ajustes.verInspector = !ajustes.verInspector
        }
        BotonIcono {
            anchors.verticalCenter: parent.verticalCenter
            icono: "engranaje"
            pista: qsTr("ajustes (Ctrl+,)")
            onPulsado: ventana.abrirAjustes()
        }
    }

    // --------------------------------------------------------- los filtros
    Item {
        id: filtros
        anchors { left: parent.left; right: parent.right }
        y: barra.altoArriba
        height: barra.altoFiltros

        Rectangle {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: 1
            color: tema.borde
        }

        Row {
            anchors.verticalCenter: parent.verticalCenter
            x: tema.margen
            spacing: tema.hueco * 0.5

            Chip {
                titulo: qsTr("tipo")
                icono: "carpeta"
                campo: "tipo"
                opciones: [
                    { texto: qsTr("todo"), valor: "", icono: "todo" },
                    { texto: qsTr("imágenes"), valor: "imagen", icono: "cuadricula" },
                    { texto: qsTr("vídeos"), valor: "video", icono: "video" },
                    { texto: qsTr("audio"), valor: "audio", icono: "orden" },
                    { texto: qsTr("3D"), valor: "modelo", icono: "forma" },
                    { texto: qsTr("documentos"), valor: "documento", icono: "pegar" },
                    { texto: qsTr("fuentes"), valor: "tipografia", icono: "etiqueta" },
                    { texto: qsTr("RAW"), valor: "raw", icono: "captura" }
                ]
            }
            Chip {
                titulo: qsTr("color")
                icono: "color"
                campo: "color"
                modo: "color"
            }
            Chip {
                titulo: qsTr("importado")
                icono: "fecha"
                campo: "fecha"
                opciones: [
                    { texto: qsTr("cuando sea"), valor: "" },
                    { texto: qsTr("hoy"), valor: "hoy" },
                    { texto: qsTr("ayer"), valor: "ayer" },
                    { texto: qsTr("7 días"), valor: "7d" },
                    { texto: qsTr("30 días"), valor: "30d" },
                    { texto: qsTr("este año"), valor: "año" }
                ]
            }
            Chip {
                titulo: qsTr("estrellas")
                icono: "estrella"
                campo: "estrellas"
                opciones: [
                    { texto: qsTr("todas"), valor: "" },
                    { texto: qsTr("sin estrellas"), valor: "0" },
                    { texto: "≥ 3", valor: ">=3" },
                    { texto: "≥ 4", valor: ">=4" },
                    { texto: "5", valor: "5" }
                ]
            }
            Chip {
                titulo: qsTr("forma")
                icono: "forma"
                campo: "orientacion"
                opciones: [
                    { texto: qsTr("cualquiera"), valor: "" },
                    { texto: qsTr("apaisada"), valor: "apaisada" },
                    { texto: qsTr("vertical"), valor: "vertical" },
                    { texto: qsTr("cuadrada"), valor: "cuadrada" }
                ]
            }
            Chip {
                titulo: qsTr("repetidos")
                icono: "repetidos"
                campo: "repetidos"
                interruptor: "si"
            }
            // Mirando repetidos, la limpieza a un clic (solo copias exactas).
            Boton {
                altura: Math.round(tema.fuente * 2)
                visible: Consulta.leer(ventana.textoBusqueda, "repetidos") === "si"
                texto: qsTr("quitar copias exactas")
                onPulsado: nucleo.quitarCopiasExactas()
            }
        }

        // Cómo se pinta cada celda: con su nombre o sin él, y de qué tamaño.
        Row {
            anchors.verticalCenter: parent.verticalCenter
            anchors.right: parent.right
            anchors.rightMargin: tema.hueco
            spacing: tema.hueco * 0.35

            Boton {
                altura: Math.round(tema.fuente * 2)
                discreto: true
                texto: qsTr("nombres")
                activo: ajustes.verNombres
                visible: disposicion.modo !== 2
                onPulsado: ajustes.verNombres = !ajustes.verNombres
            }
            BotonIcono {
                icono: "menos"
                visible: disposicion.modo !== 2
                pista: qsTr("celdas más pequeñas (Ctrl+−)")
                onPulsado: ventana.celda = Math.max(80, ventana.celda - 20)
            }
            BotonIcono {
                icono: "mas"
                visible: disposicion.modo !== 2
                pista: qsTr("celdas más grandes (Ctrl+=)")
                onPulsado: ventana.celda = Math.min(420, ventana.celda + 20)
            }
        }
    }
}
