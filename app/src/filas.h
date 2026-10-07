// La geometría de la malla, sin Qt Quick y sin modelo.
//
// Está aparte para poder probarla: es la parte de la interfaz donde un error no
// da un fallo ruidoso sino una malla ligeramente torcida, y esos son los que se
// quedan meses. Recibe cuántos elementos hay y una función que dice la
// proporción de cada uno; no sabe nada más.
#pragma once
#include <QVector>
#include <functional>

struct Fila {
    int inicio = 0;
    int n = 0;
    qreal y = 0;
    qreal alto = 0;
};

class Filas {
public:
    /// En lista cada elemento es una fila a todo el ancho y `objetivo` es el
    /// alto de esa fila: la misma maquinaria de ranuras sirve para una tabla.
    enum Modo { Cuadricula, Justificado, Lista };

    /// Proporciones extremas se recortan: un panorama de 10:1 en una fila
    /// justificada aplasta a sus vecinas hasta hacerlas ilegibles.
    static constexpr qreal PROP_MIN = 0.35;
    static constexpr qreal PROP_MAX = 3.2;
    /// Ancho mínimo de una celda, para que un formato absurdo no desaparezca.
    static constexpr qreal ANCHO_MIN = 24.0;

    using Proporcion = std::function<qreal(int)>;

    /// `pie` es el sitio que queda bajo cada fila para el nombre de sus
    /// celdas. No es hueco: el hueco separa celdas, el pie pertenece a la fila
    /// de arriba, y la celda de abajo no puede empezar dentro de él.
    void calcular(Modo modo, int total, qreal ancho, qreal objetivo, qreal margen, qreal hueco,
                  const Proporcion &proporcion, qreal pie = 0);

    const QVector<Fila> &filas() const { return m_filas; }
    qreal alturaTotal() const { return m_alturaTotal; }
    int columnas() const { return m_columnas; }
    bool vacio() const { return m_filas.isEmpty(); }

    /// Primera fila que asoma por debajo de `y`. Búsqueda binaria: el coste no
    /// depende de cuántos elementos tenga la biblioteca.
    int filaEn(qreal y) const;
    /// La fila que contiene ese elemento, o -1.
    int filaDe(int indice) const;

    /// Ancho de una celda dentro de su fila.
    qreal anchoDe(int indice, const Proporcion &proporcion) const;
    /// Igual, pero con la fila ya sabida: en el camino de cada fotograma se
    /// colocan cien celdas y buscar la fila de cada una es trabajo regalado.
    qreal anchoEn(const Fila &fila, int indice, const Proporcion &proporcion) const;
    /// Coordenada X de una celda dentro de su fila.
    qreal xDe(int indice, const Proporcion &proporcion) const;
    qreal yDe(int indice) const;
    qreal altoDe(int indice) const;

    /// La celda que queda encima o debajo de otra, en la misma columna visual.
    ///
    /// En justificado las filas no tienen las mismas celdas, así que "la de
    /// abajo" se elige por posición horizontal y no contando posiciones. Sin
    /// esto, bajar por una galería justificada da saltos laterales absurdos.
    int vecinoVertical(int indice, int direccion, const Proporcion &proporcion) const;

private:
    Modo m_modo = Justificado;
    qreal m_margen = 20;
    qreal m_hueco = 10;
    qreal m_objetivo = 200;
    qreal m_util = 0;
    qreal m_alturaTotal = 0;
    int m_columnas = 1;
    QVector<Fila> m_filas;
};
