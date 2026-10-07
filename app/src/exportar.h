// Sacar copias de los originales a una carpeta del disco.
//
// Con el nombre de cada elemento —no `original.webp`— y sin pisar nada: si en
// la carpeta ya hay un `gato.jpg`, el siguiente sale como `gato (2).jpg`. Se
// copia en otro hilo, porque exportar trescientos vídeos no puede dejar la
// ventana colgada.
#pragma once
#include <QList>
#include <QObject>
#include <QUrl>

class Exportar : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool enMarcha READ enMarcha NOTIFY enMarchaCambio)

public:
    using QObject::QObject;
    bool enMarcha() const { return m_enMarcha; }

    /// Pregunta la carpeta y copia. `urls` son los originales ya con su
    /// nombre (`Modelo::urlsElegidas`).
    Q_INVOKABLE void exportar(const QList<QUrl> &urls);

signals:
    void enMarchaCambio();
    void aviso(const QString &mensaje, bool error);

private:
    void copiar(const QList<QUrl> &urls, const QString &destino);
    bool m_enMarcha = false;
};
