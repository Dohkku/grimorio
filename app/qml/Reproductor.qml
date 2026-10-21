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
    /// Esconder los mandos y el cursor: lo pide el visor cuando el vídeo
    /// corre y el ratón lleva un rato quieto.
    property bool mandosOcultos: false
    /// Si la ventana está a pantalla completa, para el botón que la alterna.
    property bool enPantallaCompleta: false
    signal pedirPantallaCompleta()
    /// El ratón se ha movido encima del vídeo: quien esconde los mandos
    /// tiene que volver a enseñarlos.
    signal actividad(real x, real y)
    /// Si el ratón está sobre los mandos: ahí no se esconden.
    readonly property bool ratonEnMandos: transporte.encima
    /// Doble clic donde no hay pantalla completa que pedir (el panel de
    /// detalle): quien lo enseña decide, y el panel abre el visor.
    signal dobleClic()

    // ---------------------------------------------------- zoom del vídeo
    /// En el visor, la rueda acerca el vídeo sin pararlo, alrededor del
    /// ratón; acercado, arrastrar mueve el encuadre. Como en las fotos, hasta
    /// ver los píxeles: el tope es el mismo, 64 de pantalla por cada uno del
    /// vídeo, y de 2× en adelante se pinta sin suavizar.
    readonly property bool conZoom: conMandos && !compacto && !soloSonido
    property real zoom: 1
    property real despX: 0
    property real despY: 0
    /// Lo que ocupa un píxel del vídeo en pantalla, ajustado, sin zoom.
    readonly property real escalaAjustada: salidaVideo.sourceRect.width > 0
        ? salidaVideo.contentRect.width / salidaVideo.sourceRect.width : 1
    /// La escala sobre el vídeo, como el número de las fotos: 100 % es un
    /// píxel del vídeo por píxel de pantalla.
    readonly property real escalaReal: zoom * escalaAjustada
    readonly property real zoomMax: Math.max(8, 64 / Math.max(0.01, escalaAjustada))

    /// Cuánto está girado el vídeo, en grados: 0, 90, 180 o 270. Lo pone
    /// quien lo enseña, con el giro guardado del elemento: es solo la vista,
    /// el archivo no se toca.
    property int giro: 0
    readonly property bool girado: giro % 180 !== 0
    onGiroChanged: quitarZoom()
    /// Girar es pedirlo: quien enseña el reproductor lo guarda en el
    /// elemento, y el giro nuevo vuelve por `giro`.
    signal pedirGiro(int grados)

    /// Un cuarto de vuelta: 1 a la derecha, -1 a la izquierda.
    function girar(sentido) {
        pedirGiro((giro + 90 * sentido + 360) % 360)
    }

    // Las cuentas del zoom van en las medidas del marco que gira (`marcoVideo`):
    // girado un cuarto, su ancho es el alto del reproductor.
    function limitarZoom() {
        const mx = (zoom - 1) * marcoVideo.width / 2
        const my = (zoom - 1) * marcoVideo.height / 2
        despX = Math.max(-mx, Math.min(mx, despX))
        despY = Math.max(-my, Math.min(my, despY))
    }

    /// Lleva el zoom a `nuevo` dejando quieto lo que hay bajo (px, py).
    function zoomA(nuevo, px, py) {
        nuevo = Math.max(1, Math.min(zoomMax, nuevo))
        const cx = marcoVideo.width / 2, cy = marcoVideo.height / 2
        const k = nuevo / zoom
        despX = (px - cx) - (px - cx - despX) * k
        despY = (py - cy) - (py - cy - despY) * k
        zoom = nuevo
        if (zoom <= 1.0001) quitarZoom()
        else limitarZoom()
    }

    function quitarZoom() { zoom = 1; despX = 0; despY = 0 }

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

    // Saltar, con freno: uno cada 150 ms como mucho, y gana el último.
    //
    // Cada salto obliga al decodificador a volver al fotograma clave de antes
    // y decodificar desde ahí, y en un vídeo con claves cada diez segundos eso
    // son cientos de fotogramas. Pinchando la barra deprisa o arrastrando,
    // los saltos se amontonaban: medido, el doble de procesador que solo
    // reproduciendo, y la ventana a tirones. El primero sale en el acto, así
    // que un clic suelto no espera nada.
    property real saltoPendiente: -1
    /// A dónde se va o se acaba de ir: mientras el salto no ha llegado,
    /// `position` aún dice dónde se estaba, y contar desde ahí (un fotograma
    /// más, diez segundos más) repetiría el mismo paso.
    property real ultimoSalto: -1
    function irA(ms) {
        if (!reproduccion.seekable) return
        const v = Math.max(0, Math.min(reproduccion.duration, ms))
        ultimoSalto = v
        if (frenoSalto.running) {
            saltoPendiente = v
            return
        }
        reproduccion.setPosition(v)
        frenoSalto.start()
    }
    function posicionPedida() {
        return frenoSalto.running && ultimoSalto >= 0 ? ultimoSalto : reproduccion.position
    }
    Timer {
        id: frenoSalto
        interval: 150
        onTriggered: {
            if (reproductor.saltoPendiente < 0) return
            reproduccion.setPosition(reproductor.saltoPendiente)
            reproductor.saltoPendiente = -1
            restart()
        }
    }

    /// Un fotograma adelante o atrás. Se para primero: avanzar de uno en uno
    /// con la película corriendo no deja ver nada.
    function paso(n) {
        if (enMarcha) reproduccion.pause()
        pendiente = false
        irA(posicionPedida() + n * 1000 / fps)
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
    /// mira: «un poco atrás» son diez segundos, no cinco mil milisegundos.
    function saltar(segundos) {
        // Por `irA`, con su freno: pulsar diez veces «+10 s» seguidas son cien
        // segundos, contados desde el último salto pedido.
        irA(posicionPedida() + segundos * 1000)
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
            // Y se olvida el arranque pendiente: si no, el medio que acaba de
            // cargar lo cumplía detrás del visor. Era el panel de detalle
            // sonando debajo del visor abierto con doble clic: el primer clic
            // le daba el vídeo, el segundo abría el visor y lo pausaba, y a
            // los cien milisegundos el vídeo terminaba de cargar y arrancaba.
            pendiente = false
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
        quitarZoom()
        hayFotograma = false
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
        pendiente = String(fuente).length > 0 && enPantalla
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
        hayFotograma = false
        fotogramasAntes = fotogramas.cuenta(salidaVideo.videoSink)
        idCargado = idElemento
        fuenteCargada = fuente
        textoError = ""
        // Si ya se sabe que este vídeo necesita copia —se convirtió antes en
        // esta sesión—, se va directo a ella y no se pasa otra vez por el error.
        const hecha = idElemento.length > 0 ? derivados.hecho("proxy:" + idElemento) : ({})
        usandoCopia = hecha.ok === true
        efectiva = usandoCopia ? derivados.url(hecha.ruta) : fuente
        // Fuera de pantalla se carga pero no se arranca. El visor cerrado
        // tiene su reproductor vivo —ver `hizoFalta` en Visor.qml— y le llega
        // cada vídeo que se elige en la malla: arrancando aquí sonaba por
        // detrás a la vez que el del panel de detalle, el mismo vídeo dos
        // veces, y solo se callaba al elegir algo que no fuera un vídeo.
        // Ya arrancará `onEnPantallaChanged` cuando se vea.
        pendiente = String(fuente).length > 0 && enPantalla
        seguirSalida()
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
                    reproductor.pendiente = reproductor.enPantalla
                    if (reproductor.enPantalla) reproduccion.play()
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
        // Un sonido no da fotogramas nunca: «no llega ninguno» no es fallo, y
        // reiniciarlo cada tres segundos lo devolvía al principio sin parar.
        running: !reproductor.conMandos && !reproductor.soloSonido && reproductor.enPantalla
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

    // Sonar por la salida que el sistema tenga ahora, no por la que tenía al
    // abrir el programa.
    //
    // `AudioOutput` se queda con el dispositivo por defecto de cuando se crea,
    // y en Qt 6.4 `MediaDevices` no avisa cuando el sistema cambia de salida:
    // el valor sí está al día si se pregunta, pero sin señal no hay atadura
    // que se entere. Y cada tubería nueva abría el sonido pidiendo ese
    // dispositivo viejo por su nombre, así que PipeWire lo dejaba allí aunque
    // el resto del escritorio ya sonara por otro lado. Se pregunta al
    // arrancar y, mientras suena, cada dos segundos.
    MediaDevices { id: dispositivos }

    function seguirSalida() {
        if (silencio) return
        const d = dispositivos.defaultAudioOutput
        if (d.isNull || String(d.id) === String(salidaAudio.device.id)) return
        salidaAudio.device = d
    }

    Timer {
        interval: 2000
        repeat: true
        running: reproductor.enMarcha && !reproductor.silencio
        onTriggered: reproductor.seguirSalida()
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
            const debe = reproductor.enPantalla && (reproductor.conMandos
                         ? reproductor.pendiente
                         : String(reproductor.fuente).length > 0)
            reproductor.pendiente = false
            if (debe && playbackState !== MediaPlayer.PlayingState) play()
        }
        onPlaybackStateChanged: if (playbackState === MediaPlayer.PlayingState) reproductor.seguirSalida()
        loops: reproductor.bucle ? MediaPlayer.Infinite : 1
        audioOutput: AudioOutput {
            id: salidaAudio
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

    // Si ya se ve algún fotograma del vídeo cargado ahora. Hasta entonces,
    // debajo se ve la portada que pone quien enseña el reproductor; desde
    // entonces, un fondo liso. Sin él, las franjas que deja un vídeo que no
    // tiene la forma del hueco son transparentes, y por ellas asomaba lo de
    // debajo: en el panel, un vídeo apaisado con trozos de una portada
    // vertical arriba y abajo.
    property bool hayFotograma: false
    property int fotogramasAntes: 0
    Timer {
        interval: 40
        repeat: true
        running: reproductor.conMandos && !reproductor.soloSonido
                 && !reproductor.hayFotograma && reproductor.enPantalla
                 && String(reproductor.efectiva).length > 0
        onTriggered: if (!cambio.running
                         && fotogramas.cuenta(salidaVideo.videoSink) > reproductor.fotogramasAntes)
                         reproductor.hayFotograma = true
    }
    Rectangle {
        anchors.fill: parent
        color: tema.fondo
        visible: reproductor.conMandos && !reproductor.soloSonido
                 && reproductor.hayFotograma && !cambio.running
    }

    // Recortado solo con zoom: sin él el vídeo cabe entero y recortar es
    // pagar por nada en cada celda de la malla.
    Item {
        anchors.fill: parent
        clip: reproductor.zoom > 1

        // Lo que gira: el vídeo con su zoom. Girado un cuarto, con las
        // medidas cambiadas, para que el vídeo ajuste al hueco de pie.
        Item {
            id: marcoVideo
            anchors.centerIn: parent
            width: reproductor.girado ? parent.height : parent.width
            height: reproductor.girado ? parent.width : parent.height
            rotation: reproductor.giro

            VideoOutput {
                id: salidaVideo
                anchors.fill: parent
                // Escondido mientras llega la fuente nueva: en la malla el reproductor
                // ya está en la celda nueva, y enseñaría el vídeo viejo encima. Debajo
                // queda la portada.
                visible: !reproductor.soloSonido && !cambio.running
                fillMode: VideoOutput.PreserveAspectFit
                smooth: reproductor.escalaReal < 2
                transform: [
                    Scale {
                        origin.x: salidaVideo.width / 2
                        origin.y: salidaVideo.height / 2
                        xScale: reproductor.zoom
                        yScale: reproductor.zoom
                    },
                    Translate { x: reproductor.despX; y: reproductor.despY }
                ]
            }
        }
    }

    // Cuánto está acercado, mientras lo está.
    Rectangle {
        anchors { left: parent.left; top: parent.top; margins: tema.margen }
        visible: reproductor.zoom > 1 && !reproductor.mandosOcultos
        width: textoZoom.implicitWidth + tema.hueco * 1.6
        height: textoZoom.implicitHeight + tema.hueco * 0.8
        radius: tema.radio
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        opacity: 0.9
        z: 1
        Text {
            id: textoZoom
            anchors.centerIn: parent
            text: Math.round(reproductor.escalaReal * 100) + " %  ·  " + qsTr("Z vuelve")
            color: tema.texto
            font.pixelSize: tema.fuente * 0.9
            font.family: "monospace"
        }
    }

    MouseArea {
        id: ratonVideo
        anchors.fill: parent
        // Apagado en la malla: ahí el clic es de la celda, que elige y arrastra.
        // Un `MouseArea` encima que no hace nada tampoco deja pasar el ratón.
        // Con la onda delante, el clic es suyo: elige el momento, no pausa.
        enabled: reproductor.conMandos && !reproductor.soloSonido
        hoverEnabled: reproductor.conZoom
        cursorShape: reproductor.mandosOcultos ? Qt.BlankCursor
                     : reproductor.zoom > 1 ? (pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor)
                     : Qt.ArrowCursor
        property real x0: 0
        property real y0: 0
        property real dx0: 0
        property real dy0: 0
        property bool movido: false
        onPressed: function (e) {
            x0 = e.x; y0 = e.y
            dx0 = reproductor.despX; dy0 = reproductor.despY
            movido = false
        }
        onPositionChanged: function (e) {
            const g = mapToItem(null, e.x, e.y)
            reproductor.actividad(g.x, g.y)
            if (!pressed || reproductor.zoom <= 1) return
            if (Math.abs(e.x - x0) + Math.abs(e.y - y0) > 4) movido = true
            // El arrastre, en el sentido del vídeo girado.
            const a = mapToItem(marcoVideo, x0, y0)
            const b = mapToItem(marcoVideo, e.x, e.y)
            reproductor.despX = dx0 + b.x - a.x
            reproductor.despY = dy0 + b.y - a.y
            reproductor.limitarZoom()
        }
        onClicked: if (!movido) reproductor.alternar()
        // Doble clic, pantalla completa, como en cualquier reproductor. El
        // primer clic del doble ya pausó: se deshace.
        onDoubleClicked: {
            reproductor.alternar()
            if (reproductor.conZoom) reproductor.pedirPantallaCompleta()
            else reproductor.dobleClic()
        }
        // En el panel la rueda es del panel, que se desplaza.
        onWheel: function (r) {
            if (!reproductor.conZoom) { r.accepted = false; return }
            const d = r.angleDelta.y !== 0 ? r.angleDelta.y : r.angleDelta.x
            const p = mapToItem(marcoVideo, r.x, r.y)
            reproductor.zoomA(reproductor.zoom * Math.pow(1.0015, d), p.x, p.y)
        }
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
        escondido: reproductor.mandosOcultos
        visible: reproductor.conMandos && !reproductor.roto
    }
}
