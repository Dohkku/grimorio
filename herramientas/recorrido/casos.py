"""Los casos de uso del recorrido, en orden.

Cada uno hace lo que haría una persona —con el ratón y el teclado— y después
mira en disco si pasó: el `item.json` que cambió, la carpeta dinámica en
`busquedas.json`, el ajuste en `Grimorio.conf`. Mirar la pantalla diría que se
ve bien; mirar el disco dice que funciona.

Las coordenadas son las de la ventana a 1600×1000 con el tema y los anchos de
fábrica (ver `recorrido.py`).
"""
import json, os, time

from recorrido import items

# Sitios fijos de la ventana de fábrica.
BUSCADOR = (710, 21)
ENGRANAJE = (1278, 21)
TODO = (50, 64)
MAS_DINAMICAS = (222, 190)
CELDA = {"a": (610, 153), "b": (1060, 153), "c": (1220, 153), "d": (385, 347), "e": (870, 347)}
FILA_LATERAL = 31


def carpeta_paletas(rec):
    """«Paletas» baja una fila por cada carpeta dinámica de más: en la demo
    vienen tres, y el recorrido guarda otra."""
    with open(os.path.join(rec.biblio, "busquedas.json")) as f:
        n = len(json.load(f).get("busquedas", []))
    return (52, 532 + FILA_LATERAL * (n - 3))


def con_tecla(r, tecla, accion):
    """Hace `accion` con una tecla modificadora apretada (Ctrl, Mayús)."""
    import xtest
    k = r._codigo(xtest.MODS[tecla])
    xtest._t.XTestFakeKeyEvent(r.d, k, 1, 0); r._flush(); r.esperar(0.1)
    accion()
    xtest._t.XTestFakeKeyEvent(r.d, k, 0, 0); r._flush()


def cambios(antes, despues):
    """{id: {campo: (antes, después)}} de lo que cambió entre dos fotos."""
    out = {}
    for i, b in despues.items():
        a = antes.get(i, {})
        d = {k: (a.get(k), b.get(k)) for k in set(a) | set(b)
             if a.get(k) != b.get(k) and k not in ("modifiedAt",)}
        if d:
            out[i] = d
    return out


def foto(rec):
    return items(rec.biblio)


def estrellas_con_el_teclado(rec, r):
    antes = foto(rec)
    r.clic(*CELDA["a"])
    r.esperar(0.6)
    r.tecla("5")
    ok = rec.esperar_que(lambda: any(c.get("stars", (0, 0))[1] == 5 for c in cambios(antes, foto(rec)).values()))
    rec.toma("01-cinco-estrellas")
    rec.caso("elegir una imagen y ponerle 5 estrellas con la tecla 5", ok)


def etiquetar_con_ctrl_k(rec, r):
    antes = foto(rec)
    r.tecla("ctrl+k")
    r.esperar(0.5)
    r.escribir("favorita")
    r.esperar(0.4)
    r.tecla("Return")
    ok = rec.esperar_que(lambda: any("favorita" in (c.get("tags", (None, []))[1] or [])
                                     for c in cambios(antes, foto(rec)).values()))
    rec.toma("02-etiqueta-nueva")
    r.tecla("Escape")
    rec.caso("Ctrl+K y escribir una etiqueta nueva la pone", ok)


def etiquetar_varias(rec, r):
    antes = foto(rec)
    r.clic(*CELDA["d"])
    r.esperar(0.3)
    r.mover(*CELDA["e"])
    con_tecla(r, "ctrl", r.clic)
    r.esperar(0.5)
    r.tecla("ctrl+k")
    r.esperar(0.4)
    r.escribir("favorita")
    r.tecla("Return")
    def dos():
        c = cambios(antes, foto(rec))
        return sum(1 for v in c.values() if "favorita" in (v.get("tags", (None, []))[1] or [])) >= 2
    ok = rec.esperar_que(dos)
    rec.toma("03-etiqueta-en-lote")
    r.tecla("Escape")
    rec.caso("Ctrl+clic para elegir dos y etiquetarlas a la vez", ok)


def tramo_con_mayus(rec, r):
    antes = foto(rec)
    r.clic(*CELDA["a"])
    r.esperar(0.3)
    r.mover(*CELDA["c"])
    con_tecla(r, "shift", r.clic)
    r.esperar(0.5)
    r.tecla("ctrl+k")
    r.esperar(0.4)
    r.escribir("tramo")
    r.tecla("Return")
    def cuatro():
        c = cambios(antes, foto(rec))
        return sum(1 for v in c.values() if "tramo" in (v.get("tags", (None, []))[1] or [])) >= 4
    ok = rec.esperar_que(cuatro)
    rec.toma("03b-tramo-con-mayus")
    r.tecla("Escape")
    rec.caso("Mayús+clic elige el tramo entero y se etiqueta junto", ok)


def buscar_y_guardar_carpeta_dinamica(rec, r):
    r.clic(*BUSCADOR)
    r.esperar(0.3)
    r.escribir("#favorita")
    r.esperar(0.6)
    r.tecla("Return")
    r.esperar(1.0)
    rec.toma("04-busqueda-por-etiqueta")
    r.clic(*MAS_DINAMICAS)
    r.esperar(0.5)
    r.escribir("Favoritas")
    r.tecla("Return")
    ruta = os.path.join(rec.biblio, "busquedas.json")

    def guardada():
        if not os.path.exists(ruta):
            return False
        with open(ruta) as f:
            return any(b["nombre"] == "Favoritas" and "favorita" in b["consulta"]
                       for b in json.load(f).get("busquedas", []))
    ok = rec.esperar_que(guardada)
    rec.toma("05-carpeta-dinamica-guardada")
    rec.caso("buscar #favorita y guardarlo como carpeta dinámica", ok)
    r.clic(*TODO)
    r.esperar(0.8)


def menu_del_clic_derecho_marca_18(rec, r):
    antes = foto(rec)
    x, y = CELDA["c"]
    r.clic(x, y, b=3)
    r.esperar(0.6)
    rec.toma("06-menu-clic-derecho")
    # ver, abrir fuera, copiar, exportar | mover, renombrar, +18 | papelera
    fila, raya, y0 = 30, 10, y + 5
    r.clic(x + 60, y0 + fila * 6 + raya + fila // 2)
    ok = rec.esperar_que(lambda: any(c.get("adult", (None, None))[1] is True
                                     for c in cambios(antes, foto(rec)).values()))
    rec.toma("07-marcada-18")
    rec.caso("clic derecho → «marcar +18» la marca y la difumina", ok)


def papelera_y_deshacer(rec, r):
    antes = foto(rec)
    r.clic(*CELDA["b"])
    r.esperar(0.4)
    r.tecla("Delete")
    tirada = rec.esperar_que(lambda: any(c.get("trashed", (None, None))[1] is True
                                         for c in cambios(antes, foto(rec)).values()))
    rec.caso("Supr manda lo elegido a la papelera", tirada)
    r.esperar(0.6)
    r.tecla("ctrl+z")
    vuelta = rec.esperar_que(lambda: not any(v.get("trashed") for v in foto(rec).values()
                                             if not antes[v["id"]].get("trashed")))
    rec.caso("Ctrl+Z la saca de la papelera", vuelta)


def arrastrar_a_una_carpeta(rec, r):
    antes = foto(rec)
    with open(os.path.join(rec.biblio, "folders.json")) as f:
        paletas = next(c["id"] for c in json.load(f)["folders"] if c["name"] == "Paletas")
    x, y = CELDA["e"]
    r.clic(x, y)
    r.esperar(0.4)
    r.arrastrar(x, y, *carpeta_paletas(rec), pasos=40)
    ok = rec.esperar_que(lambda: any(paletas in (c.get("folders", (None, []))[1] or [])
                                     for c in cambios(antes, foto(rec)).values()))
    rec.toma("08-arrastrada-a-paletas")
    rec.caso("arrastrar una imagen a la carpeta «Paletas» la mete dentro", ok)


def visor_y_flechas(rec, r):
    r.clic(*CELDA["a"], veces=2)
    r.esperar(1.2)
    rec.toma("09-visor")
    r.tecla("Right"); r.esperar(0.7)
    r.tecla("Right"); r.esperar(0.7)
    r.rueda(True, 3); r.esperar(0.8)
    rec.toma("10-visor-acercado")
    r.tecla("Escape"); r.esperar(0.6)
    rec.caso("doble clic abre el visor, las flechas pasan y Esc cierra", rec.app.poll() is None)


def ajustes_tema_papel(rec, r):
    r.clic(*ENGRANAJE)
    r.esperar(0.8)
    rec.toma("11-ajustes")
    conf = os.path.join(rec.tmp, "config", "Grimorio", "Grimorio.conf")

    def leer():
        return open(conf).read() if os.path.exists(conf) else ""
    # Primero el «+» del tamaño: el tema cambia los márgenes y movería los
    # botones de sitio.
    r.clic(1114, 330)
    agrandada = rec.esperar_que(lambda: "escala=1.1" in leer())
    r.esperar(0.8)
    rec.toma("12-interfaz-mas-grande")
    r.clic(988, 312)  # «−» vuelve al 100 %, ya con la interfaz agrandada
    r.esperar(0.6)
    # «papel» es el segundo botón de la fila del tema, a la derecha de la hoja.
    r.clic(1099, 292)
    ok = rec.esperar_que(lambda: "tema=papel" in leer())
    r.esperar(0.8)
    rec.toma("13-tema-papel")
    r.tecla("Escape")
    r.esperar(0.5)
    rec.caso("en ajustes, elegir el tema papel lo guarda", ok)
    rec.caso("en ajustes, «+» agranda la interfaz y lo guarda", agrandada)


def visor_3d(rec, r):
    r.tecla("ctrl+f")
    r.esperar(0.3)
    r.tecla("ctrl+a")
    r.escribir("tipo:3d")
    r.tecla("Return")
    r.esperar(1.2)
    r.clic(*CELDA["a"][:0] or (349, 153), veces=2)
    r.esperar(3.0)
    rec.toma("14-visor-3d")
    r.arrastrar(800, 500, 1000, 420, pasos=40)
    r.esperar(0.8)
    rec.toma("15-visor-3d-girado")
    r.tecla("Escape")
    r.esperar(0.5)
    rec.caso("abrir un modelo 3D y girarlo arrastrando", rec.app.poll() is None)


CASOS = [estrellas_con_el_teclado, etiquetar_con_ctrl_k, etiquetar_varias, tramo_con_mayus,
         buscar_y_guardar_carpeta_dinamica, menu_del_clic_derecho_marca_18,
         papelera_y_deshacer, arrastrar_a_una_carpeta, visor_y_flechas,
         ajustes_tema_papel, visor_3d]
