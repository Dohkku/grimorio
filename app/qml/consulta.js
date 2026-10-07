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

function esCampo(trozo, campo) {
    return trozo.toLowerCase().indexOf(campo + ":") === 0
}

/// El valor de un campo, o "" si no está.
function leer(texto, campo) {
    var t = trozos(texto)
    for (var i = 0; i < t.length; ++i) {
        if (esCampo(t[i], campo)) return t[i].substring(campo.length + 1)
    }
    return ""
}

/// Pone un campo, lo cambia o lo quita si el valor va vacío.
///
/// El campo se escribe al final y en el sitio donde estaba si ya existía: que
/// pulsar un botón no le reordene a nadie lo que estaba escribiendo.
function poner(texto, campo, valor) {
    var t = trozos(texto)
    var salida = []
    var puesto = false
    var nuevo = valor.length === 0 ? "" : campo + ":" + (valor.indexOf(" ") >= 0 ? '"' + valor + '"' : valor)
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
        var esta = esCampo(t[i], campo) && t[i].substring(campo.length + 1) === valor
        if (esta) { estaba = true; continue }
        salida.push(t[i].indexOf(" ") >= 0 ? '"' + t[i] + '"' : t[i])
    }
    if (!estaba) {
        salida.push(campo + ":" + (valor.indexOf(" ") >= 0 ? '"' + valor + '"' : valor))
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
        if (t[i].indexOf(":") < 0 && t[i].charAt(0) !== "#") return true
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

/// `#etiqueta` como se escribe en el buscador: entre comillas si lleva
/// espacios, que si no serían dos palabras.
function comoEtiqueta(nombre) {
    return nombre.indexOf(" ") >= 0 ? '#"' + nombre + '"' : "#" + nombre
}

/// Filtros que se sugieren al escribir: lo que se teclea, y lo que se pone.
var SUGERENCIAS = [
    { claves: ["imagen", "imágenes", "foto", "fotos"], texto: "tipo: imágenes", token: "tipo:imagen", icono: "cuadricula" },
    { claves: ["video", "vídeo", "vídeos", "videos", "clip"], texto: "tipo: vídeos", token: "tipo:video", icono: "video" },
    { claves: ["audio", "sonido", "música", "musica"], texto: "tipo: audio", token: "tipo:audio", icono: "orden" },
    { claves: ["3d", "modelo", "stl", "malla"], texto: "tipo: 3D", token: "tipo:modelo", icono: "forma" },
    { claves: ["pdf", "documento"], texto: "tipo: documentos", token: "tipo:documento", icono: "pegar" },
    { claves: ["fuente", "tipografía", "tipografia", "letra"], texto: "tipo: fuentes", token: "tipo:tipografia", icono: "etiqueta" },
    { claves: ["hoy"], texto: "importado hoy", token: "fecha:hoy", icono: "fecha" },
    { claves: ["ayer"], texto: "importado ayer", token: "fecha:ayer", icono: "fecha" },
    { claves: ["semana", "reciente", "recientes"], texto: "importado esta semana", token: "fecha:7d", icono: "fecha" },
    { claves: ["mes"], texto: "importado este mes", token: "fecha:30d", icono: "fecha" },
    { claves: ["repetidos", "duplicados", "copias"], texto: "solo repetidos", token: "repetidos:si", icono: "repetidos" },
    { claves: ["estrellas", "favoritos", "mejores"], texto: "4 estrellas o más", token: "estrellas:>=4", icono: "estrella" },
    { claves: ["apaisada", "horizontal"], texto: "forma: apaisada", token: "orientacion:apaisada", icono: "forma" },
    { claves: ["vertical", "retrato"], texto: "forma: vertical", token: "orientacion:vertical", icono: "forma" },
    { claves: ["cuadrada"], texto: "forma: cuadrada", token: "orientacion:cuadrada", icono: "forma" },
    { claves: ["papelera", "tirado"], texto: "la papelera", token: "papelera:si", icono: "papelera" },
    { claves: ["+18", "adulto"], texto: "marcado +18", token: "adulto:si", icono: "escudo" }
]
