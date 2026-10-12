// Poner un paquete de Linux en su sitio: lo que hace actualizar desde dentro,
// y lo que completa una instalación a la que le falta algo.
//
// Va aparte de novedades.cpp, que baja y comprueba, para poder probar sin red
// lo que de verdad toca el disco (ver pruebas/paquete_test.cpp).
//
// Hasta la 0.1.5 el paquete eran dos programas, `grimorio` y `grim`, y el
// actualizador de esas versiones cambia esos dos y nada más. Desde la 0.1.6
// trae también `grim-recorte`, el modelo de quitar fondos y la carpeta de la
// extensión del navegador: quien actualiza desde una 0.1.5 se queda sin ellos,
// y la versión nueva, al arrancar, los completa (`faltan` + `instalar` con
// `soloLoQueFalta`).
#pragma once
#include <QDir>
#include <QString>
#include <QStringList>

namespace paquete {

/// Lo que tiene que estar junto al programa, relativo a su carpeta.
QStringList piezas();

/// Las de `piezas()` que no están en `dir`.
QStringList faltan(const QDir &dir);

/// La carpeta del programa dentro de un paquete descomprimido en `suelto`
/// («Grimorio-X.Y.Z-linux-x64», la que lleva `grimorio`), o vacía.
QString raizDe(const QString &suelto);

/// Copia el paquete de `raiz` a `dir`.
///
/// Con `soloLoQueFalta`, solo los archivos que no están, y nada de lo que hay
/// se toca: es completar una instalación sin cambiar el programa que corre.
/// Sin él, todo: cada cosa se copia al lado como «.nuevo» y se cambia con un
/// renombrado, que es atómico; un archivo de antes queda como «.anterior»,
/// igual que hace el lanzador. Si algo falla a mitad, se deshace lo cambiado:
/// mejor la versión vieja entera que media.
///
/// Devuelve lo que puso, relativo a `dir`, y en `error` qué falló.
bool instalar(const QString &raiz, const QDir &dir, bool soloLoQueFalta, QString *error,
              QStringList *puestos = nullptr);

} // namespace paquete
