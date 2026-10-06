// La ventana: barra lateral de carpetas, malla y panel de detalle.
//
// Regla del proyecto, también aquí: ni un color ni una medida escritos a mano.
// Todo sale de `tema`, que lee un JSON y se recarga al guardarlo. La prueba
// `sin_colores_a_mano` recorre este directorio y falla si aparece un hexadecimal.

import QtQuick
import QtQuick.Window
import "consulta.js" as Consulta
import "textos.js" as Textos

Window {
    id: ventana
    visible: true
    width: anchoInicial
    height: altoInicial
    color: tema.fondo
    title: nucleo.nombre.length > 0
           ? qsTr("%1 · Grimorio").arg(nucleo.nombre)
           : qsTr("Grimorio")

    property real celda: celdaInicial
    onCeldaChanged: disposicion.objetivo = celda
    property string carpetaActual: ""
    /// La línea del buscador entera, con sus filtros escritos: es lo único que
    /// decide qué se ve. Los botones de filtro y la papelera escriben aquí en
    /// vez de llevar un estado paralelo que pudiera discrepar de lo escrito.
    property string textoBusqueda: filtroInicial
    readonly property bool enPapelera: Consulta.leer(textoBusqueda, "papelera") === "si"
    property bool visorAbierto: false

    /// El alto de la barra de arriba, que es también el de la cabeza de la
    /// barra lateral y el del inspector: las tres rayas de debajo son una.
    readonly property real altoBarra: Math.round(tema.fuente * 3.4)

    // Las vistas fijas de la barra lateral son búsquedas como cualquier otra.
    // Escritas aquí una vez, para que la fila que las pone y la que dice cuál
    // está puesta no puedan discrepar.
    readonly property string consultaSinEtiquetar: "etiquetado:no"
    readonly property string consultaRecientes: "fecha:7d"

    /// Qué vista fija se está mirando: "todo", "sinEtiquetar", "recientes",
    /// "papelera", o "" si es una carpeta o una búsqueda cualquiera.
    readonly property string vistaFija: {
        if (enPapelera) return "papelera"
        if (carpetaActual.length > 0) return ""
        const t = textoBusqueda.trim()
        if (t.length === 0) return "todo"
        if (t === consultaSinEtiquetar) return "sinEtiquetar"
        if (t === consultaRecientes) return "recientes"
        return ""
    }

    /// La carpeta dinámica cuyo filtro es justo el que se ve, o "".
    readonly property string dinamicaAbierta: {
        if (carpetaActual.length > 0 || enPapelera) return ""
        const bs = nucleo.busquedas
        for (let i = 0; i < bs.length; i++)
            if (bs[i].consulta === textoBusqueda.trim()) return bs[i].id
        return ""
    }
    /// Si lo que se ve es un filtro que se puede guardar como carpeta
    /// dinámica: hay algo escrito o algún filtro puesto, no es una vista fija
    /// ni está ya guardado. Dentro de una carpeta no, porque la carpeta no
    /// viaja en el texto y lo guardado no sería lo que se ve.
    readonly property bool hayFiltroGuardable: vistaFija === "" && carpetaActual.length === 0
                                               && dinamicaAbierta === ""
                                               && textoBusqueda.trim().length > 0

    function guardarComoDinamica() {
        lateral.guardarDinamica()
    }

    /// El rótulo de lo que se está mirando, para la barra de arriba.
    readonly property string tituloVista: {
        switch (vistaFija) {
        case "papelera": return qsTr("Papelera")
        case "sinEtiquetar": return qsTr("Sin etiquetar")
        case "recientes": return qsTr("Recientes")
        case "todo": return qsTr("Todo")
        }
        if (carpetaActual.length > 0) return nombreDeCarpeta(carpetaActual)
        const bs = nucleo.busquedas
        for (let i = 0; i < bs.length; i++)
            if (bs[i].consulta === textoBusqueda.trim()) return bs[i].nombre
        return qsTr("Búsqueda")
    }

    /// La fila de filtros, plegada mientras no se pida. Con algún filtro
    /// puesto el botón lo dice, así que plegarla no esconde nada.
    property bool verFiltros: false

    /// Las columnas de la vista en lista, de izquierda a derecha después del
    /// nombre: anchos en múltiplos de la letra, para que crezcan con ella. La
    /// cabecera y las filas las leen de aquí y no pueden desalinearse.
    readonly property var columnasLista: [
        { clave: "tipo", titulo: qsTr("tipo"), ancho: tema.fuente * 5.5 },
        { clave: "medidas", titulo: qsTr("medidas"), ancho: tema.fuente * 8.5 },
        { clave: "peso", titulo: qsTr("peso"), ancho: tema.fuente * 6 },
        { clave: "estrellas", titulo: qsTr("estrellas"), ancho: tema.fuente * 6.5 },
        { clave: "fecha", titulo: qsTr("llegó"), ancho: tema.fuente * 8 }
    ]
    readonly property real ladoMiniaturaLista: Math.round(tema.fuente * 3.4)
    /// Si lo elegido cabe en la cabeza. Por debajo de esto la malla puede
    /// permitirse adornar cada celda elegida; por encima, adornar cien mil
    /// celdas no informa de nada y cuesta fotogramas. El número es el mismo que
    /// usa el puente para decidir si avisa de uno en uno.
    readonly property bool seleccionMenuda: modelo.elegidos <= 64
    /// Cuántos elementos se pueden sacar de la ventana arrastrando.
    ///
    /// Arrastrar fuera entrega los originales con su nombre, y darles nombre
    /// es un enlace en disco por elemento (`Modelo::urlConNombre`), hecho en el
    /// hilo de la interfaz porque el sistema pide la lista entera al empezar el
    /// arrastre. Con todo elegido en una biblioteca de cien mil eran cien mil
    /// enlaces seguidos y la ventana congelada, y bastaba con pasarse del borde
    /// al llevar algo a una carpeta. Por encima de esto se avisa y se ofrece
    /// exportar, que es el camino para sacar muchos.
    readonly property int maxArrastreFuera: 1000

    /// El aviso de que son demasiados para arrastrarlos fuera.
    function avisarArrastreGrande() {
        estado.avisar(qsTr("son %1: demasiados para arrastrarlos fuera (como mucho %2). "
                           + "Para sacarlos todos, usa Exportar")
                      .arg(Textos.numero(modelo.elegidos)).arg(Textos.numero(maxArrastreFuera)),
                      true)
    }

    // Qué se está arrastrando ahora mismo dentro de la ventana.
    //
    // Esto parece que sobra, porque el sitio evidente para llevarlo es el
    // `Drag.mimeData` del arrastre. Pero en un arrastre interno de Qt Quick ese
    // mapa **no llega**: quien lo recibe ve las `keys` y la fuente, y `formats`
    // vacío, así que `getDataAsString` devuelve siempre cadena vacía. Con eso,
    // la fila de destino se iluminaba al pasar por encima —la iluminación solo
    // mira las claves— y al soltar no pasaba nada, porque el identificador que
    // había que mover llegaba en blanco. Ni un elemento entraba en una carpeta
    // ni una carpeta cambiaba de sitio.
    //
    // Las `keys` siguen puestas y siguen sirviendo: son las que dicen qué zona
    // acepta qué. Lo que va aquí es la carga.
    property string arrastreCarpeta: ""
    property var arrastreIds: []
    /// De qué carpeta salen los elementos, para que soltarlos en otra los mueva
    /// en vez de dejarlos en las dos.
    property string arrastreOrigen: ""
    /// Dónde está el cursor, en la ventana, mientras se arrastra.
    property point arrastrePunto: Qt.point(0, 0)
    /// Si hay un arrastre **en marcha**. No basta con que haya carga: la carga
    /// se apunta al apretar, y apretar sin mover es un clic.
    property bool arrastrando: false

    function arrastrarCarpeta(id) {
        arrastreCarpeta = id
        arrastreIds = []
        arrastreOrigen = ""
    }

    function arrastrarElementos(ids, origen) {
        arrastreCarpeta = ""
        arrastreIds = ids
        arrastreOrigen = origen
    }

    function moverArrastre(punto) {
        arrastrePunto = punto
        arrastrando = true
    }

    /// Se llama al soltar el botón, **después** de entregar la caída: quien la
    /// recibe lee de aquí.
    function acabarArrastre() {
        arrastreCarpeta = ""
        arrastreIds = []
        arrastreOrigen = ""
        arrastrando = false
    }

    /// Si el foco está dentro de un campo de texto.
    ///
    /// Un `Shortcut` de ventana se lleva la tecla **antes** de que llegue al
    /// campo que tiene el foco, y eso convertía escribir en un campo de nombre
    /// en una ruleta: con una imagen elegida, el Intro que cerraba el nombre de
    /// una carpeta abría el visor; Esc no cancelaba, cerraba lo que hubiera
    /// abierto; Supr no borraba una letra, tiraba lo elegido a la papelera; y
    /// un «3» en un nombre le ponía tres estrellas a otra cosa.
    ///
    /// Todo atajo que compita con una tecla de escribir se apaga mientras haya
    /// un campo con el foco. Los que llevan Ctrl y no chocan con nada —buscar,
    /// etiquetar— se quedan.
    readonly property bool escribiendo: esCampoDeTexto(activeFocusItem)

    /// Se pregunta por lo que sabe hacer y no por lo que es: `activeFocusItem`
    /// llega como `Item` pelado, y un `instanceof TextInput` contra un tipo de
    /// QML es justo lo que `qmllint` no puede comprobar.
    function esCampoDeTexto(cosa) {
        return cosa !== null && cosa !== undefined
               && typeof cosa.selectAll === "function"
               && typeof cosa.selectedText === "string"
    }

    // Los anchos de los paneles: lo que diga el tema mientras nadie los toque,
    // y lo que haya decidido quien está delante en cuanto los mueva. Los topes
    // salen del tamaño de letra, no de números redondos: con una letra grande
    // un panel de 160 px no cabe ni el nombre de una carpeta.
    readonly property real anchoLateralMin: tema.fuente * 9
    readonly property real anchoLateralMax: Math.min(width * 0.4, tema.fuente * 30)
    readonly property real anchoInspectorMin: tema.fuente * 14
    readonly property real anchoInspectorMax: Math.min(width * 0.5, tema.fuente * 40)
    readonly property real anchoLateral: ajustes.anchoLateral > 0
        ? Math.max(anchoLateralMin, Math.min(anchoLateralMax, ajustes.anchoLateral))
        : tema.lateral
    readonly property real anchoInspector: ajustes.anchoInspector > 0
        ? Math.max(anchoInspectorMin, Math.min(anchoInspectorMax, ajustes.anchoInspector))
        : tema.inspector

    function moverLateral(dx) {
        ajustes.anchoLateral = Math.max(anchoLateralMin,
                                        Math.min(anchoLateralMax, anchoLateral + dx))
    }
    function moverInspector(dx) {
        ajustes.anchoInspector = Math.max(anchoInspectorMin,
                                          Math.min(anchoInspectorMax, anchoInspector - dx))
    }

    // El estilo manda también sobre la disposición: cambiar el tema mueve la
    // malla entera sin recompilar nada.
    Binding { target: disposicion; property: "margen"; value: tema.margen }
    Binding { target: disposicion; property: "hueco"; value: tema.hueco }
    // El sitio del nombre bajo cada celda: una línea de letra pequeña y su aire.
    Binding { target: disposicion; property: "pie"; value: ajustes.verNombres ? Math.round(tema.fuente * 1.9) : 0 }
    Binding { target: disposicion; property: "altoLista"; value: ventana.ladoMiniaturaLista + Math.round(tema.hueco * 0.8) }
    Component.onCompleted: disposicion.objetivo = ventana.celda

    // Una sola función arma la consulta: así no hay dos sitios que puedan
    // discrepar sobre qué está enseñando la malla.
    //
    // La carpeta va en la consulta y no en el texto porque se elige señalando,
    // no escribiendo; todo lo demás sale de la línea del buscador, que el
    // núcleo entiende entera.
    function consultar() {
        var q = { "limit": 0 }
        // El orden por defecto es el de cada uno: el que se pone arrastrando,
        // y por fecha de llegada lo que nunca se ha movido —así, sin tocar
        // nada, es lo mismo que «recientes»—. Buscando palabras no: ahí manda
        // lo que mejor encaja.
        if (!Consulta.tieneTextoLibre(textoBusqueda)) q["sort"] = "manual"
        if (carpetaActual.length > 0) q["folder"] = carpetaActual
        nucleo.consultar(q, textoBusqueda)
    }

    /// El nombre de una carpeta por su id, para escribirlo en un rótulo.
    function nombreDeCarpeta(id) {
        const lista = nucleo.carpetas
        for (var i = 0; i < lista.length; i++)
            if (lista[i].id === id) return lista[i].nombre
        return ""
    }

    /// La hermana que va justo detrás de `id`, o "" si es la última.
    ///
    /// Es lo que hace falta para soltar «detrás de esta»: colocarse detrás de la
    /// última es quedarse al final, y eso se dice con la cadena vacía. La lista
    /// viene aplanada, así que entre una carpeta y su hermana siguiente están
    /// sus hijas: se saltan solas porque su padre es otro.
    function hermanaSiguiente(id) {
        const lista = nucleo.carpetas
        var padre = null
        for (var i = 0; i < lista.length; i++) {
            if (padre === null) {
                if (lista[i].id === id) padre = lista[i].padre
                continue
            }
            if (lista[i].padre === padre) return lista[i].id
        }
        return ""
    }

    /// Abre el menú de una carpeta. Con `id` vacío, el del hueco del panel: solo
    /// crear, y en la raíz.
    function abrirMenuBiblioteca(px, py) {
        menuBiblioteca.abrir(px, py)
    }

    function abrirMenuCarpeta(px, py, id, nombre) {
        menuCarpeta.abrir(px, py, id, nombre)
    }

    /// Lo que se puede hacer con lo elegido. Una sola lista para el clic
    /// derecho y para los botones del panel, así no dicen cosas distintas.
    function accionesElemento() {
        const varios = modelo.elegidos > 1
        let a = [
            { texto: qsTr("ver"), valor: "ver", icono: "ojo", atajo: "Intro" },
            { texto: qsTr("abrir fuera"), valor: "fuera", icono: "externo" },
            { texto: qsTr("copiar"), valor: "copiar", icono: "copiar", atajo: "Ctrl+C" },
            { texto: qsTr("exportar…"), valor: "exportar", icono: "exportar", atajo: "Ctrl+E" },
            { separador: true },
            { texto: qsTr("mover a…"), valor: "mover", icono: "carpeta" },
            { texto: varios ? qsTr("renombrar…") : qsTr("renombrar"),
              valor: "renombrar", icono: "lapiz", atajo: "F2" }
        ]
        if (carpetaActual.length > 0)
            a.push({ texto: qsTr("sacar de la carpeta"), valor: "sacar", icono: "salir" })
        a.push({ texto: modelo.adultoFoco ? qsTr("quitar +18") : qsTr("marcar +18"),
                 valor: "adulto", icono: "escudo" })
        a.push({ separador: true })
        a.push(enPapelera ? { texto: qsTr("recuperar"), valor: "papelera", icono: "papelera", atajo: "Supr" }
                          : { texto: qsTr("a la papelera"), valor: "papelera", icono: "papelera", atajo: "Supr" })
        return a
    }

    /// Hace una de `accionesElemento()`. `px, py` es desde dónde se pidió, para
    /// lo que abre otro menú (mover a…).
    function hacerConElemento(accion, px, py) {
        if (modelo.elegidos === 0) return
        switch (accion) {
        case "ver": if (modelo.actual >= 0) visorAbierto = true; break
        case "fuera": nucleo.abrirFuera(modelo.seleccion[0]); break
        case "copiar": copiarElegidos(); break
        case "exportar": exportar.exportar(modelo.urlsElegidas()); break
        case "mover": inspector.moverA(null, px, py); break
        case "renombrar": renombrar(); break
        case "sacar": nucleo.sacarDeCarpeta(modelo.seleccion, carpetaActual); break
        case "adulto": nucleo.marcarAdulto(modelo.seleccion, !modelo.adultoFoco); break
        case "papelera": nucleo.papelera(modelo.seleccion, !enPapelera); break
        }
    }

    /// El menú del clic derecho sobre una celda, en coordenadas de la ventana.
    function abrirMenuElemento(px, py) {
        desplegable.abrirEn(px, py, {
            titulo: modelo.elegidos > 1 ? Textos.elementos(modelo.elegidos) : "",
            opciones: accionesElemento(),
            alElegir: function (v) { ventana.hacerConElemento(v, px, py) }
        })
    }

    function abrirAjustes(pagina) {
        panelAjustes.abrir(pagina || "general")
    }

    /// Captura una zona de la pantalla y la mete en la carpeta que se mira.
    ///
    /// La ventana se minimiza primero: lo que se quiere capturar casi siempre
    /// está debajo. La espera es la animación de minimizar; sin ella, la
    /// captura salía con media ventana de Grimorio encogiéndose.
    function capturarPantalla() {
        if (captura.enMarcha) return
        if (ventana.enPapelera) ventana.verPapelera(false)
        visibilidadAntes = ventana.visibility
        ventana.showMinimized()
        apartada.restart()
    }
    /// Cómo estaba antes de apartarse: maximizada tiene que volver
    /// maximizada, no al tamaño de antes de maximizar.
    property int visibilidadAntes: Window.Windowed

    Timer {
        id: apartada
        interval: 400
        onTriggered: captura.capturar(ventana.carpetaActual)
    }

    Connections {
        target: captura
        function onAcabada() {
            ventana.visibility = ventana.visibilidadAntes === Window.Minimized
                                 ? Window.Windowed : ventana.visibilidadAntes
            ventana.raise()
            ventana.requestActivate()
        }
    }

    /// Reordenar arrastrando tiene sentido mirando por orden de llegada o a
    /// mano, de uno en uno, y sin texto ni color: ordenado por nombre, o por
    /// pertinencia al buscar, «ponlo aquí» no significa nada.
    function permiteReorden() {
        if (modelo.elegidos > 1 || ventana.enPapelera) return false
        const orden = Consulta.leer(textoBusqueda, "orden")
        if (orden !== "" && orden !== "manual" && orden !== "recientes") return false
        if (Consulta.tieneTextoLibre(textoBusqueda)) return false
        const t = Consulta.trozos(textoBusqueda)
        for (let i = 0; i < t.length; i++)
            if (Consulta.esCampo(t[i], "color") || Consulta.esCampo(t[i], "repetidos")) return false
        return true
    }

    /// Guarda el orden nuevo y pasa a mirar «a mano»: primero el orden y
    /// después la consulta, para que la vista que llega ya venga ordenada y
    /// no haya un instante con el orden viejo.
    function reordenar(id, antes, despues) {
        nucleo.reordenar(id, carpetaActual, antes, despues)
        // Mirando por «recientes» a secas, se pasa al orden de cada uno, que es
        // donde se ve lo que se acaba de mover.
        if (Consulta.leer(textoBusqueda, "orden") === "recientes") ponerFiltro("orden", "")
    }

    function abrirDesplegable(anclaje, def) {
        desplegable.abrir(anclaje, def)
    }
    function abrirDesplegableEn(px, py, def) {
        desplegable.abrirEn(px, py, def)
    }

    /// Todas las maneras de meter algo, en un sitio: el botón de captura de
    /// la barra se fue aquí, con los demás.
    function abrirMenuImportar(anclaje) {
        desplegable.abrir(anclaje, {
            opciones: [
                { texto: qsTr("importar archivos…"), valor: "archivos", icono: "importar" },
                { texto: qsTr("pegar"), valor: "pegar", icono: "pegar", atajo: "Ctrl+V" },
                { texto: qsTr("desde una dirección web…"), valor: "url", icono: "exportar" },
                { texto: qsTr("capturar una zona"), valor: "captura", icono: "captura", atajo: "Ctrl+Mayús+X" },
                { texto: qsTr("vigilar una carpeta del disco…"), valor: "vigilar", icono: "vigilar" }
            ],
            alElegir: function (v) {
                if (ventana.enPapelera) ventana.verPapelera(false)
                switch (v) {
                case "archivos": traer.elegirEImportar(ventana.carpetaActual); break
                case "pegar": traer.pegar(ventana.carpetaActual); break
                case "captura": ventana.capturarPantalla(); break
                case "vigilar": vigilancia.vigilar(ventana.carpetaActual); break
                case "url":
                    desplegable.abrir(anclaje, {
                        modo: "texto",
                        pista: "https://…",
                        alElegir: function (u) { traer.descargar(u, ventana.carpetaActual) }
                    })
                    break
                }
            }
        })
    }

    function abrirEtiquetas() {
        gestorEtiquetas.abrir()
    }

    function renombrar() {
        if (modelo.elegidos === 1) inspector.renombrar()
        else if (modelo.elegidos > 1) renombrarLote.abrir()
    }

    /// Copia lo elegido al portapapeles, como lo copia el gestor de archivos.
    function copiarElegidos() {
        const n = portapapeles.copiarArchivos(modelo.urlsElegidas())
        if (n > 0) estado.avisar(n === 1 ? qsTr("copiado al portapapeles")
                                         : qsTr("%1 copiados al portapapeles").arg(n))
    }

    /// Empieza a escribir una carpeta nueva en el panel, en su sitio. `padre`
    /// vacío es la raíz.
    function nuevaCarpeta(padre) {
        lateral.nuevaCarpeta(padre)
    }

    function irACarpeta(id) {
        carpetaActual = id
        // Elegir una carpeta saca de la papelera: mirar dentro de una carpeta
        // y ver solo lo tirado sería la sorpresa más tonta posible.
        if (enPapelera) textoBusqueda = Consulta.poner(textoBusqueda, "papelera", "")
        consultar()
    }

    /// Abre una carpeta dinámica: su búsqueda en el buscador, sobre toda la
    /// biblioteca. Vacía, vuelve a «Todo».
    function aplicarBusqueda(consulta) {
        carpetaActual = ""
        textoBusqueda = consulta
        consultar()
    }

    function enfocarBusqueda() {
        superior.enfocarBusqueda()
    }

    function verPapelera(si) {
        if (si) carpetaActual = ""
        textoBusqueda = Consulta.poner(textoBusqueda, "papelera", si ? "si" : "")
        consultar()
    }

    /// Pinchar una etiqueta la añade al filtro, y volver a pincharla la quita.
    function filtrarPorEtiqueta(t) {
        textoBusqueda = Consulta.alternar(textoBusqueda, "etiqueta", t)
        consultar()
    }

    function ponerFiltro(campo, valor) {
        textoBusqueda = Consulta.poner(textoBusqueda, campo, valor)
        consultar()
    }

    function pedirVaciarPapelera() {
        confirmacion.preguntar(
            qsTr("¿Borrar %1 de la papelera?").arg(Textos.elementos(nucleo.tirados)),
            qsTr("Se borran los archivos del disco. Esto no se puede deshacer, y también se pierde el resto del historial."),
            qsTr("borrar para siempre"))
    }

    // Arrastrar archivos a cualquier punto de la ventana los importa. Si hay
    // una carpeta seleccionada, caen dentro de ella.
    //
    // La clave no es adorno. Sin ella esta zona acepta **cualquier** arrastre,
    // también los de dentro, y como ocupa la ventana entera era la que se
    // quedaba con el arrastre de una foto o de una carpeta mientras no estuviera
    // encima de una fila del árbol: el velo de «suelta aquí para importar» se
    // encendía al mover algo de sitio dentro del programa. `text/uri-list` es
    // exactamente lo que trae un archivo de fuera, y es lo mismo que ya pedía
    // este `onDropped` al preguntar por `hasUrls`.
    DropArea {
        id: soltarAqui
        anchors.fill: parent
        keys: ["text/uri-list"]
        onDropped: function (caida) {
            if (caida.hasUrls) {
                // Salir de la papelera antes de importar: lo que entra no está
                // tirado, así que se quedaría fuera de la vista y parecería que
                // no ha pasado nada.
                if (ventana.enPapelera) ventana.verPapelera(false)
                // Por `traer` y no derecho al núcleo: lo que se suelta desde
                // un navegador son direcciones web, que hay que descargar.
                traer.soltar(caida.urls, ventana.carpetaActual)
                caida.accept()
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: tema.fondo

        // Tres columnas, como un estudio: de dónde se mira a la izquierda, lo
        // que se mira en el centro y lo elegido a la derecha. Las dos de los
        // lados van de arriba abajo; la barra de arriba y la de estado son
        // solo del centro, porque solo hablan de lo que hay en el centro.
        BarraLateral {
            id: lateral
            anchors { top: parent.top; bottom: parent.bottom; left: parent.left }
            width: ventana.anchoLateral
        }

        Divisor {
            anchors { top: parent.top; bottom: parent.bottom; left: lateral.right }
            onArrastrado: function (dx) { ventana.moverLateral(dx) }
            onRestablecido: ajustes.anchoLateral = 0
        }

        BarraSuperior {
            id: superior
            anchors { top: parent.top; left: lateral.right; right: inspector.left }
        }

        // Los títulos de las columnas en la vista en lista. Fuera de la malla
        // para que no se vayan al desplazarse.
        CabeceraLista {
            id: cabeceraLista
            anchors { top: superior.bottom; left: lateral.right; right: inspector.left }
            visible: disposicion.modo === 2
            height: visible ? implicitHeight : 0
        }

        Malla {
            id: malla
            anchors {
                top: cabeceraLista.bottom
                bottom: estado.top
                left: lateral.right
                right: inspector.left
            }
        }

        // Siempre puesto, con o sin nada elegido. Antes salía al elegir y se
        // iba al soltar, y la malla rehacía sus filas en cada cambio: lo que
        // se acababa de pinchar cambiaba de sitio debajo del ratón. Se pliega
        // con Ctrl+I o con su botón, y eso es una decisión, no un efecto.
        Inspector {
            id: inspector
            anchors { top: parent.top; bottom: parent.bottom }
            width: ajustes.verInspector ? ventana.anchoInspector : 0
            x: parent.width - width
            visible: width > 0
        }

        Divisor {
            anchors { top: parent.top; bottom: parent.bottom; right: inspector.left }
            visible: inspector.visible
            onArrastrado: function (dx) { ventana.moverInspector(dx) }
            onRestablecido: ajustes.anchoInspector = 0
        }

        BarraEstado {
            id: estado
            anchors { bottom: parent.bottom; left: lateral.right; right: inspector.left }
        }
    }

    // El visor tapa todo lo demás. Se abre con Intro o doble clic.
    Visor {
        id: visor
        anchors.fill: parent
        visible: ventana.visorAbierto
        onCerrar: ventana.visorAbierto = false
    }

    // El menú de carpeta va aquí y no dentro de la fila que lo abre: dentro
    // tendría el alto de una fila, el ancho del panel y el recorte del árbol.
    MenuCarpeta {
        id: menuCarpeta
        anchors.fill: parent
    }

    MenuBiblioteca {
        id: menuBiblioteca
        anchors.fill: parent
    }

    RenombrarLote {
        id: renombrarLote
    }

    GestorEtiquetas {
        id: gestorEtiquetas
    }

    // Lo que sale bajo el buscador al escribir: etiquetas, carpetas, filtros.
    AyudanteBusqueda {
        id: ayudanteBusqueda
        caja: superior.cajaBusqueda
        campo: superior.campoBusqueda
    }
    readonly property Item ayudante: ayudanteBusqueda

    PanelAjustes {
        id: panelAjustes
    }

    // Encima de todo lo demás: los chips de filtro y el menú de importar.
    Desplegable {
        id: desplegable
    }

    Confirmar {
        id: confirmacion
        onAceptado: nucleo.vaciarPapelera()
    }

    // Un velo mientras se arrastra: la ventana entera es la zona de soltar, así
    // que hay que decirlo, no dejar que se adivine.
    Rectangle {
        anchors.fill: parent
        color: tema.seleccion
        opacity: soltarAqui.containsDrag ? tema.realce * 2 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 120 } }
    }

    // Qué se está arrastrando, pegado al cursor.
    //
    // Sin esto, arrastrar era un acto de fe: el ratón no cambiaba, no se veía
    // nada moverse y la única señal llegaba al final del camino, cuando la fila
    // de destino se encendía. Aquí se ve desde el primer píxel qué carpeta va
    // en la mano, o cuántos elementos.
    //
    // No se lleva el ratón por dentro: la posición la manda el propio bulto que
    // se arrastra, que es el que ya la sabe.
    Item {
        id: fantasmaArrastre
        visible: ventana.arrastrando
        // Abajo y a la derecha del cursor, como cualquier cosa que se arrastra:
        // encima taparía justo el sitio al que se está apuntando.
        x: ventana.arrastrePunto.x + tema.hueco
        y: ventana.arrastrePunto.y + tema.hueco
        width: caja.width
        height: caja.height
        opacity: 0.9

        Rectangle {
            id: caja
            visible: ventana.arrastreCarpeta.length > 0
            width: visible ? rotulo.width + tema.hueco * 2 : 0
            height: visible ? rotulo.height + tema.hueco : 0
            radius: tema.radio
            color: tema.panel
            border.color: tema.seleccion
            border.width: 1

            // Una carpeta viaja como su nombre: no tiene miniatura, y un icono
            // genérico diría menos que la palabra.
            Text {
                id: rotulo
                anchors.centerIn: parent
                visible: ventana.arrastreCarpeta.length > 0
                text: ventana.nombreDeCarpeta(ventana.arrastreCarpeta)
                color: tema.texto
                font.pixelSize: tema.fuente
            }

            // Lo elegido no viaja como miniatura: la celda ya se mueve en la
            // malla y se ve dónde va a caer, y una copia pegada al cursor era
            // la foto sin difuminar aunque fuera +18. Solo va el número, si
            // van varios.
        }

        // Cuántos van, cuando van varios: la celda que se mueve es una sola, y
        // eso hace dudar de si se llevan los cuarenta.
        Rectangle {
            visible: ventana.arrastreIds.length > 1
            width: cuantos.width + tema.hueco * 0.8
            height: cuantos.height + tema.hueco * 0.3
            radius: height / 2
            color: tema.seleccion

            Text {
                id: cuantos
                anchors.centerIn: parent
                text: Textos.numero(ventana.arrastreIds.length)
                color: tema.fondo
                font.pixelSize: tema.fuente * 0.9
            }
        }
    }

    // La primera consulta la manda la ventana y **solo** la ventana.
    //
    // Antes la mandaba también `main.cpp`, y pasaban dos cosas: la biblioteca
    // entera se consultaba dos veces en cada arranque, y arrancar con un filtro
    // puesto no servía de nada porque la consulta sin filtrar llegaba después y
    // pisaba a la buena.
    Connections {
        target: nucleo
        function onListo() { ventana.consultar() }
    }

    Shortcut {
        sequences: [StandardKey.Find]
        onActivated: superior.enfocarBusqueda()
    }
    Shortcut {
        sequences: [StandardKey.Copy]
        enabled: modelo.elegidos > 0 && !ventana.escribiendo
        onActivated: ventana.copiarElegidos()
    }
    Shortcut {
        sequences: [StandardKey.Paste]
        enabled: !ventana.escribiendo
        onActivated: {
            // Lo pegado entra sin tirar: desde la papelera no se vería.
            if (ventana.enPapelera) ventana.verPapelera(false)
            traer.pegar(ventana.carpetaActual)
        }
    }
    Shortcut {
        sequence: "Ctrl+Shift+X"
        enabled: !ventana.escribiendo
        onActivated: ventana.capturarPantalla()
    }
    Shortcut {
        sequences: [StandardKey.SelectAll]
        enabled: !ventana.escribiendo
        onActivated: modelo.elegirTodo()
    }
    Shortcut {
        sequence: "Escape"
        enabled: !ventana.escribiendo
        onActivated: {
            if (confirmacion.abierto) confirmacion.abierto = false
            else if (desplegable.visible) desplegable.cerrar()
            else if (panelAjustes.abierto) panelAjustes.cerrar()
            else if (ventana.visorAbierto) ventana.visorAbierto = false
            else if (modelo.elegidos > 0) modelo.limpiarSeleccion()
        }
    }
    Shortcut {
        sequence: "Return"
        enabled: modelo.actual >= 0 && !ventana.visorAbierto && !ventana.escribiendo
        onActivated: ventana.visorAbierto = true
    }
    Shortcut {
        sequences: [StandardKey.Undo]
        enabled: !ventana.escribiendo
        onActivated: nucleo.deshacer()
    }
    Shortcut {
        sequences: [StandardKey.Redo]
        enabled: !ventana.escribiendo
        onActivated: nucleo.rehacer()
    }
    Shortcut {
        sequences: [StandardKey.Delete]
        enabled: modelo.elegidos > 0 && !ventana.escribiendo
        // En la papelera, Supr recupera. Borrar de verdad desde ahí tiene su
        // propio botón con su confirmación: la tecla que se pulsa sin mirar no
        // puede ser la que no tiene vuelta atrás.
        onActivated: nucleo.papelera(modelo.seleccion, !ventana.enPapelera)
    }
    Shortcut {
        sequence: "Ctrl+Shift+N"
        enabled: !ventana.escribiendo
        onActivated: ventana.nuevaCarpeta("")
    }
    // F2 con uno, el nombre en el panel; con varios, el patrón.
    Shortcut {
        sequence: "F2"
        enabled: modelo.elegidos > 0 && !ventana.escribiendo
        onActivated: ventana.renombrar()
    }
    Shortcut {
        sequence: "Ctrl+E"
        enabled: modelo.elegidos > 0 && !ventana.escribiendo
        onActivated: exportar.exportar(modelo.urlsElegidas())
    }
    Shortcut {
        sequence: "Ctrl+K"
        enabled: modelo.elegidos > 0 && !ventana.escribiendo
        onActivated: inspector.enfocarEtiquetas()
    }
    // Ctrl+L pasa por las tres vistas: justificado, cuadrícula, lista.
    Shortcut {
        sequence: "Ctrl+L"
        onActivated: disposicion.modo = disposicion.modo === 1 ? 0 : (disposicion.modo === 0 ? 2 : 1)
    }
    Shortcut {
        sequence: "Ctrl+I"
        onActivated: ajustes.verInspector = !ajustes.verInspector
    }
    Shortcut {
        sequence: "Ctrl+,"
        onActivated: panelAjustes.abierto ? panelAjustes.cerrar() : ventana.abrirAjustes()
    }
    Shortcut {
        sequence: "Ctrl+="
        // Con el visor abierto, + y − acercan la foto: el tamaño de las
        // celdas no se ve y cambiarlo a ciegas sería una sorpresa al salir.
        enabled: !ventana.visorAbierto
        onActivated: ventana.celda = Math.min(420, ventana.celda + 20)
    }
    Shortcut {
        sequence: "Ctrl+-"
        enabled: !ventana.visorAbierto
        onActivated: ventana.celda = Math.max(80, ventana.celda - 20)
    }
    // Las estrellas del 0 al 5, como en cualquier gestor de fotos.
    Repeater {
        model: 6
        delegate: Item {
            id: atajo
            // Con id propio y no `parent.index`: dentro de un delegado `parent`
            // no siempre es lo que parece, y aquí el error habría sido que la
            // tecla 3 pusiera otra cosa.
            required property int index
            Shortcut {
                sequence: String(atajo.index)
                enabled: modelo.elegidos > 0 && !ventana.escribiendo
                onActivated: nucleo.estrellas(modelo.seleccion, atajo.index)
            }
        }
    }

    // Banco del visor: la puerta de M1 es que abrir una imagen a pantalla
    // completa se vea en menos de 120 ms. Se mide con el programa entero
    // cargado, no con una maqueta.
    Item {
        id: bancoVisor
        property int quedan: pruebaVisor
        property real t0: 0
        property var tiempos: []

        function siguiente() {
            if (quedan <= 0) {
                let ms = tiempos.slice().sort(function (a, b) { return a - b })
                const p = function (q) { return ms[Math.floor((ms.length - 1) * q)] }
                console.log("\nbanco del visor")
                console.log("  aperturas           " + ms.length)
                console.log("  media               "
                            + (ms.reduce(function (a, b) { return a + b }, 0) / ms.length).toFixed(1) + " ms")
                console.log("  p50 / p95 / máximo  " + p(0.5).toFixed(1) + " / "
                            + p(0.95).toFixed(1) + " / " + ms[ms.length - 1].toFixed(1) + " ms")
                console.log("  presupuesto         120 ms  →  "
                            + (p(0.95) <= 120 ? "dentro" : "FUERA"))
                Qt.callLater(Qt.quit)
                return
            }
            quedan -= 1
            ventana.visorAbierto = false
            // Repartidos por toda la biblioteca: abrir siempre el mismo mediría
            // el caché, no el disco.
            const i = Math.floor(modelo.total * (quedan + 1) / (pruebaVisor + 2))
            modelo.elegir(i)
            t0 = banco.transcurridos() * 1000
            ventana.visorAbierto = true
            // Si se pidieron más aperturas que elementos hay, dos seguidas caen
            // en el mismo y la imagen ya está puesta: entonces no llega ningún
            // cambio de `grandeLista` y el banco se quedaba esperando para
            // siempre. Se anota aquí y sigue.
            if (visor.grandeLista) Qt.callLater(bancoVisor.anotar)
        }

        function anotar() {
            if (!visor.grandeLista || !ventana.visorAbierto) return
            tiempos.push(banco.transcurridos() * 1000 - t0)
            Qt.callLater(siguiente)
        }

        Connections {
            target: visor
            enabled: pruebaVisor > 0
            function onGrandeListaChanged() { bancoVisor.anotar() }
        }

        Connections {
            target: nucleo
            enabled: pruebaVisor > 0
            function onVistaNueva() {
                if (bancoVisor.tiempos.length === 0 && bancoVisor.quedan === pruebaVisor)
                    Qt.callLater(bancoVisor.siguiente)
            }
        }
    }

    // `--ver`: el visor abierto con el primer elemento en cuanto llega la
    // primera vista. Solo una vez.
    Connections {
        target: nucleo
        enabled: verAlArrancar && !ventana.visorAbierto
        function onVistaNueva() {
            if (modelo.total === 0) return
            modelo.elegir(0)
            ventana.visorAbierto = true
        }
    }

    // Guion: recorre la interfaz solo y deja una captura de cada paso. No es
    // una demo bonita, es la forma de revisar diseño y comportamiento juntos
    // sin una persona delante haciendo clics.
    Item {
        id: guionista
        property int paso: 0
        property string carpeta: ""
        readonly property string etiqueta: "rótulo"
        property real escalaAntes: 1
        property string temaAntes: ""

        Timer {
            id: reloj
            interval: 900
            running: guion.length > 0 && modelo.total > 0
            repeat: true
            onTriggered: guionista.siguiente()
        }

        function siguiente() {
            paso += 1
            switch (paso) {
            case 1:
                app.capturar(guion + "1-malla-justificada.png")
                break
            case 2:
                modelo.elegir(4)
                modelo.alternar(5)
                modelo.alternar(11)
                break
            case 3:
                app.capturar(guion + "2-seleccion-e-inspector.png")
                nucleo.estrellas(modelo.seleccion, 4)
                break
            case 4:
                app.capturar(guion + "3-estrellas-puestas.png")
                nucleo.etiquetar(modelo.seleccion, [guionista.etiqueta], [])
                break
            case 5:
                app.capturar(guion + "4-etiquetas-en-lote.png")
                inspector.escribirEtiqueta(guionista.etiqueta.substring(0, 3))
                break
            case 6:
                app.capturar(guion + "5-autocompletado.png")
                inspector.escribirEtiqueta("")
                ventana.filtrarPorEtiqueta(guionista.etiqueta)
                break
            case 7:
                app.capturar(guion + "6-filtrado-por-etiqueta.png")
                ventana.filtrarPorEtiqueta(guionista.etiqueta)
                break
            case 8:
                // Uno solo: la nota es de un elemento, y el panel lo dice
                // escondiéndola cuando hay varios.
                modelo.elegir(4)
                nucleo.nota(modelo.idDe(4), "para el moodboard de otoño")
                break
            case 9:
                app.capturar(guion + "7-nota.png")
                nucleo.crearCarpeta("Moodboard", "")
                break
            case 10:
                if (nucleo.carpetas.length > 0) {
                    guionista.carpeta = nucleo.carpetas[0].id
                    nucleo.moverAcarpeta(modelo.seleccion, guionista.carpeta)
                }
                break
            case 11:
                app.capturar(guion + "8-carpeta-con-tres.png")
                ventana.irACarpeta(guionista.carpeta)
                break
            case 12:
                app.capturar(guion + "9-dentro-de-la-carpeta.png")
                ventana.visorAbierto = true
                break
            case 13:
                app.capturar(guion + "10-visor.png")
                ventana.visorAbierto = false
                ventana.irACarpeta("")
                break
            case 14:
                // Elegir en el paso siguiente y no aquí: la vista nueva llega
                // por el bus, así que elegir en el mismo paso en que se pide
                // señala filas de la vista vieja.
                modelo.elegir(4)
                modelo.alternar(5)
                nucleo.papelera(modelo.seleccion, true)
                break
            case 15:
                app.capturar(guion + "11-tirados.png")
                ventana.verPapelera(true)
                break
            case 16:
                app.capturar(guion + "12-la-papelera.png")
                ventana.pedirVaciarPapelera()
                break
            case 17:
                app.capturar(guion + "13-lo-que-no-tiene-vuelta.png")
                confirmacion.abierto = false
                ventana.verPapelera(false)
                break
            case 18:
                ventana.textoBusqueda = "tipo:imagen estrellas:>=3"
                ventana.consultar()
                break
            case 19:
                app.capturar(guion + "14-filtros.png")
                ventana.textoBusqueda = ""
                ventana.consultar()
                disposicion.modo = 0
                break
            case 20:
                app.capturar(guion + "15-cuadricula.png")
                break
            // A partir de aquí el guion se limpia lo suyo con el propio
            // deshacer: la carpeta, la papelera, la nota, la etiqueta y las
            // estrellas. Si no, cada pasada dejaría rastro y la siguiente
            // mediría otra cosa —y de paso se comprueba que la pila deshace
            // seis cosas seguidas sin equivocarse de orden.
            // Los ajustes y el clic derecho. Cambian preferencias de verdad, así
            // que se devuelven a como estaban al acabar.
            case 21:
                guionista.escalaAntes = ajustes.escala
                guionista.temaAntes = ajustes.tema
                modelo.elegir(0)
                ventana.abrirMenuElemento(ventana.width * 0.4, ventana.height * 0.35)
                break
            case 22:
                app.capturar(guion + "17-menu-del-clic-derecho.png")
                desplegable.cerrar()
                // Una pista de la barra encima de la malla: antes se metía debajo.
                ventana.pistas.mostrar(superior.cajaBusqueda, qsTr("modo seguro: lo +18 sale difuminado"))
                break
            case 23:
                app.capturar(guion + "18-pista-encima-de-la-malla.png")
                ventana.pistas.ocultar(superior.cajaBusqueda)
                ventana.abrirAjustes("general")
                break
            case 24:
                app.capturar(guion + "19-ajustes.png")
                ajustes.escala = 1.2
                ajustes.tema = "papel"
                break
            case 25:
                app.capturar(guion + "20-ajustes-papel-120.png")
                ventana.abrirAjustes("acerca")
                break
            case 26:
                app.capturar(guion + "21-acerca-de.png")
                panelAjustes.cerrar()
                ajustes.escala = guionista.escalaAntes
                ajustes.tema = guionista.temaAntes
                break
            case 27:
                if (guionista.carpeta.length > 0) nucleo.borrarCarpeta(guionista.carpeta)
                break
            default:
                if (nucleo.queSeDeshace.length > 0) {
                    nucleo.deshacer()
                    reloj.interval = 300
                    return
                }
                app.capturar(guion + "16-todo-deshecho.png")
                reloj.running = false
                Qt.quit()
            }
        }
    }

    // Banco de edición en lote: la puerta de M2. Etiqueta N elementos mientras
    // la malla se está desplazando, que es la única forma honrada de comprobar
    // «sin congelar la interfaz»: medir el hilo del núcleo por su cuenta diría
    // que va rápido aunque la ventana se quedara quieta.
    Item {
        id: bancoLote
        property bool lanzado: false
        property real t0: 0
        property real tSeleccion: 0
        // Los fotogramas de la ventana en la que el lote está en marcha, que es
        // lo único que la puerta de M2 pregunta. Medir la media de los doce
        // segundos enteros diluiría justo lo que se quiere ver.
        property real peor: 0
        property int malos: 0
        property int cuantos: 0
        property real yAntes: 0

        Connections {
            target: nucleo
            enabled: pruebaLote > 0
            function onVistaNueva(n) {
                if (bancoLote.lanzado || n < pruebaLote) return
                bancoLote.lanzado = true
                // Un segundo de desplazamiento antes de tocar nada: así el peor
                // fotograma del arranque no se cuela en la medida del lote.
                arranque.start()
            }
            function onAviso(mensaje, error) {
                if (!bancoLote.lanzado || bancoLote.t0 === 0) return
                console.log("\nbanco de edición en lote")
                console.log("  elegir " + pruebaLote + "        "
                            + bancoLote.tSeleccion.toFixed(1) + " ms")
                console.log("  etiquetar           "
                            + (banco.transcurridos() * 1000 - bancoLote.t0).toFixed(1) + " ms")
                console.log("  respuesta           " + mensaje)
                console.log("  posición al empezar " + bancoLote.yAntes.toFixed(0))
                console.log("  posición al acabar  " + malla.contentY.toFixed(0))
                console.log("  fotogramas          " + bancoLote.cuantos)
                console.log("  peor fotograma      " + bancoLote.peor.toFixed(1) + " ms")
                console.log("  por encima de 16,6  " + bancoLote.malos)
                bancoLote.t0 = 0
                // Deshacerlo al terminar: un banco que deja diez mil elementos
                // etiquetados cambia la biblioteca que mide, y la siguiente
                // pasada ya no mide lo mismo.
                nucleo.deshacer()
            }
        }

        Timer {
            id: arranque
            interval: 1000
            onTriggered: {
                const antes = banco.transcurridos() * 1000
                modelo.elegirPrimeros(pruebaLote)
                bancoLote.tSeleccion = banco.transcurridos() * 1000 - antes
                bancoLote.t0 = banco.transcurridos() * 1000
                bancoLote.yAntes = malla.contentY
                nucleo.etiquetar(modelo.seleccion, ["banco-lote"], [])
            }
        }
    }

    // Recorrido automático, el mismo que midió el spike. Mide el programa de
    // verdad, con su barra lateral y su inspector puestos.
    FrameAnimation {
        running: banco.activo
        onTriggered: {
            const t = banco.transcurridos()
            if (t > banco.segundos) {
                running = false
                app.terminar(qsTr("banco de pruebas"))
                return
            }
            if (bancoLote.t0 > 0) {
                const ms = frameTime * 1000
                bancoLote.cuantos += 1
                if (ms > bancoLote.peor) bancoLote.peor = ms
                if (ms > 16.6) bancoLote.malos += 1
            }
            const recorrido = Math.max(1, malla.contentHeight - malla.height)
            const v = (recorrido / banco.segundos) * 2.0
            const dir = Math.floor(t / (banco.segundos / 2)) % 2 === 0 ? 1 : -1
            malla.contentY = Math.max(0, Math.min(recorrido, malla.contentY + v * frameTime * dir))
        }
    }

    // Las pistas de los botones, encima de todo: ver `Pista.qml`.
    Pista {
        id: capaPistas
    }
    readonly property Item pistas: capaPistas
}
