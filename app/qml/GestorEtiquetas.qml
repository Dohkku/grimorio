// Todas las etiquetas de la biblioteca, para ordenarlas: renombrar, fusionar,
// quitar de todo y agrupar con color.
//
// Una tarjeta en medio, como la de renombrar en lote. Lo que se hace aquí toca
// muchos elementos a la vez, pero todo se deshace (Ctrl+Z), así que no hay
// preguntas: la barra de estado dice qué ha pasado.
//
//   clic en una etiqueta      filtrar por ella
//   ✎                         renombrar; si el nombre ya existe, se fusionan
//   puntos de color           meterla en ese grupo (∅, sin grupo)
//   ×                         quitarla de todo
//   muestra del grupo         cambiarle el color
//   doble clic en un grupo    renombrarlo
import QtQuick
import "consulta.js" as Consulta
import "textos.js" as Textos

Item {
    id: gestor
    anchors.fill: parent
    visible: abierto

    property bool abierto: false
    property string filtro: ""
    /// Qué se está renombrando: "e:<etiqueta>" o "g:<id de grupo>", o "".
    property string editando: ""
    property bool creandoGrupo: false

    function abrir() {
        nucleo.pedirTodasEtiquetas()
        filtro = ""
        buscador.text = ""
        editando = ""
        abierto = true
        buscador.forceActiveFocus()
    }

    function cerrar() {
        buscador.focus = false
        editando = ""
        creandoGrupo = false
        abierto = false
    }

    /// Grupos y etiquetas en una sola lista, cada etiqueta bajo su grupo y las
    /// sueltas al final.
    readonly property var filas: {
        const cuenta = {}
        const todas = nucleo.todasEtiquetas
        for (let i = 0; i < todas.length; i++) cuenta[todas[i].nombre] = todas[i].n
        const f = filtro.trim().toLowerCase()
        const pasa = function (t) { return f.length === 0 || t.toLowerCase().indexOf(f) >= 0 }
        const vistas = {}
        let out = []
        const gs = nucleo.grupos
        for (let g = 0; g < gs.length; g++) {
            out.push({ tipo: "grupo", id: gs[g].id, nombre: gs[g].nombre, color: gs[g].color,
                       n: gs[g].etiquetas.length, grupo: "" })
            for (let k = 0; k < gs[g].etiquetas.length; k++) {
                const t = gs[g].etiquetas[k]
                vistas[t] = true
                if (pasa(t)) out.push({ tipo: "etiqueta", nombre: t, n: cuenta[t] || 0,
                                        grupo: gs[g].id, color: gs[g].color })
            }
        }
        out.push({ tipo: "grupo", id: "", nombre: qsTr("sin grupo"), color: "", n: 0, grupo: "" })
        for (let i = 0; i < todas.length; i++) {
            const t = todas[i].nombre
            if (!vistas[t] && pasa(t))
                out.push({ tipo: "etiqueta", nombre: t, n: todas[i].n, grupo: "", color: "" })
        }
        return out
    }

    function siguienteColor(actual) {
        const m = Consulta.MUESTRAS
        const i = m.indexOf(actual)
        return m[(i + 1) % m.length]
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onClicked: gestor.cerrar()
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        opacity: 0.75
    }

    Rectangle {
        id: tarjeta
        anchors.centerIn: parent
        width: Math.min(640, gestor.width * 0.85)
        height: gestor.height * 0.8
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1

        MouseArea { anchors.fill: parent }

        Item {
            id: cabeza
            anchors { left: parent.left; right: parent.right; top: parent.top }
            anchors.margins: tema.margen
            height: Math.round(tema.fuente * 2.2)

            Text {
                id: titulo
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("etiquetas · %1").arg(Textos.numero(nucleo.todasEtiquetas.length))
                color: tema.texto
                font.pixelSize: tema.fuente * 1.15
                font.weight: Font.DemiBold
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: titulo.right
                anchors.leftMargin: tema.hueco
                anchors.right: botones.left
                anchors.rightMargin: tema.hueco
                height: Math.round(tema.fuente * 2)
                radius: tema.radio
                color: tema.fondo
                border.color: buscador.activeFocus ? tema.seleccion : tema.borde
                border.width: 1
                TextInput {
                    id: buscador
                    anchors.fill: parent
                    anchors.leftMargin: tema.hueco * 0.6
                    verticalAlignment: TextInput.AlignVCenter
                    clip: true
                    color: tema.texto
                    font.pixelSize: tema.fuente
                    onTextChanged: gestor.filtro = text
                    Keys.onEscapePressed: gestor.cerrar()
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: buscador.text.length === 0
                        text: qsTr("buscar etiqueta…")
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente
                    }
                }
            }

            Row {
                id: botones
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: tema.hueco * 0.5
                Boton {
                    texto: qsTr("+ grupo")
                    onPulsado: {
                        gestor.creandoGrupo = true
                        nuevoGrupo.empezar()
                    }
                }
                Boton {
                    icono: "cerrar"
                    onPulsado: gestor.cerrar()
                }
            }
        }

        FilaNueva {
            id: nuevoGrupo
            anchors { left: parent.left; right: parent.right; top: cabeza.bottom }
            anchors.leftMargin: tema.margen - tema.hueco * 0.5
            anchors.rightMargin: tema.margen - tema.hueco * 0.5
            visible: gestor.creandoGrupo
            height: visible ? Math.round(tema.fuente * 2.6) : 0
            seguir: false
            pista: qsTr("nombre del grupo nuevo…")
            onNombrado: function (nombre) {
                nucleo.crearGrupo(nombre, Consulta.MUESTRAS[nucleo.grupos.length % Consulta.MUESTRAS.length])
            }
            onAcabada: gestor.creandoGrupo = false
        }

        ListView {
            id: lista
            anchors { left: parent.left; right: parent.right; top: nuevoGrupo.bottom; bottom: pie.top }
            anchors.margins: tema.margen
            anchors.topMargin: tema.hueco
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: gestor.filas

            delegate: Item {
                id: fila
                required property var modelData
                readonly property bool esGrupo: modelData.tipo === "grupo"
                readonly property string clave: (esGrupo ? "g:" + modelData.id : "e:" + modelData.nombre)
                readonly property bool enEdicion: gestor.editando === clave
                width: lista.width
                height: Math.round(tema.fuente * (esGrupo ? 2.6 : 2.2))

                Rectangle {
                    anchors.fill: parent
                    radius: tema.radio
                    color: tema.borde
                    visible: sobreFila.containsMouse && !fila.esGrupo
                }

                // La muestra del grupo, o el punto de la etiqueta.
                Rectangle {
                    id: muestra
                    anchors.verticalCenter: parent.verticalCenter
                    x: fila.esGrupo ? 0 : tema.hueco * 1.5
                    width: fila.esGrupo ? tema.fuente * 1.1 : tema.fuente * 0.6
                    height: width
                    radius: fila.esGrupo ? tema.radio * 0.5 : width / 2
                    visible: fila.modelData.color.length > 0 || !fila.esGrupo
                    color: fila.modelData.color.length > 0 ? fila.modelData.color : "transparent"
                    border.color: tema.borde
                    border.width: 1
                    MouseArea {
                        anchors.fill: parent
                        enabled: fila.esGrupo && fila.modelData.id.length > 0
                        cursorShape: Qt.PointingHandCursor
                        onClicked: nucleo.colorGrupo(fila.modelData.id, gestor.siguienteColor(fila.modelData.color))
                    }
                }

                Text {
                    id: rotulo
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: muestra.right
                    anchors.leftMargin: tema.hueco * 0.6
                    visible: !fila.enEdicion
                    text: fila.esGrupo ? fila.modelData.nombre : "#" + fila.modelData.nombre
                    color: fila.esGrupo ? tema.textoTenue : tema.texto
                    font.pixelSize: tema.fuente * (fila.esGrupo ? 0.9 : 1)
                    font.weight: fila.esGrupo ? Font.DemiBold : Font.Normal
                    font.letterSpacing: fila.esGrupo ? tema.fuente * 0.04 : 0
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: rotulo.right
                    anchors.leftMargin: tema.hueco * 0.6
                    visible: !fila.enEdicion && !fila.esGrupo
                    text: Textos.numero(fila.modelData.n)
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.85
                }

                // Renombrar en su sitio.
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: muestra.right
                    anchors.leftMargin: tema.hueco * 0.4
                    width: Math.min(tema.fuente * 18, fila.width * 0.5)
                    height: Math.round(tema.fuente * 1.9)
                    visible: fila.enEdicion
                    radius: tema.radio
                    color: tema.fondo
                    border.color: tema.seleccion
                    border.width: 1
                    TextInput {
                        id: campo
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
                        onAccepted: {
                            const t = text.trim()
                            if (t.length > 0 && t !== fila.modelData.nombre) {
                                if (fila.esGrupo) nucleo.renombrarGrupo(fila.modelData.id, t)
                                else nucleo.renombrarEtiqueta(fila.modelData.nombre, t)
                            }
                            focus = false
                            gestor.editando = ""
                        }
                        Keys.onEscapePressed: {
                            focus = false
                            gestor.editando = ""
                        }
                    }
                }

                MouseArea {
                    id: sobreFila
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: fila.esGrupo ? Qt.ArrowCursor : Qt.PointingHandCursor
                    onClicked: {
                        if (fila.esGrupo) return
                        ventana.filtrarPorEtiqueta(fila.modelData.nombre)
                        gestor.cerrar()
                    }
                    onDoubleClicked: if (fila.esGrupo && fila.modelData.id.length > 0)
                                         gestor.editando = fila.clave
                }

                // Lo que se puede hacer con la fila, al pasar.
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: tema.hueco * 0.4
                    spacing: tema.hueco * 0.3
                    visible: !fila.enEdicion && (sobreFila.containsMouse || puntos.algunoEncima
                                                 || editar.encima || quitar.encima)
                             && !(fila.esGrupo && fila.modelData.id.length === 0)

                    // Los grupos como puntos: un clic la mete en ese.
                    Row {
                        id: puntos
                        visible: !fila.esGrupo
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: tema.hueco * 0.3
                        property bool algunoEncima: false
                        Repeater {
                            model: nucleo.grupos
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool suyo: fila.modelData.grupo === modelData.id
                                anchors.verticalCenter: parent.verticalCenter
                                width: tema.fuente * 0.9
                                height: width
                                radius: width / 2
                                color: modelData.color
                                border.color: suyo ? tema.texto : tema.borde
                                border.width: suyo ? 2 : 1
                                MouseArea {
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onContainsMouseChanged: puntos.algunoEncima = containsMouse
                                    onClicked: nucleo.agruparEtiqueta(fila.modelData.nombre, parent.modelData.id)
                                }
                            }
                        }
                        Text {
                            visible: fila.modelData.grupo.length > 0
                            anchors.verticalCenter: parent.verticalCenter
                            text: "∅"
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente
                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onContainsMouseChanged: puntos.algunoEncima = containsMouse
                                onClicked: nucleo.agruparEtiqueta(fila.modelData.nombre, "")
                            }
                        }
                    }
                    BotonIcono {
                        id: editar
                        icono: "lapiz"
                        pista: fila.esGrupo ? qsTr("renombrar el grupo")
                                            : qsTr("renombrar (si ya existe, se fusionan)")
                        onPulsado: gestor.editando = fila.clave
                    }
                    BotonIcono {
                        id: quitar
                        icono: "cerrar"
                        pista: fila.esGrupo ? qsTr("borrar el grupo (las etiquetas se quedan)")
                                            : qsTr("quitar #%1 de todo (Ctrl+Z lo deshace)").arg(fila.modelData.nombre)
                        onPulsado: fila.esGrupo ? nucleo.borrarGrupo(fila.modelData.id)
                                                : nucleo.borrarEtiqueta(fila.modelData.nombre)
                    }
                }
            }
        }

        Text {
            id: pie
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            anchors.margins: tema.margen
            wrapMode: Text.Wrap
            text: qsTr("clic: filtrar · ✎ renombrar o fusionar · puntos: grupo · × quitar de todo · todo se deshace con Ctrl+Z")
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.8
        }
    }
}
