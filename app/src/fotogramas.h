// Cuántos fotogramas llegan a la salida de un reproductor, contados aquí.
//
// El rescate de la malla necesita saberlo, y escucharlo desde QML se llevaba la
// tarjeta por delante. Cada aviso de `videoFrameChanged` entrega el fotograma a
// JavaScript como un valor, y ese valor sujeta la textura o el búfer del
// decodificador hasta que pasa el recolector de basura; el recolector no ve esa
// memoria, así que no tiene prisa. Medido con un vídeo 1080p en bucle y un
// manejador vacío: la memoria de vídeo subía 250 MB por segundo —una textura
// por fotograma— y en cuatro minutos el programa tenía 13,5 GB de la tarjeta y
// 14 GB de RAM, iba a saltos y moría. Sin manejador, plana.
//
// Aquí la conexión va por el nombre de la señal y el hueco no recibe el
// fotograma, así que nada lo retiene. Y por el nombre a propósito: con la firma
// escrita habría que enlazar QtMultimedia, que es opcional (ver
// `Reproductor.qml`).
#pragma once
#include <QObject>

class Fotogramas : public QObject {
    Q_OBJECT
public:
    using QObject::QObject;

    /// Empieza a contar los de un `QVideoSink`. Llamarlo dos veces con el
    /// mismo no cuenta doble.
    Q_INVOKABLE void vigilar(QObject *salida);
    /// Los que han llegado desde que se vigila. 0 si no se vigila.
    Q_INVOKABLE int cuenta(QObject *salida) const;
};
