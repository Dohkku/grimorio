// Plurales y números.
//
// `qsTr("%n elemento(s)")` solo elige forma si hay una traducción cargada. Sin
// ella, Qt deja el literal tal cual y la interfaz enseña "100000 elemento(s)",
// que es exactamente el detalle que hace que un programa parezca sin terminar.
// Hasta que haya traducciones de verdad, los plurales se escriben aquí.
.pragma library

/// Número con separador de millares del sistema: 100.000, no 100000.
function numero(n) {
    return Number(n).toLocaleString(Qt.locale(), 'f', 0)
}

function elementos(n) {
    return n === 1 ? "1 elemento" : numero(n) + " elementos"
}

function elegidos(n) {
    return n === 1 ? "1 elegido" : numero(n) + " elegidos"
}

function importando(hechos, total) {
    return "importando " + numero(hechos) + " de " + numero(total) + "…"
}

/// Lo que está en marcha, con su nombre: "etiquetar como «rótulo» · 3.000 de 10.000".
function enMarcha(que, hechos, total) {
    var cuenta = numero(hechos) + " de " + numero(total) + "…"
    return que && que.length > 0 ? que + " · " + cuenta : cuenta
}

function posicion(i, n) {
    return numero(i) + " de " + numero(n)
}

function peso(bytes) {
    if (bytes < 1024) return bytes + " B"
    var u = ["kB", "MB", "GB", "TB"]
    var v = bytes / 1024
    var i = 0
    while (v >= 1024 && i < u.length - 1) { v /= 1024; i += 1 }
    // Un decimal hasta 10, ninguno por encima: "9,4 MB" y "148 MB".
    return Number(v).toLocaleString(Qt.locale(), 'f', v < 10 ? 1 : 0) + " " + u[i]
}

function medidas(w, h) {
    if (!w || !h) return "—"
    return numero(w) + " × " + numero(h)
}

/// Una fecha ISO en algo que se lee de un vistazo.
function fecha(iso) {
    if (!iso) return "—"
    var d = new Date(iso)
    if (isNaN(d.getTime())) return iso
    // Formato explícito y no `Locale.ShortFormat`: este archivo es
    // `.pragma library` y ahí el enum `Locale` no existe. El nombre del mes
    // sigue saliendo en el idioma del sistema porque lo pone `Qt.locale()`.
    return d.toLocaleDateString(Qt.locale(), "d MMM yyyy")
}

function tirados(n) {
    return n === 1 ? "1 en la papelera" : numero(n) + " en la papelera"
}

function etiquetados(n) {
    return n === 1 ? "1 etiquetado" : numero(n) + " etiquetados"
}

/// Los números de `GrimFamilia`, repetidos aquí porque QML no ve el enum de C.
var IMAGEN = 0, RAW = 1, VIDEO = 2, AUDIO = 3, DOCUMENTO = 4, TIPOGRAFIA = 5, MODELO = 6

/// Segundos en "1:23" o "1:02:03". Sin ceros a la izquierda en la primera
/// cifra: "0:07" y no "00:07", que es como lo escribe todo el mundo.
function duracion(s) {
    if (!s || s <= 0) return ""
    var h = Math.floor(s / 3600)
    var m = Math.floor((s % 3600) / 60)
    var seg = Math.floor(s % 60)
    var dd = function (n) { return n < 10 ? "0" + n : String(n) }
    return h > 0 ? h + ":" + dd(m) + ":" + dd(seg) : m + ":" + dd(seg)
}

/// Lo que se escribe en la esquina de una celda que no es una foto.
///
/// Una imagen no lleva nada: la mayoría de la biblioteca son imágenes y una
/// malla con una etiqueta en cada celda deja de dejar ver las fotos.
function insignia(familia, ext, segundos) {
    switch (familia) {
    case VIDEO:
    case AUDIO:
        return duracion(segundos) || (familia === VIDEO ? "vídeo" : "audio")
    case DOCUMENTO:
    case TIPOGRAFIA:
    case RAW:
    case MODELO:
        return String(ext || "").toUpperCase()
    default:
        return ""
    }
}

function nombreFamilia(f) {
    switch (f) {
    case VIDEO: return "vídeo"
    case AUDIO: return "audio"
    case DOCUMENTO: return "documento"
    case TIPOGRAFIA: return "tipografía"
    case RAW: return "negativo"
    case MODELO: return "modelo 3D"
    default: return "imagen"
    }
}

/// Lo mismo, desde la clave con que el núcleo escribe la familia ("video",
/// "tipografia"…): así llega en el reparto del pie de la barra lateral.
function nombreFamiliaPorClave(clave) {
    var n = { "imagen": IMAGEN, "raw": RAW, "video": VIDEO, "audio": AUDIO,
              "documento": DOCUMENTO, "tipografia": TIPOGRAFIA, "modelo": MODELO }[clave]
    return nombreFamilia(n === undefined ? IMAGEN : n)
}

/// Segundos con décimas, para cuando se mira a cámara lenta o se va fotograma
/// a fotograma: "0:07.4". Con la duración de al lado, los dos se leen igual.
function tiempoFino(s) {
    if (!(s >= 0)) s = 0
    var m = Math.floor(s / 60)
    var r = s - m * 60
    var seg = Math.floor(r)
    var dec = Math.floor((r - seg) * 10)
    return m + ":" + (seg < 10 ? "0" : "") + seg + "." + dec
}

/// "213,6 × 212 × 120 mm": ancho, fondo y alto de una pieza, sin decimales que
/// no dicen nada.
function medidasMm(m) {
    if (!m || m.length < 3 || m[0] < 0) return ""
    var f = function (x) {
        var t = x >= 100 ? Math.round(x) : Math.round(x * 10) / 10
        return String(t).replace(".", ",")
    }
    return f(m[0]) + " × " + f(m[1]) + " × " + f(m[2]) + " mm"
}

/// Hercios en "440 Hz" o "12,5 kHz".
function hercios(f) {
    if (!(f > 0)) return ""
    if (f < 1000) return Math.round(f) + " Hz"
    return String(Math.round(f / 100) / 10).replace(".", ",") + " kHz"
}
