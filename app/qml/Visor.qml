// El visor a pantalla completa.
//
// Presupuesto: menos de 120 ms desde la pulsación hasta ver la imagen. Por eso
// pinta primero el color dominante, luego la miniatura de 320 px que ya está
// decodificada en el caché, y encima la previsualización de 1024 px cuando
// llega. Nunca hay un rectángulo vacío esperando al disco.
import QtQuick
import "textos.js" as Textos

Item {
    id: visor
    signal cerrar()

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
    readonly property url urlFoco:
        sePuedeReproducir && foco >= 0 ? modelo.urlDe(foco) : ""
    readonly property bool hayQueReproducir: String(urlFoco).length > 0
    /// Los PDF se leen página a página y los modelos 3D se giran: los dos
    /// tienen visor propio encima de la previsualización.
    /// Las fotos se acercan hasta el píxel (`VisorImagen.qml`). El RAW
    /// también, sobre la vista previa que lleva dentro.
    readonly property bool esImagen: familiaFoco === Textos.IMAGEN || familiaFoco === Textos.RAW
    readonly property bool esDocumento: familiaFoco === Textos.DOCUMENTO
    readonly property bool esModelo: familiaFoco === Textos.MODELO
    readonly property bool conVisorPropio: esDocumento || esModelo

    /// Si ya se ha llegado a hacer falta un reproductor.
    ///
    /// Una vez creado no se destruye hasta cerrar el programa, y eso es a
    /// propósito: el motor de multimedia de Qt 6.4 se cae al montar y desmontar
    /// la tubería muchas veces seguidas —«double free or corruption» dentro de
    /// GStreamer, reproducible con `--visor 8`—. Un solo reproductor al que se
    /// le cambia la fuente no pasa por ahí nunca.
    property bool hizoFalta: false

    focus: visible
    // Cerrado, el original decodificado para el cuentagotas sobra: puede ser
    // mucha memoria.
    onVisibleChanged: {
        if (visible) forceActiveFocus()
        else pixeles.soltar()
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
        opacity: 0.97
    }

    Rectangle {
        id: lienzo
        anchors.centerIn: parent
        width: parent.width - tema.margen * 4
        height: parent.height - tema.margen * 6
        color: "transparent"

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

    VisorImagen {
        id: imagen
        anchors.fill: parent
        visible: visor.visible && visor.esImagen
        idElemento: visible ? visor.idFoco : ""
        original: visible && visor.foco >= 0 ? modelo.urlDe(visor.foco) : ""
        anchoPista: nucleo.ficha.id === visor.idFoco ? (nucleo.ficha.ancho || 0) : 0
        altoPista: nucleo.ficha.id === visor.idFoco ? (nucleo.ficha.alto || 0) : 0
        onSiguiente: visor.mover(1)
        onCerrar: visor.cerrar()
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
        value: visor.idFoco
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
    onIdFocoChanged: darVisorPropio()

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

    Text {
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
    }

    Estrellas {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: tema.margen
        valor: modelo.estrellasFoco
        onElegido: function (n) { nucleo.estrellas(modelo.seleccion, n) }
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
        const conMayus = (evento.modifiers & Qt.ShiftModifier) !== 0
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
            case Qt.Key_Z: imagen.alternar(imagen.width / 2, imagen.height / 2); break
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
        // Mayúsculas y flecha rebobina cinco segundos. Sin mayúsculas sigue
        // siendo pasar de elemento: dentro del visor eso es lo que más se hace,
        // también cuando lo que se está viendo es un vídeo.
        case Qt.Key_Left:
            if (conMayus && visor.saltar(-5)) break
            visor.mover(-1); break
        case Qt.Key_Up:
        case Qt.Key_Backspace: visor.mover(-1); break
        case Qt.Key_Right:
            if (conMayus && visor.saltar(5)) break
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
            // Clic derecho o doble clic para salir; clic izquierdo avanza,
            // como en cualquier visor de fotos.
            if (evento.button === Qt.RightButton) visor.cerrar()
            else visor.mover(1)
        }
        onDoubleClicked: visor.cerrar()
        // La rueda pasa de elemento, salvo donde el visor propio la usa: en un
        // PDF baja páginas y en un modelo acerca. Aquí llega solo lo que cae
        // fuera de ellos.
        onWheel: function (rueda) { visor.mover(rueda.angleDelta.y > 0 ? -1 : 1) }
    }
}
