// Lo que sale debajo del buscador mientras se escribe.
//
// El buscador es la puerta a todo, y sobre todo a las etiquetas: se escribe un
// trozo y salen las que encajan, con su número y el color de su grupo; Intro
// o un clic la pone en la búsqueda. Y ahí mismo se ordenan —renombrar,
// fusionar, quitar de todo, cambiar de grupo—, que es donde uno se da cuenta
// de que «gato» y «gatos» son la misma.
//
// Debajo de las etiquetas, lo demás que encaja con la palabra: carpetas,
// carpetas dinámicas y filtros («vid» → tipo: vídeos).
//
//   ↑ ↓     moverse          Intro   elegir           Esc   cerrar
import QtQuick
import "consulta.js" as Consulta
import "textos.js" as Textos

Item {
    id: ayudante
    anchors.fill: parent

    /// El buscador del que cuelga (BarraSuperior.cajaBusqueda) y su campo.
    property Item caja: null
    property TextInput campo: null

    /// Se escribe o se ha pinchado en el buscador.
    readonly property bool enBuscador: campo !== null && campo.activeFocus
    /// Renombrando una etiqueta aquí: el campo de renombrar se lleva el foco
    /// del buscador, y el ayudante no tiene que cerrarse por eso.
    property string editando: ""
    property bool cerrado: false
    readonly property bool abierto: (enBuscador || editando.length > 0) && !cerrado
                                    && filas.length > 0
    visible: abierto

    /// La palabra que se escribe ahora.
    readonly property string palabra: campo ? Consulta.ultimaPalabra(campo.text) : ""
    readonly property bool esEtiqueta: palabra.charAt(0) === "#"
    readonly property string busca: {
        let p = palabra
        if (p.charAt(0) === "#") p = p.substring(1)
        if (p.toLowerCase().indexOf("etiqueta:") === 0) p = p.substring(9)
        return p.toLowerCase()
    }
    property int elegida: -1

    onEnBuscadorChanged: {
        if (enBuscador) {
            cerrado = false
            nucleo.pedirTodasEtiquetas()
        }
    }
    onPalabraChanged: { elegida = -1; cerrado = false }

    function grupoDe(t) {
        const gs = nucleo.grupos
        for (let i = 0; i < gs.length; i++)
            if (gs[i].etiquetas.indexOf(t) >= 0) return gs[i]
        return null
    }

    /// Todo lo que se enseña, en orden: cabeceras y filas.
    readonly property var filas: {
        let out = []
        const b = busca
        const todas = nucleo.todasEtiquetas
        // Etiquetas: con palabra, las que la contienen (primero las que
        // empiezan por ella); sin palabra, las más usadas.
        let ets = []
        for (let i = 0; i < todas.length; i++) {
            const n = todas[i].nombre
            const p = n.toLowerCase().indexOf(b)
            if (b.length === 0 || p >= 0) ets.push({ nombre: n, n: todas[i].n, empieza: p === 0 })
        }
        if (b.length > 0) ets.sort(function (x, y) { return (y.empieza - x.empieza) || (y.n - x.n) })
        const tope = esEtiqueta || b.length > 0 ? 10 : 8
        if (ets.length > 0) {
            out.push({ tipo: "seccion", texto: b.length > 0 || esEtiqueta ? qsTr("etiquetas")
                                                                          : qsTr("etiquetas más usadas") })
            for (let i = 0; i < Math.min(tope, ets.length); i++) {
                const g = grupoDe(ets[i].nombre)
                out.push({ tipo: "etiqueta", nombre: ets[i].nombre, n: ets[i].n,
                           color: g ? g.color : "", grupo: g ? g.id : "" })
            }
        }
        if (esEtiqueta) return out

        if (b.length > 0) {
            // Carpetas.
            let cs = []
            const carpetas = nucleo.carpetas
            for (let i = 0; i < carpetas.length; i++)
                if (carpetas[i].nombre.toLowerCase().indexOf(b) >= 0) cs.push(carpetas[i])
            if (cs.length > 0) {
                out.push({ tipo: "seccion", texto: qsTr("carpetas") })
                for (let i = 0; i < Math.min(5, cs.length); i++)
                    out.push({ tipo: "carpeta", id: cs[i].id, nombre: cs[i].nombre, n: cs[i].cuenta })
            }
        }
        // Dinámicas: con palabra, las que encajan; sin ella, todas.
        let bs = []
        const busquedas = nucleo.busquedas
        for (let i = 0; i < busquedas.length; i++)
            if (b.length === 0 || busquedas[i].nombre.toLowerCase().indexOf(b) >= 0) bs.push(busquedas[i])
        if (bs.length > 0) {
            out.push({ tipo: "seccion", texto: qsTr("carpetas dinámicas") })
            for (let i = 0; i < Math.min(5, bs.length); i++)
                out.push({ tipo: "inteligente", consulta: bs[i].consulta, nombre: bs[i].nombre, n: bs[i].cuenta })
        }
        if (b.length > 0) {
            let fs = []
            for (let i = 0; i < Consulta.SUGERENCIAS.length; i++) {
                const s = Consulta.SUGERENCIAS[i]
                for (let k = 0; k < s.claves.length; k++)
                    if (s.claves[k].indexOf(b) === 0) { fs.push(s); break }
            }
            if (fs.length > 0) {
                out.push({ tipo: "seccion", texto: qsTr("filtros") })
                for (let i = 0; i < fs.length; i++)
                    out.push({ tipo: "filtro", texto: fs[i].texto, token: fs[i].token, icono: fs[i].icono })
            }
        }
        return out
    }

    readonly property var elegibles: {
        let e = []
        for (let i = 0; i < filas.length; i++) if (filas[i].tipo !== "seccion") e.push(i)
        return e
    }

    function mover(d) {
        if (elegibles.length === 0) return
        let k = elegibles.indexOf(elegida)
        k = k < 0 ? (d > 0 ? 0 : elegibles.length - 1) : (k + d + elegibles.length) % elegibles.length
        elegida = elegibles[k]
        lista.positionViewAtIndex(elegida, ListView.Contain)
    }

    /// Intro: elige la marcada, si hay. Devuelve si hizo algo.
    function aceptar() {
        if (!abierto || elegida < 0 || elegida >= filas.length) return false
        usar(filas[elegida])
        return true
    }

    function ponerBusqueda(t) {
        campo.text = t
        ventana.textoBusqueda = t
        ventana.consultar()
    }

    function usar(f) {
        switch (f.tipo) {
        case "etiqueta":
            ponerBusqueda(Consulta.cambiarUltima(campo.text, Consulta.comoEtiqueta(f.nombre)))
            break
        case "filtro":
            ponerBusqueda(Consulta.cambiarUltima(campo.text, f.token))
            break
        case "carpeta":
            ponerBusqueda(Consulta.cambiarUltima(campo.text, ""))
            ventana.irACarpeta(f.id)
            cerrado = true
            break
        case "inteligente":
            campo.text = f.consulta
            ventana.aplicarBusqueda(f.consulta)
            cerrado = true
            break
        }
        elegida = -1
    }

    // ------------------------------------------------------------- el cuadro
    Rectangle {
        id: cuadro
        // Se lee la geometría del buscador para que esto se recalcule cuando
        // se mueve: `mapToItem` no avisa de nada por sí solo, y se quedaba con
        // la posición del arranque, cuando el buscador aún no estaba en su sitio.
        // Alineado con el buscador por la izquierda y nunca más estrecho que
        // lo que se lee a gusto: el buscador es estrecho y las etiquetas con
        // su grupo y su número no caben en él. Si no cabe hacia la derecha,
        // se apoya en el borde.
        x: {
            if (!ayudante.caja) return 0
            void (ayudante.caja.x + ayudante.caja.width + ayudante.width
                  + (ayudante.caja.parent ? ayudante.caja.parent.x : 0))
            const x0 = ayudante.caja.mapToItem(ayudante, 0, 0).x
            return Math.max(tema.hueco, Math.min(x0, ayudante.width - width - tema.hueco))
        }
        y: {
            if (!ayudante.caja) return 0
            void (ayudante.caja.y + ayudante.caja.height)
            return ayudante.caja.mapToItem(ayudante, 0, ayudante.caja.height).y + tema.hueco * 0.4
        }
        width: ayudante.caja ? Math.max(ayudante.caja.width, tema.fuente * 28) : 0
        height: Math.min(lista.contentHeight, ayudante.height * 0.6) + pie.height + tema.hueco
        radius: tema.radio * 1.5
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        // Que pinchar dentro no llegue a lo de debajo.
        MouseArea { anchors.fill: parent; hoverEnabled: true }

        ListView {
            id: lista
            anchors { left: parent.left; right: parent.right; top: parent.top }
            anchors.margins: tema.hueco * 0.5
            height: Math.min(contentHeight, ayudante.height * 0.6)
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: ayudante.filas

            delegate: Item {
                id: fila
                required property var modelData
                required property int index
                readonly property bool seccion: modelData.tipo === "seccion"
                readonly property bool marcada: index === ayudante.elegida
                readonly property bool esEtiqueta: modelData.tipo === "etiqueta"
                readonly property bool enEdicion: esEtiqueta && ayudante.editando === modelData.nombre
                width: lista.width
                height: Math.round(tema.fuente * (seccion ? 1.9 : 2.3))

                Text {
                    visible: fila.seccion
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: tema.hueco * 0.2
                    x: tema.hueco * 0.6
                    text: fila.modelData.texto || ""
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.8
                    font.letterSpacing: tema.fuente * 0.05
                }

                Rectangle {
                    visible: !fila.seccion
                    anchors.fill: parent
                    radius: tema.radio
                    color: tema.borde
                    opacity: fila.marcada ? 1 : (sobre.containsMouse ? 0.6 : 0)
                }

                // El icono, o el punto del grupo de la etiqueta.
                Item {
                    id: marca
                    visible: !fila.seccion
                    anchors.verticalCenter: parent.verticalCenter
                    x: tema.hueco * 0.6
                    width: Math.round(tema.fuente * 1.2)
                    height: width
                    Icono {
                        anchors.fill: parent
                        visible: !fila.esEtiqueta
                        nombre: fila.modelData.tipo === "carpeta" ? "carpeta"
                              : fila.modelData.tipo === "inteligente" ? "dinamica"
                              : (fila.modelData.icono || "filtro")
                        color: tema.textoTenue
                    }
                    Icono {
                        anchors.fill: parent
                        visible: fila.esEtiqueta && (fila.modelData.color || "").length === 0
                        nombre: "etiqueta"
                        color: tema.textoTenue
                    }
                    Rectangle {
                        anchors.centerIn: parent
                        visible: fila.esEtiqueta && (fila.modelData.color || "").length > 0
                        width: parent.width * 0.6
                        height: width
                        radius: width / 2
                        color: (fila.modelData.color || "").length > 0 ? fila.modelData.color : "transparent"
                    }
                }

                Text {
                    id: rotulo
                    visible: !fila.seccion && !fila.enEdicion
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: marca.right
                    anchors.leftMargin: tema.hueco * 0.6
                    text: fila.esEtiqueta ? "#" + fila.modelData.nombre
                          : (fila.modelData.tipo === "filtro" ? fila.modelData.texto : (fila.modelData.nombre || ""))
                    color: tema.texto
                    font.pixelSize: tema.fuente
                }
                Text {
                    visible: !fila.seccion && !fila.enEdicion && fila.modelData.n !== undefined
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: rotulo.right
                    anchors.leftMargin: tema.hueco * 0.5
                    text: Textos.numero(fila.modelData.n || 0)
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.85
                }
                Text {
                    visible: fila.modelData.tipo === "filtro"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: tema.hueco * 0.6
                    text: fila.modelData.token || ""
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.85
                    font.family: "monospace"
                }

                // Renombrar aquí mismo. Si el nombre ya existe, se fusionan.
                Rectangle {
                    visible: fila.enEdicion
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: marca.right
                    anchors.leftMargin: tema.hueco * 0.4
                    width: parent.width * 0.55
                    height: Math.round(tema.fuente * 1.9)
                    radius: tema.radio
                    color: tema.fondo
                    border.color: tema.seleccion
                    border.width: 1
                    TextInput {
                        id: nuevoNombre
                        anchors.fill: parent
                        anchors.leftMargin: tema.hueco * 0.5
                        verticalAlignment: TextInput.AlignVCenter
                        clip: true
                        color: tema.texto
                        font.pixelSize: tema.fuente
                        onVisibleChanged: if (visible) {
                            text = fila.modelData.nombre
                            forceActiveFocus()
                            selectAll()
                        }
                        function acabar() {
                            focus = false
                            ayudante.editando = ""
                            ayudante.campo.forceActiveFocus()
                        }
                        onAccepted: {
                            const t = text.trim()
                            if (t.length > 0 && t !== fila.modelData.nombre)
                                nucleo.renombrarEtiqueta(fila.modelData.nombre, t)
                            acabar()
                        }
                        Keys.onEscapePressed: acabar()
                    }
                }

                MouseArea {
                    id: sobre
                    visible: !fila.seccion
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: ayudante.usar(fila.modelData)
                }

                // Ordenar la etiqueta sin salir del buscador.
                Row {
                    visible: fila.esEtiqueta && !fila.enEdicion
                             && (sobre.containsMouse || puntos.encima || editar.encima || quitar.encima)
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: tema.hueco * 0.4
                    spacing: tema.hueco * 0.3
                    Row {
                        id: puntos
                        property bool encima: false
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: tema.hueco * 0.3
                        Repeater {
                            model: nucleo.grupos
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool suyo: fila.modelData.grupo === modelData.id
                                anchors.verticalCenter: parent.verticalCenter
                                width: tema.fuente * 0.85
                                height: width
                                radius: width / 2
                                color: modelData.color
                                border.color: suyo ? tema.texto : tema.borde
                                border.width: suyo ? 2 : 1
                                MouseArea {
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onContainsMouseChanged: puntos.encima = containsMouse
                                    onClicked: nucleo.agruparEtiqueta(fila.modelData.nombre,
                                                                      parent.suyo ? "" : parent.modelData.id)
                                }
                            }
                        }
                    }
                    BotonIcono {
                        id: editar
                        icono: "lapiz"
                        pista: qsTr("renombrar (si ya existe, se fusionan)")
                        onPulsado: ayudante.editando = fila.modelData.nombre
                    }
                    BotonIcono {
                        id: quitar
                        icono: "cerrar"
                        pista: qsTr("quitar #%1 de todo (Ctrl+Z lo deshace)").arg(fila.modelData.nombre)
                        onPulsado: nucleo.borrarEtiqueta(fila.modelData.nombre)
                    }
                }
            }
        }

        // Al pie, cómo se usa y la puerta al gestor entero (grupos).
        Item {
            id: pie
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: Math.round(tema.fuente * 2.2)
            Rectangle {
                anchors { left: parent.left; right: parent.right; top: parent.top }
                height: 1
                color: tema.borde
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                x: tema.hueco
                text: qsTr("↑↓ moverse · Intro elegir · # solo etiquetas")
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.8
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                anchors.right: parent.right
                anchors.rightMargin: tema.hueco
                text: qsTr("grupos y colores…")
                color: sobreGestor.containsMouse ? tema.texto : tema.seleccion
                font.pixelSize: tema.fuente * 0.85
                MouseArea {
                    id: sobreGestor
                    anchors.fill: parent
                    anchors.margins: -tema.hueco * 0.3
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        ayudante.cerrado = true
                        ventana.abrirEtiquetas()
                    }
                }
            }
        }
    }
}
