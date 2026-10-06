#include "modelo.h"

#include "nombres.h"
#include "nucleo.h"

#include <QColor>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSet>
#include <algorithm>
#include <filesystem>

Modelo::Modelo(Nucleo *nucleo, QObject *padre) : QAbstractListModel(padre), m_nucleo(nucleo)
{
    connect(nucleo, &Nucleo::vistaNueva, this, &Modelo::recoger);
    connect(nucleo, &Nucleo::itemCambiado, this, &Modelo::refrescarItem);
}

Modelo::~Modelo()
{
    soltarVista();
}

void Modelo::soltarVista()
{
    QMutexLocker l(&m_cerrojoVista);
    if (m_vista) {
        grim_vista_soltar(m_vista);
        m_vista = nullptr;
    }
}

void Modelo::recoger()
{
    if (!m_nucleo->crudo()) return;

    // Qué estaba elegido, por id: los índices no sobreviven a una consulta
    // nueva, pero la selección de quien está trabajando sí debería.
    QStringList elegidosAntes = seleccion();
    const QString actualAntes = idDe(m_actual);

    beginResetModel();
    // La vista nueva ya trae los cambios: el parche deja de hacer falta.
    m_parche.clear();
    soltarVista();
    {
        QMutexLocker l(&m_cerrojoVista);
        m_vista = grim_vista(m_nucleo->crudo());
    }
    m_n = m_vista ? int(grim_vista_n(m_vista)) : 0;
    m_elegidos.clear();
    m_ancla = -1;
    for (const QString &id : elegidosAntes) {
        const QByteArray b = id.toUtf8();
        const size_t i = grim_vista_indice_de(m_vista, reinterpret_cast<const uint8_t *>(b.constData()),
                                              uint32_t(b.size()));
        if (i != SIZE_MAX) m_elegidos.insert(int(i));
    }
    m_actual = -1;
    if (!actualAntes.isEmpty()) {
        const QByteArray b = actualAntes.toUtf8();
        const size_t i = grim_vista_indice_de(m_vista, reinterpret_cast<const uint8_t *>(b.constData()),
                                              uint32_t(b.size()));
        if (i != SIZE_MAX) m_actual = int(i);
    }
    // El elemento que estaba en foco puede no estar en la vista nueva mientras
    // otros de la selección sí. Sin esto, el panel de detalle decía "nada
    // elegido" con la barra de estado diciendo "1 elegido" justo al lado.
    if (m_actual < 0 && !m_elegidos.isEmpty()) {
        m_actual = *std::min_element(m_elegidos.begin(), m_elegidos.end());
    }
    endResetModel();

    emit totalCambio();
    emit seleccionCambio();
    emit actualCambio();
    emit focoCambio();
}

void Modelo::refrescarItem(const QString &id, const QVariantMap &estado)
{
    if (!m_vista) return;
    m_parche.insert(id, estado);
    const QByteArray b = id.toUtf8();
    const size_t i = grim_vista_indice_de(m_vista, reinterpret_cast<const uint8_t *>(b.constData()),
                                          uint32_t(b.size()));
    // Puede no estar en esta vista —se acaba de etiquetar algo que el filtro de
    // ahora no enseña— y eso no es un error: el parche se guarda igual por si
    // vuelve.
    if (i == SIZE_MAX) return;
    const QModelIndex idx = index(int(i), 0);
    emit dataChanged(idx, idx);
    if (int(i) == m_actual) emit focoCambio();
}

int Modelo::rowCount(const QModelIndex &padre) const
{
    return padre.isValid() ? 0 : m_n;
}

QString Modelo::idEn(size_t i) const
{
    uint32_t len = 0;
    const uint8_t *p = grim_vista_id(m_vista, i, &len);
    return p ? QString::fromUtf8(reinterpret_cast<const char *>(p), int(len)) : QString();
}

/// El parche de un elemento, si lo tiene.
///
/// Mirar el parche pide el id, y el id es una cadena que hay que construir
/// desde la vista: una reserva de memoria y una conversión de UTF-8 por papel y
/// por celda, en cada aviso. Casi siempre el parche está vacío —solo se llena
/// entre una edición y la vista nueva que la trae—, así que antes de construir
/// nada se pregunta si hay algo que buscar.
const QVariantMap *Modelo::parcheEn(size_t i) const
{
    if (m_parche.isEmpty()) return nullptr;
    const auto p = m_parche.constFind(idEn(i));
    return p != m_parche.constEnd() ? &p.value() : nullptr;
}

QVariant Modelo::data(const QModelIndex &idx, int papel) const
{
    const int i = idx.row();
    if (!m_vista || i < 0 || i >= m_n) return {};
    const size_t u = size_t(i);

    switch (papel) {
    case Indice:
        return i;
    case Id:
        return idEn(u);
    case Ancho:
        return grim_vista_ancho(m_vista, u);
    case Alto:
        return grim_vista_alto(m_vista, u);
    case Proporcion: {
        const uint32_t h = grim_vista_alto(m_vista, u);
        return h == 0 ? 1.0 : double(grim_vista_ancho(m_vista, u)) / double(h);
    }
    case Dominante: {
        const uint32_t c = grim_vista_dominante(m_vista, u);
        // 0xFFFFFFFF es el centinela de "sin paleta"; QML lo distingue por
        // alfa cero y pinta el color de marcador del tema.
        if (c == 0xFFFFFFFFu) return QColor(Qt::transparent);
        return QColor::fromRgb((c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF);
    }
    case Nombre: {
        if (const QVariantMap *parche = parcheEn(u))
            return parche->value(QStringLiteral("nombre")).toString();
        uint32_t len = 0;
        const uint8_t *p = grim_vista_nombre(m_vista, u, &len);
        return p ? QString::fromUtf8(reinterpret_cast<const char *>(p), int(len)) : QString();
    }
    case Estrellas: {
        if (const QVariantMap *p = parcheEn(u)) return p->value(QStringLiteral("estrellas")).toInt();
        return int(grim_vista_estrellas(m_vista, u));
    }
    case Familia: return int(grim_vista_familia(m_vista, u));
    case Duracion: return int(grim_vista_duracion(m_vista, u));
    case Adulto: {
        // Con parche, como las estrellas: marcar algo tiene que verse en la
        // celda antes de que llegue la vista nueva.
        const QVariantMap *p = parcheEn(u);
        if (p && p->contains(QStringLiteral("adulto")))
            return p->value(QStringLiteral("adulto")).toBool();
        return grim_vista_adulto(m_vista, u) != 0;
    }
    case TieneMiniatura: {
        uint32_t len = 0;
        return grim_vista_thumb(m_vista, u, &len) != nullptr;
    }
    case Elegido:
        return m_elegidos.contains(i);
    default:
        return {};
    }
}

QHash<int, QByteArray> Modelo::roleNames() const
{
    return {
        { Indice, "indice" },       { Id, "idElemento" },
        { Ancho, "ancho" },         { Alto, "alto" },
        { Proporcion, "proporcion" }, { Dominante, "dominante" },
        { Nombre, "nombre" },       { Estrellas, "estrellas" },
        { TieneMiniatura, "tieneMiniatura" }, { Elegido, "elegido" },
        { Familia, "familia" },     { Duracion, "duracion" },
        { Adulto, "adulto" },
    };
}

// ------------------------------------------------------------------ selección

QString Modelo::idDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return {};
    return idEn(size_t(i));
}

uint32_t Modelo::anchoDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return 0;
    return grim_vista_ancho(m_vista, size_t(i));
}

uint32_t Modelo::altoDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return 0;
    return grim_vista_alto(m_vista, size_t(i));
}

QColor Modelo::dominanteDe(int i) const
{
    return data(index(i, 0), Dominante).value<QColor>();
}

bool Modelo::tieneMiniaturaDe(int i) const
{
    return data(index(i, 0), TieneMiniatura).toBool();
}

bool Modelo::elegidoDe(int i) const
{
    return m_elegidos.contains(i);
}

QString Modelo::nombreDe(int i) const
{
    return data(index(i, 0), Nombre).toString();
}

int Modelo::estrellasDe(int i) const
{
    return data(index(i, 0), Estrellas).toInt();
}

bool Modelo::adultoDe(int i) const
{
    return data(index(i, 0), Adulto).toBool();
}

QUrl Modelo::urlDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return {};
    uint8_t buf[4096];
    const uint32_t n = grim_vista_ruta(m_vista, size_t(i), buf, sizeof(buf));
    if (n == 0) return {};
    return QUrl::fromLocalFile(QString::fromUtf8(reinterpret_cast<const char *>(buf), int(n)));
}

/// El original con el nombre del elemento, para que salga así del programa.
///
/// Dentro de la biblioteca cada original se llama `original.<ext>`, y copiado
/// o arrastrado a una carpeta llegaba con ese nombre: diez fotos pegadas eran
/// diez `original.webp` peleándose. Aquí se le da un nombre de verdad con un
/// enlace duro en la caché de la biblioteca —no ocupa nada ni copia nada— en
/// `cache/salida/<id>/<nombre>.<ext>`. Si el disco no deja (el original vive
/// en otro, importado sin copiar), sale el original tal cual.
QUrl Modelo::urlConNombre(int i) const
{
    const QUrl original = urlDe(i);
    const QString raiz = m_nucleo ? m_nucleo->raiz() : QString();
    if (!original.isLocalFile() || raiz.isEmpty()) return original;

    const QString nombre = nombreDeArchivo(nombreDe(i), QString());
    if (nombre.isEmpty()) return original;
    const QString ext = extDe(i);
    const QString archivo = ext.isEmpty() || nombre.endsWith(QLatin1Char('.') + ext, Qt::CaseInsensitive)
                                ? nombre
                                : nombre + QLatin1Char('.') + ext;
    const QString dir = raiz + QStringLiteral("/cache/salida/") + idDe(i);
    const QString destino = dir + QLatin1Char('/') + archivo;

    if (!QFileInfo::exists(destino)) {
        // Un nombre viejo de este mismo elemento —se renombró— se va.
        QDir(dir).removeRecursively();
        if (!QDir().mkpath(dir)) return original;
        // `std::filesystem` y no `::link`, que es de POSIX: en Windows el
        // enlace duro también existe (NTFS), con otra llamada. Si falla no se
        // copia: un vídeo de gigas copiado aquí congelaría la interfaz.
        std::error_code error;
        std::filesystem::create_hard_link(QFileInfo(original.toLocalFile()).filesystemFilePath(),
                                          QFileInfo(destino).filesystemFilePath(), error);
        if (error) return original;
    }
    return QUrl::fromLocalFile(destino);
}

QList<int> Modelo::indicesElegidos() const
{
    QList<int> orden(m_elegidos.cbegin(), m_elegidos.cend());
    std::sort(orden.begin(), orden.end());
    return orden;
}

QStringList Modelo::idsElegidos() const
{
    QStringList ids;
    for (int i : indicesElegidos()) ids << idDe(i);
    return ids;
}

QList<QUrl> Modelo::urlsElegidas() const
{
    QList<int> orden(m_elegidos.cbegin(), m_elegidos.cend());
    std::sort(orden.begin(), orden.end());
    QList<QUrl> urls;
    urls.reserve(orden.size());
    for (int i : orden) {
        const QUrl u = urlConNombre(i);
        if (u.isValid()) urls.append(u);
    }
    return urls;
}

QString Modelo::previaDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return {};
    uint8_t buf[4096];
    const uint32_t n = grim_vista_previa(m_vista, size_t(i), buf, sizeof(buf));
    return n ? QString::fromUtf8(reinterpret_cast<const char *>(buf), int(n)) : QString();
}

int Modelo::familiaDe(int i) const
{
    return data(index(i, 0), Familia).toInt();
}

QString Modelo::extDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return {};
    uint32_t len = 0;
    const uint8_t *p = grim_vista_ext(m_vista, size_t(i), &len);
    return p ? QString::fromUtf8(reinterpret_cast<const char *>(p), int(len)) : QString();
}

int Modelo::duracionDe(int i) const
{
    return data(index(i, 0), Duracion).toInt();
}

double Modelo::pesoDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return 0;
    return double(grim_vista_peso(m_vista, size_t(i)));
}

QDateTime Modelo::importadoDe(int i) const
{
    if (!m_vista || i < 0 || i >= m_n) return {};
    return QDateTime::fromMSecsSinceEpoch(qint64(grim_vista_importado(m_vista, size_t(i))));
}

QStringList Modelo::seleccion() const
{
    QList<int> ordenados(m_elegidos.begin(), m_elegidos.end());
    std::sort(ordenados.begin(), ordenados.end());
    QStringList ids;
    ids.reserve(ordenados.size());
    for (int i : ordenados) {
        const QString id = idDe(i);
        if (!id.isEmpty()) ids.append(id);
    }
    return ids;
}

void Modelo::setActual(int i)
{
    if (m_actual == i) return;
    m_actual = (i >= 0 && i < m_n) ? i : -1;
    emit actualCambio();
    emit focoCambio();
}

/// Avisa solo de las filas que de verdad cambian de estado.
///
/// Con un tope: "seleccionar todo" sobre 100.000 elementos emitiría 100.000
/// señales y congelaría la ventana varios segundos. Pasado el tope sale una
/// sola señal de rango, que la vista resuelve repintando lo que se ve.
static void avisar(Modelo *m, const QSet<int> &antes, const QSet<int> &ahora)
{
    constexpr int TOPE = 256;
    QSet<int> comunes = antes;
    comunes.intersect(ahora);
    QSet<int> cambiadas = antes;
    cambiadas.unite(ahora);
    cambiadas.subtract(comunes);
    if (cambiadas.isEmpty()) return;
    if (cambiadas.size() > TOPE) {
        emit m->dataChanged(m->index(0, 0), m->index(m->total() - 1, 0), { Modelo::Elegido });
        return;
    }
    for (int i : cambiadas) {
        const QModelIndex idx = m->index(i, 0);
        emit m->dataChanged(idx, idx, { Modelo::Elegido });
    }
}

void Modelo::elegir(int i)
{
    if (i < 0 || i >= m_n) return;
    const QSet<int> antes = m_elegidos;
    m_elegidos = { i };
    m_ancla = i;
    setActual(i);
    avisar(this, antes, m_elegidos);
    emit seleccionCambio();
}

void Modelo::elegirHasta(int i)
{
    if (i < 0 || i >= m_n) return;
    if (m_ancla < 0) {
        elegir(i);
        return;
    }
    const QSet<int> antes = m_elegidos;
    m_elegidos.clear();
    const int a = qMin(m_ancla, i);
    const int b = qMax(m_ancla, i);
    for (int k = a; k <= b; ++k) m_elegidos.insert(k);
    setActual(i);
    avisar(this, antes, m_elegidos);
    emit seleccionCambio();
}

void Modelo::alternar(int i)
{
    if (i < 0 || i >= m_n) return;
    const QSet<int> antes = m_elegidos;
    if (m_elegidos.contains(i)) {
        m_elegidos.remove(i);
    } else {
        m_elegidos.insert(i);
        m_ancla = i;
    }
    setActual(i);
    avisar(this, antes, m_elegidos);
    emit seleccionCambio();
}

void Modelo::limpiarSeleccion()
{
    if (m_elegidos.isEmpty()) return;
    const QSet<int> antes = m_elegidos;
    m_elegidos.clear();
    m_ancla = -1;
    avisar(this, antes, m_elegidos);
    emit seleccionCambio();
}

void Modelo::elegirTodo()
{
    const QSet<int> antes = m_elegidos;
    m_elegidos.clear();
    m_elegidos.reserve(m_n);
    for (int i = 0; i < m_n; ++i) m_elegidos.insert(i);
    avisar(this, antes, m_elegidos);
    emit seleccionCambio();
}

void Modelo::elegirPrimeros(int n)
{
    const QSet<int> antes = m_elegidos;
    m_elegidos.clear();
    const int tope = qBound(0, n, m_n);
    m_elegidos.reserve(tope);
    for (int i = 0; i < tope; ++i) m_elegidos.insert(i);
    m_ancla = 0;
    avisar(this, antes, m_elegidos);
    setActual(tope > 0 ? 0 : -1);
    emit seleccionCambio();
}
