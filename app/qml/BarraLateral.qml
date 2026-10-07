// La barra lateral: la biblioteca, las vistas fijas, las carpetas dinámicas
// y el árbol de carpetas, con el peso de la biblioteca al pie.
//
// Una carpeta aquí es una etiqueta con jerarquía, no un sitio donde vive el
// archivo: un elemento puede estar en varias. Pero arrastrar de una carpeta a
// otra **mueve**, porque eso es lo que quiere decir quien arrastra.
//
// De arriba abajo:
//
//   * el nombre de la biblioteca, que es también la puerta a las demás. Vive
//     aquí y no en la barra de arriba porque la barra de arriba es de lo que se
//     mira, y esto es de dónde se mira.
//   * las vistas fijas: «Todo», «Sin etiquetar», «Recientes» y la papelera.
//     Son búsquedas escritas en el buscador como cualquier otra —el buscador
//     sigue siendo lo único que decide qué se ve—, pero con sitio fijo porque
//     son las cuatro que se usan todos los días. No parecen carpetas a
//     propósito: no se renombran ni se arrastran.
//
//     «Todo» es además **no tener ninguna carpeta elegida**: se vuelve también
//     pinchando la carpeta elegida otra vez, o el hueco de debajo del árbol.
//   * las carpetas dinámicas, que son filtros guardados con nombre: lo que
//     hay dentro se calcula al abrirlas.
//   * el árbol. Cada carpeta lleva su punto de color, que se elige en su menú.
//     Crear es el «+» de la cabecera, el «+» que asoma al pasar por una
//     carpeta (la crea dentro) o Ctrl+Mayús+N, y se escribe en una fila que
//     sale donde va a quedar (ver `FilaNueva.qml`).
//   * el pie: cuánto hay y cuánto pesa, con el reparto por tipo en una raya.
//
// El hueco de debajo del árbol no es decoración: soltar ahí es la única manera
// de sacar algo de una carpeta sin meterlo en otra, y de devolver una carpeta a
// la raíz.

import QtQuick
import "textos.js" as Textos

Rectangle {
    id: lateral
    color: tema.panel

    Rectangle {
        anchors { top: parent.top; bottom: parent.bottom; right: parent.right }
        width: 1
        color: tema.borde
        z: 3
    }

    // ------------------------------------------------------ crear carpetas
    /// Si se está escribiendo el nombre de una carpeta nueva, y de quién
    /// colgará ("" es la raíz).
    property bool creando: false
    property string creandoDentro: ""

    function nuevaCarpeta(padre) {
        creandoDentro = padre
        creando = true
        Qt.callLater(editor.empezar)
    }

    function indiceDe(id) {
        const cs = nucleo.carpetas
        for (let i = 0; i < cs.length; i++) if (cs[i].id === id) return i
        return -1
    }

    readonly property real altoFila: Math.round(tema.fuente * 2.3)
    /// Las filas no van de borde a borde: con su margen y sus esquinas se leen
    /// como cosas que se pinchan, no como renglones de una tabla.
    readonly property real sangria: Math.round(tema.hueco * 0.6)

    /// Soltar en «Todo» o en el hueco: una carpeta vuelve a la raíz, unos
    /// elementos salen de la carpeta que se está mirando sin entrar en ninguna.
    /// Qué viaja se lee de la ventana: en un arrastre interno el `mimeData` no
    /// llega al otro lado. El porqué, en `Ventana.qml`.
    function soltarFuera(caida) {
        if (ventana.arrastreCarpeta.length > 0) {
            nucleo.moverCarpeta(ventana.arrastreCarpeta, "")
            caida.accept()
            return
        }
        // Sin carpeta de la que venir no hay de dónde sacarlos.
        if (ventana.arrastreOrigen.length === 0) return
        if (ventana.arrastreIds.length === 0) return
        nucleo.sacarDeCarpeta(ventana.arrastreIds, ventana.arrastreOrigen)
        caida.accept()
    }

    // Una fila de las de arriba: vista fija o carpeta dinámica.
    component Fila: Rectangle {
        id: fila
        property string icono: ""
        property string texto: ""
        property string cuenta: ""
        property bool elegida: false
        /// Se tapa la cuenta cuando asoma algo en su sitio (el «×» de borrar).
        property bool cuentaTapada: false
        readonly property bool encima: sobreFila.containsMouse
        signal pulsada()

        x: lateral.sangria
        width: lateral.width - lateral.sangria * 2 - 1
        height: lateral.altoFila
        radius: tema.radio
        color: elegida ? tema.marcador : (encima ? tema.borde : "transparent")
        Behavior on color { ColorAnimation { duration: 90 } }

        Icono {
            id: iconoFila
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: tema.hueco * 0.7
            width: Math.round(tema.fuente * 1.15)
            height: width
            nombre: fila.icono
            color: fila.elegida ? tema.texto : tema.textoTenue
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: iconoFila.right
            anchors.leftMargin: tema.hueco * 0.7
            anchors.right: cuentaFila.left
            anchors.rightMargin: tema.hueco * 0.5
            elide: Text.ElideRight
            text: fila.texto
            color: tema.texto
            font.pixelSize: tema.fuente
            font.weight: fila.elegida ? Font.DemiBold : Font.Normal
        }
        Text {
            id: cuentaFila
            anchors.verticalCenter: parent.verticalCenter
            anchors.right: parent.right
            anchors.rightMargin: tema.hueco * 0.7
            text: fila.cuenta
            color: tema.textoTenue
            opacity: fila.cuentaTapada ? 0 : 1
            font.pixelSize: tema.fuente * 0.88
        }
        MouseArea {
            id: sobreFila
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                // Tocar el panel se lleva el foco de la malla: si no, las
                // teclas siguen hablándole a las fotos.
                lateral.forceActiveFocus()
                fila.pulsada()
            }
        }
    }

    // Una cabecera de sección: rótulo en versalitas y, si se dice, un «+».
    component Cabecera: Item {
        id: cab
        property string texto: ""
        property string pista: ""
        property bool conMas: true
        signal mas()
        width: lateral.width
        height: Math.round(tema.fuente * 2.5)

        Text {
            anchors.left: parent.left
            anchors.leftMargin: lateral.sangria + tema.hueco * 0.7
            anchors.bottom: parent.bottom
            anchors.bottomMargin: tema.hueco * 0.45
            text: cab.texto.toUpperCase()
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.78
            font.weight: Font.Medium
            font.letterSpacing: tema.fuente * 0.06
        }
        BotonIcono {
            visible: cab.conMas
            anchors.right: parent.right
            anchors.rightMargin: lateral.sangria
            anchors.bottom: parent.bottom
            anchors.bottomMargin: tema.hueco * 0.1
            icono: "mas"
            pista: cab.pista
            onPulsado: cab.mas()
        }
    }

    // --------------------------------------------------------- la biblioteca
    // Con el alto de la barra de arriba, para que las dos rayas de debajo sean
    // una sola raya que cruza la ventana.
    Item {
        id: cabeza
        anchors { top: parent.top; left: parent.left; right: parent.right }
        height: ventana.altoBarra

        Rectangle {
            id: nombreBiblio
            anchors.verticalCenter: parent.verticalCenter
            x: lateral.sangria
            width: Math.min(parent.width - lateral.sangria * 2 - 1,
                            rotuloBiblio.implicitWidth + flecha.width + tema.hueco * 2)
            height: Math.round(tema.fuente * 2.3)
            radius: tema.radio
            color: sobreNombre.containsMouse ? tema.borde : "transparent"

            Text {
                id: rotuloBiblio
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: parent.left
                anchors.leftMargin: tema.hueco * 0.7
                width: Math.min(implicitWidth, nombreBiblio.width - flecha.width - tema.hueco * 1.6)
                elide: Text.ElideRight
                text: nucleo.nombre.length > 0 ? nucleo.nombre : qsTr("sin biblioteca")
                color: tema.texto
                font.pixelSize: tema.fuente * 1.05
                font.weight: Font.DemiBold
            }
            Icono {
                id: flecha
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: rotuloBiblio.right
                anchors.leftMargin: tema.hueco * 0.35
                nombre: "abajo"
                color: tema.textoTenue
                width: Math.round(tema.fuente * 0.9)
                height: width
            }
            MouseArea {
                id: sobreNombre
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    const p = nombreBiblio.mapToItem(ventana.contentItem, 0, nombreBiblio.height)
                    ventana.abrirMenuBiblioteca(p.x, p.y + tema.hueco * 0.3)
                }
            }
        }

        Rectangle {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: 1
            color: tema.borde
        }
    }

    // ------------------------------------------------------ las vistas fijas
    Column {
        id: vistas
        anchors { top: cabeza.bottom; left: parent.left; right: parent.right }
        anchors.topMargin: tema.hueco * 0.6
        spacing: 1

        Fila {
            id: todo
            icono: "todo"
            texto: qsTr("Todo")
            cuenta: nucleo.total > 0 ? Textos.numero(nucleo.total) : ""
            elegida: ventana.vistaFija === "todo"
            // Desde una carpeta dinámica o una vista fija, «Todo» es quitar
            // su búsqueda; desde una carpeta, salir de ella.
            onPulsada: {
                if (ventana.carpetaActual.length > 0) ventana.irACarpeta("")
                else ventana.aplicarBusqueda("")
            }
            color: soltarTodo.containsDrag ? tema.borde
                                           : (elegida ? tema.marcador : (encima ? tema.borde : "transparent"))

            DropArea {
                id: soltarTodo
                anchors.fill: parent
                keys: ["application/x-grimorio-ids", "application/x-grimorio-carpeta"]
                onDropped: function (caida) { lateral.soltarFuera(caida) }
            }
        }
        Fila {
            icono: "etiqueta"
            texto: qsTr("Sin etiquetar")
            elegida: ventana.vistaFija === "sinEtiquetar"
            onPulsada: ventana.aplicarBusqueda(elegida ? "" : ventana.consultaSinEtiquetar)
        }
        Fila {
            icono: "reloj"
            texto: qsTr("Recientes")
            elegida: ventana.vistaFija === "recientes"
            onPulsada: ventana.aplicarBusqueda(elegida ? "" : ventana.consultaRecientes)
        }
        Fila {
            icono: "papelera"
            texto: qsTr("Papelera")
            cuenta: nucleo.tirados > 0 ? Textos.numero(nucleo.tirados) : ""
            elegida: ventana.enPapelera
            onPulsada: ventana.verPapelera(!ventana.enPapelera)
        }
    }

    // ---------------------------------------------------- carpetas dinámicas
    //
    // Búsquedas con nombre (busquedas.rs). Encima de las carpetas porque se
    // usan igual —un clic y se ve lo que hay dentro— pero no son un árbol: ni
    // se anidan ni se arrastran, así que van en lista y aparte.
    property bool guardando: false
    /// Pide el nombre en la fila donde va a quedar la carpeta. La llama
    /// también el botón «guardar como carpeta» de la barra de arriba.
    function guardarDinamica() {
        guardando = true
        // Un tick después: la fila tiene que estar visible para darle el foco.
        empezarNombre.start()
    }
    Timer {
        id: empezarNombre
        interval: 0
        onTriggered: nombreBusqueda.empezar()
    }
    readonly property string busquedaActual: {
        if (ventana.carpetaActual.length > 0 || ventana.enPapelera) return ""
        const bs = nucleo.busquedas
        for (let i = 0; i < bs.length; i++)
            if (bs[i].consulta === ventana.textoBusqueda.trim()) return bs[i].id
        return ""
    }

    Cabecera {
        id: cabeceraDinamicas
        anchors.top: vistas.bottom
        texto: qsTr("carpetas dinámicas")
        // Sin nada escrito no hay qué guardar, y el botón lo dice en vez de no
        // hacer nada.
        pista: ventana.hayFiltroGuardable
               ? qsTr("guardar lo que se ve como carpeta dinámica")
               : qsTr("filtra o busca algo y guárdalo aquí")
        onMas: {
            if (!ventana.hayFiltroGuardable) {
                ventana.enfocarBusqueda()
                return
            }
            lateral.guardarDinamica()
        }
    }

    Column {
        id: dinamicas
        anchors { top: cabeceraDinamicas.bottom; left: parent.left; right: parent.right }
        spacing: 1

        Repeater {
            model: nucleo.busquedas
            delegate: Fila {
                id: filaB
                required property var modelData
                icono: "dinamica"
                texto: modelData.nombre
                cuenta: Textos.numero(modelData.cuenta)
                elegida: lateral.busquedaActual === modelData.id
                cuentaTapada: quitarB.visible
                // Pinchar la elegida vuelve a «Todo», como una carpeta.
                onPulsada: ventana.aplicarBusqueda(elegida ? "" : modelData.consulta)

                BotonIcono {
                    id: quitarB
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: tema.hueco * 0.2
                    visible: filaB.encima || encima
                    icono: "cerrar"
                    pista: qsTr("borrar «%1» (no borra nada de dentro)").arg(filaB.modelData.nombre)
                    onPulsado: nucleo.borrarBusqueda(filaB.modelData.id)
                }
            }
        }

        // Vacía, la sección se explica sola: «carpetas dinámicas» a secas no
        // dice que se llenan guardando un filtro.
        Text {
            visible: nucleo.busquedas.length === 0 && !lateral.guardando
            x: lateral.sangria + tema.hueco * 0.7
            width: lateral.width - x - lateral.sangria - tema.hueco
            topPadding: tema.hueco * 0.2
            bottomPadding: tema.hueco * 0.4
            wrapMode: Text.Wrap
            text: qsTr("Filtra o busca algo y pulsa «guardar como carpeta»: se rellena sola con lo que vaya encajando.")
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.85
            lineHeight: 1.15
        }

        FilaNueva {
            id: nombreBusqueda
            x: lateral.sangria
            width: lateral.width - lateral.sangria * 2 - 1
            visible: lateral.guardando
            height: visible ? lateral.altoFila : 0
            seguir: false
            pista: qsTr("nombre de la carpeta…")
            onNombrado: function (nombre) { nucleo.guardarBusqueda(nombre, ventana.textoBusqueda) }
            onAcabada: lateral.guardando = false
        }
    }

    // ------------------------------------------------------------ carpetas
    Cabecera {
        id: cabecera
        anchors.top: dinamicas.bottom
        texto: qsTr("carpetas")
        pista: qsTr("nueva carpeta  (Ctrl+Mayús+N)")
        onMas: lateral.nuevaCarpeta("")
    }

    ListView {
        id: arbol
        anchors { top: cabecera.bottom; left: parent.left; right: parent.right }
        // Lo que ocupe, no todo el panel: debajo tiene que quedar hueco de
        // verdad, porque ese hueco es «Todo» y es zona de soltar. Un `ListView`
        // estirado hasta abajo se queda con los clics de su parte vacía y no
        // deja que lleguen a nada.
        height: Math.max(0, Math.min(contentHeight,
                                     pie.y - cabecera.y - cabecera.height
                                     - (raiz.visible ? raiz.height : 0)))
        clip: true
        spacing: 1
        model: nucleo.carpetas
        boundsBehavior: Flickable.StopAtBounds

        // Cada fila deja debajo el sitio de la fila de escritura si es su
        // madre. La fila de escritura no va dentro: crear una carpeta rehace el
        // árbol entero, y con ella dentro se destruía a mitad de escribir.
        delegate: Item {
            id: ranura
            required property var modelData
            width: ListView.view.width
            readonly property bool esMadre: lateral.creando
                                            && lateral.creandoDentro === modelData.id
            height: fila.height + (esMadre ? lateral.altoFila : 0)

            FilaCarpeta {
                id: fila
                x: lateral.sangria
                width: parent.width - lateral.sangria * 2 - 1
                height: lateral.altoFila
                nombre: ranura.modelData.nombre
                cuenta: ranura.modelData.cuenta
                nivel: ranura.modelData.nivel
                idCarpeta: ranura.modelData.id
                padre: ranura.modelData.padre
                colorCarpeta: ranura.modelData.color || ""
                elegida: ventana.carpetaActual === ranura.modelData.id && !ventana.enPapelera
                // Pinchar la que ya está elegida vuelve a «Todo»: es la salida
                // que está siempre a mano, tenga el árbol el alto que tenga.
                onPulsada: ventana.irACarpeta(elegida ? "" : ranura.modelData.id)
                onNuevaDentro: lateral.nuevaCarpeta(ranura.modelData.id)
            }
        }
    }

    // Sitio de la fila de escritura cuando la carpeta va a la raíz: debajo de
    // la última, que es donde va a aparecer.
    Item {
        id: raiz
        anchors { top: arbol.bottom; left: parent.left; right: parent.right }
        height: lateral.altoFila
        visible: lateral.creando && lateral.creandoDentro.length === 0
    }

    FilaNueva {
        id: editor
        visible: lateral.creando
        x: lateral.sangria
        width: lateral.width - lateral.sangria * 2 - 1
        padre: lateral.creandoDentro
        readonly property int indice: lateral.creandoDentro.length > 0
                                      ? lateral.indiceDe(lateral.creandoDentro) : -1
        nivel: indice >= 0 ? nucleo.carpetas[indice].nivel + 1 : 0
        // Debajo de su madre, en el sitio que ella deja; si no está a la vista
        // (o es la raíz), al final del árbol.
        y: {
            // Se lee para que la posición se recalcule al rodar el árbol o al
            // crecer: `itemAtIndex` no avisa de nada por sí solo.
            void (arbol.contentY + arbol.contentHeight)
            const fila = indice >= 0 ? arbol.itemAtIndex(indice) : null
            if (fila) return arbol.y + fila.y - arbol.contentY + lateral.altoFila
            return raiz.y
        }
        z: 2
        onNombrado: function (nombre) { nucleo.crearCarpeta(nombre, lateral.creandoDentro) }
        onAcabada: lateral.creando = false
    }

    // El hueco: «Todo» sin escribirlo.
    Item {
        id: hueco
        anchors {
            top: raiz.visible ? raiz.bottom : arbol.bottom
            bottom: pie.top
            left: parent.left
            right: parent.right
        }

        Text {
            anchors.centerIn: parent
            width: parent.width - tema.hueco * 2
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            // Solo con el árbol vacío. Con carpetas ya creadas, el gesto está
            // aprendido y el rótulo sobra.
            visible: nucleo.carpetas.length === 0 && !lateral.creando
                     && parent.height > tema.fuente * 6
            text: qsTr("«+» arriba o Ctrl+Mayús+N para crear una carpeta")
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.9
        }

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: function (evento) {
                // Pinchar el panel se lleva el foco de la malla. Si no, seguir
                // escribiendo después de tocar aquí le hablaba a la malla, que
                // es la que tiene las teclas de estrellas y de papelera.
                lateral.forceActiveFocus()
                // El botón derecho sobre el hueco sigue creando, que es lo que
                // ya se sabía hacer, pero ahora en su sitio y sin menú.
                if (evento.button === Qt.RightButton) {
                    lateral.nuevaCarpeta("")
                    return
                }
                ventana.irACarpeta("")
            }
        }

        // Soltar en el hueco: una carpeta vuelve a la raíz, unos elementos
        // salen de la carpeta que se está mirando sin entrar en ninguna.
        DropArea {
            id: soltarHueco
            anchors.fill: parent
            keys: ["application/x-grimorio-ids", "application/x-grimorio-carpeta"]
            onDropped: function (caida) { lateral.soltarFuera(caida) }
        }

        // Que se vea que aquí se puede soltar: sin esto, el hueco parece el
        // final del panel y no un sitio.
        Rectangle {
            anchors.fill: parent
            color: tema.seleccion
            opacity: soltarHueco.containsDrag ? tema.realce * 2 : 0
            visible: opacity > 0
            Behavior on opacity { NumberAnimation { duration: 90 } }
        }
    }

    // --------------------------------------------------------------- el pie
    // Cuánto hay y cuánto pesa. La raya reparte por tipo: se ve de un vistazo
    // si la biblioteca es de fotos o de vídeos, que es lo que explica el peso.
    Item {
        id: pie
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: Math.round(tema.fuente * 3.6)

        readonly property var reparto: nucleo.reparto
        readonly property real bytes: {
            let b = 0
            for (let i = 0; i < reparto.length; i++) b += reparto[i].bytes
            return b
        }

        Rectangle {
            anchors { left: parent.left; right: parent.right; top: parent.top }
            height: 1
            color: tema.borde
        }

        Text {
            id: rotuloPie
            anchors.left: parent.left
            anchors.leftMargin: lateral.sangria + tema.hueco * 0.7
            anchors.right: parent.right
            anchors.rightMargin: lateral.sangria + tema.hueco * 0.7
            y: tema.hueco * 0.8
            elide: Text.ElideRight
            text: pie.bytes > 0 ? Textos.elementos(nucleo.total) + " · " + Textos.peso(pie.bytes)
                                : Textos.elementos(nucleo.total)
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.85
        }

        // Una pieza por tipo, del más al menos numeroso. Todas del color de
        // selección, cada una más tenue: el tema tiene un acento, no una paleta,
        // y lo que importa aquí es la proporción, no qué tipo es cuál —eso lo
        // dice la pista al pasar—.
        Row {
            id: raya
            anchors.left: rotuloPie.left
            anchors.right: rotuloPie.right
            anchors.top: rotuloPie.bottom
            anchors.topMargin: tema.hueco * 0.55
            height: Math.max(3, Math.round(tema.fuente * 0.3))
            spacing: 1

            Repeater {
                model: pie.reparto
                delegate: Rectangle {
                    id: pieza
                    required property var modelData
                    required property int index
                    width: nucleo.total > 0
                           ? Math.max(2, (raya.width - (pie.reparto.length - 1)) * modelData.n / nucleo.total)
                           : 0
                    height: raya.height
                    radius: height / 2
                    color: tema.seleccion
                    opacity: Math.max(0.25, 1 - index * 0.22)

                    MouseArea {
                        id: sobrePieza
                        anchors.fill: parent
                        anchors.margins: -tema.hueco * 0.4
                        hoverEnabled: true
                        onContainsMouseChanged: containsMouse
                            ? ventana.pistas.mostrar(pieza,
                                                     Textos.nombreFamiliaPorClave(pieza.modelData.familia) + " · "
                                                     + Textos.numero(pieza.modelData.n) + " · "
                                                     + Textos.peso(pieza.modelData.bytes), true)
                            : ventana.pistas.ocultar(pieza)
                    }
                }
            }
        }
    }
}
