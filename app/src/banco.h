// Banco de pruebas: mide fotogramas con el mismo criterio que el spike de wgpu
// para que los dos números se puedan poner uno al lado del otro.
//
// El tiempo entre fotogramas se toma en `frameSwapped`, que Qt emite en el hilo
// de render. Anotarlo ahí y no en el de interfaz es deliberado: es el instante
// en el que la imagen llega de verdad al monitor.
#pragma once
#include <QElapsedTimer>
#include <QMutex>
#include <QObject>
#include <QVector>

class QQuickWindow;
class Proveedor;

class Banco : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool activo READ activo CONSTANT)
    Q_PROPERTY(qreal segundos READ segundos CONSTANT)

public:
    explicit Banco(qreal segundos, QObject *padre = nullptr);

    /// Empieza a contar. `arranqueNs` es el reloj tomado al entrar en main(),
    /// para poder informar del tiempo hasta el primer fotograma.
    void vigilar(QQuickWindow *ventana);

    bool activo() const { return m_segundos > 0; }
    qreal segundos() const { return m_segundos; }
    /// Solo invocable, nunca propiedad: el valor cambia en cada fotograma y
    /// una propiedad obligaría a emitir una señal por fotograma para nada.
    Q_INVOKABLE qreal transcurridos() const;

    /// Imprime el informe y devuelve el código de salida (0 siempre: el juicio
    /// sobre los números lo hace quien lee, no el programa).
    Q_INVOKABLE void informe(const QString &titulo, const QString &extra);

    /// Milisegundos desde el arranque hasta que el primer fotograma llegó a
    /// pantalla. -1 si todavía no ha ocurrido.
    qreal primerFotogramaMs() const { return m_primerFotogramaMs; }

private:
    void anotarSwap();

    qreal m_segundos = 0;
    QElapsedTimer m_reloj;
    mutable QMutex m_mutex;
    QVector<qreal> m_ms;
    qint64 m_ultimoNs = 0;
    qreal m_primerFotogramaMs = -1;
};

/// Memoria propia y memoria de archivos mapeados, por separado. Mezclarlas
/// engaña: las páginas del pack son caché del sistema, no consumo del programa.
void memoriaMB(double *propia, double *archivos);
