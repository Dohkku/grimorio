// Lo que el visor pide para enseñar un elemento: proxy de vídeo, onda, páginas
// de PDF, malla 3D.
//
// Cada petición corre en un hilo propio y vuelve con `listo`. No pasa por el
// bus del núcleo: el bus tiene un hilo y es el de editar la biblioteca, y
// convertir un ProRes de diez minutos no puede dejar esperando a una estrella.
#pragma once
#include <QHash>
#include <QObject>
#include <QSet>
#include <QString>
#include <QThreadPool>
#include <QUrl>
#include <QVariantMap>

#include <memory>

struct EnlaceDerivados;

class Derivados : public QObject {
    Q_OBJECT
public:
    explicit Derivados(const QString &raiz, QObject *padre = nullptr);
    ~Derivados() override;

    /// Pide un derivado. `que` es `proxy`, `tira`, `onda`, `pdf_medidas`,
    /// `pdf_pagina` o `malla`; `extra` lleva lo que cada uno necesite
    /// (`pagina`, `ancho`, `duracion`, `ext`).
    ///
    /// Devuelve la clave con la que llegará la respuesta. Si ya estaba hecho,
    /// la respuesta llega igual por `listo`, en la siguiente vuelta del bucle
    /// y no dentro de esta llamada: quien pide se puede conectar después.
    Q_INVOKABLE QString pedir(const QString &que, const QString &id, const QUrl &original,
                              const QVariantMap &extra = {});
    /// La respuesta, si ya está. Para no esperar una vuelta cuando se sabe.
    Q_INVOKABLE QVariantMap hecho(const QString &clave) const;
    /// De una ruta de disco a una URL que entienden `Image` y `MediaPlayer`.
    Q_INVOKABLE QUrl url(const QString &ruta) const { return QUrl::fromLocalFile(ruta); }

signals:
    /// `respuesta` lleva `ok` y, si salió bien, `ruta` y lo demás; si no, `error`.
    void listo(const QString &clave, const QVariantMap &respuesta);

private:
    QString m_raiz;
    /// Los repartos van en el montón y no como miembros: el destructor de un
    /// `QThreadPool` espera sin tope a lo que tenga en marcha, y aquí hay
    /// tareas de minutos. Al cerrar se espera un rato acotado y, si algo sigue
    /// sin terminar, el reparto se abandona en vez de colgar el cierre.
    QThreadPool *m_hilos;
    /// Uno aparte para lo que tarda minutos. Sin él, dos vídeos convirtiéndose
    /// ocuparían todo el reparto y las páginas de un PDF esperarían detrás.
    QThreadPool *m_lentos;
    /// Lo que comparten las tareas con este objeto. Sobrevive a `Derivados`
    /// para que una tarea abandonada al cerrar sepa que ya no hay a quién
    /// contestar, sin tocar memoria liberada.
    std::shared_ptr<EnlaceDerivados> m_enlace;
    QHash<QString, QVariantMap> m_hechos;
    QSet<QString> m_enCamino;
};
