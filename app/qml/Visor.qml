// El visor a pantalla completa.
//
// Presupuesto: menos de 120 ms desde la pulsación hasta ver la imagen. Por eso
// pinta primero el color dominante, luego la miniatura de 320 px que ya está
// decodificada en el caché, y encima la previsualización de 1024 px cuando
// llega. Nunca hay un rectángulo vacío esperando al disco.
import QtQuick
import QtQuick.Window
import "textos.js" as Textos

Item {
    id: visor
    signal cerrar()
    /// Clic derecho en (x, y) de la ventana: el menú de lo que se ve, en vez
    /// de salir. Para salir quedan Esc y el doble clic.
    signal menu(real x, real y)

    readonly property int foco: modelo.actual
    readonly property string idFoco: foco >= 0 ? modelo.idDe(foco) : ""
    /// Si la imagen grande ya está en pantalla. Lo mira el banco del visor.
    readonly property bool grandeLista: grande.status === Image.Ready

    readonly property int familiaFoco: foco >= 0 ? modelo.familiaDe(foco) : -1
    readonly property bool esSonido: familiaFoco === Textos.AUDIO
    readonly property bool sePuedeReproducir: familiaFoco === Textos.VIDEO || esSonido
    /// La URL del original. La da la vista, no la ficha: pedir la ficha y
    /// esperarla metía una ida y vuelta al hilo del núcleo entre abrir un vídeo
    /// y que empezara a sonar, y durante esa vuelta la ruta era la del anterior.
    /// Con una versión de la pila elegida, la de esa versión.
    readonly property url urlFoco:
        !sePuedeReproducir || foco < 0 ? ""
        : version ? nucleo.urlDeRuta(version.ruta) : modelo.urlDe(foco)
    readonly property bool hayQueReproducir: String(urlFoco).length > 0
    /// Los PDF se leen página a página y los modelos 3D se giran: los dos
    /// tienen visor propio encima de la previsualización.
    /// Las fotos se acercan hasta el píxel (`VisorImagen.qml`). El RAW
    /// también, sobre la vista previa que lleva dentro.
    readonly property bool esImagen: familiaFoco === Textos.IMAGEN || familiaFoco === Textos.RAW
    readonly property bool esVideo: familiaFoco === Textos.VIDEO
    readonly property bool esDocumento: familiaFoco === Textos.DOCUMENTO
    readonly property bool esModelo: familiaFoco === Textos.MODELO
    readonly property bool conVisorPropio: esDocumento || esModelo

    /// La pila del elemento en foco, de la versión más pesada a la más ligera,
    /// con la principal marcada. Vacía si no es de ninguna. Sale de la ficha:
    /// la vista solo dice cuántas hay, no cuáles.
    readonly property var pila: nucleo.ficha.id === idFoco && nucleo.ficha.versiones
                                ? nucleo.ficha.versiones : []
    /// La versión que se está mirando, o vacío para la principal. Se vuelve a
    /// la principal al cambiar de elemento: una elección es de una pila.
    property string idVersion: ""
    readonly property var version: {
        for (let i = 0; i < pila.length; i++)
            if (pila[i].id === idVersion && !pila[i].principal) return pila[i]
        return null
    }

    /// Si ya se ha llegado a hacer falta un reproductor.
    ///
    /// Una vez creado no se destruye hasta cerrar el programa, y eso es a
    /// propósito: el motor de multimedia de Qt 6.4 se cae al montar y desmontar
    /// la tubería muchas veces seguidas —«double free or corruption» dentro de
    /// GStreamer, reproducible con `--visor 8`—. Un solo reproductor al que se
    /// le cambia la fuente no pasa por ahí nunca.
    property bool hizoFalta: false

    // ------------------------------------------------- pantalla completa
    /// La ventana entera a pantalla completa y lo que se mira de borde a
    /// borde. Cerrar el visor la devuelve como estaba.
    property bool pantallaCompleta: false
    property int visibilidadAntes: Window.Windowed

    function alternarPantallaCompleta() {
        if (!pantallaCompleta) {
            visibilidadAntes = ventana.visibility
            ventana.visibility = Window.FullScreen
            pantallaCompleta = true
        } else {
            ventana.visibility = visibilidadAntes === Window.FullScreen ? Window.Windowed
                                                                        : visibilidadAntes
            pantallaCompleta = false
        }
        despertar()
    }

    // ---------------------------------------------- la interfaz se aparta
    /// Con un vídeo en marcha y el ratón quieto un rato, los mandos, las
    /// estrellas, el nombre y el cursor se apartan: lo que se mira es el
    /// vídeo. Mover el ratón o tocar una tecla los trae de vuelta. Antes los
    /// mandos solo bajaban a medio transparentar y nunca se iban.
    property bool quietoRaton: false
    readonly property bool interfazFuera: quietoRaton && visible && !ratonEnBarras
        && !(rep !== null && hayQueReproducir && rep.ratonEnMandos)
        && (sinMarcos || hayQueReproducir && !esSonido && rep !== null && rep.enMarcha)

    // ------------------------------------------------ solo el elemento
    /// «Solo el elemento» (H o el botón de arriba): la foto o el vídeo de
    /// borde a borde en la ventana, sin pasar a pantalla completa, y la
    /// interfaz encima apartándose sola en cuanto el ratón se queda quieto.
    /// Se queda puesto al pasar de elemento y al cerrar y volver a abrir.
    property bool limpio: false
    /// Sin hueco para la interfaz: con «solo el elemento» o a pantalla
    /// completa, lo que se mira llega a los bordes y la interfaz va encima.
    readonly property bool sinMarcos: limpio || pantallaCompleta

    function alternarLimpio() {
        limpio = !limpio
        despertar()
        estado.avisar(limpio ? qsTr("solo el elemento · H para volver") : qsTr("con la interfaz"))
    }

    /// Lo que la interfaz ocupa arriba y abajo cuando tiene su sitio. Lo que
    /// se mira se encaja en lo que queda: antes iba de borde a borde y la
    /// barra de un vídeo le tapaba el pie, y las versiones el de una foto.
    readonly property real reservaArriba: sinMarcos ? 0
        : estrellas.y + estrellas.height + tema.hueco
          + (esVideo && versiones.visible ? versiones.height + tema.hueco : 0)
    readonly property real reservaAbajo: sinMarcos ? 0
        : tema.margen + rotulo.height + tema.hueco
          + (!esVideo && versiones.visible ? versiones.height + tema.hueco : 0)
    readonly property real reservaLados: sinMarcos ? 0 : tema.margen * 2
    /// El ratón encima de las versiones o de los botones: ahí no se apartan.
    readonly property bool ratonEnBarras: sobreVersiones.hovered || sobreAcciones.hovered
    /// El reproductor cargado, o null. Como `var`: sus propiedades son de un
    /// archivo que importa QtMultimedia, y qmllint no las ve.
    readonly property var rep: reproductor.status === Loader.Ready ? reproductor.item : null

    function despertar() {
        quietoRaton = false
        reposo.restart()
    }

    /// Dónde estaba el ratón la última vez que se movió de verdad.
    property point ultimoRaton: Qt.point(-1, -1)

    /// Qt Quick vuelve a mandar el «ratón encima» en cada fotograma que pinta,
    /// aunque el ratón no se mueva, y con un vídeo en marcha eso son treinta
    /// avisos por segundo: la cuenta no llegaba nunca al final. Solo cuenta
    /// moverse de verdad, unos píxeles.
    function ratonEn(x, y) {
        if (Math.abs(x - ultimoRaton.x) + Math.abs(y - ultimoRaton.y) < 3) return
        ultimoRaton = Qt.point(x, y)
        despertar()
    }

    Timer {
        id: reposo
        interval: 2500
        onTriggered: visor.quietoRaton = true
    }

    // El ratón fuera del vídeo —en los márgenes— lo ve este; encima del
    // vídeo lo ve el reproductor, que se queda el ratón, y avisa.
    HoverHandler {
        onPointChanged: visor.ratonEn(point.scenePosition.x, point.scenePosition.y)
    }

    focus: visible
    // Cerrado, el original decodificado para el cuentagotas sobra: puede ser
    // mucha memoria.
    onVisibleChanged: {
        if (visible) {
            forceActiveFocus()
            despertar()
            // Sin el panel de detalle nadie habría pedido la ficha, y es la
            // que dice qué versiones hay.
            if (idFoco.length > 0 && nucleo.ficha.id !== idFoco) nucleo.pedirFicha(idFoco)
        } else {
            pixeles.soltar()
            if (pantallaCompleta) alternarPantallaCompleta()
        }
    }
    onHayQueReproducirChanged: if (hayQueReproducir && !hizoFalta) espera.restart()

    // Un respiro antes de arrancar el motor de multimedia.
    //
    // Montarlo en el mismo fotograma que se abre el visor cuesta el fotograma
    // entero: medido con `--visor`, abrir pasaba de 6 ms a 691 ms, y lo que se
    // ve mientras es nada. Con esto la portada aparece a la primera y el vídeo
    // se pone en marcha justo después, que es el orden en el que a alguien le
    // importan las dos cosas.
    Timer {
        id: espera
        interval: 120
        onTriggered: visor.hizoFalta = true
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        opacity: visor.sinMarcos ? 1 : 0.97
    }

    Rectangle {
        id: lienzo
        anchors.fill: parent
        anchors.topMargin: visor.reservaArriba
        anchors.bottomMargin: visor.reservaAbajo
        anchors.leftMargin: visor.reservaLados
        anchors.rightMargin: visor.reservaLados
        color: "transparent"

        // Donde cae el vídeo: con los mandos en su sitio, encima de ellos, y
        // la portada en el mismo sitio para que no salte al arrancar.
        Item {
            id: huecoPortada
            anchors.fill: parent
            anchors.bottomMargin: visor.esVideo && visor.rep !== null && !visor.sinMarcos
                                  ? visor.rep.alturaMandos : 0
        }

        // La portada, con el giro guardado del elemento. Es lo que se ve
        // mientras arranca el vídeo o llega la previa de una foto: sin girar,
        // al pasar con las flechas a algo girado se veía un momento de pie y
        // luego se daba la vuelta.
        Item {
            readonly property bool deLado: modelo.giroFoco % 180 !== 0
            anchors.centerIn: huecoPortada
            width: deLado ? huecoPortada.height : huecoPortada.width
            height: deLado ? huecoPortada.width : huecoPortada.height
            rotation: modelo.giroFoco

            Image {
                id: borrosa
                anchors.fill: parent
                visible: grande.status !== Image.Ready && !visorPropio.listo
                source: visor.visible && visor.idFoco.length > 0 ? "image://grim/" + visor.idFoco : ""
                asynchronous: true
                cache: true
                fillMode: Image.PreserveAspectFit
                smooth: true
            }

            Image {
                id: grande
                anchors.fill: parent
                source: visor.visible && visor.idFoco.length > 0 ? "image://grim/previa/" + visor.idFoco : ""
                asynchronous: true
                cache: true
                fillMode: Image.PreserveAspectFit
                // Con una foto la pinta `VisorImagen`, que la acerca; esta se
                // sigue cargando —comparten caché— porque el banco del visor mide
                // cuándo llega.
                visible: !visorPropio.listo && !visor.esImagen
                opacity: status === Image.Ready ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }
            }
        }
    }

    VisorImagen {
        id: imagen
        anchors.fill: parent
        visible: visor.visible && visor.esImagen
        // Una versión es un elemento como otro: tiene su previsualización y su
        // archivo, y se enseña con los suyos.
        idElemento: !visible ? "" : visor.version ? visor.version.id : visor.idFoco
        fuenteArchivo: visible && visor.version ? nucleo.imagenDeRuta(visor.version.ruta) : ""
        original: !visible || visor.foco < 0 ? ""
                  : visor.version ? nucleo.urlDeRuta(visor.version.ruta)
                  : modelo.urlDe(visor.foco)
        anchoPista: visor.version ? visor.version.ancho
                    : nucleo.ficha.id === visor.idFoco ? (nucleo.ficha.ancho || 0) : 0
        altoPista: visor.version ? visor.version.alto
                   : nucleo.ficha.id === visor.idFoco ? (nucleo.ficha.alto || 0) : 0
        giro: modelo.giroFoco
        margenArriba: visor.reservaArriba
        margenAbajo: visor.reservaAbajo
        margenLados: visor.reservaLados
        interfazFuera: visor.interfazFuera
        onPedirGiro: function (grados) { visor.guardarGiro(grados) }
        onSiguiente: visor.mover(1)
        onCerrar: visor.cerrar()
        onMenu: function (x, y) { visor.menu(x, y) }
    }

    // Encima de la previsualización y no en su lugar: mientras el reproductor
    // arranca, lo que se ve es el fotograma de portada que ya estaba puesto.
    Loader {
        id: reproductor
        anchors.fill: lienzo
        z: 1
        active: visor.hizoFalta
        visible: visor.hayQueReproducir
        source: "qrc:/qml/Reproductor.qml"
    }

    // El id va antes que la fuente: al cambiar la fuente el reproductor pide
    // los derivados del elemento, y tiene que pedirlos del nuevo.
    Binding {
        target: reproductor.item
        property: "idElemento"
        value: visor.version ? visor.version.id : visor.idFoco
        when: reproductor.status === Loader.Ready
    }
    Binding {
        target: reproductor.item
        property: "fuente"
        value: visor.urlFoco
        when: reproductor.status === Loader.Ready
    }
    Binding {
        target: reproductor.item
        property: "soloSonido"
        value: visor.esSonido
        when: reproductor.status === Loader.Ready
    }
    // Cerrar el visor calla lo que estuviera sonando. El reproductor le
    // sobrevive —no se destruye nunca, ver `hizoFalta`—, así que si no se le
    // dice nada el sonido sigue detrás de la malla.
    Binding {
        target: reproductor.item
        property: "enPantalla"
        value: visor.visible
        when: reproductor.status === Loader.Ready
    }
    // En bucle también a pantalla completa: un vídeo corto que se queda
    // congelado en su último fotograma parece que se ha roto.
    Binding {
        target: reproductor.item
        property: "bucle"
        value: true
        when: reproductor.status === Loader.Ready
    }
    Binding {
        target: reproductor.item
        property: "giro"
        value: modelo.giroFoco
        when: reproductor.status === Loader.Ready
    }
    Binding {
        target: reproductor.item
        property: "mandosOcultos"
        value: visor.interfazFuera
        when: reproductor.status === Loader.Ready
    }
    Binding {
        target: reproductor.item
        property: "reservarMandos"
        value: !visor.sinMarcos
        when: reproductor.status === Loader.Ready
    }
    Binding {
        target: reproductor.item
        property: "enPantallaCompleta"
        value: visor.pantallaCompleta
        when: reproductor.status === Loader.Ready
    }
    Connections {
        target: reproductor.item
        ignoreUnknownSignals: true
        function onPedirPantallaCompleta() { visor.alternarPantallaCompleta() }
        function onActividad(x, y) { visor.ratonEn(x, y) }
        function onPedirGiro(grados) { visor.guardarGiro(grados) }
    }
    Binding {
        target: reproductor.item
        property: "caratula"
        value: visor.idFoco.length > 0 ? "image://grim/" + visor.idFoco : ""
        when: reproductor.status === Loader.Ready
    }

    // El PDF y el modelo 3D, con su visor. Se monta solo con el visor abierto:
    // cerrado, cada flecha de la malla pediría una malla o un documento entero.
    Loader {
        id: visorPropio
        anchors.fill: lienzo
        z: 1
        active: visor.visible && visor.conVisorPropio && visor.idFoco.length > 0
        source: visor.esDocumento ? "qrc:/qml/VisorPdf.qml" : "qrc:/qml/VisorModelo.qml"
        readonly property bool listo: status === Loader.Ready && active
        onLoaded: visor.darVisorPropio()
    }
    onIdFocoChanged: {
        // La cuenta para apartar la interfaz empieza también al cambiar de
        // elemento: con el ratón quieto desde antes no empezaba nunca.
        despertar()
        idVersion = ""
        if (visible && idFoco.length > 0) nucleo.pedirFicha(idFoco)
        darVisorPropio()
    }

    function darVisorPropio() {
        const v = visorPropio.item
        if (!v || !visor.conVisorPropio || visor.foco < 0) return
        // La url antes que el id: es el id el que dispara la petición.
        v.original = modelo.urlDe(visor.foco)
        if (visor.esModelo) v.ext = modelo.extDe(visor.foco).toLowerCase()
        v.idElemento = visor.idFoco
    }

    // Sin el módulo de multimedia de Qt el `Loader` no carga, y eso no puede
    // quedar en un vídeo que no hace nada al pulsarlo. Se dice qué falta y se
    // ofrece la salida que sí existe siempre.
    Column {
        anchors.centerIn: parent
        z: 2
        spacing: tema.hueco
        width: parent.width * 0.6
        visible: reproductor.status === Loader.Error && visor.hayQueReproducir

        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: qsTr("para ver vídeo y oír sonido dentro de Grimorio falta el módulo "
                       + "de multimedia de Qt:\n\nsudo apt install qml6-module-qtmultimedia")
            color: tema.textoTenue
            font.pixelSize: tema.fuente
        }

        Boton {
            anchors.horizontalCenter: parent.horizontalCenter
            // Dentro de una `Column` el botón no puede centrarse solo: la
            // columna ya coloca, y dos cosas colocando lo mismo la rompen.
            centrado: false
            texto: qsTr("abrir con el sistema")
            onPulsado: nucleo.abrirFuera(visor.idFoco)
        }
    }

    // Las versiones de la pila, por peso. Elegir una la enseña en su lugar;
    // arrastrarla la lleva a otra aplicación tal cual, y el botón la copia.
    // Es para lo que existen: pasar la ligera sin tener que exportar nada.
    //
    // Con vídeos también: un vídeo repetido se apila igual que una foto, y
    // sin esto la única forma de ver las otras copias era sacarlas de la pila.
    // Ahí va arriba, debajo de las estrellas: abajo están los mandos.
    //
    // Por encima del reproductor y del visor propio, que van en `z: 1`: sin
    // esto el vídeo tapa la fila.
    Row {
        id: versiones
        z: 3
        anchors.horizontalCenter: parent.horizontalCenter
        // Con `y` y no con anclas: al pasar de un vídeo a una foto las dos
        // anclas, arriba y abajo, llegaban a estar puestas a la vez, la fila
        // se estiraba a todo el alto y el hueco de la foto se quedaba en nada.
        y: visor.esVideo ? estrellas.y + estrellas.height + tema.hueco
                         : rotulo.y - height - tema.hueco
        spacing: tema.hueco * 0.5
        visible: (visor.esImagen || visor.esVideo) && visor.pila.length > 1
        opacity: visor.interfazFuera ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }
        HoverHandler { id: sobreVersiones }

        Repeater {
            model: versiones.visible ? visor.pila : []
            delegate: Rectangle {
                id: chip
                required property var modelData
                readonly property bool elegida: modelData.principal ? visor.version === null
                                                                    : visor.idVersion === modelData.id
                width: textosChip.implicitWidth + tema.hueco * 1.6
                height: textosChip.implicitHeight + tema.hueco * 0.8
                radius: tema.radio
                color: tema.panel
                border.color: elegida ? tema.seleccion : tema.borde
                border.width: elegida ? 2 : 1

                Column {
                    id: textosChip
                    anchors.centerIn: parent
                    spacing: tema.hueco * 0.1
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: Textos.peso(chip.modelData.peso)
                        color: tema.texto
                        font.pixelSize: tema.fuente
                        font.weight: chip.elegida ? Font.DemiBold : Font.Normal
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: chip.modelData.ancho + "×" + chip.modelData.alto + " · "
                              + chip.modelData.ext
                              + (chip.modelData.principal ? " · " + qsTr("principal") : "")
                        color: tema.textoTenue
                        font.pixelSize: tema.fuente * 0.78
                    }
                }

                MouseArea {
                    id: agarre
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    property point desde
                    onPressed: function (e) { desde = Qt.point(e.x, e.y) }
                    onClicked: visor.idVersion = chip.modelData.principal ? "" : chip.modelData.id
                    onPositionChanged: function (e) {
                        if (!pressed) return
                        if (Math.abs(e.x - desde.x) + Math.abs(e.y - desde.y) < tema.fuente) return
                        portapapeles.arrastrarFuera(agarre, [nucleo.urlDeRuta(chip.modelData.ruta)], "")
                    }
                }
            }
        }

        Boton {
            icono: "copiar"
            pista: qsTr("copiar esta versión")
            onPulsado: {
                const v = visor.version
                const ruta = v ? v.ruta : ""
                const n = ruta.length > 0
                          ? portapapeles.copiarArchivos([nucleo.urlDeRuta(ruta)])
                          : portapapeles.copiarArchivos([modelo.urlDe(visor.foco)])
                if (n > 0) estado.avisar(qsTr("copiada esta versión"))
            }
        }
        Boton {
            icono: "pila"
            visible: visor.version !== null
            texto: qsTr("hacer principal")
            onPulsado: {
                nucleo.hacerPrincipal(visor.version.id)
                visor.idVersion = ""
            }
        }
    }

    Text {
        id: rotulo
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: tema.margen
        width: parent.width * 0.7
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        text: visor.foco >= 0
              ? modelo.nombreDe(visor.foco) + "  ·  " + Textos.posicion(visor.foco + 1, modelo.total)
              : ""
        color: tema.textoTenue
        font.pixelSize: tema.fuente
        // Pegado al vídeo a pantalla completa, y fuera mientras se mira.
        opacity: visor.interfazFuera || visor.pantallaCompleta && visor.hayQueReproducir ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }
    }

    // Lo que se hace con lo que se mira, arriba a la derecha: editar, quitar
    // el fondo, y dos formas de quitarse la interfaz de en medio. Con una
    // versión de la pila delante, editar y quitar el fondo no salen: harían
    // su trabajo sobre la principal, que no es lo que se ve.
    Row {
        id: acciones
        z: 3
        anchors.right: parent.right
        anchors.rightMargin: tema.margen
        anchors.verticalCenter: estrellas.verticalCenter
        spacing: tema.hueco * 0.5
        visible: visor.esImagen || visor.esVideo
        opacity: visor.interfazFuera ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }
        HoverHandler { id: sobreAcciones }

        Boton {
            id: botonEditar
            icono: "lapiz"
            texto: qsTr("editar")
            readonly property bool conEditor: visor.foco >= 0 && editor.sabe(visor.foco)
            pista: conEditor ? qsTr("recortar en el editor (E)") : qsTr("girar, sacar versiones…")
            visible: visor.version === null && (conEditor || visor.esImagen)
            onPulsado: {
                if (conEditor) { ventana.hacerConElemento("editor"); return }
                // Una foto no tiene editor propio todavía: lo que se le puede
                // hacer, en un menú debajo del botón.
                let a = [
                    { texto: qsTr("girar a la izquierda"), valor: "girarIzq", icono: "girarIzq", atajo: "Mayús+R" },
                    { texto: qsTr("girar a la derecha"), valor: "girarDer", icono: "girarDer", atajo: "R" },
                    { texto: qsTr("crear versión…"), valor: "version", icono: "pila" }
                ]
                a.push({ separador: true },
                       { texto: qsTr("abrir con otro programa"), valor: "fuera", icono: "externo" })
                const p = botonEditar.mapToItem(null, 0, botonEditar.height + tema.hueco * 0.5)
                desplegable.abrirEn(p.x, p.y, {
                    opciones: a,
                    alElegir: function (v) { ventana.hacerConElemento(v, p.x, p.y) }
                })
            }
        }
        Boton {
            icono: "fotograma"
            texto: qsTr("guardar fotograma")
            pista: qsTr("lo que se ve ahora, como imagen aparte en tu biblioteca (S)")
            visible: visor.esVideo
            onPulsado: visor.sacarFotograma()
        }
        Boton {
            icono: "recorte"
            texto: qsTr("quitar el fondo")
            pista: qsTr("saca una versión sin fondo en la pila; se hace en tu ordenador")
            visible: visor.esImagen && visor.version === null && recorte.hayPrograma
            onPulsado: ventana.hacerConElemento("fondo")
        }
        Boton {
            icono: "ojo"
            activo: visor.limpio
            pista: visor.limpio ? qsTr("volver a la interfaz (H)")
                                : qsTr("solo el elemento, de borde a borde (H)")
            onPulsado: visor.alternarLimpio()
        }
        Boton {
            icono: "pantalla"
            activo: visor.pantallaCompleta
            // El vídeo ya lo tiene en sus mandos.
            visible: !visor.hayQueReproducir
            pista: qsTr("pantalla completa (F)")
            onPulsado: visor.alternarPantallaCompleta()
        }
    }

    Estrellas {
        id: estrellas
        // Igual que las versiones: a pantalla completa el vídeo llega arriba.
        z: 3
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: tema.margen
        opacity: visor.interfazFuera ? 0 : 1
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }
        valor: modelo.estrellasFoco
        onElegido: function (n) { nucleo.estrellas(modelo.seleccion, n) }
    }

    /// Guarda lo que se ve del vídeo ahora mismo como una imagen aparte.
    function sacarFotograma() {
        if (!esVideo || rep === null) return
        ventana.sacarFotograma(version ? version.id : idFoco, urlFoco, idFoco, rep.posicionPedida())
    }

    /// El giro se guarda en el elemento: se queda al volver a verlo, en la
    /// malla y en el panel, y se deshace con Ctrl+Z como lo demás.
    function guardarGiro(grados) {
        if (idFoco.length > 0) nucleo.girar([idFoco], grados)
    }

    /// Pasa a la versión siguiente de la pila, dando la vuelta. Con la tecla V
    /// se comparan copias sin soltar el teclado.
    function cambiarVersion(paso) {
        if (pila.length < 2) return false
        let i = 0
        for (let j = 0; j < pila.length; j++)
            if (version ? pila[j].id === version.id : pila[j].principal) i = j
        const otra = pila[(i + paso + pila.length) % pila.length]
        idVersion = otra.principal ? "" : otra.id
        return true
    }

    /// Supr con una pila delante tira la versión que se está viendo en
    /// grande, no la pila entera ni la que manda en la galería: es la única
    /// que se tiene delante y la única que uno cree estar borrando.
    ///
    /// Si es una versión, se va sola y el visor pasa a la de al lado, como al
    /// tirar en la galería. Si es la principal, también se va sola: la más
    /// pesada de las que quedan hereda la pila —la misma regla que el núcleo,
    /// misma cuenta aquí— y es la que se enseña al volver la vista. Las dos
    /// cosas se deshacen con Ctrl+Z. Devuelve si se ocupó de la tecla.
    function tirarLoQueSeVe() {
        if (!(esImagen || esVideo) || pila.length < 2) return false
        let i = 0
        for (let j = 0; j < pila.length; j++)
            if (version ? pila[j].id === version.id : pila[j].principal) i = j
        const tirada = pila[i]
        if (!tirada.principal) {
            const otra = pila[i + 1 < pila.length ? i + 1 : i - 1]
            idVersion = otra.principal ? "" : otra.id
            nucleo.papelera([tirada.id], true, false)
            return true
        }
        let nueva = null
        for (let k = 0; k < pila.length; k++) {
            const v = pila[k]
            if (v.principal) continue
            if (!nueva || v.peso > nueva.peso || (v.peso === nueva.peso && v.id < nueva.id))
                nueva = v
        }
        // Por si la heredera no entra en la búsqueda que hay puesta: entonces
        // se va de la vista y se pasa página como al tirar cualquier cosa.
        ventana.apuntarSitio()
        ventana.porEnsenar = nueva.id
        ventana.herencia = { de: tirada.id, a: nueva.id }
        nucleo.papelera([tirada.id], true, false)
        return true
    }

    function mover(paso) {
        const destino = Math.max(0, Math.min(modelo.total - 1, modelo.actual + paso))
        if (destino !== modelo.actual) modelo.elegir(destino)
    }

    /// Adelantar o atrasar, si lo que hay delante se reproduce.
    function saltar(segundos) {
        if (reproductor.status !== Loader.Ready) return false
        reproductor.item.saltar(segundos)
        return true
    }

    /// Lo que solo tiene sentido con algo reproduciéndose delante.
    function mandar(que) {
        if (reproductor.status !== Loader.Ready || !visor.sePuedeReproducir) return false
        const r = reproductor.item
        switch (que) {
        case "paso-": r.paso(-1); break
        case "paso+": r.paso(1); break
        case "lento": r.cambiarVelocidad(-1); break
        case "rapido": r.cambiarVelocidad(1); break
        case "a": r.marcar("a"); break
        case "b": r.marcar("b"); break
        case "tramo": r.quitarTramo(); break
        case "bucle": r.bucle = !r.bucle; break
        case "silencio": r.silencio = !r.silencio; break
        }
        return true
    }

    Keys.onPressed: function (evento) {
        despertar()
        const conMayus = (evento.modifiers & Qt.ShiftModifier) !== 0
        if (evento.key === Qt.Key_F && !(evento.modifiers & Qt.ControlModifier)) {
            alternarPantallaCompleta()
            evento.accepted = true
            return
        }
        if (evento.key === Qt.Key_S && !(evento.modifiers & Qt.ControlModifier) && esVideo) {
            sacarFotograma()
            evento.accepted = true
            return
        }
        if (evento.key === Qt.Key_H && !(evento.modifiers & Qt.ControlModifier)) {
            alternarLimpio()
            evento.accepted = true
            return
        }
        // R gira también un vídeo, como una foto.
        if (evento.key === Qt.Key_R && hayQueReproducir && !esSonido && rep !== null) {
            rep.girar(conMayus ? -1 : 1)
            evento.accepted = true
            return
        }
        // Z, con un vídeo acercado, lo devuelve a su tamaño.
        if (evento.key === Qt.Key_Z && hayQueReproducir && rep !== null && rep.zoom > 1) {
            rep.quitarZoom()
            evento.accepted = true
            return
        }
        if (evento.key === Qt.Key_V && !(evento.modifiers & Qt.ControlModifier)
                && visor.cambiarVersion(conMayus ? -1 : 1)) {
            evento.accepted = true
            return
        }
        // Las teclas de los mandos. Son las de los editores de vídeo —coma y
        // punto para el fotograma, I y O para entrada y salida— para que la
        // mano vaya sola.
        const mandos = {}
        mandos[Qt.Key_Comma] = "paso-"
        mandos[Qt.Key_Period] = "paso+"
        mandos[Qt.Key_BracketLeft] = "lento"
        mandos[Qt.Key_BracketRight] = "rapido"
        mandos[Qt.Key_I] = "a"
        mandos[Qt.Key_O] = "b"
        mandos[Qt.Key_X] = "tramo"
        mandos[Qt.Key_L] = "bucle"
        mandos[Qt.Key_M] = "silencio"
        if (mandos[evento.key] !== undefined && visor.mandar(mandos[evento.key])) {
            evento.accepted = true
            return
        }
        // El zoom de las fotos.
        if (visor.esImagen) {
            let hecho = true
            switch (evento.key) {
            case Qt.Key_Plus:
            case Qt.Key_Equal: imagen.acercar(1.25); break
            case Qt.Key_Minus: imagen.acercar(0.8); break
            case Qt.Key_Z: imagen.alternarCentro(); break
            case Qt.Key_R: imagen.girar(conMayus ? -1 : 1); break
            case Qt.Key_C: hecho = imagen.copiarColor(); break
            default: hecho = false
            }
            if (hecho) {
                evento.accepted = true
                return
            }
        }
        if (visor.esDocumento && visorPropio.listo
                && (evento.key === Qt.Key_PageDown || evento.key === Qt.Key_PageUp)) {
            visorPropio.item.pagina(evento.key === Qt.Key_PageDown ? 1 : -1)
            evento.accepted = true
            return
        }
        switch (evento.key) {
        case Qt.Key_Escape: visor.cerrar(); break
        // Mayúsculas y flecha salta diez segundos. Sin mayúsculas sigue
        // siendo pasar de elemento: dentro del visor eso es lo que más se hace,
        // también cuando lo que se está viendo es un vídeo.
        case Qt.Key_Left:
            if (conMayus && visor.saltar(-10)) break
            visor.mover(-1); break
        case Qt.Key_Up:
        case Qt.Key_Backspace: visor.mover(-1); break
        case Qt.Key_Right:
            if (conMayus && visor.saltar(10)) break
            visor.mover(1); break
        case Qt.Key_Down: visor.mover(1); break
        // El espacio pasa a la siguiente foto, salvo si lo que hay delante se
        // reproduce: ahí todo el mundo espera que pare y siga.
        case Qt.Key_Space:
            if (reproductor.status === Loader.Ready) reproductor.item.alternar()
            else visor.mover(1)
            break
        case Qt.Key_Home: modelo.elegir(0); break
        case Qt.Key_End: modelo.elegir(modelo.total - 1); break
        default: return
        }
        evento.accepted = true
    }

    MouseArea {
        anchors.fill: parent
        z: -1
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: function (evento) {
            // Clic izquierdo avanza, como en cualquier visor de fotos; el
            // derecho abre el menú de lo que se ve y el doble clic sale.
            if (evento.button === Qt.RightButton) {
                const p = mapToItem(null, evento.x, evento.y)
                visor.menu(p.x, p.y)
            } else {
                visor.mover(1)
            }
        }
        onDoubleClicked: visor.cerrar()
        // La rueda pasa de elemento, salvo donde el visor propio la usa: en un
        // PDF baja páginas y en un modelo acerca. Aquí llega solo lo que cae
        // fuera de ellos.
        onWheel: function (rueda) { visor.mover(rueda.angleDelta.y > 0 ? -1 : 1) }
    }
}
