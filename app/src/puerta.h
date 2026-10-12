// La puerta: por donde el navegador mete cosas en la biblioteca abierta.
//
// La extensión del navegador no habla con la aplicación directamente: el
// navegador lanza `grim` como «programa nativo» (native messaging), y `grim`
// llama aquí. Esto es un socket local —un archivo en `$XDG_RUNTIME_DIR`, que
// solo puede abrir el mismo usuario; una tubería con nombre en Windows—, no un
// puerto: un servidor en `localhost` lo puede llamar cualquier página web que
// se visite, y esto no.
//
// Una conexión, una petición: una línea de JSON y una de respuesta.
//
//   {"accion":"hola"}       → {"ok":true,"biblioteca":"…","version":"…"}
//   {"accion":"carpetas"}   → {"ok":true,"carpetas":[{"id","nombre","nivel"}]}
//   {"accion":"traer", "url" | "datos"+"tipo", "origen", "nombre", "carpeta"}
//                           → {"ok":true,"estado":"guardada"|"repetida","id","nombre"}
//                             o {"ok":false,"error":"…"}
//   {"accion":"mostrar","id"} → {"ok":true}: enseña ese elemento en la ventana
//
// «traer» contesta cuando ha acabado de verdad —entró, ya estaba o falló—, no
// cuando empieza: así el navegador puede decir lo que pasó. Si tarda más de
// la cuenta, contesta que sigue en camino.
#pragma once
#include <QHash>
#include <QObject>
#include <QPointer>
#include <QString>
#include <QVariantMap>

class Nucleo;
class Traer;
class QLocalServer;
class QLocalSocket;
class QTimer;

class Puerta : public QObject {
    Q_OBJECT
    /// Si está escuchando: lo que el navegador mande llega aquí.
    Q_PROPERTY(bool abierta READ abierta CONSTANT)
    /// La carpeta de la extensión, para cargarla en el navegador.
    Q_PROPERTY(QString carpetaExtension READ carpetaExtension CONSTANT)
public:
    /// Sin `escuchar` no abre el socket: las pasadas automáticas no pueden
    /// quedarse con lo que mande el navegador de quien usa el programa.
    Puerta(Nucleo *nucleo, Traer *traer, const QString &version, bool escuchar,
           QObject *padre = nullptr);

    QString carpetaExtension() const;
    /// Registra `grim` en los navegadores instalados (`grim navegador
    /// instalar`) y devuelve lo que dijo, para enseñarlo.
    Q_INVOKABLE QString conectarNavegadores();
    Q_INVOKABLE void abrirCarpetaExtension();

signals:
    /// El navegador pide ver un elemento («ver en Grimorio»).
    void mostrar(const QString &id);

public:

    /// Dónde escucha. `grim` calcula lo mismo por su cuenta: ver
    /// `nativo::ruta_puerta` en crates/grimorio-cli.
    static QString nombre();

    /// Si consiguió abrirla. Con dos ventanas abiertas, la primera se la queda.
    bool abierta() const;

private:
    bool intentarAbrir();
    void atender(QLocalSocket *s);
    /// La respuesta, o vacía si se contestará más tarde (ver `pendientes`).
    QByteArray responder(const QByteArray &linea, QLocalSocket *s);
    void contestar(QLocalSocket *s, const QByteArray &respuesta);
    /// Lo que se trae esperando a que acabe, por su etiqueta.
    void acabado(const QVariantMap &resultado);
    void fallido(const QString &etiqueta, const QString &mensaje);

    Nucleo *m_nucleo;
    Traer *m_traer;
    QString m_version;
    QLocalServer *m_servidor;
    QTimer *m_reintento = nullptr;
    QHash<QString, QPointer<QLocalSocket>> m_pendientes;
};
