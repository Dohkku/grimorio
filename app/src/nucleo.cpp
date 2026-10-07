#include "nucleo.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QMetaObject>
#include <algorithm>
#include <functional>

Nucleo::Nucleo(QObject *padre) : QObject(padre) { }

Nucleo::~Nucleo()
{
    // Parar antes de morir: el hilo trabajador tiene un puntero a este objeto y
    // `grim_parar` no vuelve hasta que ese hilo ha terminado.
    if (m_n) grim_parar(m_n);
}

bool Nucleo::abrir(const QString &ruta)
{
    if (m_n) {
        grim_parar(m_n);
        m_n = nullptr;
    }
    m_n = grim_iniciar(ruta.toUtf8().constData(), &Nucleo::recibir, this);
    return m_n != nullptr;
}

void Nucleo::recibir(void *usuario, const uint8_t *json, uint32_t len)
{
    auto *self = static_cast<Nucleo *>(usuario);
    const QString texto = QString::fromUtf8(reinterpret_cast<const char *>(json), int(len));
    // Cola, no llamada directa: esto corre en el hilo del núcleo y todo lo que
    // hay al otro lado (modelo, ventana, QML) es del hilo de interfaz.
    QMetaObject::invokeMethod(self, "procesar", Qt::QueuedConnection, Q_ARG(QString, texto));
}

void Nucleo::procesar(const QString &json)
{
    const QJsonObject ev = QJsonDocument::fromJson(json.toUtf8()).object();
    const QString tipo = ev.value(QStringLiteral("tipo")).toString();

    if (tipo == QLatin1String("listo")) {
        m_nombre = ev.value(QStringLiteral("nombre")).toString();
        m_raiz = ev.value(QStringLiteral("raiz")).toString();
        m_total = ev.value(QStringLiteral("total")).toInt();
        emit totalCambio();
        emit listo();
    } else if (tipo == QLatin1String("vista")) {
        emit vistaNueva(ev.value(QStringLiteral("n")).toInt());
    } else if (tipo == QLatin1String("carpetas")) {
        m_tirados = ev.value(QStringLiteral("papelera")).toInt();
        m_adultos = ev.value(QStringLiteral("adultos")).toInt();
        m_reparto = ev.value(QStringLiteral("reparto")).toArray().toVariantList();
        const int antes = m_total;
        m_total = ev.value(QStringLiteral("total")).toInt();
        if (antes != m_total) emit totalCambio();
        reconstruirCarpetas(ev);
        emit carpetasCambiaron();
    } else if (tipo == QLatin1String("busquedas")) {
        m_busquedas = ev.value(QStringLiteral("lista")).toArray().toVariantList();
        emit busquedasCambiaron();
    } else if (tipo == QLatin1String("item")) {
        QVariantMap estado = ev.toVariantMap();
        estado.remove(QStringLiteral("tipo"));
        estado.remove(QStringLiteral("cmd"));
        emit itemCambiado(ev.value(QStringLiteral("id")).toString(), estado);
    } else if (tipo == QLatin1String("ficha")) {
        m_ficha = ev.toVariantMap();
        m_ficha.remove(QStringLiteral("tipo"));
        m_ficha.remove(QStringLiteral("cmd"));
        emit fichaCambio();
    } else if (tipo == QLatin1String("etiquetas")) {
        m_etiquetas.clear();
        for (const QJsonValue &v : ev.value(QStringLiteral("lista")).toArray()) {
            const QJsonObject o = v.toObject();
            m_etiquetas.append(QVariantMap {
                { QStringLiteral("nombre"), o.value(QStringLiteral("nombre")).toString() },
                { QStringLiteral("n"), o.value(QStringLiteral("n")).toInt() },
            });
        }
        emit etiquetasCambiaron();
    } else if (tipo == QLatin1String("todas_etiquetas")) {
        m_todasEtiquetas = ev.value(QStringLiteral("lista")).toArray().toVariantList();
        emit todasEtiquetasCambiaron();
    } else if (tipo == QLatin1String("vigiladas")) {
        m_vigiladas = ev.value(QStringLiteral("lista")).toArray().toVariantList();
        emit vigiladasCambiaron();
    } else if (tipo == QLatin1String("grupos")) {
        m_grupos = ev.value(QStringLiteral("lista")).toArray().toVariantList();
        m_colorEtiqueta.clear();
        for (const QVariant &g : std::as_const(m_grupos)) {
            const QVariantMap m = g.toMap();
            const QString color = m.value(QStringLiteral("color")).toString();
            for (const QVariant &t : m.value(QStringLiteral("etiquetas")).toList())
                m_colorEtiqueta.insert(t.toString(), color);
        }
        emit gruposCambiaron();
    } else if (tipo == QLatin1String("registro")) {
        m_deshacer = ev.value(QStringLiteral("deshacer")).toString();
        m_rehacer = ev.value(QStringLiteral("rehacer")).toString();
        emit registroCambio();
    } else if (tipo == QLatin1String("progreso")) {
        emit progreso(ev.value(QStringLiteral("que")).toString(),
                      ev.value(QStringLiteral("hechos")).toInt(),
                      ev.value(QStringLiteral("total")).toInt());
    } else if (tipo == QLatin1String("hecho")) {
        const QString m = ev.value(QStringLiteral("mensaje")).toString();
        // Un "hecho" sin mensaje es una acción que se ve sola —una estrella que
        // cambia— y no merece ocupar la barra de estado.
        if (!m.isEmpty()) {
            m_ultimoAviso = m;
            emit aviso(m, false);
        }
    } else if (tipo == QLatin1String("error")) {
        m_ultimoAviso = ev.value(QStringLiteral("mensaje")).toString();
        emit aviso(m_ultimoAviso, true);
    }
}

namespace {

struct Cruda {
    QString id, nombre, padre, color;
    qint64 pos = 0;
    int propios = 0;
};

} // namespace

void Nucleo::reconstruirCarpetas(const QJsonObject &ev)
{
    QHash<QString, Cruda> porId;
    QList<QString> orden;
    const QJsonObject conteos = ev.value(QStringLiteral("conteos")).toObject();
    for (const QJsonValue &v : ev.value(QStringLiteral("arbol")).toArray()) {
        const QJsonObject o = v.toObject();
        Cruda c;
        c.id = o.value(QStringLiteral("id")).toString();
        c.nombre = o.value(QStringLiteral("nombre")).toString();
        c.padre = o.value(QStringLiteral("padre")).toString();
        c.color = o.value(QStringLiteral("color")).toString();
        c.pos = qint64(o.value(QStringLiteral("pos")).toDouble());
        c.propios = conteos.value(c.id).toInt();
        porId.insert(c.id, c);
        orden.append(c.id);
    }

    // Cuenta acumulada: lo propio más todo lo que cuelga. Es lo que espera
    // quien mira la barra lateral, y calcularlo aquí evita que QML lo recorra
    // en cada repintado.
    QHash<QString, int> totales;
    for (const QString &id : orden) totales.insert(id, porId.value(id).propios);
    for (const QString &id : orden) {
        const int propios = porId.value(id).propios;
        if (propios == 0) continue;
        QString p = porId.value(id).padre;
        // El contador de saltos es el seguro contra un ciclo en el archivo:
        // sin él, dos carpetas que se apuntan la una a la otra cuelgan la
        // ventana en vez de dar una cuenta rara.
        int saltos = 0;
        while (!p.isEmpty() && saltos++ <= orden.size()) {
            totales[p] += propios;
            p = porId.value(p).padre;
        }
    }

    // Aplanar en orden de pintado: cada rama entera antes de la siguiente.
    m_carpetas.clear();
    std::function<void(const QString &, int)> bajar = [&](const QString &padre, int nivel) {
        QList<Cruda> hijas;
        for (const QString &id : orden) {
            const Cruda &c = porId[id];
            if (c.padre == padre) hijas.append(c);
        }
        std::sort(hijas.begin(), hijas.end(), [](const Cruda &a, const Cruda &b) {
            if (a.pos != b.pos) return a.pos < b.pos;
            return a.nombre.localeAwareCompare(b.nombre) < 0;
        });
        for (const Cruda &c : hijas) {
            QVariantMap m;
            m[QStringLiteral("id")] = c.id;
            m[QStringLiteral("nombre")] = c.nombre;
            m[QStringLiteral("padre")] = c.padre;
            m[QStringLiteral("color")] = c.color;
            m[QStringLiteral("nivel")] = nivel;
            m[QStringLiteral("cuenta")] = totales.value(c.id);
            m[QStringLiteral("propios")] = c.propios;
            m_carpetas.append(m);
            bajar(c.id, nivel + 1);
        }
    };
    bajar(QString(), 0);
}

// ------------------------------------------------------------------ comandos

void Nucleo::mandar(const QJsonObject &o)
{
    if (!m_n) return;
    const QByteArray bytes = QJsonDocument(o).toJson(QJsonDocument::Compact);
    grim_mandar(m_n, reinterpret_cast<const uint8_t *>(bytes.constData()),
                uint32_t(bytes.size()));
}

namespace {

QJsonArray deLista(const QStringList &l)
{
    QJsonArray a;
    for (const QString &s : l) a.append(s);
    return a;
}

} // namespace

void Nucleo::consultar(const QVariantMap &q, const QString &filtro)
{
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("consulta") },
                    { QStringLiteral("q"), QJsonObject::fromVariantMap(q) } };
    if (!filtro.trimmed().isEmpty()) o[QStringLiteral("filtro")] = filtro;
    mandar(o);
}

void Nucleo::guardarBusqueda(const QString &nombre, const QString &consulta)
{
    if (nombre.trimmed().isEmpty() || consulta.trimmed().isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("guardar_busqueda") },
             { QStringLiteral("nombre"), nombre.trimmed() },
             { QStringLiteral("consulta"), consulta.trimmed() } });
}

void Nucleo::renombrarBusqueda(const QString &id, const QString &nombre)
{
    if (id.isEmpty() || nombre.trimmed().isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("renombrar_busqueda") },
             { QStringLiteral("id"), id },
             { QStringLiteral("nombre"), nombre.trimmed() } });
}

void Nucleo::borrarBusqueda(const QString &id)
{
    if (id.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("borrar_busqueda") },
             { QStringLiteral("id"), id } });
}

void Nucleo::renombrarEnLote(const QStringList &ids, const QString &patron, int inicio)
{
    if (ids.isEmpty() || patron.trimmed().isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("renombrar_en_lote") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("patron"), patron },
             { QStringLiteral("inicio"), qMax(0, inicio) } });
}

void Nucleo::quitarCopiasExactas()
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("quitar_copias_exactas") } });
}

void Nucleo::crearCarpeta(const QString &nombre, const QString &padre)
{
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("crear_carpeta") },
                    { QStringLiteral("nombre"), nombre } };
    if (!padre.isEmpty()) o[QStringLiteral("padre")] = padre;
    mandar(o);
}

void Nucleo::renombrarCarpeta(const QString &id, const QString &nombre)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("renombrar_carpeta") },
             { QStringLiteral("id"), id },
             { QStringLiteral("nombre"), nombre } });
}

void Nucleo::colorCarpeta(const QString &id, const QString &color)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("color_carpeta") },
             { QStringLiteral("id"), id },
             { QStringLiteral("color"), color } });
}

void Nucleo::moverCarpeta(const QString &id, const QString &padre, const QString &antesDe)
{
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("mover_carpeta") },
                    { QStringLiteral("id"), id } };
    // Sin padre significa "a la raíz", no "no cambiar": por eso va explícito.
    o[QStringLiteral("padre")] = padre.isEmpty() ? QJsonValue() : QJsonValue(padre);
    // Sin hermana significa "la última", que es lo que hace soltar sobre una
    // carpeta en vez de entre dos.
    o[QStringLiteral("antes_de")] = antesDe.isEmpty() ? QJsonValue() : QJsonValue(antesDe);
    mandar(o);
}

void Nucleo::borrarCarpeta(const QString &id)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("borrar_carpeta") },
             { QStringLiteral("id"), id } });
}

void Nucleo::moverAcarpeta(const QStringList &ids, const QString &carpeta)
{
    if (ids.isEmpty() || carpeta.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("carpetas_de_elementos") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("anadir"), deLista({ carpeta }) } });
}

void Nucleo::sacarDeCarpeta(const QStringList &ids, const QString &carpeta)
{
    if (ids.isEmpty() || carpeta.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("carpetas_de_elementos") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("quitar"), deLista({ carpeta }) } });
}

void Nucleo::moverEntreCarpetas(const QStringList &ids, const QString &de, const QString &a)
{
    if (ids.isEmpty() || a.isEmpty() || de == a) return;
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("carpetas_de_elementos") },
                    { QStringLiteral("ids"), deLista(ids) },
                    { QStringLiteral("anadir"), deLista({ a }) } };
    if (!de.isEmpty()) o[QStringLiteral("quitar")] = deLista({ de });
    mandar(o);
}

void Nucleo::estrellas(const QStringList &ids, int valor)
{
    if (ids.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("estrellas") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("valor"), qBound(0, valor, 5) } });
}

void Nucleo::etiquetar(const QStringList &ids, const QStringList &anadir,
                       const QStringList &quitar)
{
    if (ids.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("etiquetar") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("anadir"), deLista(anadir) },
             { QStringLiteral("quitar"), deLista(quitar) } });
}

void Nucleo::nota(const QString &id, const QString &texto)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("nota") },
             { QStringLiteral("id"), id },
             { QStringLiteral("texto"), texto } });
}

void Nucleo::renombrar(const QString &id, const QString &nombre)
{
    if (id.isEmpty() || nombre.trimmed().isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("renombrar") },
             { QStringLiteral("id"), id },
             { QStringLiteral("nombre"), nombre.trimmed() } });
}

void Nucleo::papelera(const QStringList &ids, bool dentro)
{
    if (ids.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("papelera") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("dentro"), dentro } });
}

void Nucleo::marcarAdulto(const QStringList &ids, bool si)
{
    if (ids.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("adulto") },
             { QStringLiteral("ids"), deLista(ids) },
             { QStringLiteral("si"), si } });
}

void Nucleo::vaciarPapelera()
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("vaciar_papelera") } });
}

void Nucleo::deshacer()
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("deshacer") } });
}

void Nucleo::rehacer()
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("rehacer") } });
}

void Nucleo::pedirEtiquetas(const QString &prefijo)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("etiquetas") },
             { QStringLiteral("prefijo"), prefijo } });
}

void Nucleo::pedirFicha(const QString &id)
{
    if (id.isEmpty()) {
        m_ficha.clear();
        emit fichaCambio();
        return;
    }
    mandar({ { QStringLiteral("cmd"), QStringLiteral("ficha") },
             { QStringLiteral("id"), id } });
}

void Nucleo::importar(const QStringList &rutas, const QString &carpeta)
{
    if (rutas.isEmpty()) return;
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("importar") },
                    { QStringLiteral("rutas"), deLista(rutas) } };
    if (!carpeta.isEmpty()) o[QStringLiteral("carpeta")] = carpeta;
    mandar(o);
}

void Nucleo::importarDeFuera(const QStringList &rutas, const QString &carpeta,
                             const QString &origen, bool mover)
{
    if (rutas.isEmpty()) return;
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("importar") },
                    { QStringLiteral("rutas"), deLista(rutas) },
                    { QStringLiteral("mover"), mover } };
    if (!carpeta.isEmpty()) o[QStringLiteral("carpeta")] = carpeta;
    if (!origen.isEmpty()) o[QStringLiteral("origen")] = origen;
    mandar(o);
}

void Nucleo::importarUrls(const QList<QUrl> &urls, const QString &carpeta)
{
    QStringList rutas;
    for (const QUrl &u : urls) {
        if (u.isLocalFile()) rutas.append(u.toLocalFile());
    }
    if (rutas.isEmpty()) {
        emit aviso(tr("eso no son archivos de este ordenador"), true);
        return;
    }
    importar(rutas, carpeta);
}

void Nucleo::abrirFuera(const QString &id)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("abrir_fuera") },
             { QStringLiteral("id"), id } });
}


bool Nucleo::cuelgaDe(const QString &hija, const QString &padre) const
{
    if (hija.isEmpty() || padre.isEmpty()) return false;
    QHash<QString, QString> padres;
    for (const QVariant &v : m_carpetas) {
        const QVariantMap m = v.toMap();
        padres.insert(m.value(QStringLiteral("id")).toString(),
                      m.value(QStringLiteral("padre")).toString());
    }
    QString p = hija;
    // Tope de saltos: si el archivo trajera un ciclo, esto colgaría la ventana
    // en vez de contestar. Es el mismo seguro que lleva el recuento.
    for (int saltos = 0; !p.isEmpty() && saltos <= padres.size(); ++saltos) {
        if (p == padre) return true;
        p = padres.value(p);
    }
    return false;
}

void Nucleo::pedirTodasEtiquetas()
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("todas_etiquetas") } });
}

void Nucleo::renombrarEtiqueta(const QString &vieja, const QString &nueva)
{
    if (vieja.isEmpty() || nueva.trimmed().isEmpty() || vieja == nueva.trimmed()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("renombrar_etiqueta") },
             { QStringLiteral("vieja"), vieja },
             { QStringLiteral("nueva"), nueva.trimmed() } });
}

void Nucleo::borrarEtiqueta(const QString &nombre)
{
    if (nombre.isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("borrar_etiqueta") },
             { QStringLiteral("nombre"), nombre } });
}

void Nucleo::crearGrupo(const QString &nombre, const QString &color)
{
    if (nombre.trimmed().isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("crear_grupo") },
             { QStringLiteral("nombre"), nombre.trimmed() },
             { QStringLiteral("color"), color } });
}

void Nucleo::renombrarGrupo(const QString &id, const QString &nombre)
{
    if (id.isEmpty() || nombre.trimmed().isEmpty()) return;
    mandar({ { QStringLiteral("cmd"), QStringLiteral("renombrar_grupo") },
             { QStringLiteral("id"), id },
             { QStringLiteral("nombre"), nombre.trimmed() } });
}

void Nucleo::colorGrupo(const QString &id, const QString &color)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("color_grupo") },
             { QStringLiteral("id"), id },
             { QStringLiteral("color"), color } });
}

void Nucleo::borrarGrupo(const QString &id)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("borrar_grupo") },
             { QStringLiteral("id"), id } });
}

void Nucleo::agruparEtiqueta(const QString &etiqueta, const QString &grupo)
{
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("agrupar_etiqueta") },
                    { QStringLiteral("etiqueta"), etiqueta } };
    if (!grupo.isEmpty()) o[QStringLiteral("grupo")] = grupo;
    mandar(o);
}

QString Nucleo::colorDeEtiqueta(const QString &etiqueta) const
{
    return m_colorEtiqueta.value(etiqueta);
}

void Nucleo::vigilar(const QString &ruta, const QString &carpeta)
{
    if (ruta.isEmpty()) return;
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("vigilar_carpeta") },
                    { QStringLiteral("ruta"), ruta } };
    if (!carpeta.isEmpty()) o[QStringLiteral("carpeta")] = carpeta;
    mandar(o);
}

void Nucleo::dejarDeVigilar(const QString &id)
{
    mandar({ { QStringLiteral("cmd"), QStringLiteral("dejar_de_vigilar") },
             { QStringLiteral("id"), id } });
}

QString Nucleo::vigiladaDe(const QString &carpeta) const
{
    for (const QVariant &v : m_vigiladas) {
        const QVariantMap m = v.toMap();
        if (m.value(QStringLiteral("carpeta")).toString() == carpeta)
            return m.value(QStringLiteral("ruta")).toString();
    }
    return {};
}

void Nucleo::reordenar(const QString &id, const QString &carpeta, const QString &antes,
                       const QString &despues)
{
    if (id.isEmpty() || (antes.isEmpty() && despues.isEmpty())) return;
    QJsonObject o { { QStringLiteral("cmd"), QStringLiteral("reordenar") },
                    { QStringLiteral("id"), id } };
    if (!carpeta.isEmpty()) o[QStringLiteral("carpeta")] = carpeta;
    if (!antes.isEmpty()) o[QStringLiteral("antes")] = antes;
    if (!despues.isEmpty()) o[QStringLiteral("despues")] = despues;
    mandar(o);
}
