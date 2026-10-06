// Una petición a los portales del escritorio (xdg-desktop-portal), que es como
// se le pide a GNOME —o a KDE— algo que es suyo: una captura, elegir una
// carpeta con su propio diálogo.
//
// Todas van igual: se llama a un método, el portal contesta «te lo haré» con
// la ruta de un objeto Request, y la respuesta de verdad llega después por la
// señal `Response` de ese objeto. La ruta se puede saber antes de pedir, y hay
// que escuchar **antes**: si no, una respuesta rápida (cancelar al momento)
// llegaba antes de que hubiera nadie escuchando y se perdía.
#pragma once
#include <QObject>
#include <functional>
#include <QString>
#include <QVariantList>
#include <QVariantMap>

class PeticionPortal : public QObject {
    Q_OBJECT
public:
    /// Pide `metodo` de `interfaz` (sin el prefijo org.freedesktop.portal.)
    /// con `args` delante de las `opciones`. Devuelve null si no hay portal;
    /// si no, la petición se borra sola al contestar.
    static PeticionPortal *pedir(const QString &interfaz, const QString &metodo,
                                 const QVariantList &args, QVariantMap opciones,
                                 QObject *padre);

signals:
    /// 0 hecho, 1 cancelado por quien está delante, otro número: error.
    void respuesta(uint codigo, const QVariantMap &resultados);

private slots:
    void recibir(uint codigo, const QVariantMap &resultados);

private:
    using QObject::QObject;
    QString m_ruta;
};

/// Pide una carpeta con el diálogo del escritorio (o zenity, si no hay
/// portal) y llama a `listo` con su ruta. Si se cancela no llama. Devuelve
/// false si no hay ninguna forma de preguntar.
bool elegirCarpeta(QObject *dueno, const QString &titulo, std::function<void(const QString &)> listo);

/// Igual, pero archivos, varios a la vez.
bool elegirArchivos(QObject *dueno, const QString &titulo, std::function<void(const QStringList &)> listo);
