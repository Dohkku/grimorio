// Proveedor de imágenes: decodifica fuera del hilo de interfaz.
//
// Sirve cuatro cosas por la misma puerta, y se distinguen por la URL:
//   * `image://grim/<indice>`   miniatura de malla, desde el pack mapeado
//   * `image://grim/previa/<id>` previsualización de 1024 px, desde disco
//   * `image://grim/original/<id>` el original entero, para el zoom del visor
//   * `image://grim/esquinas/<radio>/<fondo>`  la máscara que redondea las
//     celdas; radio y color van en la URL para que cambiar de tema o de
//     tamaño pida otra y el caché de Qt no sirva la vieja
#pragma once
#include <QAtomicInteger>
#include <QColor>
#include <QQuickAsyncImageProvider>

extern "C" {
#include "grimorio.h"
}

class Modelo;

class Proveedor : public QQuickAsyncImageProvider {
public:
    explicit Proveedor(Modelo *modelo);

    QQuickImageResponse *requestImageResponse(const QString &id, const QSize &pedido) override;

    /// Cuántas imágenes se han decodificado de verdad. Comparado con cuántas
    /// celdas han pasado por pantalla, delata si el caché de Qt recicla o
    /// vuelve a decodificar lo mismo una y otra vez.
    int decodificadas() const { return m_decodificadas.loadRelaxed(); }
    int fallidas() const { return m_fallidas.loadRelaxed(); }
    /// Las que Qt dio por innecesarias antes de que les llegara el turno: el
    /// trabajo que el desplazamiento rápido se ahorra.
    int canceladas() const { return m_canceladas.loadRelaxed(); }
    void contarDecodificada() { m_decodificadas.fetchAndAddRelaxed(1); }
    void contarFallo() { m_fallidas.fetchAndAddRelaxed(1); }
    void contarCancelada() { m_canceladas.fetchAndAddRelaxed(1); }

private:
    static QString rutaPrevia(const GrimVista *v, int indice);
    /// El índice de un elemento en la vista viva, o -1.
    /// La posición de un id en **esa** vista. Se le pasa la vista y no la
    /// busca: buscar en una y decodificar de otra es como salían miniaturas
    /// de un elemento puestas en otro al cambiar rápido de carpeta.
    static int indiceDe(const GrimVista *v, const QString &id);

    Modelo *m_modelo = nullptr;
    QAtomicInteger<int> m_avisadoSinPrevias { 0 };
    QAtomicInteger<int> m_decodificadas { 0 };
    QAtomicInteger<int> m_fallidas { 0 };
    QAtomicInteger<int> m_canceladas { 0 };
};
