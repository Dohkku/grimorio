// El panel de detalle: lo que hay elegido y lo que se puede hacer con ello.
//
// Es la columna derecha del estudio y está siempre puesta. De arriba abajo:
// la vista grande —o, con varios elegidos, un mosaico de los primeros—, el
// nombre y las estrellas, la paleta, las etiquetas, la nota, la ficha técnica y
// las acciones. Cada bloque con su título en versalitas y una raya debajo: se
// lee como una ficha, no como un formulario.
//
// Funciona igual con uno que con mil: las estrellas, las etiquetas y la
// papelera se aplican a toda la selección. Editar en lote no es una función
// aparte, es la misma.
//
// Lo que la malla no lleva —peso, medidas, fechas, nota, paleta, ruta— se pide
// por su id cuando cambia el foco. La vista guarda solo lo que hace falta para
// pintar cien mil celdas; el panel enseña una cosa cada vez.
import QtQuick
import "textos.js" as Textos

Rectangle {
    id: inspector
    color: tema.panel
    clip: true

    readonly property int cuantos: modelo.elegidos
    // Con varios elegidos se enseña uno; el que sea, pero nunca "nada" cuando
    // hay algo. `actual` puede quedarse a -1 tras cambiar de filtro.
    readonly property int foco: modelo.actual
    readonly property string idFoco: foco >= 0 ? modelo.idDe(foco) : ""
    // La ficha llega asíncrona: hasta que la que hay es la del elemento en
    // foco, se enseña lo que la vista sí sabe en vez de datos de otro.
    readonly property bool alDia: nucleo.ficha.id === idFoco
    readonly property var ficha: nucleo.ficha

    readonly property int familiaFoco: foco >= 0 ? modelo.familiaDe(foco) : -1
    /// Tapado aquí, que no es lo mismo que estar marcado. El visor sí lo enseña:
    /// entrar en un elemento es pedir verlo.
    readonly property bool censurado: cuantos === 1 && modelo.adultoFoco && ajustes.modoSeguro
    readonly property bool sePuedeReproducir: cuantos === 1
        && (familiaFoco === Textos.VIDEO || familiaFoco === Textos.AUDIO)
    readonly property url urlFoco: sePuedeReproducir && foco >= 0 ? modelo.urlDe(foco) : ""
    /// Suena aquí salvo si el visor está abierto: ahí suena el de dentro, y dos
    /// copias del mismo vídeo a destiempo es lo peor que puede pasar.
    readonly property bool hayQueReproducir: String(urlFoco).length > 0
        && !ventana.visorAbierto && !censurado && visible
    /// Igual que en el visor: una vez creado el reproductor no se destruye.
    /// Montar y desmontar la tubería de multimedia tira el programa.
    property bool hizoFalta: false
    onHayQueReproducirChanged: if (hayQueReproducir) hizoFalta = true

    /// Con varios elegidos, el mosaico de los primeros. Solo si la selección
    /// cabe en la cabeza: leer los ids de una selección de cien mil para
    /// enseñar nueve sería pagar cien mil cadenas por cada clic.
    readonly property var mosaico: cuantos > 1 && ventana.seleccionMenuda
                                   ? modelo.seleccion.slice(0, 9) : []

    /// La fuente que llega al reproductor, con un respiro de por medio.
    ///
    /// Moverse con las flechas cambia el foco muchas veces por segundo, y darle
    /// cada uno al reproductor monta y desmonta una tubería de multimedia por
    /// tecla, en el hilo de la interfaz. Pasar por encima de los vídeos con el
    /// teclado costaba eso por cada uno; medido en una ráfaga de veinticinco
    /// teclas, la ventana tardaba 3,3 s en asentarse.
    ///
    /// Solo se le da la fuente a quien se queda mirando. Vaciarla no espera:
    /// callar es urgente, empezar no.
    property url urlDiferida: ""
    /// El id que va con `urlDiferida`, con el mismo retraso.
    ///
    /// Si el id llegaba al momento y la fuente 220 ms después, durante ese rato
    /// el reproductor tenía el id de un vídeo y el archivo de otro. Un error del
    /// vídeo viejo en ese hueco pedía su copia reproducible con el id del
    /// nuevo, y el núcleo la guardaba ahí: un vídeo de cruces que se reproducía
    /// como uno de cubos, para siempre, porque la copia se queda en la caché.
    property string idDiferido: ""

    Timer {
        id: reposo
        interval: 220
        onTriggered: {
            inspector.idDiferido = inspector.idFoco
            inspector.urlDiferida = inspector.urlFoco
        }
    }

    onUrlFocoChanged: {
        if (String(urlFoco).length === 0) {
            reposo.stop()
            urlDiferida = ""
            idDiferido = ""
        } else {
            reposo.restart()
        }
    }

    function renombrar() {
        if (cuantos === 1) nombre.editar()
    }
    function enfocarEtiquetas() {
        etiquetas.enfocar()
    }
    function escribirEtiqueta(t) {
        etiquetas.escribir(t)
    }

    /// Mover lo elegido a una carpeta, eligiéndola de una lista. Desde una
    /// carpeta, mover es sacar de esa y meter en la otra —lo mismo que
    /// arrastrar—; desde «Todo», solo meter.
    /// Abre la lista de carpetas colgando de `anclaje`, o en (px, py) si no
    /// hay anclaje (desde el clic derecho).
    function moverA(anclaje, px, py) {
        const cs = nucleo.carpetas
        let ops = []
        for (let i = 0; i < cs.length; i++)
            ops.push({ texto: " ".repeat(cs[i].nivel) + cs[i].nombre, valor: cs[i].id, icono: "carpeta" })
        if (ops.length === 0) {
            ventana.nuevaCarpeta("")
            return
        }
        const def = {
            titulo: qsTr("mover a"),
            opciones: ops,
            alElegir: function (id) {
                const origen = ventana.enPapelera ? "" : ventana.carpetaActual
                if (origen.length > 0 && origen !== id) nucleo.moverEntreCarpetas(modelo.seleccion, origen, id)
                else nucleo.moverAcarpeta(modelo.seleccion, id)
            }
        }
        if (anclaje) ventana.abrirDesplegable(anclaje, def)
        else ventana.abrirDesplegableEn(px, py, def)
    }

    /// De dónde vino, en pocas palabras: la web de la que se bajó o la carpeta
    /// del disco de la que se importó. La ruta entera va en la pista.
    function origenCorto(f) {
        if (f.fuente) {
            const m = String(f.fuente).match(/^[a-z]+:\/\/([^\/]+)/i)
            return m ? m[1].replace(/^www\./, "") : String(f.fuente)
        }
        if (f.procedencia) {
            const partes = String(f.procedencia).split("/").filter(function (p) { return p.length > 0 })
            return partes.length > 1 ? partes[partes.length - 2] : String(f.procedencia)
        }
        return ""
    }

    function nombresDeCarpetas(ids) {
        if (!ids) return ""
        let n = []
        for (let i = 0; i < ids.length; i++) {
            const t = ventana.nombreDeCarpeta(ids[i])
            if (t.length > 0) n.push(t)
        }
        return n.join(", ")
    }

    onIdFocoChanged: nucleo.pedirFicha(idFoco)

    Connections {
        target: nucleo
        function onItemCambiado(id, estado) {
            if (id === inspector.idFoco) nucleo.pedirFicha(id)
        }
    }

    // Un bloque de la ficha: título en versalitas, lo de dentro y una raya.
    component Bloque: Item {
        id: bloque
        property string titulo: ""
        default property alias contenido: cuerpo.data
        width: parent ? parent.width : 0
        height: visible ? cuerpo.height + tema.hueco * 2.2 : 0

        Column {
            id: cuerpo
            x: tema.hueco * 1.4
            y: tema.hueco * 1.1
            width: parent.width - tema.hueco * 2.8
            spacing: tema.hueco * 0.7

            Text {
                visible: bloque.titulo.length > 0
                text: bloque.titulo.toUpperCase()
                color: tema.textoTenue
                font.pixelSize: tema.fuente * 0.78
                font.weight: Font.Medium
                font.letterSpacing: tema.fuente * 0.06
            }
        }
        Rectangle {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: 1
            color: tema.borde
        }
    }

    Rectangle {
        anchors { top: parent.top; bottom: parent.bottom; left: parent.left }
        width: 1
        color: tema.borde
        z: 2
    }

    // Nada elegido: el panel lo dice y explica cómo elegir varios, que es lo
    // que no se adivina.
    Text {
        anchors.centerIn: parent
        width: parent.width - tema.hueco * 4
        visible: inspector.cuantos === 0
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        lineHeight: 1.3
        text: qsTr("Elige algo para ver sus detalles.\n\nCtrl+clic para elegir varios; Mayús+clic, un tramo.")
        color: tema.textoTenue
        font.pixelSize: tema.fuente * 0.95
    }

    Flickable {
        anchors.fill: parent
        visible: inspector.cuantos > 0
        contentHeight: columna.height
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        Column {
            id: columna
            width: parent.width

            // ---------------------------------------------------- la vista
            // Ancha, de borde a borde, y con forma fija: con la forma de cada
            // foto el resto de la ficha subía y bajaba al pasar de una a otra,
            // y los botones nunca estaban donde se dejaron.
            Rectangle {
                id: vista
                width: parent.width
                height: Math.min(Math.round(width * 0.78), inspector.height * 0.45)
                color: tema.fondo
                clip: true

                Image {
                    anchors.fill: parent
                    visible: inspector.foco >= 0 && inspector.mosaico.length === 0
                    source: inspector.idFoco.length > 0
                            ? "image://grim/" + inspector.idFoco : ""
                    asynchronous: true
                    fillMode: Image.PreserveAspectFit
                    // Igual que en la malla: pedida diminuta y estirada. Aquí
                    // el panel es grande, así que hace falta pedir menos para
                    // que se difumine lo mismo.
                    sourceSize.height: inspector.censurado
                                       ? Math.max(4, Math.round(tema.fuente * 0.4)) : 0
                    smooth: true
                }

                // Varios elegidos: los primeros nueve en mosaico. Dice de un
                // vistazo qué se va a cambiar, que es la pregunta de antes de
                // pulsar nada.
                // Las piezas llenan la caja sea cuantos sean: dos van lado a
                // lado, cuatro en dos por dos, de cinco a nueve en tres.
                Grid {
                    id: rejilla
                    anchors.fill: parent
                    anchors.margins: tema.hueco
                    visible: inspector.mosaico.length > 0
                    readonly property int n: inspector.mosaico.length
                    columns: n <= 1 ? 1 : (n <= 4 ? 2 : 3)
                    readonly property int filas: Math.max(1, Math.ceil(n / columns))
                    spacing: Math.max(2, Math.round(tema.hueco * 0.4))
                    readonly property real lado: (width - spacing * (columns - 1)) / columns
                    readonly property real alto: (height - spacing * (filas - 1)) / filas

                    Repeater {
                        model: inspector.mosaico
                        delegate: Image {
                            required property string modelData
                            width: rejilla.lado
                            height: rejilla.alto
                            source: "image://grim/" + modelData
                            asynchronous: true
                            fillMode: Image.PreserveAspectCrop
                            sourceSize.height: 160
                            clip: true
                        }
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: inspector.censurado
                    text: qsTr("+18 · el modo seguro está puesto")
                    color: tema.texto
                    style: Text.Outline
                    styleColor: tema.fondo
                    font.pixelSize: tema.fuente * 0.9
                }

                // Encima de la miniatura, como en el visor: mientras arranca se
                // sigue viendo la portada.
                Loader {
                    id: reproductor
                    anchors.fill: parent
                    active: inspector.hizoFalta
                    visible: inspector.hayQueReproducir
                    source: "qrc:/qml/Reproductor.qml"
                }

                Binding {
                    target: reproductor.item
                    property: "idElemento"
                    value: inspector.idDiferido
                    when: reproductor.status === Loader.Ready
                }
                Binding {
                    target: reproductor.item
                    property: "compacto"
                    value: true
                    when: reproductor.status === Loader.Ready
                }
                Binding {
                    target: reproductor.item
                    property: "fuente"
                    value: inspector.urlDiferida
                    when: reproductor.status === Loader.Ready
                }
                Binding {
                    target: reproductor.item
                    property: "enPantalla"
                    value: inspector.hayQueReproducir
                                && String(inspector.urlDiferida).length > 0
                    when: reproductor.status === Loader.Ready
                }
                Binding {
                    target: reproductor.item
                    property: "soloSonido"
                    value: inspector.familiaFoco === Textos.AUDIO
                    when: reproductor.status === Loader.Ready
                }
                Binding {
                    target: reproductor.item
                    property: "caratula"
                    value: inspector.idFoco.length > 0
                           ? "image://grim/" + inspector.idFoco : ""
                    when: reproductor.status === Loader.Ready
                }
                Binding {
                    target: reproductor.item
                    property: "bucle"
                    value: true
                    when: reproductor.status === Loader.Ready
                }

                // Doble clic, a pantalla completa: lo mismo que en la malla.
                MouseArea {
                    anchors.fill: parent
                    enabled: !inspector.hayQueReproducir
                    onDoubleClicked: if (inspector.foco >= 0) ventana.visorAbierto = true
                }

                Rectangle {
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                    height: 1
                    color: tema.borde
                }
            }

            // --------------------------------------- nombre y estrellas
            Bloque {
                // El nombre se edita aquí mismo, pinchándolo o con F2. Un
                // diálogo para cambiar un nombre es una ventana de más.
                Item {
                    id: nombre
                    width: parent.width
                    height: Math.round(tema.fuente * 2.2)
                    property bool editando: false

                    function editar() {
                        if (inspector.foco < 0 || inspector.cuantos !== 1) return
                        entrada.text = modelo.nombreDe(inspector.foco)
                        nombre.editando = true
                        entrada.forceActiveFocus()
                        entrada.selectAll()
                    }

                    // Al pasar por encima se nota que es un campo: el nombre
                    // se puede cambiar, y eso no se adivina de un rótulo.
                    Rectangle {
                        anchors.fill: parent
                        anchors.leftMargin: -tema.hueco * 0.5
                        anchors.rightMargin: -tema.hueco * 0.5
                        radius: tema.radio
                        color: nombre.editando ? tema.fondo
                               : (sobreNombre.containsMouse ? tema.marcador : "transparent")
                        border.color: nombre.editando ? tema.seleccion : "transparent"
                        border.width: 1
                    }

                    Text {
                        anchors.fill: parent
                        verticalAlignment: Text.AlignVCenter
                        visible: !nombre.editando
                        elide: Text.ElideMiddle
                        text: inspector.cuantos > 1
                              ? Textos.elegidos(inspector.cuantos)
                              : (inspector.foco >= 0 ? modelo.nombreDe(inspector.foco) : qsTr("—"))
                        color: tema.texto
                        font.pixelSize: tema.fuente * 1.08
                        font.weight: Font.DemiBold
                    }

                    TextInput {
                        id: entrada
                        anchors.fill: parent
                        visible: nombre.editando
                        verticalAlignment: TextInput.AlignVCenter
                        clip: true
                        color: tema.texto
                        selectionColor: tema.seleccion
                        selectedTextColor: tema.fondo
                        font.pixelSize: tema.fuente * 1.08
                        font.weight: Font.DemiBold
                        onAccepted: {
                            nucleo.renombrar(inspector.idFoco, text)
                            nombre.editando = false
                        }
                        Keys.onEscapePressed: nombre.editando = false
                        onActiveFocusChanged: if (!activeFocus) nombre.editando = false
                    }

                    MouseArea {
                        id: sobreNombre
                        anchors.fill: parent
                        enabled: !nombre.editando && inspector.cuantos === 1
                        hoverEnabled: true
                        cursorShape: Qt.IBeamCursor
                        onClicked: nombre.editar()
                    }
                }

                Estrellas {
                    width: parent.width
                    valor: modelo.estrellasFoco
                    onElegido: function (n) { nucleo.estrellas(modelo.seleccion, n) }
                }
            }

            // ------------------------------------------------- la paleta
            // Los colores que de verdad tiene, cada uno del ancho de lo que
            // ocupa. Pinchar uno busca lo que se le parece.
            Bloque {
                titulo: qsTr("paleta")
                visible: inspector.cuantos === 1 && inspector.alDia
                         && inspector.ficha.paleta !== undefined && inspector.ficha.paleta.length > 0

                Item {
                    width: parent.width
                    height: Math.round(tema.fuente * 2.2)

                    Row {
                        id: paleta
                        anchors.fill: parent
                        readonly property var muestras: inspector.alDia && inspector.ficha.paleta
                                                        ? inspector.ficha.paleta : []
                        readonly property real suma: {
                            let s = 0
                            for (let i = 0; i < muestras.length; i++) s += muestras[i].peso
                            return s > 0 ? s : 1
                        }

                        Repeater {
                            model: paleta.muestras
                            delegate: Rectangle {
                                id: muestra
                                required property var modelData
                                width: paleta.width * modelData.peso / paleta.suma
                                height: paleta.height
                                color: modelData.color

                                MouseArea {
                                    id: sobreMuestra
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: ventana.ponerFiltro("color", muestra.modelData.color)
                                }
                            }
                        }
                    }

                    // Las esquinas de la tira, sin máscaras: un marco del color
                    // del panel con el radio por dentro.
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: -tema.radio
                        radius: tema.radio * 2
                        color: "transparent"
                        border.color: tema.panel
                        border.width: tema.radio
                    }
                }

                Text {
                    width: parent.width
                    text: qsTr("pincha un color para buscar lo que se le parece")
                    color: tema.textoTenue
                    font.pixelSize: tema.fuente * 0.8
                    elide: Text.ElideRight
                }
            }

            // ------------------------------------------------ etiquetas
            Bloque {
                // Con varios, el propio campo avisa de que lo que se haga va a
                // todos; el título no lo repite.
                titulo: qsTr("etiquetas")

                Etiquetas {
                    id: etiquetas
                    width: parent.width
                    cuantos: inspector.cuantos
                    lista: inspector.alDia && inspector.ficha.etiquetas ? inspector.ficha.etiquetas : []
                    onAnadir: function (t) { nucleo.etiquetar(modelo.seleccion, [t], []) }
                    onQuitar: function (t) { nucleo.etiquetar(modelo.seleccion, [], [t]) }
                    onBuscar: function (t) { ventana.filtrarPorEtiqueta(t) }
                }
            }

            // ----------------------------------------------------- nota
            // Se guarda al salir del campo, no con un botón: un botón de
            // guardar es una forma de perder lo escrito por no pulsarlo.
            Bloque {
                titulo: qsTr("nota")
                visible: inspector.cuantos === 1

                Rectangle {
                    width: parent.width
                    height: Math.max(Math.round(tema.fuente * 4.4), nota.implicitHeight + tema.hueco * 1.4)
                    radius: tema.radio
                    color: tema.fondo
                    border.color: nota.activeFocus ? tema.seleccion : tema.borde
                    border.width: 1

                    TextEdit {
                        id: nota
                        anchors.fill: parent
                        anchors.margins: tema.hueco * 0.7
                        wrapMode: TextEdit.Wrap
                        color: tema.texto
                        selectionColor: tema.seleccion
                        selectedTextColor: tema.fondo
                        font.pixelSize: tema.fuente * 0.95
                        text: inspector.alDia && inspector.ficha.nota ? inspector.ficha.nota : ""
                        onActiveFocusChanged: {
                            if (activeFocus) return
                            const antes = inspector.alDia && inspector.ficha.nota ? inspector.ficha.nota : ""
                            if (text !== antes) nucleo.nota(inspector.idFoco, text)
                        }
                        Keys.onEscapePressed: focus = false
                    }

                    Text {
                        anchors.left: nota.left
                        anchors.top: nota.top
                        visible: nota.text.length === 0 && !nota.activeFocus
                        text: qsTr("por qué lo guardaste…")
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente * 0.95
                    }
                }
            }

            // ----------------------------------------------- información
            // Sin adornos: son datos, y se leen mejor en columna que dentro de
            // una frase. Las filas que no aplican no se enseñan vacías: un «—»
            // en «duración» debajo de una foto no informa de nada.
            Bloque {
                titulo: qsTr("información")
                visible: inspector.cuantos === 1 && inspector.alDia

                Repeater {
                    model: [
                        { que: qsTr("medidas"), valor: Textos.medidas(inspector.ficha.ancho, inspector.ficha.alto) },
                        { que: qsTr("formato"), valor: (inspector.ficha.ext || "").toUpperCase()
                                                       + " · " + Textos.peso(inspector.ficha.peso || 0) },
                        { que: qsTr("duración"), valor: Textos.duracion(inspector.ficha.duracion_s) },
                        { que: qsTr("páginas"), valor: inspector.ficha.paginas ? String(inspector.ficha.paginas) : "" },
                        { que: qsTr("tamaño"), valor: Textos.medidasMm(inspector.ficha.medidas_mm) },
                        { que: qsTr("triángulos"), valor: inspector.ficha.triangulos ? Textos.numero(inspector.ficha.triangulos) : "" },
                        { que: qsTr("llegó"), valor: Textos.fecha(inspector.ficha.importado) },
                        { que: qsTr("carpetas"), valor: inspector.nombresDeCarpetas(inspector.ficha.carpetas) },
                        { que: qsTr("origen"), valor: inspector.origenCorto(inspector.ficha),
                          pista: inspector.ficha.fuente || inspector.ficha.procedencia || "" }
                    ].filter(function (f) { return f.valor && f.valor.length > 0 })
                    delegate: Item {
                        id: dato
                        required property var modelData
                        width: parent.width
                        height: valorDato.implicitHeight + 2

                        Text {
                            width: Math.round(tema.fuente * 6.5)
                            text: dato.modelData.que
                            color: tema.textoTenue
                            font.pixelSize: tema.fuente * 0.9
                        }
                        Text {
                            id: valorDato
                            x: Math.round(tema.fuente * 6.5)
                            width: dato.width - x
                            elide: Text.ElideMiddle
                            text: dato.modelData.valor
                            color: tema.texto
                            font.pixelSize: tema.fuente * 0.9
                        }
                        // La ruta o la dirección entera, al quedarse encima.
                        MouseArea {
                            id: sobreDato
                            anchors.fill: valorDato
                            hoverEnabled: dato.modelData.pista !== undefined && dato.modelData.pista.length > 0
                            onContainsMouseChanged: containsMouse
                                ? ventana.pistas.mostrar(valorDato, dato.modelData.pista, true)
                                : ventana.pistas.ocultar(valorDato)
                        }
                    }
                }
            }

            // ------------------------------------------------- acciones
            // Mover y la papelera primero y anchos, que son lo de todos los
            // días; lo demás debajo, en pequeño.
            Bloque {
                Row {
                    width: parent.width
                    spacing: tema.hueco * 0.6
                    Boton {
                        id: botonMover
                        width: (parent.width - parent.spacing) / 2
                        icono: "carpeta"
                        texto: qsTr("mover a…")
                        onPulsado: inspector.moverA(botonMover)
                    }
                    Boton {
                        width: (parent.width - parent.spacing) / 2
                        icono: "papelera"
                        texto: ventana.enPapelera ? qsTr("recuperar") : qsTr("papelera")
                        onPulsado: nucleo.papelera(modelo.seleccion, !ventana.enPapelera)
                    }
                }

                // Lo demás, con su icono y en dos columnas, como los de
                // arriba pero más bajos. Son las mismas acciones que el clic
                // derecho (`ventana.accionesElemento()`), menos las que ya
                // tienen botón grande.
                Grid {
                    id: rejillaAcciones
                    width: parent.width
                    columns: 2
                    columnSpacing: tema.hueco * 0.6
                    rowSpacing: tema.hueco * 0.5
                    Repeater {
                        model: ventana.accionesElemento().filter(function (a) {
                            return !a.separador && a.valor !== "mover" && a.valor !== "papelera"
                        })
                        delegate: Boton {
                            required property var modelData
                            centrado: false
                            recortar: true
                            width: (rejillaAcciones.width - rejillaAcciones.columnSpacing) / 2
                            altura: Math.round(tema.fuente * 1.9)
                            icono: modelData.icono
                            texto: modelData.texto
                            activo: modelData.valor === "adulto" && modelo.adultoFoco
                            onPulsado: ventana.hacerConElemento(modelData.valor)
                        }
                    }
                }
            }
        }
    }
}
