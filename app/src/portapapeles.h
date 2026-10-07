// Lo que sale del programa y lo que entra por fuera de la importación: el
// portapapeles y el arrastre hacia otras aplicaciones.
//
// QML no tiene portapapeles propio (el de Controls solo copia texto de un
// campo) ni sabe empezar un arrastre del sistema: su `Drag` sirve para dentro
// de la ventana. Copiar unos archivos y arrastrarlos fuera llevan además el
// mismo paquete, así que viven juntos.
#pragma once
#include <QList>
#include <QObject>
#include <QString>
#include <QUrl>

class QQuickItem;

class Portapapeles : public QObject {
    Q_OBJECT
public:
    using QObject::QObject;

    Q_INVOKABLE void copiarTexto(const QString &texto);

    /// Copia unos archivos como los copia el gestor de archivos: pegarlos en
    /// Nautilus los copia, en un editor de texto da sus rutas. Si es una sola
    /// imagen va también la imagen, para pegarla en GIMP, en un chat o en la
    /// web. Devuelve cuántos copió.
    Q_INVOKABLE int copiarArchivos(const QList<QUrl> &urls);

    /// Convierte un arrastre de dentro en uno del sistema, con los archivos,
    /// para soltarlos en otra aplicación. `agarre` es el `MouseArea` que lo
    /// llevaba: se le quita el ratón, porque el arrastre del sistema se queda
    /// con él y si no, al volver, el `MouseArea` creería seguir apretado.
    /// Bloquea hasta que se suelta. Devuelve si hubo arrastre.
    Q_INVOKABLE bool arrastrarFuera(QQuickItem *agarre, const QList<QUrl> &urls,
                                    const QString &previa);
};
