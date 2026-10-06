// Vídeo y sonido dentro del visor.
//
// Vive en un archivo aparte a propósito. `import QtMultimedia` falla entero si
// el módulo no está instalado, y un import que falla se lleva por delante el
// archivo que lo escribe: puesto en `Visor.qml` no habría visor, ni para las
// fotos. Aquí lo peor que puede pasar es que un `Loader` se quede en `Error` y
// el visor siga enseñando el fotograma de portada, que es lo que hacía antes.
import QtQuick
import QtMultimedia
import "textos.js" as Textos

Item {
    id: reproductor

    /// La URL del archivo original, hecha por el núcleo. Construirla a mano
    /// con "file://" + ruta se rompe con los espacios y las almohadillas.
    property url fuente
    /// El id del elemento, para pedir sus derivados: la copia reproducible de
    /// un vídeo que no se abre, la onda de un sonido, la tira de fotogramas.
    property string idElemento: ""
    /// El sonido no tiene imagen: detrás de los mandos va su carátula.
    property bool soloSonido: false
    property url caratula
    /// Si lo que reproduce está a la vista. Dejar de estarlo lo calla: el
    /// reproductor sobrevive al visor que lo enseña.
    property bool enPantalla: true
    /// Volver a empezar al terminar. En la malla, siempre: un vídeo que se
    /// queda congelado en su último fotograma parece roto.
    property bool bucle: false
    property bool silencio: false
    /// Los mandos de abajo y el triángulo de en medio. En una celda de la malla
    /// no caben ni hacen falta: ahí el vídeo es una miniatura que se mueve.
    property bool conMandos: true
    /// Mandos de lo justo —parar, tiempo y barra— para el panel de la
    /// derecha, donde la fila entera no cabe. El sonido se queda en carátula.
    property bool compacto: false

    readonly property bool enMarcha: reproduccion.playbackState === MediaPlayer.PlayingState
    /// Roto de verdad: con error y sin una copia reproducible en camino. Mientras
    /// se prepara la copia no se dice nada alarmante —se dice que se prepara—.
    ///
    /// Con un error propio y no con `reproduccion.error`: en Qt 6.4 `play()`
    /// borra el error por dentro sin avisar, así que la propiedad que ve QML
    /// se quedaba con el último para siempre. Un fallo pasajero —«Internal data
    /// stream error» al soltar una tubería a medio montar— dejaba el rótulo
    /// encima de un vídeo que se estaba viendo perfectamente.
    readonly property bool roto: textoError.length > 0 && !preparando
    /// Lo que dijo el motor al fallar el medio cargado; vacío si va bien.
    property string textoError: ""
    readonly property real posicion: reproduccion.position
    readonly property real duracion: reproduccion.duration
    readonly property bool sePuedeSaltar: reproduccion.seekable

    // ---------------------------------------------- lo que se maneja a mano
    /// De 0,1× a 2×. A cámara lenta el sonido baja de tono: el motor de
    /// multimedia de Qt 6.4 no corrige el tono al cambiar de ritmo.
    property real velocidad: 1
    readonly property var velocidades: [0.1, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2]
    property real volumen: 1
    /// Bucle entre dos puntos, en milisegundos. -1 es «sin poner».
    property real puntoA: -1
    property real puntoB: -1
    readonly property bool hayTramo: puntoA >= 0 && puntoB > puntoA
    /// Fotogramas por segundo, para ir de uno en uno. Lo dice el archivo; si
    /// no, 30, que es lo más común y con lo que un paso se nota igual.
    readonly property real fps: {
        const f = Number(reproduccion.metaData.value(MediaMetaData.VideoFrameRate))
        return f > 1 && f < 1000 ? f : 30
    }

    // ------------------------------------------------- copia reproducible
    /// Lo que de verdad se reproduce: el original o su copia en H.264.
    property url efectiva
    property bool usandoCopia: false
    property bool preparando: false
    property string errorCopia: ""
    property string claveCopia: ""

    /// La tira de fotogramas para la vista previa de la barra, cuando llega.
    property var tira: ({})
    property string claveTira: ""
    /// El `.onda` del sonido, cuando llega.
    property string onda: ""
    property string claveOnda: ""
    property string errorOnda: ""

    /// Se ha pedido que corra, pero el medio todavía no estaba cargado.
    ///
    /// `play()` sobre un medio que aún no ha cargado no hace nada y no deja
    /// aviso, y nadie lo vuelve a intentar: era lo que dejaba el vídeo del
    /// panel de detalle parado en su portada, con el triángulo puesto,
    /// esperando un clic que no hacía falta.
    property bool pendiente: false

    function alternar() {
        pendiente = false
        if (enMarcha) reproduccion.pause()
        else reproduccion.play()
    }

    function irA(ms) {
        if (reproduccion.seekable)
            reproduccion.setPosition(Math.max(0, Math.min(reproduccion.duration, ms)))
    }

    /// Un fotograma adelante o atrás. Se para primero: avanzar de uno en uno
    /// con la película corriendo no deja ver nada.
    function paso(n) {
        if (enMarcha) reproduccion.pause()
        pendiente = false
        irA(reproduccion.position + n * 1000 / fps)
    }

    /// Un escalón más rápido (+1) o más lento (-1) en la lista de velocidades.
    function cambiarVelocidad(d) {
        let i = velocidades.indexOf(velocidad)
        if (i < 0) i = velocidades.indexOf(1)
        velocidad = velocidades[Math.max(0, Math.min(velocidades.length - 1, i + d))]
    }

    /// Pone el principio o el final del tramo en la posición actual. Si queda
    /// al revés, se dan la vuelta: nadie quiere un bucle que no avanza.
    function marcar(cual) {
        const t = reproduccion.position
        if (cual === "a") puntoA = t
        else puntoB = t
        if (puntoA >= 0 && puntoB >= 0 && puntoB < puntoA) {
            const x = puntoA; puntoA = puntoB; puntoB = x
        }
    }

    function quitarTramo() { puntoA = -1; puntoB = -1 }


    /// Adelantar o atrasar. Se pide en segundos porque es lo que piensa quien
    /// mira: «un poco atrás» son cinco segundos, no cinco mil milisegundos.
    function saltar(segundos) {
        // `setPosition` y no `position = ...`: en Qt 6 la posición es de solo
        // lectura y asignarla no da error, simplemente no hace nada.
        if (reproduccion.seekable)
            reproduccion.setPosition(Math.max(
                0, Math.min(reproduccion.duration, reproduccion.position + segundos * 1000)))
    }

    // Salir de pantalla lo para; volver a entrar lo arranca.
    //
    // Cambiar de elemento con las flechas trae otra fuente: que siga sonando la
    // anterior mientras se ve la siguiente sería lo peor de los dos mundos. Y
    // la vuelta hace falta porque parar es fácil de pedir y arrancar no tenía
    // quién lo pidiera: en la malla, un reproductor que se queda sin sitio y
    // vuelve a tener el mismo se paraba y ya no volvía, porque la fuente no
    // había cambiado y era el cambio de fuente lo único que llamaba a `play`.
    // Se veía al entrar en una carpeta cuyos vídeos ya estaban en marcha: la
    // celda se quedaba en su portada para siempre.
    onEnPantallaChanged: {
        // Pausa y no `stop`: parar suelta la tubería de multimedia entera y
        // volver a arrancarla cuesta cientos de milisegundos en el hilo de
        // interfaz. Pausado no decodifica nada y vuelve al instante. Lo que sí
        // hace `stop` es soltar la memoria, y de eso se encarga `soltar`.
        if (!enPantalla) {
            reproduccion.pause()
        } else if (cambio.running) {
            // La fuente nueva está en camino: arrancar la vieja sería oírla un
            // momento. Ya arrancará la nueva al llegar.
            pendiente = true
        } else if (String(fuente).length > 0) {
            pendiente = true
            reproduccion.play()
        }
    }

    // Pausado se vuelve al instante; pausado para siempre se paga.
    //
    // Una tubería en pausa no decodifica, pero se queda con todo lo suyo: en
    // una máquina con decodificador de vídeo por GPU, cada una arrastra su
    // contexto y su memoria. Medido rodando la malla arriba y abajo cinco
    // veces: pausando y ya está, de 0,93 a 1,33 GB; soltándolas, de 0,73 a
    // 0,81. Así que se pausa primero —volver a la carpeta de la que se acaba de
    // salir no cuesta nada— y se suelta si el hueco no vuelve.
    Timer {
        interval: 3000
        running: !reproductor.conMandos && !reproductor.enPantalla
                 && String(reproductor.fuente).length > 0
        onTriggered: reproduccion.stop()
    }

    // Cambiar de fuente: se pausa ya y se cambia un poco después.
    //
    // Lo caro de cambiar no es abrir el vídeo nuevo, que va en segundo plano y
    // vuelve en un milisegundo: es soltar el viejo. Qt 6.4 lo baja a NULL y
    // espera a que acabe, en el hilo de interfaz. Medido con el banco de
    // cambiar entre dos vídeos cada segundo y medio: con `stop` y fuente
    // seguidos, 733 ms de interfaz congelada de mediana y 1,4 s de máximo;
    // sin el `stop`, 455; soltando una tubería que ya estaba en pausa, 116.
    // La pausa en sí no espera a nada (0 ms) y con 100 ms ya ha asentado: con
    // 50 todavía se notaba (170 ms), con 200 no se gana más.
    //
    // Y de paso, una ráfaga de cambios —pasar el ratón por varias celdas
    // seguidas— solo paga el último.
    onFuenteChanged: {
        rescate.intentos = 0
        quitarTramo()
        preparando = false
        errorCopia = ""
        tira = ({})
        onda = ""
        errorOnda = ""
        // Y se olvida qué onda se pidió: si no, volver a un sonido ya visto
        // —sonido, vídeo, el mismo sonido— daba la petición por hecha con la
        // onda ya borrada, y el panel se quedaba en «analizando» para siempre.
        claveOnda = ""
        pendiente = String(fuente).length > 0
        if (String(efectiva).length === 0) {
            aplicarFuente()
        } else {
            reproduccion.pause()
            cambio.restart()
        }
        Qt.callLater(pedirOnda)
    }

    Timer {
        id: cambio
        interval: 100
        onTriggered: reproductor.aplicarFuente()
    }

    /// El elemento y el archivo que de verdad están cargados. Los derivados se
    /// piden con estos y no con `idElemento` y `fuente`, que pueden ir ya por
    /// el siguiente: un error o una duración que llega tarde es del que está
    /// cargado, y pedir su copia con el id del otro la guarda donde no es.
    property string idCargado: ""
    property url fuenteCargada

    function aplicarFuente() {
        idCargado = idElemento
        fuenteCargada = fuente
        textoError = ""
        // Si ya se sabe que este vídeo necesita copia —se convirtió antes en
        // esta sesión—, se va directo a ella y no se pasa otra vez por el error.
        const hecha = idElemento.length > 0 ? derivados.hecho("proxy:" + idElemento) : ({})
        usandoCopia = hecha.ok === true
        efectiva = usandoCopia ? derivados.url(hecha.ruta) : fuente
        pendiente = String(fuente).length > 0
        if (pendiente) reproduccion.play()
    }

    // La onda se pide cuando están las tres cosas: la fuente, el id y saber
    // que es sonido. Llegan por separado —el visor las da con un `Binding`
    // cada una y el orden entre ellos no está dicho—, así que se pregunta
    // cada vez que cambia cualquiera, y una vez por vuelta.
    onSoloSonidoChanged: Qt.callLater(pedirOnda)
    onIdElementoChanged: Qt.callLater(pedirOnda)
    function pedirOnda() {
        if (!conMandos || !soloSonido || String(fuente).length === 0
                || idElemento.length === 0) return
        const clave = "onda:" + idElemento
        if (clave === claveOnda && (onda.length > 0 || errorOnda.length === 0)) return
        claveOnda = derivados.pedir("onda", idElemento, fuente)
    }

    // Cuando el reproductor no sabe abrir el vídeo —ProRes, DNxHD sin
    // `gstreamer1.0-libav`— se pide al núcleo una copia en H.264 y se cambia a
    // ella. La primera vez tarda lo que tarde ffmpeg; luego está en la caché.
    Connections {
        target: reproduccion
        function onErrorOccurred(error, texto) {
            // Con un cambio en camino, el que falla es el que se va.
            if (cambio.running || error === MediaPlayer.NoError) return
            reproductor.textoError = texto.length > 0 ? texto : qsTr("error desconocido")
            if (reproductor.usandoCopia || reproductor.soloSonido
                    || reproductor.idCargado.length === 0
                    || String(reproductor.fuenteCargada).length === 0) return
            reproductor.preparando = true
            reproductor.claveCopia = derivados.pedir("proxy", reproductor.idCargado,
                                                     reproductor.fuenteCargada)
        }
        function onDurationChanged() {
            // La tira se pide con la duración, que no se sabe hasta que carga.
            if (cambio.running || !reproductor.conMandos || reproductor.compacto
                    || reproductor.soloSonido || reproduccion.duration <= 0
                    || reproductor.idCargado.length === 0) return
            reproductor.claveTira = derivados.pedir(
                "tira", reproductor.idCargado, reproductor.fuenteCargada,
                { duracion: reproduccion.duration / 1000 })
        }
    }

    Connections {
        target: derivados
        function onListo(clave, r) {
            if (clave === reproductor.claveCopia && reproductor.preparando) {
                if (r.ok) {
                    reproductor.usandoCopia = true
                    reproductor.textoError = ""
                    reproductor.efectiva = derivados.url(r.ruta)
                    reproductor.pendiente = true
                    reproduccion.play()
                } else {
                    reproductor.errorCopia = r.error
                }
                reproductor.preparando = false
            } else if (clave === reproductor.claveTira && r.ok) {
                reproductor.tira = r
            } else if (clave === reproductor.claveOnda) {
                if (r.ok) reproductor.onda = r.ruta
                else reproductor.errorOnda = r.error
            }
        }
    }

    // El bucle entre A y B. Un temporizador corto y no el aviso de posición:
    // la posición avisa cuando le parece, y un bucle de medio segundo que se
    // pasa de largo cien milisegundos no es un bucle.
    Timer {
        interval: 15
        repeat: true
        running: reproductor.hayTramo && reproductor.enMarcha
        onTriggered: {
            const p = reproduccion.position
            if (p >= reproductor.puntoB || p < reproductor.puntoA - 100)
                reproduccion.setPosition(reproductor.puntoA)
        }
    }

    // Vuelve a intentarlo cuando la tubería anda y no llega ni un fotograma.
    //
    // Qt lo avisa por la salida de errores —«Failed to start video surface due
    // to main thread blocked»— y ahí se queda: el motor de multimedia le pide
    // el relevo al hilo de interfaz para montar la salida, se cansa de esperar
    // y no vuelve a pedirlo. El reproductor sigue contando posición, pero la
    // celda es una foto quieta para siempre. Pasa al arrancar, que es cuando la
    // malla está decodificando la primera pantalla.
    //
    // Se cuentan los fotogramas que llegan a la salida. Ni la posición ni el
    // tamaño valen para esto: la posición avanza igual —lo que falla es
    // enseñar, no decodificar— y el tamaño se rellena en cuanto se conoce el
    // formato, se llegue a ver algo o no. Medido: con ocho avisos de esos, los
    // cinco reproductores decían tener tamaño y posición.
    //
    // Solo en la malla. En el visor hay un botón de play a mano y el vídeo
    // ocupa la pantalla: ahí un reintento a ciegas se ve más que el fallo.
    //
    // Se cuentan en C++ y no con un `Connections` a `videoFrameChanged`: aquí
    // cada aviso dejaba el fotograma retenido en JavaScript hasta el siguiente
    // recolector, una textura por fotograma, y en cuatro minutos de vídeo la
    // tarjeta estaba llena y el programa moría. El porqué, en `fotogramas.h`.
    Component.onCompleted: fotogramas.vigilar(salidaVideo.videoSink)

    Timer {
        id: rescate
        interval: 3000
        repeat: true
        running: !reproductor.conMandos && reproductor.enPantalla
                 && String(reproductor.fuente).length > 0 && intentos < 3
        /// Tres y para. Si a la tercera no llega nada, no es la carrera del
        /// arranque, y reintentar cada tres segundos sería un tic para siempre.
        property int intentos: 0
        property int vistos: 0
        onTriggered: {
            const total = fotogramas.cuenta(salidaVideo.videoSink)
            const llegados = total - vistos
            vistos = total
            // Cualquier fotograma vale. Con la pantalla apagada el compositor
            // deja de pedir dibujado y bajan a un puñado por vuelta: eso no es
            // estar roto, y reiniciar ahí sería un tic contra nadie.
            // Con error no se insiste: si la tarjeta se ha quedado sin
            // decodificadores, reintentar es pedirle otro al que no tiene.
            if (cambio.running) return
            if (reproductor.roto) {
                intentos = 3
                return
            }
            if (llegados > 0 || reproduccion.playbackState !== MediaPlayer.PlayingState) {
                intentos = 0
                return
            }
            intentos += 1
            reproduccion.stop()
            reproduccion.play()
        }
    }

    MediaPlayer {
        id: reproduccion
        source: reproductor.efectiva
        playbackRate: reproductor.velocidad
        // El segundo intento, cuando el medio por fin está.
        //
        // Y no basta con recordar que había uno pendiente: cambiar de fuente
        // dispara dos cosas —el manejador del que la pone y la recarga del
        // medio— y el orden entre ellas no está dicho. Con el apunte de «tenía
        // uno pendiente» se lo gastaba el medio viejo y el nuevo se quedaba
        // parado para siempre: era lo que dejaba la malla sin mover un vídeo al
        // pasar el ratón, con la tubería montada y todo.
        //
        // En la malla no hay botón de pausa: si tiene sitio y tiene fuente,
        // tiene que correr, y eso se puede preguntar en cualquier momento. En
        // el visor y en el panel sí lo hay, así que ahí se respeta lo que se
        // haya pedido a mano y solo se arranca el que estaba esperando.
        onMediaStatusChanged: {
            // Con un cambio en camino, el medio que avisa es el viejo.
            if (cambio.running) return
            if (mediaStatus !== MediaPlayer.LoadedMedia
                    && mediaStatus !== MediaPlayer.BufferedMedia) return
            // Si ha cargado, un error de antes ya no es de este medio.
            reproductor.textoError = ""
            const debe = reproductor.conMandos
                         ? reproductor.pendiente
                         : (reproductor.enPantalla && String(reproductor.fuente).length > 0)
            reproductor.pendiente = false
            if (debe && playbackState !== MediaPlayer.PlayingState) play()
        }
        loops: reproductor.bucle ? MediaPlayer.Infinite : 1
        audioOutput: AudioOutput {
            muted: reproductor.silencio
            volume: reproductor.volumen
        }
        videoOutput: salidaVideo
    }

    // La carátula del sonido va debajo, y el vídeo encima. Mientras el primer
    // fotograma no llega, `VideoOutput` no pinta nada y se sigue viendo la
    // previsualización que el visor ya tenía puesta detrás.
    Image {
        anchors.fill: parent
        readonly property bool toca: reproductor.soloSonido && !reproductor.conMandos
        visible: toca
        source: toca ? reproductor.caratula : ""
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        cache: true
    }

    // Con mandos, el sonido se enseña como se trabaja con él: la onda en el
    // tiempo y el espectro en frecuencia, y se elige el momento con el ratón.
    Loader {
        id: sonidoVisto
        anchors.fill: parent
        anchors.bottomMargin: transporte.height
        // También en el panel, en pequeño: sin cabecera ni zoom, pero con la
        // onda, el espectro y el cabezal, que es lo que dice qué suena.
        active: reproductor.soloSonido && reproductor.conMandos
        source: "qrc:/qml/VisorSonido.qml"
        onLoaded: item.rep = reproductor
    }
    Binding {
        target: sonidoVisto.item
        property: "compacto"
        value: reproductor.compacto
        when: sonidoVisto.status === Loader.Ready
    }

    VideoOutput {
        id: salidaVideo
        anchors.fill: parent
        // Escondido mientras llega la fuente nueva: en la malla el reproductor
        // ya está en la celda nueva, y enseñaría el vídeo viejo encima. Debajo
        // queda la portada.
        visible: !reproductor.soloSonido && !cambio.running
        fillMode: VideoOutput.PreserveAspectFit
    }

    MouseArea {
        anchors.fill: parent
        // Apagado en la malla: ahí el clic es de la celda, que elige y arrastra.
        // Un `MouseArea` encima que no hace nada tampoco deja pasar el ratón.
        // Con la onda delante, el clic es suyo: elige el momento, no pausa.
        enabled: reproductor.conMandos && !reproductor.soloSonido
        onClicked: reproductor.alternar()
    }

    // Un triángulo en medio cuando está parado. Es la única señal de que eso
    // que se ve quieto se puede poner en marcha.
    Text {
        anchors.centerIn: parent
        visible: reproductor.conMandos && !reproductor.enMarcha && !reproductor.roto
                 && !reproductor.soloSonido && !reproductor.preparando
        text: "▶"
        color: tema.texto
        opacity: 0.75
        font.pixelSize: tema.fuente * 4
        style: Text.Outline
        styleColor: tema.fondo
    }

    Text {
        anchors.centerIn: parent
        width: parent.width * 0.7
        // Solo en el visor. En una celda de la malla, un vídeo que no se puede
        // reproducir no tiene que taparla con un rótulo: se calla y deja ver la
        // portada, que es lo que había antes de que se moviera nada.
        visible: reproductor.conMandos && reproductor.roto
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        text: reproductor.errorCopia.length > 0
              ? qsTr("no se pudo reproducir (%1), y tampoco convertirlo: %2")
                    .arg(reproductor.textoError).arg(reproductor.errorCopia)
              : qsTr("no se pudo reproducir: %1").arg(reproductor.textoError)
        color: tema.textoTenue
        font.pixelSize: tema.fuente
    }

    // Mientras se prepara la copia reproducible se dice qué pasa y por qué:
    // si no, un vídeo que no arranca durante un minuto parece colgado.
    Rectangle {
        anchors.centerIn: parent
        visible: reproductor.conMandos && reproductor.preparando
        width: Math.min(parent.width * 0.8, aviso.implicitWidth + tema.margen * 2)
        height: aviso.implicitHeight + tema.margen
        radius: tema.radio
        color: tema.panel
        opacity: 0.95

        Text {
            id: aviso
            anchors.centerIn: parent
            width: Math.min(implicitWidth, reproductor.width * 0.8 - tema.margen * 2)
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: qsTr("este vídeo usa un formato que el reproductor no abre:\n"
                       + "preparando una copia para verlo (solo la primera vez)…")
            color: tema.texto
            font.pixelSize: tema.fuente
        }
    }

    Transporte {
        id: transporte
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        rep: reproductor
        compacto: reproductor.compacto
        visible: reproductor.conMandos && !reproductor.roto
    }
}
