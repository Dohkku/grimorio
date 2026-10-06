#include "onda.h"

#include <QFile>
#include <QLineF>
#include <QPainter>
#include <QtEndian>
#include <cmath>

Onda::Onda(QQuickItem *padre) : QQuickPaintedItem(padre)
{
    setAntialiasing(true);
    connect(this, &Onda::pintar, this, [this] { update(); });
    connect(this, &Onda::colores, this, [this] {
        colorear();
        update();
    });
    connect(this, &QQuickItem::widthChanged, this, [this] { update(); });
    connect(this, &QQuickItem::heightChanged, this, [this] { update(); });
}

void Onda::setArchivo(const QString &a)
{
    if (a == m_archivo)
        return;
    m_archivo = a;
    m_picos.clear();
    m_espectro.clear();
    m_imagen = QImage();
    m_tasa = 0;
    m_muestras = 0;

    QFile f(a);
    if (!a.isEmpty() && f.open(QIODevice::ReadOnly)) {
        const QByteArray b = f.readAll();
        const auto u32 = [&](int o) { return qFromLittleEndian<quint32>(b.constData() + o); };
        const auto f32 = [&](int o) { return qFromLittleEndian<float>(b.constData() + o); };
        // Formato en grimorio-core/src/senal.rs, `Analisis::escribir`.
        if (b.size() >= 52 && b.startsWith("GRIMONDA")) {
            m_tasa = u32(12);
            m_muestras = qFromLittleEndian<quint64>(b.constData() + 16);
            m_hopPicos = u32(24);
            const quint32 nPicos = u32(28);
            m_columnas = u32(36);
            m_filas = u32(40);
            m_fMin = f32(44);
            m_fMax = f32(48);
            const qint64 finPicos = 52 + qint64(nPicos) * 8;
            if (b.size() >= finPicos + qint64(m_columnas) * m_filas) {
                m_picos.resize(int(nPicos) * 2);
                for (quint32 i = 0; i < nPicos * 2; ++i)
                    m_picos[int(i)] = f32(52 + int(i) * 4);
                m_espectro = b.mid(int(finPicos), int(m_columnas * m_filas));
            }
        }
    }
    colorear();
    emit archivoCambio();
    update();
}

qreal Onda::frecuenciaEn(qreal h) const
{
    if (m_fMin <= 0 || m_fMax <= m_fMin)
        return 0;
    return std::exp(std::log(m_fMin) + (std::log(m_fMax) - std::log(m_fMin)) * qBound(0.0, h, 1.0));
}

void Onda::colorear()
{
    if (m_espectro.isEmpty() || !m_fondo.isValid())
        return;
    // La escala va del fondo al color de selección y de ahí al del texto: lo
    // flojo se funde con la pantalla y lo fuerte se lee como lo más claro.
    QRgb lut[256];
    const auto mezcla = [](const QColor &a, const QColor &b, qreal t) {
        return qRgb(int(a.red() + (b.red() - a.red()) * t), int(a.green() + (b.green() - a.green()) * t),
                    int(a.blue() + (b.blue() - a.blue()) * t));
    };
    for (int i = 0; i < 256; ++i) {
        const qreal t = i / 255.0;
        if (t < 0.55)
            lut[i] = mezcla(m_fondo, m_medio, t / 0.55);
        else
            lut[i] = mezcla(m_medio, m_alto, (t - 0.55) / 0.45);
    }
    m_imagen = QImage(int(m_columnas), int(m_filas), QImage::Format_RGB32);
    const auto *d = reinterpret_cast<const uchar *>(m_espectro.constData());
    for (quint32 c = 0; c < m_columnas; ++c)
        for (quint32 r = 0; r < m_filas; ++r)
            // Lo agudo arriba, como en cualquier editor de sonido.
            m_imagen.setPixel(int(c), int(m_filas - 1 - r), lut[d[c * m_filas + r]]);
}

void Onda::paint(QPainter *p)
{
    const qreal w = width(), h = height();
    if (m_fondo.isValid())
        p->fillRect(QRectF(0, 0, w, h), m_fondo);
    if (m_picos.isEmpty() || w < 1)
        return;
    const qreal desde = qBound(0.0, m_desde, 1.0);
    const qreal hasta = qBound(desde + 1e-6, m_hasta, 1.0);

    if (m_modo == 1) {
        if (m_imagen.isNull())
            return;
        const QRectF fuente(desde * m_columnas, 0, (hasta - desde) * m_columnas, m_filas);
        p->setRenderHint(QPainter::SmoothPixmapTransform, true);
        p->drawImage(QRectF(0, 0, w, h), m_imagen, fuente);
        return;
    }

    // Una línea vertical por columna de píxeles, del mínimo al máximo del tramo
    // que le toca. Con el tramo más corto que un pico, se toma el de al lado:
    // así acercarse mucho no deja huecos.
    const qint64 n = m_picos.size() / 2;
    const qreal medio = h / 2;
    QVector<QLineF> lineas;
    lineas.reserve(int(w) + 1);
    for (int x = 0; x < int(std::ceil(w)); ++x) {
        const qreal a = (desde + (hasta - desde) * x / w) * n;
        const qreal b = (desde + (hasta - desde) * (x + 1) / w) * n;
        qint64 i0 = qint64(std::floor(a));
        qint64 i1 = qMax(i0 + 1, qint64(std::ceil(b)));
        i0 = qBound<qint64>(0, i0, n - 1);
        i1 = qBound<qint64>(i0 + 1, i1, n);
        float lo = 0, hi = 0;
        for (qint64 i = i0; i < i1; ++i) {
            lo = qMin(lo, m_picos[int(i * 2)]);
            hi = qMax(hi, m_picos[int(i * 2 + 1)]);
        }
        // Una línea de al menos un píxel: el silencio se ve como silencio y no
        // como un hueco en el dibujo.
        const qreal y0 = medio - hi * medio * 0.95, y1 = medio - lo * medio * 0.95;
        lineas.append(QLineF(x + 0.5, qMin(y0, medio - 0.5), x + 0.5, qMax(y1, medio + 0.5)));
    }
    p->setRenderHint(QPainter::Antialiasing, false);
    p->setPen(QPen(m_onda, 1));
    p->drawLines(lineas);
}
