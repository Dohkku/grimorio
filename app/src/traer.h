// Lo que entra de fuera sin pasar por «importar»: pegar con Ctrl+V y lo que se
// suelta en la ventana, venga de un gestor de archivos o de un navegador.
//
// Las dos cosas llegan igual —unos archivos, una imagen suelta, unas URL— y se
// reparten igual:
//
//   archivos del ordenador        se importan, como siempre
//   elementos de esta biblioteca  no se duplican: se añaden a la carpeta que
//                                 se está mirando (copiar en Grimorio y pegar
//                                 en otra carpeta es «ponlo también aquí»)
//   una imagen sin archivo        se guarda y se importa (una captura, «copiar
//                                 imagen» en el navegador)
//   una URL http(s)               se descarga: si es una imagen o un vídeo, eso;
//                                 si es una página, su imagen principal
//                                 (og:image). La URL queda como origen.
#pragma once
#include <QImage>
#include <QList>
#include <QObject>
#include <QString>
#include <QUrl>

class Nucleo;
class QNetworkAccessManager;

class Traer : public QObject {
    Q_OBJECT
public:
    Traer(Nucleo *nucleo, QObject *padre = nullptr);

    /// Lo que haya en el portapapeles, a la carpeta `carpeta` ("" es ninguna).
    Q_INVOKABLE void pegar(const QString &carpeta);
    /// Lo que se soltó en la ventana.
    Q_INVOKABLE void soltar(const QList<QUrl> &urls, const QString &carpeta);
    /// Descarga una URL y la importa con ella como origen.
    Q_INVOKABLE void descargar(const QUrl &url, const QString &carpeta);
    /// Pregunta qué archivos importar (diálogo del escritorio) y los importa.
    Q_INVOKABLE void elegirEImportar(const QString &carpeta);

signals:
    /// Para la barra de estado.
    void aviso(const QString &mensaje, bool error);

private:
    void repartir(const QList<QUrl> &urls, const QString &carpeta);
    /// El id si la ruta es de esta biblioteca (un original o su copia con
    /// nombre en cache/salida), o "".
    QString idPropio(const QString &ruta) const;
    QString dirTemporal() const;
    void guardarImagen(const QImage &img, const QString &carpeta, const QString &origen);
    void bajar(const QUrl &url, const QString &carpeta, const QString &origen, int saltos,
               const QImage &reserva);

    Nucleo *m_nucleo;
    QNetworkAccessManager *m_red;
};
