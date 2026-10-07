#!/usr/bin/env python3
"""Una biblioteca de demostración, hecha entera aquí.

Imágenes generadas —carteles geométricos, paisajes, degradados, tipografía,
paletas, texturas, bloques isométricos— y un par de piezas 3D, con etiquetas,
estrellas, carpetas y carpetas dinámicas puestas. Sirve para las capturas, los
vídeos y la demo de la web sin enseñar la biblioteca de nadie ni imágenes con
derechos de otros.

    python3 herramientas/demo/generar.py ~/Demo.grimorio [--grim ./target/release/grim]

Solo pide Pillow. Es determinista: la misma semilla da la misma biblioteca.
"""
import argparse, json, math, os, random, shutil, struct, subprocess, sys, tempfile
from PIL import Image, ImageDraw, ImageFilter, ImageFont

PALETAS = {
    "cálido": ["#e8a33d", "#d9603b", "#9c2f2f", "#f3e3c3", "#2b211c"],
    "frío":   ["#1f4e5f", "#3f8ea0", "#9fd3d6", "#e6f1f2", "#13212a"],
    "bosque": ["#2f4a35", "#6f8f58", "#c9c38f", "#efe9d2", "#1b2620"],
    "neón":   ["#ff3c7e", "#7a2cff", "#18e0d2", "#0d0b1e", "#f7f05a"],
    "arena":  ["#d8c3a0", "#b08a5b", "#6b5138", "#f4ece0", "#3a2d22"],
    "pastel": ["#f4b6c2", "#b8d8e8", "#cfe5b8", "#fbe7b5", "#6c6a80"],
}
PALABRAS = ["NORTE", "LUZ", "RITMO", "ECO", "FORMA", "SAL", "ÓRBITA", "MAREA",
            "HUECO", "PULSO", "TRAZO", "VÉRTICE", "BRUMA", "ÁMBAR"]
FUENTES = ["/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
           "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
           "/usr/share/fonts/truetype/dejavu/DejaVuSans-ExtraLight.ttf"]
FORMATOS = [(1200, 1600), (1600, 1200), (1400, 1400), (1200, 1800), (1800, 1000), (1000, 1500)]


def hex2rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def fuente(i, tam):
    try:
        return ImageFont.truetype(FUENTES[i % len(FUENTES)], tam)
    except OSError:
        return ImageFont.load_default()


def grano(img, r, fuerza=10):
    """Un poco de ruido: lo plano generado se nota a la legua."""
    w, h = img.size
    ruido = Image.effect_noise((w, h), fuerza).convert("L")
    return Image.blend(img, Image.merge("RGB", (ruido, ruido, ruido)), 0.06)


def bauhaus(r, w, h, pal):
    img = Image.new("RGB", (w, h), hex2rgb(pal[3]))
    d = ImageDraw.Draw(img)
    for _ in range(r.randint(4, 8)):
        c = hex2rgb(r.choice(pal[:3] + [pal[4]]))
        s = r.randint(w // 6, w // 2)
        x, y = r.randint(-s // 3, w - s * 2 // 3), r.randint(-s // 3, h - s * 2 // 3)
        t = r.random()
        if t < 0.4:
            d.ellipse([x, y, x + s, y + s], fill=c)
        elif t < 0.7:
            d.rectangle([x, y, x + s, y + s * r.uniform(0.3, 1.2)], fill=c)
        elif t < 0.85:
            d.polygon([(x, y + s), (x + s / 2, y), (x + s, y + s)], fill=c)
        else:
            d.pieslice([x, y, x + s, y + s], r.randint(0, 3) * 90, r.randint(0, 3) * 90 + 180, fill=c)
    d.line([(w * 0.08, h * 0.9), (w * 0.92, h * 0.9)], fill=hex2rgb(pal[4]), width=max(3, w // 200))
    return grano(img, r)


def paisaje(r, w, h, pal):
    img = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(img)
    a, b = hex2rgb(pal[3]), hex2rgb(pal[1])
    for y in range(h):
        t = y / h
        d.line([(0, y), (w, y)], fill=tuple(int(a[i] * (1 - t) + b[i] * t) for i in range(3)))
    sol = r.randint(w // 10, w // 5)
    sx, sy = r.randint(w // 5, w * 4 // 5), r.randint(h // 8, h // 3)
    d.ellipse([sx - sol, sy - sol, sx + sol, sy + sol], fill=hex2rgb(pal[0]))
    capas = [pal[2], pal[1], pal[0], pal[4]]
    for k, col in enumerate(capas):
        base = h * (0.45 + k * 0.13)
        pts = [(0, h)]
        fase, amp = r.random() * 6, h * (0.08 - k * 0.012)
        for x in range(0, w + 40, 40):
            pts.append((x, base + math.sin(x / w * (3 + k) + fase) * amp + r.uniform(-amp, amp) * 0.3))
        pts.append((w, h))
        d.polygon(pts, fill=hex2rgb(col))
    return grano(img, r)


def degradado(r, w, h, pal):
    img = Image.new("RGB", (w // 4, h // 4), hex2rgb(pal[4]))
    d = ImageDraw.Draw(img)
    for _ in range(7):
        c = hex2rgb(r.choice(pal[:4]))
        x, y, s = r.randint(-40, w // 4), r.randint(-40, h // 4), r.randint(w // 10, w // 4)
        d.ellipse([x - s, y - s, x + s, y + s], fill=c)
    img = img.filter(ImageFilter.GaussianBlur(w // 22)).resize((w, h), Image.BICUBIC)
    return grano(img, r, 14)


def tipografia(r, w, h, pal):
    fondo, tinta, acento = hex2rgb(pal[4]), hex2rgb(pal[3]), hex2rgb(pal[0])
    if r.random() < 0.5:
        fondo, tinta = tinta, fondo
    img = Image.new("RGB", (w, h), fondo)
    d = ImageDraw.Draw(img)
    palabra = r.choice(PALABRAS)
    cual, tam = r.randint(0, 2), int(w / max(3, len(palabra)) * 1.6)
    # Que quepa con margen: se encoge hasta ocupar como mucho el 84 % del ancho.
    while True:
        f = fuente(cual, tam)
        caja = d.textbbox((0, 0), palabra, font=f)
        if caja[2] - caja[0] <= w * 0.84 or tam < 20:
            break
        tam = int(tam * 0.92)
    tx, ty = (w - (caja[2] - caja[0])) / 2, h * r.uniform(0.25, 0.55)
    d.ellipse([w * 0.55, h * 0.08, w * 0.95, h * 0.08 + w * 0.4], fill=acento)
    d.text((tx, ty), palabra, font=f, fill=tinta)
    pie = fuente(0, max(18, w // 40))
    d.text((w * 0.08, h * 0.88), "colección · %d" % r.randint(1, 99), font=pie, fill=tinta)
    d.line([(w * 0.08, h * 0.86), (w * 0.4, h * 0.86)], fill=acento, width=max(2, w // 300))
    return grano(img, r)


def paleta(r, w, h, pal):
    img = Image.new("RGB", (w, h), hex2rgb("#f4f1ec"))
    d = ImageDraw.Draw(img)
    m, n = w * 0.08, len(pal)
    alto = (h - 2 * m) / n
    pie = fuente(0, max(16, w // 45))
    for i, c in enumerate(pal):
        y = m + i * alto
        d.rectangle([m, y, w - m, y + alto * 0.82], fill=hex2rgb(c))
        d.text((m + 12, y + alto * 0.82 - pie.size - 10), c.upper(), font=pie,
               fill=(255, 255, 255) if sum(hex2rgb(c)) < 380 else (30, 30, 30))
    return img


def textura(r, w, h, pal):
    img = Image.new("RGB", (w, h), hex2rgb(pal[4]))
    d = ImageDraw.Draw(img)
    paso = r.randint(w // 60, w // 25)
    tipo = r.random()
    for i in range(-h, w + h, paso):
        c = hex2rgb(pal[(i // paso) % 4])
        if tipo < 0.5:
            d.line([(i, 0), (i + h * 0.6, h)], fill=c, width=paso // 2)
        else:
            cx, cy = w / 2, h / 2
            rr = abs(i)
            d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=c, width=max(2, paso // 3))
    return grano(img, r, 16)


def isometrico(r, w, h, pal):
    img = Image.new("RGB", (w, h), hex2rgb(pal[3]))
    d = ImageDraw.Draw(img)
    s = w // 12
    for _ in range(r.randint(10, 18)):
        gx, gy, alto = r.randint(1, 8), r.randint(1, 8), r.randint(1, 4)
        x, y = w / 2 + (gx - gy - 0) * s * 0.866, h * 0.18 + (gx + gy) * s * 0.42
        top = [(x, y - alto * s), (x + s * 0.866, y - alto * s + s * 0.5), (x, y - alto * s + s), (x - s * 0.866, y - alto * s + s * 0.5)]
        d.polygon(top, fill=hex2rgb(pal[0]))
        d.polygon([top[3], top[2], (x, y + s), (x - s * 0.866, y + s * 0.5)], fill=hex2rgb(pal[1]))
        d.polygon([top[2], top[1], (x + s * 0.866, y + s * 0.5), (x, y + s)], fill=hex2rgb(pal[2]))
    return grano(img, r)


GENERADORES = {
    "cartel": (bauhaus, "Carteles"),
    "paisaje": (paisaje, "Paisajes"),
    "degradado": (degradado, "Fondos"),
    "tipografía": (tipografia, "Tipografía"),
    "paleta": (paleta, "Paletas"),
    "textura": (textura, "Fondos"),
    "isométrico": (isometrico, "Arquitectura"),
}


def stl(ruta, piezas):
    tris = []
    def caja(x0, y0, z0, x1, y1, z1):
        v = [(x, y, z) for x in (x0, x1) for y in (y0, y1) for z in (z0, z1)]
        for a, b, c, dd in [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]:
            tris.append((v[a], v[b], v[c])); tris.append((v[a], v[c], v[dd]))
    for p in piezas:
        caja(*p)
    with open(ruta, "wb") as f:
        f.write(b"\0" * 80); f.write(struct.pack("<I", len(tris)))
        for t in tris:
            f.write(struct.pack("<3f", 0, 0, 0))
            for p in t:
                f.write(struct.pack("<3f", *p))
            f.write(b"\0\0")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("biblioteca")
    ap.add_argument("--grim", default="grim")
    ap.add_argument("--n", type=int, default=140)
    ap.add_argument("--semilla", type=int, default=7)
    a = ap.parse_args()
    r = random.Random(a.semilla)
    grim = lambda *args: subprocess.run([a.grim, "-L", a.biblioteca, *args], check=True,
                                        capture_output=True, text=True).stdout

    if os.path.exists(a.biblioteca):
        sys.exit(f"ya existe {a.biblioteca}: bórrala o elige otra ruta")
    subprocess.run([a.grim, "init", a.biblioteca], check=True, capture_output=True)

    tmp = tempfile.mkdtemp(prefix="grimorio-demo-")
    plan = []
    tipos = list(GENERADORES)
    for i in range(a.n):
        tipo = tipos[i % len(tipos)]
        nombre_pal = r.choice(list(PALETAS))
        w, h = r.choice(FORMATOS)
        if tipo == "paleta":
            w, h = 1200, 1500
        img = GENERADORES[tipo][0](r, w, h, PALETAS[nombre_pal])
        nombre = f"{tipo} {nombre_pal} {i:03d}".replace(" ", "-")
        ruta = os.path.join(tmp, nombre + ".jpg")
        img.save(ruta, quality=88)
        plan.append((ruta, tipo, nombre_pal))
        print(f"\r  imágenes {i + 1}/{a.n}", end="", flush=True)
    print()
    stl(os.path.join(tmp, "torre-modular.stl"),
        [(-20, -20, 0, 20, 20, 6), (-8, -8, 6, 8, 8, 50), (-14, -14, 50, 14, 14, 56), (-4, -4, 56, 4, 4, 70)])
    stl(os.path.join(tmp, "soporte-escalonado.stl"),
        [(0, 0, 0, 60, 30, 8), (0, 0, 8, 40, 30, 16), (0, 0, 16, 20, 30, 24), (50, 10, 8, 56, 20, 40)])

    grim("import", tmp)
    shutil.rmtree(tmp)

    # Lo importado, por nombre de archivo, para ponerle lo demás.
    items = {}
    for raiz, _, archivos in os.walk(os.path.join(a.biblioteca, "items")):
        if "item.json" in archivos:
            p = os.path.join(raiz, "item.json")
            with open(p) as f:
                items[p] = json.load(f)

    def nuevo_id(k):
        # ULID a ojo: lo bastante único para una demo y con el mismo aspecto.
        return "01DEMO" + "".join(r.choice("0123456789ABCDEFGHJKMNPQRSTVWXYZ") for _ in range(20))

    carpetas = {}
    def carpeta(nombre, madre=None, color=None):
        c = {"id": nuevo_id(nombre), "name": nombre, "pos": len(carpetas) * 10}
        if madre: c["parent"] = carpetas[madre]["id"]
        if color: c["color"] = color
        carpetas[nombre] = c
        return c
    for nombre, color in [("Carteles", "#d9603b"), ("Paisajes", "#6f8f58"), ("Fondos", "#3f8ea0"),
                          ("Tipografía", None), ("Paletas", "#e8a33d"), ("Arquitectura", None), ("Modelos 3D", "#7a2cff")]:
        carpeta(nombre, color=color)
    carpeta("Proyecto Norte", "Carteles")
    carpeta("Portadas", "Tipografía")

    por_tipo = {}
    for p, it in items.items():
        nombre = it.get("name") or os.path.basename(p)
        tipo = next((t for t in GENERADORES if nombre.startswith(t)), None)
        etiquetas, fs = [], []
        if tipo:
            pal = next((k for k in PALETAS if k in nombre), None)
            etiquetas = [tipo, pal] + r.sample(["moodboard", "referencia", "cliente", "inspiración", "web", "impresión"], 2)
            fs = [carpetas[GENERADORES[tipo][1]]["id"]]
            if tipo == "cartel" and r.random() < 0.4: fs.append(carpetas["Proyecto Norte"]["id"])
            if tipo == "tipografía" and r.random() < 0.5: fs.append(carpetas["Portadas"]["id"])
        else:
            etiquetas, fs = ["3d", "impresión"], [carpetas["Modelos 3D"]["id"]]
        por_tipo.setdefault(tipo, []).append(it["id"])
        it.setdefault("tags", [])
        it["tags"] = sorted(set(it["tags"]) | set(etiquetas))
        it["folders"] = fs
        it["stars"] = r.choice([0, 0, 2, 3, 3, 4, 4, 5])
        if r.random() < 0.15:
            it["note"] = r.choice(["para la portada de otoño", "mirar el contraste", "buena base para el cliente"])
        with open(p, "w") as f:
            json.dump(it, f, ensure_ascii=False, indent=2)

    with open(os.path.join(a.biblioteca, "folders.json"), "w") as f:
        json.dump({"schema": 0, "folders": list(carpetas.values())}, f, ensure_ascii=False, indent=2)
    with open(os.path.join(a.biblioteca, "busquedas.json"), "w") as f:
        json.dump({"schema": 0, "busquedas": [
            {"id": nuevo_id("a"), "nombre": "Lo mejor", "consulta": "estrellas:5"},
            {"id": nuevo_id("b"), "nombre": "Cálidos para el cliente", "consulta": "etiqueta:cálido etiqueta:cliente"},
            {"id": nuevo_id("c"), "nombre": "Moodboard en vertical", "consulta": "etiqueta:moodboard orientacion:vertical"},
        ]}, f, ensure_ascii=False, indent=2)
    grim("reindex")
    print(grim("stats"))


if __name__ == "__main__":
    main()
