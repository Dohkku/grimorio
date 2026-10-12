"""Prepara el modelo que quita fondos tal y como lo lleva Grimorio.

    python3 herramientas/modelo/preparar.py [ENTRADA.onnx] [SALIDA.onnx]

Parte de ISNet (`isnet-general-use.onnx`, de DIS, Apache-2.0) y guarda sus
pesos en f16 sin cambiar el cálculo: cada peso f32 pasa a guardarse en f16 y
delante del grafo se pone un `Cast` a f32 con su nombre de siempre, así que
el resto del grafo no se entera. El archivo pesa la mitad —de 179 a 91 MB— y
el recorte sale igual: medido sobre un retrato, la máscara difiere como mucho
en 1/255 y en ningún píxel en más de 8. Convertir el grafo entero a f16 lo
dejaría en lo mismo, pero `tract` no lo sabe ejecutar.

Sin ENTRADA, baja el original de la release de rembg y comprueba su huella.
La conversión es determinista: la salida tiene siempre la misma huella, que es
la que la aplicación comprueba (`HUELLA_MODELO` en app/src/recorte.cpp). El
resultado se publica como archivo de la release `modelo-isnet-1` de Grimorio,
de donde lo toman los paquetes; este script es para poder rehacerlo.

Hace falta `onnx` y `numpy` (`pip install onnx numpy`).
"""
import hashlib
import sys
import urllib.request

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

ORIGEN = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx"
HUELLA_ORIGEN = "60920e99c45464f2ba57bee2ad08c919a52bbf852739e96947fbb4358c0d964a"
HUELLA_SALIDA = "ed44f83dfa784f3c7a1a8cf7928e9ff348eaad4d8a4a2ac4421eeb013637b91e"


def huella(ruta):
    h = hashlib.sha256()
    with open(ruta, "rb") as f:
        for trozo in iter(lambda: f.read(1 << 20), b""):
            h.update(trozo)
    return h.hexdigest()


def main():
    entrada = sys.argv[1] if len(sys.argv) > 1 else "isnet-general-use.onnx"
    salida = sys.argv[2] if len(sys.argv) > 2 else "modelos/isnet-general-use-pesos16.onnx"
    if len(sys.argv) <= 1:
        print("bajando", ORIGEN)
        urllib.request.urlretrieve(ORIGEN, entrada)
    if huella(entrada) != HUELLA_ORIGEN:
        sys.exit(f"{entrada} no es el ISNet esperado (la huella no coincide)")

    modelo = onnx.load(entrada)
    g = modelo.graph
    pesos, casts = [], []
    for init in g.initializer:
        if init.data_type == TensorProto.FLOAT and init.dims and np.prod(init.dims) >= 16:
            nombre16 = init.name + "__f16"
            pesos.append(numpy_helper.from_array(numpy_helper.to_array(init).astype(np.float16), nombre16))
            casts.append(helper.make_node("Cast", [nombre16], [init.name], to=TensorProto.FLOAT))
        else:
            pesos.append(init)
    del g.initializer[:]
    g.initializer.extend(pesos)
    # Lo que era entrada del grafo por ser un peso deja de serlo.
    convertidos = {c.output[0] for c in casts}
    quedan = [i for i in g.input if i.name not in convertidos]
    del g.input[:]
    g.input.extend(quedan)
    nodos = list(g.node)
    del g.node[:]
    g.node.extend(casts + nodos)
    onnx.checker.check_model(modelo)
    onnx.save(modelo, salida)

    h = huella(salida)
    print(f"{salida}: {len(casts)} pesos a f16, huella {h}")
    if h != HUELLA_SALIDA:
        sys.exit("la huella no es la esperada: ¿otra versión de onnx o de numpy?")


if __name__ == "__main__":
    main()
