// Frontera C con `grimorio-core`. La implementación está en app/puente/src/.
//
// Dos caminos, deliberadamente distintos:
//   * el bus de comandos, en JSON, asíncrono, para lo que hace una persona;
//   * el camino rápido, por índice y sin copias, para lo que pinta la malla.
#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Nucleo GrimNucleo;
typedef struct Vista GrimVista;

/// Se llama desde el hilo trabajador del núcleo. Puede llegar desde cualquier
/// hilo, pero nunca desde dos a la vez. `json` no sobrevive a la llamada.
typedef void (*GrimEventoFn)(void *usuario, const uint8_t *json, uint32_t len);

/// Abre la biblioteca y arranca el hilo. NULL si la ruta no es una biblioteca.
/// `usuario` tiene que seguir vivo hasta `grim_parar`.
GrimNucleo *grim_iniciar(const char *ruta, GrimEventoFn cb, void *usuario);
void grim_parar(GrimNucleo *n);

/// Encola un comando. Devuelve su número, que vuelve en el evento de respuesta;
/// 0 si el JSON no se entiende (y entonces llega un evento de error).
uint64_t grim_mandar(GrimNucleo *n, const uint8_t *json, uint32_t len);

// --------------------------------------------------------------- camino rápido

/// Toma una referencia a la vista publicada. Mientras no se suelte, todos sus
/// punteros son válidos aunque el núcleo publique otra.
GrimVista *grim_vista(const GrimNucleo *n);
/// Otra referencia a la misma vista, para quien la necesite más allá del
/// fotograma actual. Se suelta igual que la primera.
const GrimVista *grim_vista_clonar(const GrimVista *v);
void grim_vista_soltar(const GrimVista *v);

size_t grim_vista_n(const GrimVista *v);
uint32_t grim_vista_ancho(const GrimVista *v, size_t i);
uint32_t grim_vista_alto(const GrimVista *v, size_t i);
uint8_t grim_vista_estrellas(const GrimVista *v, size_t i);

/// La familia del elemento. Los números son los del enum `Familia` de Rust y
/// están repetidos aquí a propósito: el lado de C++ no debe adivinarlos.
enum GrimFamilia {
    GRIM_IMAGEN = 0,
    GRIM_RAW = 1,
    GRIM_VIDEO = 2,
    GRIM_AUDIO = 3,
    GRIM_DOCUMENTO = 4,
    GRIM_TIPOGRAFIA = 5,
    GRIM_MODELO = 6,
};
uint8_t grim_vista_familia(const GrimVista *v, size_t i);
/// 1 si está marcado como contenido adulto.
uint8_t grim_vista_adulto(const GrimVista *v, size_t i);
/// La extensión, apuntando dentro de la vista. No copia.
const uint8_t *grim_vista_ext(const GrimVista *v, size_t i, uint32_t *len);
/// Duración en segundos, o 0 si el archivo no dura.
uint32_t grim_vista_duracion(const GrimVista *v, size_t i);
/// Peso del original en bytes.
uint64_t grim_vista_peso(const GrimVista *v, size_t i);
/// Cuándo se importó, en milisegundos Unix.
uint64_t grim_vista_importado(const GrimVista *v, size_t i);
/// 0x00RRGGBB, o 0xFFFFFFFF si el elemento no tiene paleta.
uint32_t grim_vista_dominante(const GrimVista *v, size_t i);
/// UTF-8 sin terminador: usa la longitud.
const uint8_t *grim_vista_nombre(const GrimVista *v, size_t i, uint32_t *len);
const uint8_t *grim_vista_id(const GrimVista *v, size_t i, uint32_t *len);
/// JPEG de la miniatura, apuntando al pack mapeado. No liberar.
const uint8_t *grim_vista_thumb(const GrimVista *v, size_t i, uint32_t *len);
/// Posición de un id, o SIZE_MAX si no está en esta vista.
size_t grim_vista_indice_de(const GrimVista *v, const uint8_t *id, uint32_t len);
/// Escribe la ruta de la previsualización de 1024 px en `salida` y devuelve
/// cuántos bytes ocupa; 0 si no cabe o el índice no existe.
uint32_t grim_vista_previa(const GrimVista *v, size_t i, uint8_t *salida, uint32_t cap);
uint32_t grim_vista_ruta(const GrimVista *v, size_t i, uint8_t *salida, uint32_t cap);
uint64_t grim_vista_bytes_pack(const GrimVista *v);

/// Decodifica una imagen entera a RGBA8 (filas de ancho × 4 bytes). Para lo
/// que Qt no sabe abrir. Null si no se pudo; liberar con `grim_pixeles_soltar`.
uint8_t *grim_decodificar(const uint8_t *ruta, uint32_t len, uint32_t *ancho, uint32_t *alto);
void grim_pixeles_soltar(uint8_t *p, size_t bytes);

/// Crea una biblioteca en `ruta` si no la hay. 1 si se puede abrir.
int32_t grim_crear_biblioteca(const uint8_t *ruta, uint32_t len);
/// 1 si en `ruta` hay una biblioteca de Grimorio.
int32_t grim_es_biblioteca(const uint8_t *ruta, uint32_t len);

// ---------------------------------------------------------------- derivados

/// Pide un derivado —proxy de vídeo, onda, página de PDF, malla 3D— y espera a
/// que esté. Puede tardar minutos: solo desde un hilo propio. Escribe la
/// respuesta JSON en `salida` y devuelve su tamaño; si es mayor que `cap` no
/// escribe nada y hay que volver a pedir con un búfer de ese tamaño.
uint32_t grim_derivar(const uint8_t *peticion, uint32_t len, uint8_t *salida, uint32_t cap);

#ifdef __cplusplus
}
#endif
