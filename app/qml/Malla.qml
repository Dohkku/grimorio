// La malla.
//
// Un Flickable con la disposición calculada en C++ y un anillo de ranuras que
// se reasignan al desplazarse. No es GridView porque GridView solo hace celdas
// iguales, y las filas justificadas son la mitad de por qué una galería de
// referencias se mira a gusto.
import QtQuick
import "textos.js" as Textos

Flickable {
    id: malla

    contentWidth: width
    contentHeight: Math.max(height, disposicion.alturaTotal)
    boundsBehavior: Flickable.StopAtBounds
    clip: true
    focus: true
    maximumFlickVelocity: 6000
    flickDeceleration: 3500

    // Tres cuartos de pantalla de precarga a cada lado, que es lo que midió el
    // spike: con menos, un desplazamiento rápido enseña celdas vacías; con más,
    // se decodifica trabajo que nunca se ve.
    function actualizar() {
        const margen = height * 0.75
        disposicion.mirar(contentY - margen, contentY + height + margen)
        pedirReparto()
    }

    /// Qué vídeo se mueve y dónde: `{indice, x, y, ancho, alto}`, o nada.
    ///
    /// Uno. Hubo un modo que movía todos los que se veían y se quitó midiendo:
    /// cada vídeo en marcha es una tubería de decodificación entera, y cinco
    /// costaban 0,88 GB y un tercio de un núcleo contra los 0,19 GB del
    /// programa sin vídeo.
    property var videoEncima: undefined


    // El reparto se pide, no se hace en el acto.
    //
    // Al desplazarse cambia en cada fotograma, y cambiar la fuente sesenta
    // veces por segundo no reproduce nada: solo tira tuberías de decodificación
    // a medio montar. Y al pasar el ratón por encima de una fila entera, sin
    // espera, cada celda por la que se pasa de largo arrancaría su vídeo.
    Timer {
        id: espera
        interval: 180
        onTriggered: malla.repartirVideos()
    }

    // Mientras la malla se mueve, la cuenta se aplaza —cada reasignación tira
    // una tubería de decodificación a medio montar—. Parada, el reparto llega
    // como muy tarde a los 180 ms de la primera petición, no de la última: al
    // arrancar, la geometría se asienta en varias pasadas y con `restart` en
    // todas el primer vídeo tardaba tres segundos en moverse.
    function pedirReparto() {
        if (malla.moving) espera.restart()
        else if (!espera.running) espera.start()
    }

    function repartirVideos() {
        videos.sitio = ajustes.videoAlPasar ? videoEncima : undefined
    }

    function marcarPasada(dentro, indice, x, y, ancho, alto) {
        if (dentro) {
            videoEncima = { "indice": indice, "x": x, "y": y,
                            "ancho": ancho, "alto": alto }
        } else if (videoEncima !== undefined && videoEncima.indice === indice) {
            videoEncima = undefined
        }
        pedirReparto()
    }

    Connections {
        target: ajustes
        function onCambio() { malla.repartirVideos() }
    }

    // Otra consulta: lo que había repartido ya no vale, y hay que soltarlo en
    // el acto.
    //
    // El reparto lleva índices, y un índice de la vista de antes apunta en la
    // nueva a otra cosa. Muchas veces a una foto —el reproductor la decodifica
    // como un vídeo de un solo fotograma y la pinta donde estaba el vídeo
    // viejo, con el tamaño del vídeo viejo—: eso es el cuadro que aparecía
    // flotando fuera de su sitio al entrar en una carpeta. Y el vídeo bueno de
    // la carpeta no arrancaba hasta que el reparto se rehacía.
    //
    // Vaciarlo aquí también apaga el que estuviera sonando: la vista que lo
    // enseñaba ya no está.
    Connections {
        target: modelo
        function onModelReset() {
            videos.sitio = undefined
            malla.videoEncima = undefined
            malla.pedirReparto()
        }
    }

    function llevarHasta(i) {
        const y = disposicion.yDe(i)
        const h = disposicion.altoDe(i)
        if (y < contentY) contentY = Math.max(0, y - tema.hueco)
        else if (y + h > contentY + height)
            contentY = Math.min(contentHeight - height, y + h - height + tema.hueco)
    }

    onWidthChanged: disposicion.ancho = width
    onContentYChanged: actualizar()
    onMovingChanged: pedirReparto()
    onHeightChanged: actualizar()
    Component.onCompleted: disposicion.ancho = width

    Connections {
        target: disposicion
        function onGeometriaCambio() {
            malla.actualizar()
            malla.situarFoco()
        }
    }

    // --------------------------------------------------------------- el foco

    /// Dónde está ahora mismo lo que enseña el panel de detalle, o `undefined`.
    property var focoSitio: undefined
    /// Para poner el anillo en su sitio de golpe la primera vez, en vez de
    /// hacerlo cruzar la malla desde la esquina.
    property bool focoSinViaje: true

    /// Pone el anillo donde estaba la celda al pincharla, sin viaje: ese es el
    /// punto de partida del que sale cuando la malla se rehace.
    function sembrarFoco(x, y, ancho, alto) {
        ultimoFoco = 0
        latido.restart()
        focoSinViaje = true
        focoSitio = { "x": x, "y": y, "ancho": ancho, "alto": alto }
        focoSinViaje = false
    }

    /// Cuándo se movió el foco por última vez, para saber si vienen seguidos.
    property double ultimoFoco: 0

    function situarFoco() {
        // Con las teclas apretadas seguidas, el anillo salta en vez de viajar.
        //
        // El viaje y el brillo están para no perderle la pista a un salto, y
        // eso solo tiene sentido cuando hay un salto que mirar. Yendo deprisa
        // no se mira ninguno —se mira el final del camino— y en cambio se
        // pagan: cada uno repinta las tres bandas del halo sesenta veces por
        // segundo durante medio segundo, y con una tecla cada sesenta
        // milisegundos eso no para nunca. Medido paseando con las flechas:
        // 262 ms por tecla con el anillo, 175 ms sin él.
        const ahora = Date.now()
        const seguido = ahora - ultimoFoco < 250
        ultimoFoco = ahora

        const habia = focoSitio !== undefined
        const s = modelo.actual >= 0 ? disposicion.sitioDe(modelo.actual) : undefined
        focoSinViaje = !habia || seguido
        focoSitio = (s !== undefined && s.alto !== undefined) ? s : undefined
        focoSinViaje = false
        if (!seguido) latido.restart()
    }

    Connections {
        target: modelo
        function onActualCambio() { malla.situarFoco() }
    }

    // Un clic en el hueco entre celdas deselecciona: es lo que espera cualquiera
    // que venga de un explorador de archivos. Y también devuelve el foco, que
    // el hueco es malla igual que las celdas.
    MouseArea {
        anchors.fill: parent
        z: -1
        onClicked: {
            malla.forceActiveFocus()
            modelo.limpiarSeleccion()
        }
    }

    Repeater {
        model: disposicion
        delegate: Celda {
            onElegida: function (cx, cy, cancho, calto) {
                // Tocar la malla se lleva el foco de vuelta. Pinchar una
                // carpeta o escribir en el buscador se lo lleva —tiene que
                // llevárselo—, y sin esto las flechas se quedaban hablándole al
                // panel el resto de la sesión: se elegía una foto con el ratón
                // y moverse con el teclado no hacía nada.
                malla.forceActiveFocus()
                malla.sembrarFoco(cx, cy, cancho, calto)
            }
            onPasada: function (dentro, indice, cx, cy, cancho, calto) {
                if (familia === Textos.VIDEO && !censurado)
                    malla.marcarPasada(dentro, indice, cx, cy, cancho, calto)
            }
        }
    }

    // Después del `Repeater`: los vídeos van por encima de las celdas.
    MallaVideos {
        id: videos
        anchors.fill: parent
    }

    // El anillo del foco, por encima de todo, incluidos los vídeos.
    //
    // Elegir algo abre el panel de detalle, y abrirlo estrecha la malla: la
    // fila entera se rehace y lo que se acaba de elegir cambia de sitio, a
    // veces de fila. Con el borde de la propia celda y nada más hay que volver
    // a buscarlo con los ojos. Este anillo va aparte de las celdas y se desliza
    // hasta el sitio nuevo, así que se sigue mirando y no hay que buscarlo.
    //
    // Uno solo para toda la malla, no uno por celda: preguntar en cada una de
    // las trescientas ochenta y cuatro si es ella la elegida son trescientas
    // ochenta y cuatro ataduras a `modelo.actual`, y esto pinta lo mismo con un
    // nodo. Es la misma razón por la que los vídeos van aquí arriba y no dentro.
    Item {
        id: foco
        visible: malla.focoSitio !== undefined && !disposicion.reordenando
        x: malla.focoSitio !== undefined ? malla.focoSitio.x : 0
        y: malla.focoSitio !== undefined ? malla.focoSitio.y : 0
        width: malla.focoSitio !== undefined ? malla.focoSitio.ancho : 0
        height: malla.focoSitio !== undefined ? malla.focoSitio.alto : 0

        readonly property int grosor: Math.max(2, Math.round(tema.fuente * 0.25))
        readonly property int aire: Math.max(2, Math.round(tema.fuente * 0.35))
        /// El golpe de luz mientras se mueve, que es lo que engancha el ojo.
        property real pulso: 0

        // El viaje es lo que hace el trabajo: se ve **ir**, no aparecer en otro
        // sitio. Corto, que esto acompaña a un cambio de sitio, no lo anuncia.
        Behavior on x { enabled: !malla.focoSinViaje; NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
        Behavior on y { enabled: !malla.focoSinViaje; NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
        Behavior on width { enabled: !malla.focoSinViaje; NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
        Behavior on height { enabled: !malla.focoSinViaje; NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }

        // Brilla al cambiar de sitio y se apaga solo.
        //
        // Lo enciende quien mueve el foco, no el propio movimiento: atado a
        // `onXChanged` se reiniciaba en cada fotograma del viaje, y con el
        // brillo clavado arriba las tres bandas del halo se repintaban a
        // sesenta por segundo sin parar. Paseando con las flechas eso costaba
        // 87 ms por tecla, un tercio de todo lo que costaba una tecla.

        NumberAnimation {
            id: latido
            target: foco
            property: "pulso"
            from: 0.5
            to: 0
            duration: 550
            easing.type: Easing.OutCubic
        }

        // El halo: lo que se ve de reojo. Un anillo pegado al borde y nada más
        // se pierde contra una foto clara.
        //
        // Tres bandas y no una, cada una más ancha y más tenue, porque sin
        // módulo de efectos —el Qt de Ubuntu 24.04 no trae `QtQuick.Effects`—
        // un degradado es esto. Son tres nodos de una sola instancia, no tres
        // por celda.
        //
        // Bordes anchos y no rectángulos rellenos: relleno tiñe la foto entera
        // de cian, y lo que hay que resaltar es la foto, no taparla. Un borde
        // se dibuja hacia dentro de sus límites, así que con el margen negativo
        // justo cae **fuera** de la celda y ni la roza.
        Repeater {
            model: 3
            delegate: Rectangle {
                required property int index
                readonly property int fuera: foco.aire * (index + 1)
                anchors.fill: parent
                anchors.margins: -fuera
                radius: tema.radio + fuera
                color: "transparent"
                border.color: tema.seleccion
                border.width: fuera
                opacity: (0.34 / (index + 1)) + foco.pulso / (index + 1)
            }
        }

        // Un filo del color del fondo por fuera del anillo. Sin él, el cian
        // sobre una foto con cian dentro deja de ser un borde.
        Rectangle {
            anchors.fill: parent
            anchors.margins: -foco.grosor
            radius: tema.radio + foco.grosor
            color: "transparent"
            border.color: tema.fondo
            border.width: foco.grosor
            opacity: 0.55
        }

        Rectangle {
            anchors.fill: parent
            radius: tema.radio
            color: "transparent"
            border.color: tema.seleccion
            border.width: foco.grosor
        }
    }

    Text {
        // Dentro del contenido pero anclado a la vista: si no, con la malla
        // vacía el mensaje se iría a la coordenada cero y quedaría fuera.
        y: malla.contentY + malla.height / 2 - height / 2
        x: malla.width / 2 - width / 2
        visible: modelo.total === 0
        width: malla.width * 0.6
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        // Tres mensajes y no uno: una papelera vacía es una buena noticia, una
        // biblioteca vacía es una invitación, y una búsqueda sin resultados es
        // lo único que merece que se repita lo que se buscó.
        text: ventana.enPapelera
              ? qsTr("la papelera está vacía")
              : (ventana.textoBusqueda.length > 0
                 ? qsTr("nada que encaje con «%1»").arg(ventana.textoBusqueda)
                 : qsTr("arrastra imágenes aquí para empezar"))
        color: tema.textoTenue
        font.pixelSize: tema.fuente * 1.1
    }

    Keys.onPressed: function (evento) {
        let destino = modelo.actual
        if (destino < 0) destino = 0
        switch (evento.key) {
        case Qt.Key_Left:  destino = Math.max(0, destino - 1); break
        case Qt.Key_Right: destino = Math.min(modelo.total - 1, destino + 1); break
        case Qt.Key_Up:    destino = disposicion.vecinoVertical(destino, -1); break
        case Qt.Key_Down:  destino = disposicion.vecinoVertical(destino, 1); break
        case Qt.Key_Home:  destino = 0; break
        case Qt.Key_End:   destino = modelo.total - 1; break
        case Qt.Key_PageDown: malla.contentY = Math.min(malla.contentHeight - malla.height,
                                                        malla.contentY + malla.height * 0.9)
            evento.accepted = true; return
        case Qt.Key_PageUp: malla.contentY = Math.max(0, malla.contentY - malla.height * 0.9)
            evento.accepted = true; return
        default: return
        }
        if (evento.modifiers & Qt.ShiftModifier) modelo.elegirHasta(destino)
        else modelo.elegir(destino)
        malla.llevarHasta(destino)
        evento.accepted = true
    }
}
