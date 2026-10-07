// El color de un píxel del original, para el cuentagotas del visor.
//
// QML enseña la imagen pero no deja leerla: `Image` no da sus píxeles. Aquí se
// decodifica el original una vez, en otro hilo —una foto de 50 megapíxeles
// tarda lo suyo y el visor no puede pararse—, y luego leer un píxel es mirar
// en memoria.
#pragma once
#include <QImage>
#include <QObject>
#include <QUrl>

class Pixeles : public QObject {
    Q_OBJECT
    /// Si ya se puede preguntar por píxeles de `fuente`.
    Q_PROPERTY(bool listo READ listo NOTIFY cambio)
    Q_PROPERTY(QUrl fuente READ fuente NOTIFY cambio)
    Q_PROPERTY(int ancho READ ancho NOTIFY cambio)
    Q_PROPERTY(int alto READ alto NOTIFY cambio)

public:
    using QObject::QObject;

    bool listo() const { return !m_imagen.isNull(); }
    QUrl fuente() const { return m_fuente; }
    int ancho() const { return m_imagen.width(); }
    int alto() const { return m_imagen.height(); }

    /// Empieza a decodificar. Pedir la misma otra vez no hace nada; pedir otra
    /// suelta la anterior al momento, que puede ser mucha memoria.
    Q_INVOKABLE void cargar(const QUrl &url);
    /// Suelta la imagen: al cerrar el visor no hace falta tenerla.
    Q_INVOKABLE void soltar();
    /// `#rrggbb` (o `#rrggbbaa` si no es opaco), o "" fuera de la imagen.
    Q_INVOKABLE QString hex(int x, int y) const;

signals:
    void cambio();

private:
    QUrl m_fuente;
    QImage m_imagen;
    /// Cada carga lleva su número: si llega tarde una que ya no se quiere, se
    /// tira en vez de pisar a la buena.
    quint64 m_vuelta = 0;
};
