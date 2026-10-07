// Una celda de la malla. Su posición y su contenido vienen del mismo modelo,
// así que nunca se pinta en un sitio con los datos de otro.
import QtQuick
import "textos.js" as Textos

Item {
    id: celda

    required property int celdaIndice
    required property real celdaX
    required property real celdaY
    required property real celdaAncho
    required property real celdaAlto
    required property bool celdaActiva
    required property color dominante
    required property bool tieneMiniatura
    required property bool elegido
    required property int estrellas
    required property string idItem
    required property int familia
    required property int duracion
    required property bool adulto
    required property string nombre
    /// En lista la celda es una fila: miniatura pequeña a la izquierda y sus
    /// datos en columnas. La misma celda y no otra, para que elegir, arrastrar
    /// a una carpeta, reordenar y moverse con el teclado sean exactamente lo
    /// mismo en las tres vistas.
    readonly property bool enLista: disposicion.modo === 2
    /// Marcado como +18 **y** el modo seguro puesto. La marca sola no esconde
    /// nada: es el modo el que decide, y se apaga desde la barra de arriba.
    readonly property bool censurado: adulto && ajustes.modoSeguro
    /// «Está puesta y no tiene miniatura», en una sola propiedad. Ver el
    /// comentario del papel `SinDibujo` en disposicion.cpp: preguntarlo con dos
    /// propiedades costaba un tercio de los fotogramas.
    required property bool sinDibujo

    /// El ratón entra o sale. Lleva la geometría puesta porque quien la escucha
    /// —la malla— tendría que volver a preguntársela al modelo para saber dónde
    /// poner el vídeo encima, y aquí ya está.
    signal pasada(bool dentro, int indice, real x, real y, real ancho, real alto)

    /// Dónde estaba justo antes de elegirla.
    ///
    /// Elegir abre el panel de detalle, y eso estrecha la malla y rehace las
    /// filas: para cuando alguien se entera de cuál es la elegida, ya no está
    /// donde se pinchó. Sin este aviso, el anillo del foco no tiene de dónde
    /// salir y aparece en el sitio nuevo sin más, que es justo lo que hace que
    /// haya que buscarlo con los ojos.
    signal elegida(real x, real y, real ancho, real alto)


    x: celdaX
    y: celdaY
    width: celdaAncho
    height: celdaAlto
    // Reordenando, las celdas se apartan deslizándose en vez de saltar. Solo
    // entonces: al desplazar la malla una ranura cambia de elemento y de
    // sitio, y animado se vería a cada celda cruzar la pantalla.
    Behavior on x { enabled: disposicion.animando; NumberAnimation { duration: 190; easing.type: Easing.OutCubic } }
    Behavior on y { enabled: disposicion.animando; NumberAnimation { duration: 190; easing.type: Easing.OutCubic } }
    Behavior on width { enabled: disposicion.animando; NumberAnimation { duration: 190; easing.type: Easing.OutCubic } }
    Behavior on height { enabled: disposicion.animando; NumberAnimation { duration: 190; easing.type: Easing.OutCubic } }
    /// Lo que se está arrastrando deja su hueco a la vista, apagado: dice
    /// dónde va a caer.
    readonly property bool esElArrastrado: disposicion.reordenando && celdaIndice === disposicion.arrastrado
    opacity: esElArrastrado ? 0.3 : 1
    Behavior on opacity { NumberAnimation { duration: 120 } }
    visible: celdaActiva
    // Una ranura inactiva no debe seguir pidiendo miniaturas ni respondiendo al
    // ratón: es una celda que ahora mismo no existe.
    enabled: celdaActiva

    // El nombre debajo, en el pie que la disposición deja a cada fila. Fuera
    // del dibujo para no crecer con él al pasar el ratón.
    Loader {
        active: !celda.enLista && disposicion.pie > 0 && celda.celdaActiva
        y: celda.height + Math.round(tema.hueco * 0.35)
        width: celda.width
        sourceComponent: Text {
            elide: Text.ElideMiddle
            text: celda.nombre
            color: celda.elegido ? tema.texto : tema.textoTenue
            font.pixelSize: tema.fuente * 0.82
        }
    }

    // En lista, la fila entera se marca al pasar y al elegir: un anillo
    // alrededor de una miniatura de cuarenta píxeles no se ve.
    Loader {
        anchors.fill: parent
        active: celda.enLista && (celda.elegido || raton.containsMouse)
        sourceComponent: Rectangle {
            radius: tema.radio
            color: celda.elegido ? tema.marcador : tema.borde
        }
    }

    Item {
        anchors.fill: parent
        scale: raton.containsMouse && !celda.enLista ? 1 + tema.realcePx / Math.max(1, width) : 1
        Behavior on scale { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }

        // Lo que se dibuja: la celda entera, o en lista la miniatura de la
        // izquierda. Lo de encima —el ratón, el bulto— sigue siendo la celda
        // entera, así que la fila se pincha por cualquier sitio.
        Item {
            id: dibujo
            readonly property real lado: ventana.ladoMiniaturaLista
            x: celda.enLista ? Math.round(tema.hueco * 0.4) : 0
            y: celda.enLista ? Math.round((parent.height - lado) / 2) : 0
            width: celda.enLista ? Math.round(lado * 1.3) : parent.width
            height: celda.enLista ? lado : parent.height

            // Debajo de todo, el color dominante: la celda nunca es un agujero
            // mientras la miniatura viaja desde el disco.
            Rectangle {
                anchors.fill: parent
                radius: tema.radio
                color: celda.dominante.a > 0 ? celda.dominante : tema.marcador
            }

            Image {
                anchors.fill: parent
                // Por id y no por índice: el caché de Qt tiene por clave la URL, y
                // con la posición dentro, filtrar hacía que una celda sirviera la
                // miniatura de la que ocupaba ese sitio en la vista anterior.
                source: celda.celdaActiva && celda.tieneMiniatura && celda.idItem.length > 0
                        ? "image://grim/" + celda.idItem : ""
                asynchronous: true
                cache: true
                // Recortado en cuadrícula, entero en justificado: en justificado la
                // celda ya tiene la proporción de la imagen, así que recortar solo
                // quitaría píxeles por un error de redondeo.
                fillMode: disposicion.modo === 1 ? Image.PreserveAspectFit
                                                 : Image.PreserveAspectCrop
                horizontalAlignment: Image.AlignHCenter
                verticalAlignment: Image.AlignVCenter
                // El difuminado es la propia miniatura pedida diminuta y estirada
                // hasta la celda. Sin módulos de efectos —el Qt de Ubuntu 24.04 no
                // trae `QtQuick.Effects` ni el compilador de sombreadores— y sin
                // que los píxeles de verdad lleguen a estar en pantalla: lo que se
                // decodifica son unas decenas de píxeles, no la imagen tapada.
                //
                // Un solo tamaño de decodificación para toda la malla, atado al
                // tamaño objetivo y no al de cada celda. El caché de Qt tiene por
                // clave (imagen, tamaño pedido): con el tamaño exacto de la celda,
                // en justificado cada fila pide uno distinto y la misma miniatura
                // se decodifica una y otra vez. Y sin pedir tamaño se decodifica
                // siempre a 320 px, con lo que caben menos entradas en el caché y
                // vuelve a fallar. Medido en un recorrido completo: tamaño exacto
                // 139.768 decodificaciones, a saltos de 64 px 153.029, sin pedir
                // tamaño 94.976.
                //
                // Y no el objetivo exacto, sino uno de tres escalones. Con el
                // exacto, cada paso del zoom —van de veinte en veinte— cambiaba
                // el tamaño pedido, la clave del caché dejaba de valer y todas
                // las celdas visibles se volvían a decodificar a la vez, justo
                // mientras se está mirando cómo crecen. Con escalones, solo
                // decodifica el paso que cruza uno. Se redondea siempre hacia
                // arriba, para no estirar nunca una miniatura más pequeña que su
                // celda, y lo que sobra lo encoge `mipmap` sin que se note el
                // dentado.
                sourceSize.height: celda.censurado
                                   ? Math.max(4, Math.round(tema.fuente * 0.45))
                                   : escalonDecodificado
                readonly property int escalonDecodificado:
                    disposicion.objetivo <= 120 ? 120 : disposicion.objetivo <= 200 ? 200 : 320
                mipmap: !celda.censurado
                smooth: true
                opacity: status === Image.Ready ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: tema.aparicionS * 1000 } }
            }

            // Máscara de esquinas: una textura compartida por toda la malla.
            BorderImage {
                // La máscara pinta las esquinas del color del fondo, y en lista
                // la fila elegida no es del color del fondo: ahí se verían
                // cuatro picos. Una miniatura de cuarenta píxeles no pide
                // esquinas redondas.
                visible: !celda.enLista
                anchors.fill: parent
                // Radio y fondo en la URL: con otro tema u otro tamaño de
                // interfaz se pide otra máscara en vez de reutilizar la vieja.
                readonly property int r: Math.max(1, Math.round(tema.radio))
                source: "image://grim/esquinas/" + r + "/" + tema.fondo.toString().substring(1)
                border.left: r
                border.right: r
                border.top: r
                border.bottom: r
                smooth: true
            }

            // Realce y anillo de selección solo existen cuando hacen falta. Un
            // rectángulo invisible sigue costando un nodo del grafo de escena por
            // celda, y eso son cientos de nodos que no pintan nada.
            Loader {
                anchors.fill: parent
                active: raton.containsMouse && !celda.elegido && !celda.enLista
                sourceComponent: Rectangle {
                    radius: tema.radio
                    color: tema.seleccion
                    opacity: tema.realce
                }
            }

            Loader {
                anchors.fill: parent
                active: celda.elegido && !celda.enLista
                sourceComponent: Rectangle {
                    radius: tema.radio
                    color: "transparent"
                    border.color: tema.seleccion
                    border.width: 2
                }
            }

            // Todo lo que una foto no necesita, en un solo `Loader`.
            //
            // Van juntos y no separados por una razón medida: cada `Loader` es un
            // objeto y unas ataduras por celda, y con trescientas ochenta y cuatro
            // celdas eso se nota aunque nunca llegue a instanciar nada. Dos
            // costaban un seis por ciento de fotogramas en una biblioteca que es
            // toda fotos, que es el caso normal.
            Loader {
                anchors.fill: parent
                active: celda.sinDibujo || celda.familia !== Textos.IMAGEN
                sourceComponent: Item {

                    // Lo que todavía no sabemos dibujar no puede ser un rectángulo
                    // gris sin más: un cajón de rectángulos iguales no se puede
                    // usar. Se enseña qué es y cómo se llama, que es lo que hay.
                    Column {
                        anchors.centerIn: parent
                        spacing: tema.hueco * 0.3
                        visible: celda.sinDibujo

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: Textos.nombreFamilia(celda.familia)
                            color: tema.textoTenue
                            font.pixelSize: Math.min(tema.fuente * 1.4,
                                                     Math.max(9, celda.celdaAlto * 0.16))
                            font.weight: Font.DemiBold
                        }
                        Text {
                            width: Math.max(10, celda.celdaAncho - tema.hueco * 2)
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideMiddle
                            text: modelo.nombreDe(celda.celdaIndice)
                            color: tema.textoTenue
                            font.pixelSize: Math.min(tema.fuente * 0.85,
                                                     Math.max(8, celda.celdaAlto * 0.1))
                            visible: celda.celdaAlto > tema.fuente * 4
                        }
                    }

                    // La insignia: cuánto dura un vídeo, de qué es un documento.
                    // Solo en lo que no es una foto, porque una malla con una
                    // etiqueta en cada celda deja de dejar ver las fotos.
                    Rectangle {
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.margins: tema.hueco * 0.5
                        width: texto.implicitWidth + tema.hueco * 0.8
                        height: texto.implicitHeight + tema.hueco * 0.3
                        radius: height / 2
                        color: tema.fondo
                        opacity: 0.82
                        visible: texto.text.length > 0 && celda.celdaAlto > tema.fuente * 3

                        Text {
                            id: texto
                            anchors.centerIn: parent
                            // La extensión se pide aquí dentro y no como propiedad
                            // de la celda: esto solo existe en lo que no es una
                            // foto, así que la cadena se construye una vez de cada
                            // veinte celdas en vez de en todas.
                            text: Textos.insignia(celda.familia,
                                                  modelo.extDe(celda.celdaIndice),
                                                  celda.duracion)
                            color: tema.texto
                            font.pixelSize: tema.fuente * 0.8
                        }
                    }
                }
            }

            // La marca de +18, solo mientras el modo seguro la está tapando. Con el
            // modo apagado la imagen se ve entera y un cartel encima sobraría.
            Loader {
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: tema.hueco * 0.5
                active: celda.censurado
                sourceComponent: Rectangle {
                    width: marca.implicitWidth + tema.hueco * 0.8
                    height: marca.implicitHeight + tema.hueco * 0.3
                    radius: height / 2
                    color: tema.fondo
                    opacity: 0.82

                    Text {
                        id: marca
                        anchors.centerIn: parent
                        text: "+18"
                        color: tema.texto
                        font.pixelSize: tema.fuente * 0.8
                        font.weight: Font.DemiBold
                    }
                }
            }

            // Las estrellas se ven al pasar por encima, y en lo elegido solo si es
            // un puñado.
            //
            // Las dos razones van juntas. La de mirar: una selección de diez mil
            // con una hilera de estrellas en cada celda deja de dejar ver las
            // fotos, que es justo para lo que sirve la malla. Y la medida: el texto
            // con contorno es caro, y en un recorrido con diez mil elegidos eran 33
            // fotogramas por encima de 16,6 ms; sin las estrellas, 6. Veintiocho
            // fotogramas por un adorno que además estorbaba.
            Loader {
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: tema.hueco * 0.5
                active: celda.estrellas > 0 && !celda.enLista
                        && (raton.containsMouse || (celda.elegido && ventana.seleccionMenuda))
                sourceComponent: Row {
                    spacing: 1
                    Repeater {
                        model: celda.estrellas
                        Text {
                            text: "★"
                            color: tema.seleccion
                            font.pixelSize: tema.fuente * 0.9
                            style: Text.Outline
                            styleColor: tema.fondo
                        }
                    }
                }
            }

        }

        // Los datos de la fila, en las mismas columnas que la cabecera.
        Loader {
            active: celda.enLista
            x: dibujo.x + dibujo.width + Math.round(tema.hueco * 1.2)
            width: parent.width - x
            height: parent.height
            sourceComponent: Item {
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.right: columnas.left
                    anchors.rightMargin: tema.hueco
                    elide: Text.ElideMiddle
                    text: celda.nombre
                    color: tema.texto
                    font.pixelSize: tema.fuente
                    font.weight: celda.elegido ? Font.DemiBold : Font.Normal
                }
                Row {
                    id: columnas
                    anchors.right: parent.right
                    anchors.rightMargin: tema.hueco
                    height: parent.height

                    Repeater {
                        model: ventana.columnasLista
                        delegate: Item {
                            id: columna
                            required property var modelData
                            width: modelData.ancho
                            height: columnas.height

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - tema.hueco
                                elide: Text.ElideRight
                                visible: columna.modelData.clave !== "estrellas"
                                text: {
                                    const i = celda.celdaIndice
                                    switch (columna.modelData.clave) {
                                    case "tipo": return modelo.extDe(i).toUpperCase()
                                    case "medidas": return Textos.medidas(modelo.anchoDe(i), modelo.altoDe(i))
                                    case "peso": return Textos.peso(modelo.pesoDe(i))
                                    case "fecha": return Textos.fecha(modelo.importadoDe(i))
                                    }
                                    return ""
                                }
                                color: tema.textoTenue
                                font.pixelSize: tema.fuente * 0.88
                            }
                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: columna.modelData.clave === "estrellas"
                                spacing: 1
                                Repeater {
                                    model: 5
                                    Text {
                                        required property int index
                                        text: "★"
                                        color: index < celda.estrellas ? tema.seleccion : tema.borde
                                        font.pixelSize: tema.fuente * 0.88
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // El clic derecho, aparte: el ratón de abajo solo coge el izquierdo,
        // y así el derecho no puede empezar un arrastre. Sobre algo que no
        // estaba elegido lo elige primero, como en cualquier gestor de
        // archivos; sobre parte de la selección, la deja como está.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.RightButton
            onPressed: function (evento) {
                if (!celda.elegido) modelo.elegir(celda.celdaIndice)
                else modelo.actual = celda.celdaIndice
                const p = mapToItem(null, evento.x, evento.y)
                ventana.abrirMenuElemento(p.x, p.y)
            }
        }

        MouseArea {
            id: raton
            /// Si ya se dijo, en este arrastre, que son demasiados para sacarlos.
            property bool avisadoFuera: false
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton
            // La mano cerrada es la otra mitad del aviso de que se está
            // arrastrando: el fantasma dice qué va, el cursor dice que va.
            cursorShape: raton.drag.active ? Qt.ClosedHandCursor : Qt.ArrowCursor
            onContainsMouseChanged: celda.pasada(containsMouse, celda.celdaIndice,
                                                 celda.celdaX, celda.celdaY,
                                                 celda.celdaAncho, celda.celdaAlto)
            // Con Ctrl o Mayús, la selección ya cambió al apretar (abajo); aquí
            // solo el clic a secas, que deja elegida esta y nada más.
            onClicked: function (evento) {
                if (!(evento.modifiers & (Qt.ShiftModifier | Qt.ControlModifier)))
                    modelo.elegir(celda.celdaIndice)
            }
            onDoubleClicked: {
                modelo.elegir(celda.celdaIndice)
                ventana.visorAbierto = true
            }

            // Arrastrar a la barra lateral. Lo que viaja son los ids de la
            // selección, no los índices: la vista puede cambiar por el camino.
            drag.target: fantasma
            onPressed: function (evento) {
                // Antes de tocar la selección: elegir ya rehace la malla, y el
                // aviso tiene que salir cuando esta celda todavía está donde se
                // pinchó. Va aquí y no en `onClicked` porque elegir pasa al
                // apretar; al soltar ya se ha movido todo.
                celda.elegida(celda.celdaX, celda.celdaY, celda.celdaAncho, celda.celdaAlto)
                raton.avisadoFuera = false
                // Ctrl y Mayús deciden aquí y no al soltar. Antes se elegía
                // esta sola al apretar y luego Ctrl la alternaba al soltar: la
                // quitaba, y Ctrl+clic dejaba la selección vacía; Mayús hacía
                // un tramo desde ella misma.
                if (evento.modifiers & Qt.ShiftModifier) modelo.elegirHasta(celda.celdaIndice)
                else if (evento.modifiers & Qt.ControlModifier) modelo.alternar(celda.celdaIndice)
                else if (!celda.elegido) modelo.elegir(celda.celdaIndice)
                fantasma.seguir(evento.x, evento.y)
                // De dónde salen, además de cuáles son. Soltarlos en otra
                // carpeta los mueve, y para moverlos hay que saber de dónde:
                // el destino solo no basta. Va a la ventana y no al `mimeData`
                // del arrastre porque en un arrastre interno ese mapa no llega
                // al otro lado; el porqué, entero, en `Ventana.qml`.
                ventana.arrastrarElementos(modelo.seleccion,
                                           ventana.enPapelera ? "" : ventana.carpetaActual)
            }
            onPositionChanged: function (evento) {
                if (!raton.drag.active) return
                fantasma.seguir(evento.x, evento.y)
                // Dentro de la malla, arrastrar reordena: el hueco va donde
                // caería y las demás se apartan. Fuera (sobre una carpeta), el
                // hueco vuelve a su sitio y el arrastre es el de siempre.
                if (ventana.permiteReorden()) {
                    const enMalla = raton.mapToItem(malla, evento.x, evento.y)
                    const dentro = enMalla.x >= 0 && enMalla.y >= 0
                                   && enMalla.x <= malla.width && enMalla.y <= malla.height
                    if (dentro) {
                        if (!disposicion.reordenando) disposicion.empezarReorden(celda.celdaIndice)
                        const c = raton.mapToItem(malla.contentItem, evento.x, evento.y)
                        disposicion.moverReorden(c.x, c.y)
                    } else if (disposicion.reordenando) {
                        disposicion.devolverReorden()
                    }
                }
                // Salir de la ventana arrastrando es querer llevárselo a otra
                // aplicación. Ahí el arrastre de dentro ya no sirve —ninguna
                // otra sabe qué es un id de Grimorio— y se cambia por uno del
                // sistema con los archivos. Dentro de la ventana todo sigue
                // igual: carpetas, bandas, soltar en el hueco.
                const p = raton.mapToItem(null, evento.x, evento.y)
                if (p.x < 0 || p.y < 0 || p.x > ventana.width || p.y > ventana.height) {
                    // Demasiados: se avisa una vez por arrastre y el arrastre
                    // de dentro sigue, para que volver a la ventana y soltar en
                    // una carpeta funcione igual. Ver `maxArrastreFuera`.
                    if (modelo.elegidos > ventana.maxArrastreFuera) {
                        if (!raton.avisadoFuera) ventana.avisarArrastreGrande()
                        raton.avisadoFuera = true
                    }
                    // Censurada, sin estampa: el sistema la pintaría nítida.
                    else if (portapapeles.arrastrarFuera(raton, modelo.urlsElegidas(),
                                                    celda.censurado ? ""
                                                    : modelo.previaDe(celda.celdaIndice)))
                        ventana.acabarArrastre()
                }
            }
            // Primero se entrega la caída y después se limpia.
            onReleased: {
                if (disposicion.reordenando) {
                    const enMalla = raton.mapToItem(malla, raton.mouseX, raton.mouseY)
                    const dentro = enMalla.x >= 0 && enMalla.y >= 0
                                   && enMalla.x <= malla.width && enMalla.y <= malla.height
                    if (dentro && disposicion.reordenMovido()) {
                        const v = disposicion.vecinosReorden()
                        ventana.reordenar(celda.idItem, v.antes, v.despues)
                        disposicion.acabarReorden(true)
                        ventana.acabarArrastre()
                        return
                    }
                    disposicion.acabarReorden(false)
                }
                fantasma.Drag.drop()
                ventana.acabarArrastre()
            }
            onCanceled: if (disposicion.reordenando) disposicion.acabarReorden(false)
        }

        // El bulto que se arrastra. La carga está en la ventana.
        Bulto {
            id: fantasma
            gesto: raton
            clave: "application/x-grimorio-ids"
        }
    }
}
