// Un botón cuadrado de un solo signo —«+», «×»— con su pista al quedarse
// encima. Discreto: sin borde hasta que se pasa por él, porque va dentro de
// otras cosas (la cabecera de las carpetas, una fila) y no tiene que pesar.
import QtQuick

Boton {
    id: icono
    discreto: true
    altura: Math.round(tema.fuente * 1.8)
    width: altura
}
