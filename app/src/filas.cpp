#include "filas.h"

#include <QtGlobal>
#include <cmath>

void Filas::calcular(Modo modo, int total, qreal ancho, qreal objetivo, qreal margen, qreal hueco,
                     const Proporcion &proporcion, qreal pie)
{
    m_modo = modo;
    m_margen = margen;
    m_hueco = hueco;
    m_objetivo = objetivo;
    m_filas.clear();

    const qreal util = qMax(50.0, ancho - margen * 2);
    m_util = util;
    qreal y = margen;

    if (modo == Lista) {
        m_columnas = 1;
        m_filas.reserve(total);
        for (int i = 0; i < total; ++i) {
            m_filas.append({ i, 1, y, objetivo });
            y += objetivo + hueco;
        }
    } else if (modo == Cuadricula) {
        const int cols = qMax(1, int(std::floor((util + hueco) / (objetivo + hueco))));
        m_columnas = cols;
        const qreal lado = (util - hueco * (cols - 1)) / cols;
        m_filas.reserve(total / cols + 1);
        for (int i = 0; i < total; i += cols) {
            m_filas.append({ i, qMin(cols, total - i), y, lado });
            y += lado + pie + hueco;
        }
    } else {
        m_columnas = 0;
        m_filas.reserve(total / 4 + 1);
        int inicio = 0;
        qreal sumaProp = 0;
        for (int i = 0; i < total; ++i) {
            sumaProp += qBound(PROP_MIN, proporcion(i), PROP_MAX);
            const int cuantas = i - inicio + 1;
            const qreal alto = (util - hueco * (cuantas - 1)) / sumaProp;
            // La fila se cierra cuando la altura resultante baja del objetivo:
            // así todas las filas quedan parecidas de alto sin fijarlo a mano.
            // La segunda condición es el freno para una racha de imágenes muy
            // estrechas, que si no llenaría la fila de rendijas.
            if (alto <= objetivo || cuantas * objetivo > util * 1.6) {
                m_filas.append({ inicio, cuantas, y, alto });
                y += alto + pie + hueco;
                inicio = i + 1;
                sumaProp = 0;
            }
        }
        // La última fila incompleta se deja a la altura objetivo en vez de
        // estirarla: estirarla es lo que hace que las galerías se vean rotas.
        if (inicio < total) {
            m_filas.append({ inicio, total - inicio, y, objetivo });
            y += objetivo + pie + hueco;
        }
    }

    m_alturaTotal = qMax(0.0, y - hueco + margen);
}

int Filas::filaEn(qreal y) const
{
    int lo = 0, hi = int(m_filas.size());
    while (lo < hi) {
        const int mid = (lo + hi) / 2;
        if (m_filas.at(mid).y + m_filas.at(mid).alto < y) lo = mid + 1;
        else hi = mid;
    }
    return lo;
}

int Filas::filaDe(int indice) const
{
    if (indice < 0) return -1;
    int lo = 0, hi = int(m_filas.size());
    while (lo < hi) {
        const int mid = (lo + hi) / 2;
        const Fila &f = m_filas.at(mid);
        if (f.inicio + f.n <= indice) lo = mid + 1;
        else hi = mid;
    }
    return lo < m_filas.size() ? lo : -1;
}

qreal Filas::anchoEn(const Fila &fila, int indice, const Proporcion &proporcion) const
{
    if (m_modo == Lista) return m_util;
    if (m_modo == Cuadricula) return fila.alto;
    return qMax(ANCHO_MIN, fila.alto * qBound(PROP_MIN, proporcion(indice), PROP_MAX));
}

qreal Filas::anchoDe(int indice, const Proporcion &proporcion) const
{
    const int f = filaDe(indice);
    if (f < 0) return m_objetivo;
    return anchoEn(m_filas.at(f), indice, proporcion);
}

qreal Filas::xDe(int indice, const Proporcion &proporcion) const
{
    const int f = filaDe(indice);
    if (f < 0) return m_margen;
    const Fila &fila = m_filas.at(f);
    qreal x = m_margen;
    for (int i = fila.inicio; i < indice; ++i) x += anchoDe(i, proporcion) + m_hueco;
    return x;
}

qreal Filas::yDe(int indice) const
{
    const int f = filaDe(indice);
    return f >= 0 ? m_filas.at(f).y : 0;
}

qreal Filas::altoDe(int indice) const
{
    const int f = filaDe(indice);
    return f >= 0 ? m_filas.at(f).alto : m_objetivo;
}

int Filas::vecinoVertical(int indice, int direccion, const Proporcion &proporcion) const
{
    const int f = filaDe(indice);
    if (f < 0) return indice;
    const int destinoFila = f + (direccion > 0 ? 1 : -1);
    if (destinoFila < 0 || destinoFila >= m_filas.size()) return indice;

    const qreal centro = xDe(indice, proporcion) + anchoDe(indice, proporcion) / 2;

    const Fila &destino = m_filas.at(destinoFila);
    qreal x = m_margen;
    int mejor = destino.inicio;
    qreal mejorDist = -1;
    for (int k = 0; k < destino.n; ++k) {
        const int i = destino.inicio + k;
        const qreal w = anchoDe(i, proporcion);
        const qreal d = qAbs(x + w / 2 - centro);
        if (mejorDist < 0 || d < mejorDist) {
            mejorDist = d;
            mejor = i;
        }
        x += w + m_hueco;
    }
    return mejor;
}
