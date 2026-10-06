// El vídeo que se mueve dentro de la malla: el de debajo del ratón.
//
// Va por encima de las celdas y no dentro de ellas, y esa es la decisión que
// sostiene todo lo demás. Dentro habría que crear un reproductor al entrar el
// ratón en una celda y destruirlo al salir, y montar y desmontar la tubería de
// multimedia muchas veces seguidas tira el programa —está medido, ver
// app/README.md—. Aquí hay **uno** que no se destruye nunca: se le cambia el
// sitio y la fuente, que es barato y no toca esa herida.
//
// Uno y no varios. Hubo un modo que movía todos los que se veían, con un tope
// de ocho, y se quitó midiendo: cada vídeo en marcha es una tubería de
// decodificación entera, y cinco costaban 0,88 GB de memoria y un tercio de un
// núcleo contra los 0,19 GB del programa sin vídeo. En una máquina con
// decodificador por GPU, además, cada tubería arrastra su contexto.
import QtQuick

Item {
    id: videos

    /// Dónde va: `{indice, x, y, ancho, alto}`, o `undefined` si ahora mismo no
    /// hay ninguno. Quien lo rellena decide la política.
    property var sitio: undefined
    Reproductor {
        id: uno
        readonly property var sitio: videos.sitio

        // La fuente se pone, no se ata.
        //
        // Atada, quedarse sin sitio la dejaba en blanco, y eso desmonta la
        // tubería de multimedia; volver a tener sitio la montaba otra vez. Sin
        // sitio se calla y se esconde, pero se queda con lo que tenía: si
        // vuelve a tocarle el mismo vídeo no cuesta nada.
        onSitioChanged: if (sitio !== undefined) {
            idElemento = modelo.idDe(sitio.indice)
            fuente = modelo.urlDe(sitio.indice)
        }

        visible: sitio !== undefined
        x: sitio !== undefined ? sitio.x : 0
        y: sitio !== undefined ? sitio.y : 0
        width: sitio !== undefined ? sitio.ancho : 0
        height: sitio !== undefined ? sitio.alto : 0

        enPantalla: sitio !== undefined
        conMandos: false
        silencio: true
        bucle: true
    }
}
