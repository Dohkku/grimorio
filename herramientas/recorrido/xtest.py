"""Ratón y teclado de mentira para X11, con XTest por ctypes.

Sin dependencias: libX11 y libXtst ya están en cualquier escritorio X y en
Xvfb. Los movimientos van en pasos cortos para que en un vídeo se vea un
ratón que se desplaza, no uno que salta.
"""
import ctypes, ctypes.util, time

_x = ctypes.cdll.LoadLibrary(ctypes.util.find_library("X11") or "libX11.so.6")
_t = ctypes.cdll.LoadLibrary(ctypes.util.find_library("Xtst") or "libXtst.so.6")
_x.XOpenDisplay.restype = ctypes.c_void_p
_x.XOpenDisplay.argtypes = [ctypes.c_char_p]
_x.XStringToKeysym.restype = ctypes.c_ulong
_x.XStringToKeysym.argtypes = [ctypes.c_char_p]
_x.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
_x.XFlush.argtypes = [ctypes.c_void_p]
_t.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ulong]
_t.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
_t.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]

MODS = {"ctrl": "Control_L", "shift": "Shift_L", "alt": "Alt_L"}
# Lo que no es letra ni número se escribe con su keysym, y con Mayús si hace
# falta (teclado estadounidense, el de Xvfb).
SIGNOS = {":": "shift+colon", "#": "shift+numbersign", ">": "shift+greater", "<": "shift+less",
          "=": "equal", ".": "period", "-": "minus", "/": "slash", "_": "shift+underscore",
          " ": "space", ",": "comma", "+": "shift+plus"}


class Raton:
    def __init__(self, pantalla, ritmo=1.0):
        self.d = _x.XOpenDisplay(pantalla.encode())
        if not self.d:
            raise RuntimeError(f"no puedo abrir la pantalla {pantalla}")
        self.ritmo = ritmo
        self.x, self.y = 0, 0

    def _flush(self):
        _x.XFlush(self.d)

    def esperar(self, s):
        time.sleep(s * self.ritmo)

    def mover(self, x, y, pasos=None):
        pasos = pasos or max(1, int(18 * self.ritmo))
        ox, oy = self.x, self.y
        for k in range(1, pasos + 1):
            t = k / pasos
            t = t * t * (3 - 2 * t)  # suave al salir y al llegar
            _t.XTestFakeMotionEvent(self.d, -1, int(ox + (x - ox) * t), int(oy + (y - oy) * t), 0)
            self._flush()
            time.sleep(0.012 * self.ritmo)
        self.x, self.y = x, y

    def boton(self, b, abajo):
        _t.XTestFakeButtonEvent(self.d, b, 1 if abajo else 0, 0)
        self._flush()

    def clic(self, x=None, y=None, b=1, veces=1):
        if x is not None:
            self.mover(x, y)
        for _ in range(veces):
            self.boton(b, True); time.sleep(0.04); self.boton(b, False); time.sleep(0.06)

    def arrastrar(self, x0, y0, x1, y1, pasos=30):
        self.mover(x0, y0)
        self.boton(1, True)
        time.sleep(0.15)
        self.mover(x1, y1, pasos)
        time.sleep(0.15)
        self.boton(1, False)

    def rueda(self, arriba=True, veces=1):
        for _ in range(veces):
            self.clic(b=4 if arriba else 5)
            time.sleep(0.05)

    def _codigo(self, nombre):
        return _x.XKeysymToKeycode(self.d, _x.XStringToKeysym(nombre.encode()))

    def tecla(self, combo):
        partes = combo.split("+")
        mods = [self._codigo(MODS[p.lower()]) for p in partes[:-1]]
        for m in mods:
            _t.XTestFakeKeyEvent(self.d, m, 1, 0)
        k = self._codigo(partes[-1])
        _t.XTestFakeKeyEvent(self.d, k, 1, 0); self._flush(); time.sleep(0.03)
        _t.XTestFakeKeyEvent(self.d, k, 0, 0)
        for m in reversed(mods):
            _t.XTestFakeKeyEvent(self.d, m, 0, 0)
        self._flush()
        time.sleep(0.03)

    def escribir(self, texto, pausa=0.07):
        for c in texto:
            if c in SIGNOS:
                self.tecla(SIGNOS[c])
            elif c.isupper():
                self.tecla("shift+" + c.lower())
            elif c in "áéíóúñ":
                # Sin compositor en Xvfb: los acentos van por su keysym latin1.
                self.tecla({"á": "aacute", "é": "eacute", "í": "iacute", "ó": "oacute",
                            "ú": "uacute", "ñ": "ntilde"}[c])
            else:
                self.tecla(c)
            time.sleep(pausa * self.ritmo)
