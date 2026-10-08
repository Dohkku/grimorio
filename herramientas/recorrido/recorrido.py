#!/usr/bin/env python3
"""Pruebas de usuario de verdad: la aplicación entera, manejada con ratón y
teclado de mentira en una pantalla virtual, y comprobando en disco que cada
gesto hizo lo que tenía que hacer.

    python3 herramientas/recorrido/recorrido.py BIBLIOTECA [--app app/build/grimorio]
                                                [--grabar video.mp4] [--ritmo 1.0]
                                                [--capturas carpeta/]

- Trabaja sobre una **copia** de la biblioteca y con una configuración propia
  (XDG_CONFIG_HOME temporal): no toca los ajustes ni la biblioteca de nadie.
- Pantalla virtual con Xvfb de 1600×1000; sin gestor de ventanas, la ventana
  ocupa la pantalla entera desde (0, 0) y las coordenadas son fijas.
- Con --grabar, ffmpeg graba la pantalla mientras tanto: el mismo recorrido
  sirve de prueba y de vídeo de presentación (con --ritmo 1.6, más calmado).

Sale con código 1 si algún caso falla, y dice cuál.
"""
import argparse, glob, json, os, shutil, signal, subprocess, sys, tempfile, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from xtest import Raton  # noqa: E402

ANCHO, ALTO = 1600, 1000
PANTALLA = ":97"


def items(biblio):
    out = {}
    for p in glob.glob(os.path.join(biblio, "items", "**", "item.json"), recursive=True):
        with open(p) as f:
            it = json.load(f)
        out[it["id"]] = it
    return out


class Recorrido:
    def __init__(self, a):
        self.a = a
        self.tmp = tempfile.mkdtemp(prefix="grimorio-recorrido-")
        self.biblio = os.path.join(self.tmp, "biblioteca.grimorio")
        shutil.copytree(a.biblioteca, self.biblio, ignore=shutil.ignore_patterns("abierta.lock"))
        self.fallos, self.casos = [], 0
        self.procs = []

    # ------------------------------------------------------------ arranque
    def arrancar(self):
        xvfb = subprocess.Popen(["Xvfb", PANTALLA, "-screen", "0", f"{ANCHO}x{ALTO}x24", "-nolisten", "tcp"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.procs.append(xvfb)
        time.sleep(1.0)
        env = dict(os.environ, DISPLAY=PANTALLA, XDG_CONFIG_HOME=os.path.join(self.tmp, "config"), GRIMORIO_SIN_RED="1",
                   # Sin bus de sesión: nada de portales ni diálogos que se abran fuera.
                   DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent", QT_QPA_PLATFORM="xcb")
        env.pop("WAYLAND_DISPLAY", None)
        self.env = env
        if self.a.grabar:
            self.grab = subprocess.Popen(
                ["ffmpeg", "-v", "error", "-y", "-f", "x11grab", "-video_size", f"{ANCHO}x{ALTO}",
                 "-framerate", "30", "-draw_mouse", "1", "-i", PANTALLA,
                 "-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-pix_fmt", "yuv420p", self.a.grabar],
                stdin=subprocess.PIPE, env=env)
            self.t0 = time.time()
        self.marcas = []
        self.app = subprocess.Popen([self.a.app, self.biblio, "--ancho", str(ANCHO), "--alto", str(ALTO)],
                                    env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        self.procs.append(self.app)
        self.r = Raton(PANTALLA, self.a.ritmo)
        self.r.mover(ANCHO // 2, ALTO // 2, 1)
        time.sleep(4)

    def parar(self):
        if self.a.grabar and getattr(self, "grab", None):
            self.grab.communicate(b"q", timeout=30)
            # Cuándo empieza cada caso en el vídeo: para ponerle rótulos o
            # cortarlo por partes sin tener que buscar a ojo.
            with open(os.path.splitext(self.a.grabar)[0] + ".marcas.json", "w") as f:
                json.dump(self.marcas, f, ensure_ascii=False, indent=1)
        for p in reversed(self.procs):
            if p.poll() is None:
                p.send_signal(signal.SIGTERM)
                try:
                    p.wait(5)
                except subprocess.TimeoutExpired:
                    p.kill()
        salida = self.app.stdout.read() if self.app.stdout else ""
        avisos = [l for l in salida.splitlines() if "qrc:/" in l and ("Error" in l or "TypeError" in l or "ReferenceError" in l)]
        if avisos:
            self.fallo("consola de QML sin errores", "\n".join(avisos[:8]))
        if not self.a.conservar:
            shutil.rmtree(self.tmp, ignore_errors=True)

    def toma(self, nombre):
        if not self.a.capturas:
            return
        os.makedirs(self.a.capturas, exist_ok=True)
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "x11grab", "-video_size", f"{ANCHO}x{ALTO}",
                        "-i", PANTALLA, "-frames:v", "1", os.path.join(self.a.capturas, nombre + ".png")],
                       env=self.env)

    # ------------------------------------------------------------ casos
    def caso(self, nombre, ok, detalle=""):
        self.casos += 1
        print(("  ✓ " if ok else "  ✗ ") + nombre + ("" if ok else f" — {detalle}"))
        if not ok:
            self.fallos.append(nombre)

    def fallo(self, nombre, detalle):
        self.caso(nombre, False, detalle)

    def esperar_que(self, cond, segundos=5):
        fin = time.time() + segundos
        while time.time() < fin:
            if cond():
                return True
            time.sleep(0.2)
        return cond()

    def recorrer(self):
        r = self.r
        self.toma("00-arranque")
        self.caso("la ventana arranca y sigue viva", self.app.poll() is None)
        if self.a.solo_arrancar:
            return
        from casos import CASOS
        for c in CASOS:
            if self.a.grabar:
                self.marcas.append({"caso": c.__name__, "t": round(time.time() - self.t0, 2)})
            try:
                c(self, r)
            except Exception as e:  # un caso roto no tapa los demás
                self.fallo(c.__name__, repr(e))
            if self.app.poll() is not None:
                self.fallo("la aplicación sigue viva tras " + c.__name__, f"salió con {self.app.returncode}")
                break


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("biblioteca")
    ap.add_argument("--app", default="app/build/grimorio")
    ap.add_argument("--grabar")
    ap.add_argument("--capturas")
    ap.add_argument("--ritmo", type=float, default=1.0)
    ap.add_argument("--solo-arrancar", action="store_true")
    ap.add_argument("--conservar", action="store_true", help="no borrar la copia temporal")
    a = ap.parse_args()
    a.app = os.path.abspath(a.app)
    rec = Recorrido(a)
    try:
        rec.arrancar()
        rec.recorrer()
    finally:
        rec.parar()
    print(f"\n{rec.casos - len(rec.fallos)}/{rec.casos} casos bien")
    sys.exit(1 if rec.fallos else 0)


if __name__ == "__main__":
    main()
