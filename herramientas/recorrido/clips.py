#!/usr/bin/env python3
"""Clips cortos de cada función, para la web: la app de verdad en la pantalla
virtual del recorrido, manejada con ratón y teclado de mentira, grabada
escena por escena.

    python3 herramientas/recorrido/clips.py BIBLIOTECA SALIDA/ [--app app/build/grimorio]
                                            [--solo escena,escena] [--ritmo 1.3]

Cada escena deja `SALIDA/<escena>.mp4` y `.webm` a 1200 px de ancho, sin
sonido y pensados para ir en bucle, y un `.jpg` con el primer fotograma para
usarlo de póster. La biblioteca se copia: no se toca la original.

Las coordenadas son las de la ventana de 1600×1000 con la biblioteca de
demostración (`herramientas/demo/generar.py` más un vídeo, un sonido y un
PDF); con otra biblioteca las celdas caen en otro sitio.
"""
import argparse, os, subprocess, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import recorrido  # noqa: E402
from recorrido import ANCHO, ALTO, PANTALLA, Recorrido  # noqa: E402
from casos import con_tecla  # noqa: E402

BUSCADOR = (710, 21)
ENGRANAJE = (1278, 21)
TODO = (50, 64)
C = {
    "automata": (415, 160), "fractal": (752, 160), "sonido": (1107, 160),
    "pdf": (322, 357), "stl": (485, 357), "circulos": (744, 357),
    "paisaje": (490, 538), "norte": (791, 538), "forma": (917, 538), "ritmo": (1130, 538),
    "hueco": (315, 712), "eco": (608, 712),
}
PORTADAS = (72, 500)


class Grabador:
    def __init__(self, rec, salida):
        self.rec, self.salida, self.proc = rec, salida, None

    def empezar(self, nombre):
        self.nombre = nombre
        self.crudo = os.path.join(self.rec.tmp, nombre + ".crudo.mp4")
        self.proc = subprocess.Popen(
            ["ffmpeg", "-v", "error", "-y", "-f", "x11grab", "-video_size", f"{ANCHO}x{ALTO}",
             "-framerate", "30", "-draw_mouse", "1", "-i", PANTALLA,
             "-c:v", "libx264", "-preset", "ultrafast", "-crf", "12", "-pix_fmt", "yuv420p", self.crudo],
            stdin=subprocess.PIPE, env=self.rec.env)
        time.sleep(0.6)

    def acabar(self):
        time.sleep(0.4)
        self.proc.communicate(b"q", timeout=60)
        base = os.path.join(self.salida, self.nombre)
        escala = "fps=30,scale=1200:-2:flags=lanczos"
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", self.crudo, "-vf", escala, "-an",
                        "-c:v", "libx264", "-preset", "slow", "-crf", "24", "-pix_fmt", "yuv420p",
                        "-movflags", "+faststart", base + ".mp4"], check=True)
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", self.crudo, "-vf", escala, "-an",
                        "-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "36", "-row-mt", "1", base + ".webm"],
                       check=True)
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", "0.8", "-i", self.crudo, "-frames:v", "1",
                        "-vf", "scale=1200:-2", "-q:v", "3", base + ".jpg"], check=True)
        os.remove(self.crudo)
        print(f"  ✓ {self.nombre}  {os.path.getsize(base + '.webm') // 1024} KB")


# ------------------------------------------------------------------ escenas
# Cada una empieza y acaba con la malla en reposo y nada elegido, para que
# las escenas no dependan unas de otras.

def malla(rec, r, g):
    g.empezar("malla")
    r.mover(*C["fractal"]); r.esperar(2.4)          # el vídeo se mueve al pasar
    r.mover(*C["sonido"]); r.esperar(0.8)
    r.clic(*C["norte"]); r.esperar(1.4)             # el panel de detalle
    r.clic(*C["ritmo"]); r.esperar(1.2)
    r.mover(780, 700); r.rueda(False, 6); r.esperar(1.0)
    r.rueda(True, 6); r.esperar(0.8)
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def organizar(rec, r, g):
    g.empezar("organizar")
    r.clic(*C["norte"]); r.esperar(0.6)
    r.tecla("5"); r.esperar(0.7)
    r.mover(*C["forma"]); con_tecla(r, "ctrl", r.clic); r.esperar(0.5)
    r.mover(*C["ritmo"]); con_tecla(r, "ctrl", r.clic); r.esperar(0.8)
    r.tecla("ctrl+k"); r.esperar(0.5)
    r.escribir("portada", 0.09); r.esperar(0.5)
    r.tecla("Return"); r.esperar(0.9)
    r.tecla("Escape"); r.esperar(0.3)
    r.arrastrar(*C["ritmo"], *PORTADAS, pasos=45); r.esperar(1.2)
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def buscar(rec, r, g):
    g.empezar("buscar")
    r.clic(*BUSCADOR); r.esperar(0.4)
    # Con «#» solo, el ayudante enseña todas las etiquetas; se elige de la
    # lista. Escribirla letra a letra deja la malla vacía hasta completarla.
    r.escribir("#", 0.1); r.esperar(1.4)
    r.tecla("Down"); r.esperar(0.5); r.tecla("Down"); r.esperar(0.5)
    r.tecla("Return"); r.esperar(1.2)
    # De seguido y más rápido que el rebote del buscador (140 ms): a medio
    # escribir, «estr» es texto libre y la malla se queda vacía un momento.
    r.escribir(" estrellas:>=4", 0.008)
    r.esperar(0.5); r.tecla("Return"); r.esperar(1.6)
    r.clic(1100, 21); r.esperar(1.6)                # la fila de filtros
    r.clic(1100, 21); r.esperar(0.5)
    r.clic(*BUSCADOR); r.tecla("ctrl+a"); r.tecla("BackSpace"); r.tecla("Return"); r.esperar(0.6)
    r.clic(*TODO); r.esperar(0.6)
    g.acabar()


def color(rec, r, g):
    g.empezar("color")
    r.clic(*C["paisaje"]); r.esperar(1.0)
    r.clic(1365, 510); r.esperar(1.8)               # una muestra de la paleta
    r.mover(780, 600); r.rueda(False, 3); r.esperar(0.8); r.rueda(True, 3)
    r.esperar(0.6)
    r.clic(*TODO); r.esperar(0.6); r.tecla("Escape"); r.esperar(0.4)
    g.acabar()


def visor(rec, r, g):
    g.empezar("visor")
    r.clic(*C["ritmo"], veces=2); r.esperar(1.4)
    r.mover(820, 470)
    r.rueda(True, 8); r.esperar(1.0)
    r.mover(900, 520, 30); r.esperar(0.6)
    r.rueda(True, 10); r.esperar(1.2)
    r.tecla("c"); r.esperar(1.0)                    # el cuentagotas
    r.tecla("z"); r.esperar(0.8)
    r.tecla("Right"); r.esperar(1.0); r.tecla("Right"); r.esperar(1.0)
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def video(rec, r, g):
    g.empezar("video")
    r.mover(*C["fractal"]); r.esperar(2.0)
    r.clic(*C["fractal"], veces=2); r.esperar(3.2)
    r.tecla("bracketright"); r.esperar(1.2)         # más rápido
    r.tecla("comma"); r.esperar(0.6); r.tecla("period"); r.esperar(0.6)
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def sonido(rec, r, g):
    g.empezar("sonido")
    r.clic(*C["sonido"], veces=2); r.esperar(4.0)
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def pdf(rec, r, g):
    g.empezar("pdf")
    r.clic(*C["pdf"], veces=2); r.esperar(2.4)
    for _ in range(3):
        r.tecla("Next"); r.esperar(1.1)
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def modelo3d(rec, r, g):
    g.empezar("modelo-3d")
    r.clic(*C["stl"], veces=2); r.esperar(2.6)
    r.arrastrar(760, 520, 980, 450, pasos=50); r.esperar(0.8)
    r.arrastrar(980, 450, 700, 560, pasos=50); r.esperar(0.8)
    r.rueda(True, 3); r.esperar(0.8)
    r.clic(*rec.gizmo_x); r.esperar(1.2)            # mirar desde X
    r.clic(*rec.gizmo_y); r.esperar(1.2)            # desde arriba
    r.tecla("Escape"); r.esperar(0.6)
    g.acabar()


def temas(rec, r, g):
    g.empezar("temas")
    r.clic(*ENGRANAJE); r.esperar(1.0)
    r.clic(1099, 292); r.esperar(1.6)               # papel
    r.tecla("Escape"); r.esperar(1.6)
    r.clic(*ENGRANAJE); r.esperar(0.8)
    r.clic(1040, 292); r.esperar(1.2)               # noche
    r.tecla("Escape"); r.esperar(0.8)
    g.acabar()


ESCENAS = [malla, organizar, buscar, color, visor, video, sonido, pdf, modelo3d, temas]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("biblioteca")
    ap.add_argument("salida")
    ap.add_argument("--app", default="app/build/grimorio")
    ap.add_argument("--solo", default="")
    ap.add_argument("--ritmo", type=float, default=1.3)
    ap.add_argument("--gizmo", default="1545,98:1517,70",
                    help="centro de las bolas X e Y de los ejes del visor 3D")
    a = ap.parse_args()
    os.makedirs(a.salida, exist_ok=True)
    opciones = argparse.Namespace(biblioteca=a.biblioteca, app=os.path.abspath(a.app), grabar=None,
                                  capturas=os.path.join(a.salida, "_tomas"), ritmo=a.ritmo,
                                  solo_arrancar=True, conservar=False)
    rec = Recorrido(opciones)
    x, y = a.gizmo.split(":")
    rec.gizmo_x = tuple(int(v) for v in x.split(","))
    rec.gizmo_y = tuple(int(v) for v in y.split(","))
    solo = set(filter(None, a.solo.split(",")))
    try:
        rec.arrancar()
        g = Grabador(rec, a.salida)
        for e in ESCENAS:
            nombre = e.__name__
            if solo and nombre not in solo:
                continue
            e(rec, rec.r, g)
            if rec.app.poll() is not None:
                print("la aplicación se cerró en", nombre)
                break
    finally:
        rec.parar()


if __name__ == "__main__":
    main()
