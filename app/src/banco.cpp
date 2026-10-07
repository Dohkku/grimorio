#include "banco.h"

#include <QQuickWindow>
#include <QTextStream>
#include <QFile>
#include <QRegularExpression>
#include <algorithm>

Banco::Banco(qreal segundos, QObject *padre) : QObject(padre), m_segundos(segundos)
{
    m_reloj.start();
}

void Banco::vigilar(QQuickWindow *ventana)
{
    // DirectConnection a propósito: el `slot` corre en el hilo de render, que es
    // donde ocurre el intercambio de buffers. Todo lo que toca está protegido
    // por el mutex y no llama a nada de Qt Quick.
    connect(ventana, &QQuickWindow::frameSwapped, this, &Banco::anotarSwap,
            Qt::DirectConnection);
}

void Banco::anotarSwap()
{
    const qint64 ahora = m_reloj.nsecsElapsed();
    QMutexLocker cierre(&m_mutex);
    if (m_primerFotogramaMs < 0) m_primerFotogramaMs = ahora / 1e6;
    if (m_ultimoNs != 0) m_ms.append((ahora - m_ultimoNs) / 1e6);
    m_ultimoNs = ahora;
}

qreal Banco::transcurridos() const
{
    return m_reloj.nsecsElapsed() / 1e9;
}

void memoriaMB(double *propia, double *archivos)
{
    *propia = 0;
    *archivos = 0;
    QFile f(QStringLiteral("/proc/self/status"));
    if (!f.open(QIODevice::ReadOnly | QIODevice::Text)) return;
    // Leer entero y partir a mano, no con QTextStream: QFile dice que los
    // archivos de /proc miden cero bytes y `atEnd()` se lo cree, así que el
    // bucle terminaba antes de leer nada y la memoria salía siempre a cero.
    const QStringList lineas =
        QString::fromLatin1(f.readAll()).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    for (const QString &linea : lineas) {
        const auto valor = [&linea] {
            const auto partes = linea.split(QRegularExpression(QStringLiteral("[ \\t]+")), Qt::SkipEmptyParts);
            return partes.size() > 1 ? partes.at(1).toDouble() / 1024.0 : 0.0;
        };
        if (linea.startsWith(QLatin1String("RssAnon:"))) *propia = valor();
        else if (linea.startsWith(QLatin1String("RssFile:"))) *archivos = valor();
    }
}

void Banco::informe(const QString &titulo, const QString &extra)
{
    QVector<qreal> ms;
    qreal primero = -1;
    {
        QMutexLocker cierre(&m_mutex);
        ms = m_ms;
        primero = m_primerFotogramaMs;
    }
    // Cuándo ocurrió el peor, no solo cuánto duró: un pico a los 0,4 s es el
    // arranque y no importa; uno a los 8 s es algo que va a pasar mientras
    // alguien trabaja.
    qreal peor = 0, peorEn = 0, acumulado = 0;
    for (qreal v : ms) {
        acumulado += v;
        if (v > peor) {
            peor = v;
            peorEn = acumulado;
        }
    }
    QTextStream out(stdout);
    if (ms.isEmpty()) {
        out << "  sin fotogramas medidos\n";
        return;
    }
    // El primer intervalo incluye compilar el pipeline gráfico; se anota aparte,
    // igual que en el spike de wgpu.
    const qreal inicial = ms.first();
    ms.removeFirst();
    if (ms.isEmpty()) ms.append(inicial);
    std::sort(ms.begin(), ms.end());
    const auto p = [&ms](qreal q) { return ms.at(int((ms.size() - 1) * q)); };
    qreal suma = 0;
    for (qreal v : ms) suma += v;
    const qreal media = suma / ms.size();
    const auto sobre = [&ms](qreal lim) {
        return int(std::count_if(ms.begin(), ms.end(), [lim](qreal v) { return v > lim; }));
    };

    out << Qt::endl << titulo << Qt::endl;
    out << QStringLiteral("  fotogramas          %1\n").arg(ms.size() + 1);
    out << QStringLiteral("  primero             %1 ms (incluye compilar el pipeline)\n")
               .arg(inicial, 0, 'f', 1);
    out << QStringLiteral("  media               %1 ms  (%2 fps)\n")
               .arg(media, 0, 'f', 2).arg(1000.0 / media, 0, 'f', 0);
    out << QStringLiteral("  p50 / p95 / p99     %1 / %2 / %3 ms\n")
               .arg(p(0.5), 0, 'f', 2).arg(p(0.95), 0, 'f', 2).arg(p(0.99), 0, 'f', 2);
    out << QStringLiteral("  máximo              %1 ms  (a los %2 s)\n")
               .arg(peor, 0, 'f', 2)
               .arg(peorEn / 1000.0, 0, 'f', 1);
    out << QStringLiteral("  por encima de 8,3 ms %1   (%2%)\n")
               .arg(sobre(8.3), 5).arg(sobre(8.3) * 100.0 / ms.size(), 0, 'f', 2);
    out << QStringLiteral("  por encima de 16,6 ms%1   (%2%)\n")
               .arg(sobre(16.6), 5).arg(sobre(16.6) * 100.0 / ms.size(), 0, 'f', 2);
    if (primero >= 0)
        out << QStringLiteral("  primer fotograma a los %1 ms desde el arranque\n")
                   .arg(primero, 0, 'f', 0);
    if (!extra.isEmpty()) out << extra;
    out.flush();
}
