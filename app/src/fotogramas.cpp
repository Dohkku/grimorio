#include "fotogramas.h"

#include <atomic>

namespace {

// Un contador por salida, colgado de ella: muere con ella y no hay lista que
// limpiar. La señal sale del hilo de la tubería, así que la conexión es directa
// y el número, atómico; en cola habría que copiar el fotograma para entregarlo,
// que es justo lo que se quiere evitar.
class Contador : public QObject {
    Q_OBJECT
public:
    using QObject::QObject;
    std::atomic<int> n{0};
public slots:
    void uno() { n.fetch_add(1, std::memory_order_relaxed); }
};

} // namespace

void Fotogramas::vigilar(QObject *salida)
{
    if (!salida || salida->findChild<Contador *>(QString(), Qt::FindDirectChildrenOnly))
        return;
    auto *c = new Contador(salida);
    connect(salida, SIGNAL(videoFrameChanged(QVideoFrame)), c, SLOT(uno()),
            Qt::DirectConnection);
}

int Fotogramas::cuenta(QObject *salida) const
{
    auto *c = salida ? salida->findChild<Contador *>(QString(), Qt::FindDirectChildrenOnly)
                     : nullptr;
    return c ? c->n.load(std::memory_order_relaxed) : 0;
}

#include "fotogramas.moc"
