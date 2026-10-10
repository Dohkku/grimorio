// Cambiar de biblioteca, abrir otra y crear una nueva, desde la ventana.
//
// Cambiar de biblioteca es **volver a arrancar** con la otra, no recargar por
// dentro. El núcleo, el índice, las miniaturas mapeadas, las tuberías de vídeo,
// el cerrojo: todo cuelga de qué biblioteca está abierta, y soltarlo todo a
// mano para coger lo de otra es justo el tipo de cosa que deja un puntero
// colgando. Un proceso nuevo empieza limpio y tarda menos de un segundo.
//
// Antes de irse se comprueba que la otra se puede abrir: que es una biblioteca
// y que no está abierta ya en otra ventana. Si no, el proceso nuevo moriría
// diciéndolo por una consola que nadie ve, y este ya se habría cerrado: la
// persona se quedaría sin ninguna ventana.
#pragma once
#include <QObject>
#include <QSettings>
#include <QStringList>

class Bibliotecas : public QObject {
    Q_OBJECT
    /// Las últimas abiertas, la de ahora primero.
    Q_PROPERTY(QStringList recientes READ recientes NOTIFY cambio)
    Q_PROPERTY(QString actual READ actual CONSTANT)

public:
    /// `extra` son los argumentos que el proceso nuevo tiene que heredar (el
    /// tema, por ejemplo); la biblioteca la pone esta clase.
    Bibliotecas(const QString &actual, const QStringList &extra, QObject *padre = nullptr);

    QStringList recientes() const { return m_recientes; }
    QString actual() const { return m_actual; }

    /// El nombre que se enseña: el de su `library.json`, o el de la carpeta
    /// sin el «.grimorio» si no se puede leer.
    Q_INVOKABLE QString nombreDe(const QString &ruta) const;
    /// Cambia a esa biblioteca (relanzando), si se puede.
    Q_INVOKABLE void abrir(const QString &ruta);
    /// Abre el diálogo de carpetas. Con `nueva`, la carpeta elegida pasa a ser
    /// una biblioteca si no lo era.
    Q_INVOKABLE void elegir(bool nueva);
    /// Quita una de la lista de recientes (no la borra del disco).
    Q_INVOKABLE void olvidar(const QString &ruta);

signals:
    void cambio();
    void aviso(const QString &mensaje, bool error);

private:
    void elegida(const QString &ruta, bool nueva);
    void guardar();

    QSettings m_disco;
    QString m_actual;
    QStringList m_extra;
    QStringList m_recientes;
};
