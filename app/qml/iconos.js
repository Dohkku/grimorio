// Los iconos, como trazos sobre una cuadrícula de 24×24.
//
// Son datos, no estilo: el color, el grosor y el tamaño los pone `Icono.qml`
// desde el tema. Por eso aquí no hay ni un color, y por eso son trazos y no
// imágenes: una imagen trae su color puesto y no cambia con el tema.
//
// `trazos` se dibuja con línea; `rellenos`, con el mismo color de relleno
// (los pocos puntos y triángulos que con línea no se leen).
.pragma library

var trazos = {
    "buscar":      "M10.5 4 A6.5 6.5 0 1 0 10.5 17 A6.5 6.5 0 1 0 10.5 4 M15.3 15.3 L20 20",
    "filtro":      "M4 5 L20 5 L14 12.5 L14 19 L10 20.5 L10 12.5 Z",
    "cuadricula":  "M4 4 H10 V10 H4 Z M14 4 H20 V10 H14 Z M4 14 H10 V20 H4 Z M14 14 H20 V20 H14 Z",
    "justificado": "M3.5 5 H13 V10.5 H3.5 Z M15.5 5 H20.5 V10.5 H15.5 Z M3.5 13.5 H8.5 V19 H3.5 Z M11 13.5 H20.5 V19 H11 Z",
    "video":       "M3 6 H21 V18 H3 Z",
    "ojo":         "M2.5 12 C5 7.5 8.3 5.5 12 5.5 C15.7 5.5 19 7.5 21.5 12 C19 16.5 15.7 18.5 12 18.5 C8.3 18.5 5 16.5 2.5 12 Z M12 9.3 A2.7 2.7 0 1 0 12 14.7 A2.7 2.7 0 1 0 12 9.3",
    "escudo":      "M12 3 L19 6 V11.5 C19 15.8 16.2 19.2 12 21 C7.8 19.2 5 15.8 5 11.5 V6 Z M9 12 L11.2 14.2 L15.2 10",
    "papelera":    "M4 7 H20 M9.5 7 V4.5 H14.5 V7 M6.2 7 L7.2 20 H16.8 L17.8 7 M10.2 11 V16.5 M13.8 11 V16.5",
    "mas":         "M12 5 V19 M5 12 H19",
    "menos":       "M5 12 H19",
    "importar":    "M12 3.5 V14.5 M7.5 10 L12 14.5 L16.5 10 M4 15.5 V20 H20 V15.5",
    "captura":     "M4 9 V4 H9 M15 4 H20 V9 M20 15 V20 H15 M9 20 H4 V15",
    "pegar":       "M8.5 5 H6 V21 H18 V5 H15.5 M9 3 H15 V7 H9 Z",
    "carpeta":     "M3 6 H9.5 L11.5 8 H21 V19 H3 Z",
    "vincular":    "M10 14 L14 10 M8.5 11.5 L6.6 13.4 A3.1 3.1 0 0 0 10.6 17.4 L12.5 15.5 M15.5 12.5 L17.4 10.6 A3.1 3.1 0 0 0 13.4 6.6 L11.5 8.5",
    "nodos":       "M6 9.5 A2.5 2.5 0 1 0 6 14.5 A2.5 2.5 0 1 0 6 9.5 M18 3.5 A2.5 2.5 0 1 0 18 8.5 A2.5 2.5 0 1 0 18 3.5 M18 15.5 A2.5 2.5 0 1 0 18 20.5 A2.5 2.5 0 1 0 18 15.5 M8.3 11 L15.7 7 M8.3 13 L15.7 17",
    "etiqueta":    "M3.5 12.2 V4 H11.7 L20.5 12.8 L12.8 20.5 Z",
    "inteligente": "M13.5 3 L5.5 13.5 H11.5 L10.5 21 L18.5 10.5 H12.5 Z",
    "dinamica":    "M3 6 H9.5 L11.5 8 H21 V19 H3 Z M13.2 10 L10.4 14 H12.6 L11.8 17 L14.6 13 H12.4 Z",
    "color":       "M12 3.5 C7.3 3.5 3.5 7.1 3.5 11.7 C3.5 16.6 7.4 20.5 12 20.5 C13.2 20.5 13.8 19.6 13.4 18.6 C13 17.6 13.6 16.5 14.8 16.5 H16.5 C18.8 16.5 20.5 14.8 20.5 12.5 C20.5 7.5 16.7 3.5 12 3.5 Z",
    "fecha":       "M4 6 H20 V20 H4 Z M4 10.5 H20 M8.5 3.5 V7.5 M15.5 3.5 V7.5",
    "estrella":    "M12 3.8 L14.5 9 L20.2 9.7 L16 13.6 L17.1 19.3 L12 16.5 L6.9 19.3 L8 13.6 L3.8 9.7 L9.5 9 Z",
    "forma":       "M3.5 7 H14 V17 H3.5 Z M17 9 H20.5 V15 H17 Z",
    "orden":       "M7 4 V20 M4 16.8 L7 19.8 L10 16.8 M17 20 V4 M14 7.2 L17 4.2 L20 7.2",
    "repetidos":   "M8.5 8.5 H20 V20 H8.5 Z M4 15.5 V4 H15.5",
    "cerrar":      "M6 6 L18 18 M18 6 L6 18",
    "lapiz":       "M4 20 H8 L19.5 8.5 L15.5 4.5 L4 16 Z M13.5 6.5 L17.5 10.5",
    "vigilar":     "M12 4 A8 8 0 1 0 12 20 A8 8 0 1 0 12 4",
    "todo":        "M4 4 H20 V20 H4 Z M4 9.3 H20 M4 14.7 H20 M9.3 4 V20 M14.7 4 V20",
    "abajo":       "M6 9.5 L12 15.5 L18 9.5",
    "derecha":     "M9.5 6 L15.5 12 L9.5 18",
    "lista":       "M8.5 6 H20.5 M8.5 12 H20.5 M8.5 18 H20.5 M3.5 6 H5 M3.5 12 H5 M3.5 18 H5",
    "reloj":       "M12 3.5 A8.5 8.5 0 1 0 12 20.5 A8.5 8.5 0 1 0 12 3.5 M12 7.5 V12 L15 14",
    "panel":       "M3.5 5 H20.5 V19 H3.5 Z M14.5 5 V19",
    "ajustes":     "M4 7 H14 M18 7 H20 M4 17 H6 M10 17 H20 M16 4.5 V9.5 M8 14.5 V19.5",
    "biblioteca":  "M4 4.5 H8 V19.5 H4 Z M10 4.5 H14 V19.5 H10 Z M16.2 5.3 L19.8 4.4 L22.3 18.7 L18.7 19.6 Z",
    "copiar":      "M8.5 8.5 H20 V20 H8.5 Z M4 15.5 V4 H15.5",
    "exportar":    "M12 14.5 V3.5 M7.5 8 L12 3.5 L16.5 8 M4 15.5 V20 H20 V15.5",
    "externo":     "M13.5 4 H20 V10.5 M20 4 L11 13 M17 14 V20 H4 V7 H10",
    "salir":       "M3 6 H9.5 L11.5 8 H21 V19 H3 Z M9 13.5 H15",
    "engranaje":   "M12 9 A3 3 0 1 0 12 15 A3 3 0 1 0 12 9 M12 2.8 V5.3 M12 18.7 V21.2 M2.8 12 H5.3 M18.7 12 H21.2 M5.5 5.5 L7.3 7.3 M16.7 16.7 L18.5 18.5 M5.5 18.5 L7.3 16.7 M16.7 7.3 L18.5 5.5 M12 5.3 A6.7 6.7 0 1 0 12 18.7 A6.7 6.7 0 1 0 12 5.3",
    "cubo":        "M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z M4 7.5 L12 12 L20 7.5 M12 12 V21",
    "alambre":     "M12 3.5 A8.5 8.5 0 1 0 12 20.5 A8.5 8.5 0 1 0 12 3.5 M3.5 12 H20.5 M12 3.5 C8.2 6.5 8.2 17.5 12 20.5 C15.8 17.5 15.8 6.5 12 3.5",
    "girarX":      "M18.6 9.5 A7 7 0 1 0 19 13.5 M19 4.8 V9.5 H14.3",
    "girarZ":      "M19.5 10.2 A8 3.6 0 1 1 15.2 8.4 M15.2 8.4 L12.6 6.2 M15.2 8.4 L12.8 10.9 M12 3.5 V20.5",
    "restablecer": "M9 5.5 L5 9.5 L9 13.5 M5 9.5 H14.5 A4.75 4.75 0 0 1 14.5 19 H10",
    "info":        "M12 3.5 A8.5 8.5 0 1 0 12 20.5 A8.5 8.5 0 1 0 12 3.5 M12 11 V16.5"
}

var rellenos = {
    "video":    "M10 9.2 L15.2 12 L10 14.8 Z",
    "etiqueta": "M8 6.3 A1.7 1.7 0 1 0 8 9.7 A1.7 1.7 0 1 0 8 6.3",
    "vigilar":  "M12 9 A3 3 0 1 0 12 15 A3 3 0 1 0 12 9",
    "info":     "M12 6.9 A1.1 1.1 0 1 0 12 9.1 A1.1 1.1 0 1 0 12 6.9"
}

function trazo(nombre) { return trazos[nombre] || "" }
function relleno(nombre) { return rellenos[nombre] || "" }
