// Elegir una carpeta del disco para vincularla. Lo demás —qué está vinculado,
// el hilo que mira— vive en el núcleo (vigiladas.rs) y en el puente.
//
// En la interfaz se dice «vincular» y no «vigilar»: lo que se hace es unir
// una carpeta del disco a la biblioteca para que lo nuevo entre solo. Por
// dentro (archivos, comandos, `vigiladas.json`) se sigue llamando vigilar,
// para no cambiar el formato de las bibliotecas que ya existen.
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
    /// Si la carpeta del disco sigue ahí. Una vinculada en un disco que no
    /// está montado no se pierde: se enseña como «no se encuentra».
    Q_INVOKABLE bool existe(const QString &ruta) const;

signals:
    void aviso(const QString &mensaje, bool error);

private:
    Nucleo *m_nucleo;
};
