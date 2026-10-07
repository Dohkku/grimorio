#include "disposicion.h"

#include "modelo.h"

#include <QTimer>

Disposicion::Disposicion(Modelo *modelo, QObject *padre)
    : QAbstractListModel(padre), m_modelo(modelo)
{
    // Ranuras de sobra desde el principio: el reparto por `indice % ranuras`
    // solo es único mientras quepan todas las celdas visibles a la vez, y
    // crecer obliga a un reset del modelo.
    m_ranuras.resize(384);
    connect(modelo, &Modelo::totalCambio, this, &Disposicion::recalcular);
    connect(modelo, &QAbstractItemModel::dataChanged, this, &Disposicion::refrescarFilas);
    // Una vista nueva cambia **qué** hay en cada sitio aunque los sitios sean
    // los mismos. `repartir` solo avisa de las ranuras cuya posición o medida
    // cambia, así que si en la vista nueva el elemento 0 tenía la misma forma
    // que el de antes, la ranura se quedaba con el id viejo: al cambiar rápido
    // entre dos carpetas, una enseñaba la miniatura de un elemento de la otra,
    // y así se quedaba. Al reiniciarse el modelo se refresca el contenido de
    // todas; lo que además cambie de sitio lo avisa `recalcular` justo después.
    connect(modelo, &QAbstractItemModel::modelReset, this, &Disposicion::refrescarContenido);
    // La vista nueva después de soltar un reordenado ya viene en el orden
    // nuevo: es el momento de soltar la permutación. Sin animar: los índices
    // del modelo han cambiado y cada celda pasa a otra ranura; animado, media
    // malla saldría volando hacia su sitio desde el de otra.
    connect(modelo, &QAbstractItemModel::modelReset, this, [this] {
        if (!m_esperando) return;
        soltarReorden();
    });
}

int Disposicion::enPosicion(int p) const
{
    if (m_arrastrado < 0) return p;
    if (p == m_destino) return m_arrastrado;
    const int j = p < m_destino ? p : p - 1; // índice en la lista sin él
    return j < m_arrastrado ? j : j + 1;
}

void Disposicion::empezarReorden(int indice)
{
    if (indice < 0 || indice >= m_modelo->total()) return;
    m_arrastrado = indice;
    m_origen = indice;
    m_destino = indice;
    m_esperando = false;
    m_animando = true;
    emit reordenCambio();
}

void Disposicion::moverReorden(qreal x, qreal y)
{
    if (m_arrastrado < 0 || m_esperando) return;
    const int n = m_modelo->total();
    const QVector<Fila> &filas = m_filas.filas();
    if (filas.isEmpty()) return;
    // La posición de la lista que se ve ahora (con lo arrastrado en el hueco)
    // delante de la cual caería.
    int ins = 0;
    if (y < filas.first().y) {
        ins = 0;
    } else {
        const int f = qBound(0, m_filas.filaEn(y), int(filas.size()) - 1);
        const Fila &fila = filas.at(f);
        if (y > fila.y + fila.alto + m_hueco && f == filas.size() - 1) {
            ins = n;
        } else if (m_modo == Lista) {
            // Una fila, un elemento: delante o detrás según la mitad de alto.
            ins = y < fila.y + fila.alto / 2 ? fila.inicio : fila.inicio + 1;
        } else {
            const auto prop = proporcion();
            ins = fila.inicio + fila.n;
            qreal cx = m_margen;
            for (int k = 0; k < fila.n; ++k) {
                const qreal w = m_filas.anchoEn(fila, fila.inicio + k, prop);
                if (x < cx + w / 2) {
                    ins = fila.inicio + k;
                    break;
                }
                cx += w + m_hueco;
            }
        }
    }
    // Pasado a la lista final: lo que va detrás del hueco actual corre uno.
    int destino = ins > m_destino ? ins - 1 : ins;
    destino = qBound(0, destino, n - 1);
    if (destino == m_destino) return;
    m_destino = destino;
    recalcular();
}

void Disposicion::devolverReorden()
{
    if (m_arrastrado < 0 || m_esperando || m_destino == m_origen) return;
    m_destino = m_origen;
    recalcular();
}

bool Disposicion::reordenMovido() const
{
    return m_arrastrado >= 0 && m_destino != m_origen;
}

QVariantMap Disposicion::vecinosReorden() const
{
    QVariantMap m;
    const int n = m_modelo->total();
    m[QStringLiteral("antes")] = m_destino > 0 ? m_modelo->idDe(enPosicion(m_destino - 1)) : QString();
    m[QStringLiteral("despues")] = m_destino < n - 1 ? m_modelo->idDe(enPosicion(m_destino + 1)) : QString();
    return m;
}

void Disposicion::acabarReorden(bool guardado)
{
    if (m_arrastrado < 0) return;
    if (!guardado || m_destino == m_origen) {
        // Sin cambio, de vuelta y animado: es lo que se espera al soltar
        // donde no hay sitio nuevo.
        m_destino = m_origen;
        recalcular();
        m_arrastrado = -1;
        emit reordenCambio();
        QTimer::singleShot(300, this, [this] {
            if (m_arrastrado < 0 && m_animando) {
                m_animando = false;
                emit reordenCambio();
            }
        });
        return;
    }
    m_esperando = true;
    emit reordenCambio();
    // Por si la vista nueva no llega (el núcleo dio error): no dejar la malla
    // con una permutación colgada para siempre.
    QTimer::singleShot(4000, this, [this] {
        if (m_esperando) soltarReorden();
    });
}

void Disposicion::soltarReorden()
{
    m_animando = false;
    m_esperando = false;
    m_arrastrado = -1;
    m_origen = m_destino = -1;
    emit reordenCambio();
    recalcular();
}

Filas::Proporcion Disposicion::proporcion() const
{
    // Por posición: reordenando, en la posición `p` va otro elemento.
    return [this](int p) {
        const int i = enPosicion(p);
        const uint32_t h = m_modelo->altoDe(i);
        return h == 0 ? 1.0 : qreal(m_modelo->anchoDe(i)) / qreal(h);
    };
}

int Disposicion::rowCount(const QModelIndex &padre) const
{
    return padre.isValid() ? 0 : int(m_ranuras.size());
}

QVariant Disposicion::data(const QModelIndex &idx, int papel) const
{
    const int k = idx.row();
    if (k < 0 || k >= m_ranuras.size()) return {};
    const Ranura &r = m_ranuras.at(k);
    switch (papel) {
    case Indice: return r.indice;
    case X: return r.x;
    case Y: return r.y;
    case Ancho_: return r.w;
    case Alto_: return r.h;
    case Activo: return r.indice >= 0;
    case Dominante: return m_modelo->dominanteDe(r.indice);
    case TieneMiniatura: return m_modelo->tieneMiniaturaDe(r.indice);
    case Elegido: return m_modelo->elegidoDe(r.indice);
    case Estrellas: return m_modelo->estrellasDe(r.indice);
    case IdItem: return m_modelo->idDe(r.indice);
    case Familia: return m_modelo->familiaDe(r.indice);
    case Duracion: return m_modelo->duracionDe(r.indice);
    case Adulto: return r.indice >= 0 && m_modelo->adultoDe(r.indice);
    // Un solo papel y no la conjunción de dos en QML, por una razón medida.
    // Las propiedades de un delegado se actualizan de una en una, así que al
    // reasignar una ranura había un instante en que la celda ya estaba activa
    // pero todavía decía no tener miniatura: el `Loader` del hueco se creaba y
    // se destruía en cada reasignación. Eran 62 fps en vez de 97.
    case SinDibujo: return r.indice >= 0 && !m_modelo->tieneMiniaturaDe(r.indice);
    // La cadena solo se construye si alguien la lee: la celda la mira cuando
    // hay pie o lista, no en la malla desnuda.
    case Nombre: return r.indice >= 0 ? m_modelo->nombreDe(r.indice) : QString();
    default: return {};
    }
}

QHash<int, QByteArray> Disposicion::roleNames() const
{
    return {
        { Indice, "celdaIndice" },  { X, "celdaX" },           { Y, "celdaY" },
        { Ancho_, "celdaAncho" },   { Alto_, "celdaAlto" },    { Activo, "celdaActiva" },
        { Dominante, "dominante" }, { TieneMiniatura, "tieneMiniatura" },
        { Elegido, "elegido" },     { Estrellas, "estrellas" },
        { IdItem, "idItem" },       { Familia, "familia" },
        { Duracion, "duracion" },   { SinDibujo, "sinDibujo" },
        { Adulto, "adulto" },       { Nombre, "nombre" },
    };
}

void Disposicion::setModo(Modo m)
{
    if (m_modo == m) return;
    m_modo = m;
    emit modoCambio();
    recalcular();
}

void Disposicion::setObjetivo(qreal v)
{
    v = qBound(80.0, v, 420.0);
    if (qFuzzyCompare(m_objetivo, v)) return;
    m_objetivo = v;
    recalcular();
}

void Disposicion::setAncho(qreal v)
{
    // Medio píxel de tolerancia: redimensionar una ventana emite decenas de
    // cambios por segundo y rehacer catorce mil filas en cada uno se nota.
    if (qAbs(m_ancho - v) < 0.5) return;
    m_ancho = v;
    recalcular();
}

void Disposicion::setMargen(qreal v)
{
    if (qFuzzyCompare(m_margen, v)) return;
    m_margen = v;
    recalcular();
}

void Disposicion::setHueco(qreal v)
{
    if (qFuzzyCompare(m_hueco, v)) return;
    m_hueco = v;
    recalcular();
}

void Disposicion::setPie(qreal v)
{
    if (qFuzzyCompare(m_pie + 1, v + 1)) return;
    m_pie = v;
    recalcular();
}

void Disposicion::setAltoLista(qreal v)
{
    if (qFuzzyCompare(m_altoLista, v)) return;
    m_altoLista = v;
    recalcular();
}

void Disposicion::recalcular()
{
    // En lista las filas van pegadas —la raya de debajo las separa— y no hay
    // pie: el nombre ya va dentro de la fila.
    const bool lista = m_modo == Lista;
    m_filas.calcular(lista ? Filas::Lista
                           : (m_modo == Cuadricula ? Filas::Cuadricula : Filas::Justificado),
                     m_modelo->total(), m_ancho, lista ? m_altoLista : m_objetivo, m_margen,
                     lista ? 0 : m_hueco, proporcion(), lista ? 0 : m_pie);

    // Repartir de nuevo sobre la ventana que ya se estaba mirando, en vez de
    // vaciar las ranuras y esperar a que la vista pida.
    //
    // Vaciar era lo que hacía antes, y costaba un parpadeo de la malla entera
    // cada vez que cambiaba algo en lote: cada celda visible perdía su imagen,
    // volvía a pedirla y volvía a hacer su aparición. Como `repartir` compara
    // ranura por ranura, si la geometría no ha cambiado no se avisa de nada.
    //
    // El orden sigue importando: primero repartir y después avisar. La vista
    // responde a `geometriaCambio` llamando a `mirar` en el acto, y si se
    // repartiera después se borraría justo lo que se acaba de colocar.
    repartir(m_y0, m_y1);
    emit geometriaCambio();
}

// Lo que de una ranura depende del elemento que tiene, y no de dónde está.
//
// Una sola lista para los dos avisos. Eran dos copias a mano, y `Adulto` no
// estaba en ninguna: marcar algo como +18 cambiaba el panel al momento, pero
// la celda seguía sin difuminar hasta que algo rehacía la malla entera.
static const QList<int> &papelesDeContenido()
{
    static const QList<int> p = { Disposicion::Dominante, Disposicion::TieneMiniatura,
                                  Disposicion::Elegido,   Disposicion::Estrellas,
                                  Disposicion::IdItem,    Disposicion::Familia,
                                  Disposicion::Duracion,  Disposicion::Adulto,
                                  Disposicion::SinDibujo, Disposicion::Nombre };
    return p;
}

// Lo que de una ranura depende de dónde está y no de qué tiene.
static const QList<int> &papelesDeGeometria()
{
    static const QList<int> p = { Disposicion::X, Disposicion::Y, Disposicion::Ancho_,
                                  Disposicion::Alto_ };
    return p;
}

void Disposicion::refrescarContenido()
{
    if (m_ranuras.isEmpty()) return;
    emit dataChanged(index(0, 0), index(int(m_ranuras.size()) - 1, 0), papelesDeContenido());
}

// El modelo avisa por elemento; aquí se sirve por ranura.
//
// Este atajo ignoraba de qué elementos se avisaba y avisaba de todas las
// ranuras y de todos los papeles. Elegir con las flechas manda dos avisos —el
// que se deja y el que se toma—, así que cada tecla rehacía las ataduras de las
// cuatrocientas ranuras dos veces. Medido paseando con las flechas por una
// biblioteca de fotos: 262 ms de CPU por tecla, con la ventana arrastrándose
// detrás.
//
// Los papeles no se traducen uno a uno —los del modelo y los de aquí no son los
// mismos— y no hace falta: sobre dos ranuras da igual avisar de más.
void Disposicion::refrescarFilas(const QModelIndex &desde, const QModelIndex &hasta,
                                 const QList<int> &)
{
    if (m_ranuras.isEmpty()) return;
    const int a = desde.row();
    const int b = hasta.row();
    // Un aviso más ancho que las ranuras que hay: buscar cuáles caen dentro
    // cuesta más que darlas todas por cambiadas.
    if (b - a + 1 >= int(m_ranuras.size())) {
        refrescarContenido();
        return;
    }
    for (int k = 0; k < m_ranuras.size(); ++k) {
        const int i = m_ranuras[k].indice;
        if (i < a || i > b) continue;
        emit dataChanged(index(k, 0), index(k, 0), papelesDeContenido());
    }
}

void Disposicion::colocarFila(const Fila &f, QVector<Ranura> &destino) const
{
    const auto prop = proporcion();
    qreal x = m_margen;
    for (int k = 0; k < f.n; ++k) {
        const int p = f.inicio + k;
        const qreal w = m_filas.anchoEn(f, p, prop);
        // La ranura es la del elemento, no la del sitio (ver «reordenar» en el
        // .h). Sin reordenar es lo mismo: elemento y sitio coinciden.
        const int i = enPosicion(p);
        Ranura &r = destino[i % destino.size()];
        r.indice = i;
        r.x = x;
        r.y = f.y;
        r.w = w;
        r.h = f.alto;
        x += w + m_hueco;
    }
}

void Disposicion::vaciar()
{
    bool alguna = false;
    for (int k = 0; k < m_ranuras.size(); ++k) {
        if (m_ranuras[k].indice < 0) continue;
        m_ranuras[k].indice = -1;
        emit dataChanged(index(k, 0), index(k, 0));
        alguna = true;
    }
    Q_UNUSED(alguna);
}

void Disposicion::asegurarRanuras(int hacenFalta)
{
    if (hacenFalta <= m_ranuras.size()) return;
    // Con holgura: crecer de una en una haría un aviso por cada píxel de
    // ventana ganado.
    const int nuevo = hacenFalta + hacenFalta / 2;
    // Añadir filas y no reiniciar el modelo. Con un reinicio, alejar el zoom lo
    // bastante para que no cupieran las ranuras destruía todas las celdas —más
    // de trescientas ochenta— y las volvía a crear, cada una pidiendo otra vez
    // su miniatura. Así solo se crean las nuevas; las que ya estaban se quedan
    // con lo que tenían, y `repartir`, que viene justo detrás, compara ranura
    // por ranura y avisa de las que cambian de elemento.
    const int antes = int(m_ranuras.size());
    beginInsertRows(QModelIndex(), antes, nuevo - 1);
    m_ranuras.resize(nuevo);
    endInsertRows();
}

void Disposicion::mirar(qreal y0, qreal y1)
{
    m_y0 = y0;
    m_y1 = y1;
    repartir(y0, y1);
}

void Disposicion::repartir(qreal y0, qreal y1)
{
    if (m_ranuras.isEmpty() && m_filas.vacio()) return;
    if (m_filas.vacio()) {
        vaciar();
        return;
    }
    const QVector<Fila> &filas = m_filas.filas();

    const int desde = m_filas.filaEn(y0);
    int hasta = desde;
    int cuantas = 0;
    while (hasta < filas.size() && filas.at(hasta).y <= y1) {
        cuantas += filas.at(hasta).n;
        ++hasta;
    }
    // La ventana se ha quedado fuera del contenido: la biblioteca encogió por
    // debajo. Dejar las ranuras como están enseñaría elementos que ya no hay.
    if (cuantas == 0) {
        vaciar();
        return;
    }
    asegurarRanuras(cuantas);

    // Se calcula el reparto nuevo aparte y luego se compara: así solo se avisa
    // de las ranuras que de verdad cambian, que al desplazarse son unas pocas
    // por fotograma en vez de todas.
    QVector<Ranura> nuevas(m_ranuras.size());
    for (Ranura &r : nuevas) r.indice = -1;
    for (int f = desde; f < hasta; ++f) colocarFila(filas.at(f), nuevas);

    for (int k = 0; k < m_ranuras.size(); ++k) {
        const Ranura &a = m_ranuras.at(k);
        const Ranura &b = nuevas.at(k);
        if (a.indice == b.indice && qFuzzyCompare(a.y, b.y) && qFuzzyCompare(a.x, b.x)
            && qFuzzyCompare(a.w, b.w) && qFuzzyCompare(a.h, b.h))
            continue;
        const bool mismoElemento = a.indice == b.indice;
        m_ranuras[k] = b;
        // Si la ranura sigue con el mismo elemento y solo se ha movido —un paso
        // de zoom, la ventana que cambia de ancho, un reordenado que la
        // aparta—, se avisa solo de la geometría. Sin papeles, cada celda
        // volvía a leer los dieciséis, y unos cuantos cuestan: el id, el nombre
        // y el parche construyen cadenas desde la vista. Con un elemento nuevo
        // sí cambia todo y se avisa de todo.
        if (mismoElemento)
            emit dataChanged(index(k, 0), index(k, 0), papelesDeGeometria());
        else
            emit dataChanged(index(k, 0), index(k, 0));
    }
}

qreal Disposicion::yDe(int indice) const
{
    return m_filas.yDe(indice);
}

qreal Disposicion::altoDe(int indice) const
{
    return m_filas.altoDe(indice);
}

QVariantMap Disposicion::sitioDe(int indice) const
{
    QVariantMap m;
    if (indice < 0 || m_filas.filaDe(indice) < 0) return m;
    const auto prop = proporcion();
    m[QStringLiteral("x")] = m_filas.xDe(indice, prop);
    m[QStringLiteral("y")] = m_filas.yDe(indice);
    m[QStringLiteral("ancho")] = m_filas.anchoDe(indice, prop);
    m[QStringLiteral("alto")] = m_filas.altoDe(indice);
    return m;
}


int Disposicion::vecinoVertical(int indice, int direccion) const
{
    return m_filas.vecinoVertical(indice, direccion, proporcion());
}
