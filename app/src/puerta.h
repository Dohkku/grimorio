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
//                           → {"ok":true}
//
// «traer» contesta en cuanto empieza: bajar e importar puede tardar, y lo que
// pase después se cuenta en la barra de estado, como cualquier importación.
#pragma once
#include <QObject>
#include <QString>

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

    /// Dónde escucha. `grim` calcula lo mismo por su cuenta: ver
    /// `nativo::ruta_puerta` en crates/grimorio-cli.
    static QString nombre();

    /// Si consiguió abrirla. Con dos ventanas abiertas, la primera se la queda.
    bool abierta() const;

private:
    bool intentarAbrir();
    void atender(QLocalSocket *s);
    QByteArray responder(const QByteArray &linea);

    Nucleo *m_nucleo;
    Traer *m_traer;
    QString m_version;
    QLocalServer *m_servidor;
    QTimer *m_reintento = nullptr;
};
