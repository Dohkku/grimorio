// Leer un original entero, sea del formato que sea.
//
// Primero Qt, que respeta la orientación del EXIF y es rápido con JPEG y PNG.
// Si Qt no sabe —WebP y TIFF sin el paquete de formatos de Qt, que no viene
// instalado—, lo decodifica el núcleo, que ya lo hacía para las miniaturas.
// Sin esto el zoom del visor se quedaba en la previsualización de 1024 px en
// cuanto la foto era WebP, que en la biblioteca de casa es casi todo.
#pragma once
#include <QImage>
#include <QString>

QImage leerOriginal(const QString &ruta);
