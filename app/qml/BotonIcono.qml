// Un botón cuadrado de un solo signo —«+», «×»— con su pista al quedarse
// encima. Discreto: sin borde hasta que se pasa por él, porque va dentro de
// otras cosas (la cabecera de las carpetas, una fila) y no tiene que pesar.
import QtQuick

Boton {
    id: icono
    /// Lo que hace, dicho con palabras. Un «+» suelto no dice qué añade.
    property string pista: ""

    discreto: true
    altura: Math.round(tema.fuente * 1.8)
    width: altura

    Timer {
        id: espera
        interval: 500
        running: icono.encima && icono.pista.length > 0
    }

    // Pulsado, la pista sobra —ya se sabe qué hace— y además se metía debajo
    // del menú que el botón abre. Vuelve al salir y entrar otra vez.
    property bool pistaUsada: false
    onEncimaChanged: if (!encima) pistaUsada = false
    Connections {
        target: icono
        function onPulsado() { icono.pistaUsada = true }
    }

    // La pista se pinta en la capa de pistas de la ventana, no aquí: dentro
    // del botón la tapaba la malla. Ver `Pista.qml`.
    readonly property bool conPista: icono.encima && !espera.running
                                      && icono.pista.length > 0 && !icono.pistaUsada
    onConPistaChanged: conPista ? ventana.pistas.mostrar(icono, icono.pista)
                                : ventana.pistas.ocultar(icono)
    onPistaChanged: if (conPista) ventana.pistas.mostrar(icono, icono.pista)
    Component.onDestruction: ventana.pistas.ocultar(icono)
}
