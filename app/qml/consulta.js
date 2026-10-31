// Leer y escribir la línea del buscador.
//
// Los botones de filtro no llevan un estado paralelo: escriben en la misma
// línea de texto que se puede teclear a mano, y la leen de ahí. Así no hay dos
// verdades que puedan discrepar, y lo que se elige con el ratón se puede
// copiar, pegar y —el día que existan las carpetas inteligentes— guardar.
//
// El lenguaje de verdad vive en Rust, con sus pruebas. Aquí solo hace falta
// saber trocear y sustituir un campo.
.pragma library

/// Trocea respetando las comillas, igual que el lado de Rust.
function trozos(texto) {
    var out = []
    var actual = ""
    var comillas = false
    for (var i = 0; i < texto.length; ++i) {
        var c = texto.charAt(i)
        if (c === '"') {
            comillas = !comillas
        } else if (c === " " && !comillas) {
            if (actual.length > 0) { out.push(actual); actual = "" }
        } else {
            actual += c
        }
    }
    if (actual.length > 0) out.push(actual)
    return out
}

/// Los campos que escriben los botones, con todos los nombres con que el
/// núcleo los entiende (`filtro.rs`). El primero es el de dentro, con el que
/// los piden los botones; el segundo, el que se escribe con la interfaz en
/// inglés. Así quien la tiene en inglés ve `type:image` en el buscador, y una
/// carpeta dinámica guardada en un idioma se sigue leyendo en el otro.
var CAMPOS = {
    tipo: ["tipo", "type", "kind", "familia"],
    orden: ["orden", "sort"],
    fecha: ["fecha", "date", "importado", "cuando"],
    estrellas: ["estrellas", "stars"],
    orientacion: ["orientacion", "orientation", "orientación", "shape", "orient"],
    repetidos: ["repetidos", "duplicates", "repetido", "duplicados", "dupes"],
    papelera: ["papelera", "trash"],
    etiquetado: ["etiquetado", "tagged", "etiquetada"],
    etiqueta: ["etiqueta", "tag", "etiquetas", "tags"],
    adulto: ["adulto", "adult", "+18"]
}

/// Los valores que cambian de un idioma a otro, de dentro → en inglés. Los que
/// no están aquí (números, colores, `7d`) se escriben igual en los dos.
var VALORES = {
    tipo: { imagen: "image", video: "video", audio: "audio", modelo: "model",
            documento: "document", tipografia: "font", raw: "raw" },
    orden: { recientes: "newest", antiguos: "oldest", nombre: "name", peso: "size",
             estrellas: "stars", azar: "random", manual: "manual" },
    fecha: { hoy: "today", ayer: "yesterday", "año": "year" },
    orientacion: { apaisada: "landscape", vertical: "portrait", cuadrada: "square" },
    repetidos: { si: "yes", no: "no" },
    papelera: { si: "yes", no: "no" },
    etiquetado: { si: "yes", no: "no" },
    adulto: { si: "yes", no: "no" }
}

function enIngles() {
    return Qt.uiLanguage === "en"
}

/// Cómo se escribe un campo en el buscador, en el idioma de la interfaz.
function nombreCampo(campo) {
    var c = CAMPOS[campo]
    return c && enIngles() ? c[1] : campo
}

/// Un valor de dentro, como se escribe en el buscador.
function valorEscrito(campo, valor) {
    var v = VALORES[campo]
    return v && enIngles() && v[valor] !== undefined ? v[valor] : valor
}

/// Un valor escrito en cualquiera de los dos idiomas, como se piensa dentro.
function valorDeDentro(campo, valor) {
    var v = VALORES[campo]
    if (!v) return valor
    var l = valor.toLowerCase()
    if (l === "sí") return "si"
    for (var k in v)
        if (v[k] === l || k === l) return k
    return valor
}

/// Si el trozo es ese campo, con cualquiera de sus nombres.
function esCampo(trozo, campo) {
    var l = trozo.toLowerCase()
    var nombres = CAMPOS[campo] || [campo]
    for (var i = 0; i < nombres.length; ++i)
        if (l.indexOf(nombres[i] + ":") === 0) return true
    return false
}

/// Lo que va detrás de los dos puntos.
function valorDe(trozo) {
    return trozo.substring(trozo.indexOf(":") + 1)
}

/// El valor de un campo, o "" si no está. Como se piensa dentro: `si`, y no
/// `yes`, aunque esté escrito en inglés.
function leer(texto, campo) {
    var t = trozos(texto)
    for (var i = 0; i < t.length; ++i) {
        if (esCampo(t[i], campo)) return valorDeDentro(campo, valorDe(t[i]))
    }
    return ""
}

/// Un filtro entero como se escribe en el buscador: `tipo:video`, o
/// `type:video` con la interfaz en inglés.
function filtro(campo, valor) {
    return nombreCampo(campo) + ":" + valorEscrito(campo, valor)
}

/// Pone un campo, lo cambia o lo quita si el valor va vacío.
///
/// El campo se escribe al final y en el sitio donde estaba si ya existía: que
/// pulsar un botón no le reordene a nadie lo que estaba escribiendo.
function poner(texto, campo, valor) {
    var t = trozos(texto)
    var salida = []
    var puesto = false
    var escrito = valorEscrito(campo, valor)
    var nuevo = valor.length === 0 ? "" : nombreCampo(campo) + ":"
                + (escrito.indexOf(" ") >= 0 ? '"' + escrito + '"' : escrito)
    for (var i = 0; i < t.length; ++i) {
        if (esCampo(t[i], campo)) {
            if (nuevo.length > 0 && !puesto) { salida.push(nuevo); puesto = true }
        } else {
            salida.push(t[i].indexOf(" ") >= 0 ? '"' + t[i] + '"' : t[i])
        }
    }
    if (nuevo.length > 0 && !puesto) salida.push(nuevo)
    return salida.join(" ")
}

/// Añade o quita un valor de un campo que admite varios, como `etiqueta:`.
function alternar(texto, campo, valor) {
    var t = trozos(texto)
    var salida = []
    var estaba = false
    for (var i = 0; i < t.length; ++i) {
        var esta = esCampo(t[i], campo) && valorDe(t[i]) === valor
        if (esta) { estaba = true; continue }
        salida.push(t[i].indexOf(" ") >= 0 ? '"' + t[i] + '"' : t[i])
    }
    if (!estaba) {
        salida.push(nombreCampo(campo) + ":" + (valor.indexOf(" ") >= 0 ? '"' + valor + '"' : valor))
    }
    return salida.join(" ")
}

/// Las muestras del filtro de color. Son valores que se buscan, no colores de
/// la interfaz: por eso viven aquí como datos y no en el tema. Doce tonos que
/// cubren la rueda y los neutros, que es lo que se busca de verdad —«algo
/// azul», «algo oscuro»—; para afinar está el campo de al lado.
var MUESTRAS = [
    "#d93636", "#e8862a", "#e8c82a", "#3fae4a", "#2ab3b0", "#2f6fd6",
    "#8a46c9", "#e05aa8", "#8a5a36", "#141414", "#8a8a8a", "#f2f2f2"
]

/// Las muestras con una vacía delante: «sin color», para el color de carpeta.
function muestrasConNinguna() {
    return [""].concat(MUESTRAS)
}

/// Si hay palabras sueltas (texto que buscar), no solo filtros y etiquetas.
function tieneTextoLibre(texto) {
    var t = trozos(texto)
    for (var i = 0; i < t.length; ++i)
        if (t[i].indexOf(":") < 0 && t[i].charAt(0) !== "#" && t[i].charAt(0) !== "-") return true
    return false
}

/// La palabra que se está escribiendo: el último trozo, o "" si el texto acaba
/// en espacio (se empieza una nueva).
function ultimaPalabra(texto) {
    if (texto.length === 0 || texto.charAt(texto.length - 1) === " ") return ""
    var t = trozos(texto)
    return t.length > 0 ? t[t.length - 1] : ""
}

/// Cambia la palabra que se está escribiendo por `nuevo` y deja un espacio
/// detrás, para seguir. Si `nuevo` va vacío, la quita.
function cambiarUltima(texto, nuevo) {
    var t = trozos(texto)
    var empieza = texto.length === 0 || texto.charAt(texto.length - 1) === " "
    if (!empieza && t.length > 0) t.pop()
    var salida = []
    for (var i = 0; i < t.length; ++i)
        salida.push(t[i].indexOf(" ") >= 0 ? '"' + t[i] + '"' : t[i])
    if (nuevo.length > 0) salida.push(nuevo)
    var s = salida.join(" ")
    return s.length > 0 ? s + " " : ""
}

/// Un nombre para una carpeta dinámica hecho con sus filtros: «comida · sin
/// picante · vídeos». Se pone de partida; quien quiera otro, la renombra.
function nombreDe(texto) {
    var en = Qt.uiLanguage === "en"
    var FAMILIAS = en
        ? { imagen: "images", video: "videos", audio: "audio", modelo: "3D",
            documento: "documents", tipografia: "fonts", raw: "RAW" }
        : { imagen: "imágenes", video: "vídeos", audio: "audio", modelo: "3D",
            documento: "documentos", tipografia: "fuentes", raw: "RAW" }
    var o = en ? " or " : " o "
    var partes = []
    var t = trozos(texto)
    for (var i = 0; i < t.length; ++i) {
        var trozo = t[i]
        var fuera = trozo.charAt(0) === "-" && trozo.length > 1
        if (fuera) trozo = trozo.substring(1)
        var dicho = ""
        var sug = null
        for (var k = 0; k < SUGERENCIAS.length; ++k)
            if (esSugerencia(trozo, SUGERENCIAS[k])) sug = SUGERENCIAS[k]
        if (sug) {
            dicho = textoDe(sug).replace(/^(tipo|type|forma|shape): /, "")
        } else if (trozo.charAt(0) === "#") {
            dicho = trozo.substring(1).split("|").join(o)
        } else if (esCampo(trozo, "etiqueta")) {
            dicho = valorDe(trozo).split("|").join(o)
        } else if (esCampo(trozo, "tipo") || esCampo(trozo, "ext")) {
            var v = valorDeDentro("tipo", valorDe(trozo).toLowerCase())
            dicho = FAMILIAS[v] || v
        } else if (esCampo(trozo, "estrellas")) {
            dicho = valorDe(trozo).replace(">=", "≥").replace("<=", "≤") + " ★"
        } else if (esCampo(trozo, "orden")) {
            continue
        } else {
            dicho = trozo.replace(":", " ")
        }
        if (dicho.length > 0) partes.push((fuera ? (en ? "without " : "sin ") : "") + dicho)
    }
    var n = partes.join(" · ")
    if (n.length > 60) n = n.substring(0, 59) + "…"
    return n.length > 0 ? n : (en ? "smart folder" : "carpeta dinámica")
}

/// `#etiqueta` como se escribe en el buscador: entre comillas si lleva
/// espacios, que si no serían dos palabras.
function comoEtiqueta(nombre) {
    return nombre.indexOf(" ") >= 0 ? '#"' + nombre + '"' : "#" + nombre
}

/// Filtros que se sugieren al escribir: lo que se teclea, y lo que se pone.
var SUGERENCIAS = [
    { claves: ["imagen", "imágenes", "foto", "fotos", "image", "images", "photo", "photos", "picture"], texto: "tipo: imágenes", en: "type: images", token: "tipo:imagen", icono: "cuadricula" },
    { claves: ["video", "vídeo", "vídeos", "videos", "clip"], texto: "tipo: vídeos", en: "type: videos", token: "tipo:video", icono: "video" },
    { claves: ["audio", "sonido", "música", "musica", "sound", "music"], texto: "tipo: audio", en: "type: audio", token: "tipo:audio", icono: "orden" },
    { claves: ["3d", "modelo", "stl", "malla", "model", "mesh"], texto: "tipo: 3D", en: "type: 3D", token: "tipo:modelo", icono: "forma" },
    { claves: ["pdf", "documento", "document"], texto: "tipo: documentos", en: "type: documents", token: "tipo:documento", icono: "pegar" },
    { claves: ["fuente", "tipografía", "tipografia", "letra", "font", "typeface"], texto: "tipo: fuentes", en: "type: fonts", token: "tipo:tipografia", icono: "etiqueta" },
    { claves: ["hoy", "today"], texto: "importado hoy", en: "imported today", token: "fecha:hoy", icono: "fecha" },
    { claves: ["ayer", "yesterday"], texto: "importado ayer", en: "imported yesterday", token: "fecha:ayer", icono: "fecha" },
    { claves: ["semana", "reciente", "recientes", "week", "recent"], texto: "importado esta semana", en: "imported this week", token: "fecha:7d", icono: "fecha" },
    { claves: ["mes", "month"], texto: "importado este mes", en: "imported this month", token: "fecha:30d", icono: "fecha" },
    { claves: ["repetidos", "duplicados", "copias", "duplicates", "dupes"], texto: "solo repetidos", en: "duplicates only", token: "repetidos:si", icono: "repetidos" },
    { claves: ["estrellas", "favoritos", "mejores", "stars", "rating", "favorites", "best"], texto: "4 estrellas o más", en: "4 stars or more", token: "estrellas:>=4", icono: "estrella" },
    { claves: ["apaisada", "horizontal", "landscape"], texto: "forma: apaisada", en: "shape: landscape", token: "orientacion:apaisada", icono: "forma" },
    { claves: ["vertical", "retrato", "portrait"], texto: "forma: vertical", en: "shape: portrait", token: "orientacion:vertical", icono: "forma" },
    { claves: ["cuadrada", "square"], texto: "forma: cuadrada", en: "shape: square", token: "orientacion:cuadrada", icono: "forma" },
    { claves: ["papelera", "tirado", "trash", "deleted"], texto: "la papelera", en: "the trash", token: "papelera:si", icono: "papelera" },
    { claves: ["+18", "adulto", "adult", "nsfw"], texto: "marcado +18", en: "marked 18+", token: "adulto:si", icono: "escudo" }
]

/// Lo que escribe una sugerencia en el buscador, en el idioma de la interfaz.
function tokenDe(sug) {
    var i = sug.token.indexOf(":")
    return filtro(sug.token.substring(0, i), sug.token.substring(i + 1))
}

/// Si un trozo del buscador es esa sugerencia, escrita en cualquier idioma.
function esSugerencia(trozo, sug) {
    var i = sug.token.indexOf(":")
    var campo = sug.token.substring(0, i)
    return esCampo(trozo, campo)
           && valorDeDentro(campo, valorDe(trozo)) === sug.token.substring(i + 1)
}

/// Lo que se enseña de una sugerencia, en el idioma de la interfaz. El token
/// que se escribe en el buscador es el mismo en los dos: el núcleo los
/// entiende todos (ver `filtro.rs`).
function textoDe(sug) {
    return Qt.uiLanguage === "en" ? sug.en : sug.texto
}
