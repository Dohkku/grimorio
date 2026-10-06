// Elegir una carpeta del disco para vigilarla. Lo demás —qué se vigila, el
// hilo que mira— vive en el núcleo (vigiladas.rs) y en el puente.
#pragma once
#include <QObject>
#include <QString>

class Nucleo;

class Vigilancia : public QObject {
    Q_OBJECT
public:
    Vigilancia(Nucleo *nucleo, QObject *padre = nullptr) : QObject(padre), m_nucleo(nucleo) {}

    /// Pregunta qué carpeta del disco vigilar; lo nuevo entrará en la
    /// carpeta de la biblioteca `carpeta` ("" es ninguna).
    Q_INVOKABLE void vigilar(const QString &carpeta);

signals:
    void aviso(const QString &mensaje, bool error);

private:
    Nucleo *m_nucleo;
};
