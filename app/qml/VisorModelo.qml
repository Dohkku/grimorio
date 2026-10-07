// Un modelo 3D que se puede girar.
//
//   arrastrar                 girar
//   botón derecho o central   desplazar
//   rueda                     acercar y alejar
//   doble clic                volver a la vista de partida
//   los ejes de arriba        mirar desde X, Y o Z (clic en una bola)
//
// Los mandos van en una columna de iconos bajo los ejes, como en Blender, y
// no en una fila de palabras abajo: «frente», «arriba» y «lado» ya son los
// ejes, y lo demás se explica en su pista.
//
// Un modelo que llega tumbado o boca abajo —Z arriba, o exportado con los
// ejes de otro programa— se endereza a cuartos de vuelta. La corrección se
// recuerda por elemento en los ajustes, no toca el archivo.
//
// La malla la prepara el núcleo la primera vez (`.malla` en la caché): STL,
// OBJ, PLY, glTF y 3MF los lee él; `.blend` y `.fbx` los abre Blender sin
// ventana. Arranca con el mismo ángulo que la miniatura, así abrir no es un
// salto: se ve lo que se estaba viendo, y luego se puede girar.
import QtQuick
import Grimorio
import "textos.js" as Textos

Item {
    id: modelo3d
    property string idElemento: ""
    property url original
    property string ext: ""
    property string clave: ""
    property string error: ""
    property var medidas: null
    property bool cargando: false

    readonly property real giroInicial: -35
    readonly property real inclinacionInicial: 25

    function vista(giro, inclinacion) {
        // Por el camino corto: de 170 a -170 son veinte grados, no
        // trescientos cuarenta.
        let d = ((giro - visor.giro) % 360 + 540) % 360 - 180
        animGiro.to = visor.giro + d
        animIncl.to = inclinacion
        animZoom.to = 1
        visor.panX = 0
        visor.panY = 0
        anim.restart()
    }
    function reiniciar() { vista(giroInicial, inclinacionInicial) }

    /// Un cuarto de vuelta más a la pieza, alrededor de X o de Z.
    function enderezar(eje) {
        if (eje === "x") visor.vueltasX = (visor.vueltasX + 1) % 4
        else visor.vueltasZ = (visor.vueltasZ + 1) % 4
        guardarOrientacion()
    }
    function quitarCorreccion() {
        visor.vueltasX = 0
        visor.vueltasZ = 0
        guardarOrientacion()
    }
    function guardarOrientacion() {
        if (idElemento.length > 0)
            ajustes.ponerOrientacion3d(idElemento, visor.vueltasX + 4 * visor.vueltasZ)
    }
    function girarArrastrando(dx, dy) {
        anim.stop()
        visor.giro += dx * 0.4
        visor.inclinacion = Math.max(-90, Math.min(90, visor.inclinacion + dy * 0.4))
    }

    onIdElementoChanged: {
        error = ""
        medidas = null
        visor.archivo = ""
        visor.giro = giroInicial
        visor.inclinacion = inclinacionInicial
        visor.acercar = 1
        visor.panX = 0
        visor.panY = 0
        const o = idElemento.length > 0 ? ajustes.orientacion3d(idElemento) : 0
        visor.vueltasX = o % 4
        visor.vueltasZ = Math.floor(o / 4) % 4
        if (idElemento.length > 0) {
            cargando = true
            clave = derivados.pedir("malla", idElemento, original, { ext: ext })
        }
    }

    Connections {
        target: derivados
        function onListo(clave, r) {
            if (clave !== modelo3d.clave) return
            modelo3d.cargando = false
            if (r.ok) {
                modelo3d.medidas = r.medidas
                visor.archivo = r.ruta
            } else {
                modelo3d.error = r.error
            }
        }
    }

    Visor3D {
        id: visor
        anchors.fill: parent
        colorFondo: tema.fondo
        colorPieza: tema.pieza3d
        colorAlambre: tema.seleccion
    }

    ParallelAnimation {
        id: anim
        NumberAnimation { id: animGiro; target: visor; property: "giro"; duration: 220; easing.type: Easing.OutCubic }
        NumberAnimation { id: animIncl; target: visor; property: "inclinacion"; duration: 220; easing.type: Easing.OutCubic }
        NumberAnimation { id: animZoom; target: visor; property: "acercar"; duration: 220; easing.type: Easing.OutCubic }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        property point antes
        onPressed: function (e) { antes = Qt.point(e.x, e.y); anim.stop() }
        onPositionChanged: function (e) {
            const dx = e.x - antes.x, dy = e.y - antes.y
            antes = Qt.point(e.x, e.y)
            if (pressedButtons & Qt.LeftButton) {
                modelo3d.girarArrastrando(dx, dy)
            } else {
                // Desplazar en unidades del modelo: lo que mide la pantalla
                // depende de cuánto se haya acercado.
                const k = 2.3 / Math.max(1, Math.min(width, height)) / visor.acercar
                visor.panX += dx * k
                visor.panY -= dy * k
            }
        }
        onDoubleClicked: modelo3d.reiniciar()
        onWheel: function (r) {
            visor.acercar = Math.max(0.2, Math.min(40, visor.acercar * (r.angleDelta.y > 0 ? 1.15 : 1 / 1.15)))
        }
    }

    // Lo que importa al imprimir: cuánto mide. Arriba a la izquierda.
    Column {
        anchors { left: parent.left; top: parent.top; margins: tema.hueco }
        spacing: 2
        Text {
            text: Textos.medidasMm(modelo3d.medidas)
            visible: text.length > 0
            color: tema.texto
            font.pixelSize: tema.fuente * 1.2
        }
        Text {
            text: visor.triangulos > 0 ? qsTr("%1 triángulos").arg(Textos.numero(visor.triangulos)) : ""
            visible: text.length > 0
            color: tema.textoTenue
            font.pixelSize: tema.fuente * 0.9
        }
    }

    // Arriba a la derecha: los ejes, y debajo los mandos en columna.
    EjesVista {
        id: ejes
        anchors { right: parent.right; top: parent.top; margins: tema.hueco }
        giro: visor.giro
        inclinacion: visor.inclinacion
        onElegido: function (g, i) { modelo3d.vista(g, i) }
        onArrastrado: function (dx, dy) { modelo3d.girarArrastrando(dx, dy) }
    }

    Rectangle {
        id: mandos
        anchors { horizontalCenter: ejes.horizontalCenter; top: ejes.bottom; topMargin: tema.hueco * 0.5 }
        width: columna.width + tema.hueco * 0.6
        height: columna.height + tema.hueco * 0.6
        radius: tema.radio * 1.5
        color: tema.panel
        border.color: tema.borde
        border.width: 1
        opacity: 0.92

        Column {
            id: columna
            anchors.centerIn: parent
            spacing: tema.hueco * 0.2

            BotonIcono {
                centrado: false
                icono: "cubo"
                pista: qsTr("vista de partida (doble clic)")
                onPulsado: modelo3d.reiniciar()
            }
            BotonIcono {
                centrado: false
                icono: "alambre"
                activo: visor.alambre
                pista: visor.alambre ? qsTr("quitar las aristas") : qsTr("ver las aristas")
                onPulsado: visor.alambre = !visor.alambre
            }

            Rectangle { width: parent.width; height: 1; color: tema.borde }

            BotonIcono {
                centrado: false
                icono: "girarX"
                pista: qsTr("enderezar: un cuarto de vuelta hacia delante")
                onPulsado: modelo3d.enderezar("x")
            }
            BotonIcono {
                centrado: false
                icono: "girarZ"
                pista: qsTr("enderezar: un cuarto de vuelta de lado")
                onPulsado: modelo3d.enderezar("z")
            }
            BotonIcono {
                centrado: false
                visible: visor.vueltasX !== 0 || visor.vueltasZ !== 0
                icono: "restablecer"
                activo: true
                pista: qsTr("quitar la corrección (como viene en el archivo)")
                onPulsado: modelo3d.quitarCorreccion()
            }

            Rectangle { width: parent.width; height: 1; color: tema.borde }

            BotonIcono {
                centrado: false
                icono: "externo"
                pista: qsTr("abrir fuera")
                onPulsado: nucleo.abrirFuera(modelo3d.idElemento)
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: modelo3d.cargando || modelo3d.error.length > 0
        width: parent.width * 0.7
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        text: modelo3d.error.length > 0 ? qsTr("no pude abrir el modelo: %1").arg(modelo3d.error)
              : (modelo3d.ext === "blend" || modelo3d.ext === "fbx")
                ? qsTr("abriendo con Blender…") : qsTr("cargando el modelo…")
        color: tema.textoTenue
        font.pixelSize: tema.fuente
    }
}
