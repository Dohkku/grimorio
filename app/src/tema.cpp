#include "tema.h"

#include <QFile>
#include <QFileSystemWatcher>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QTimer>

Tema::Tema(QObject *padre) : QObject(padre) { }

bool Tema::cargar(const QString &ruta, QString *error)
{
    QFile f(ruta);
    if (!f.open(QIODevice::ReadOnly)) {
        if (error) *error = f.errorString();
        return false;
    }
    QJsonParseError pe {};
    const QJsonDocument doc = QJsonDocument::fromJson(f.readAll(), &pe);
    if (pe.error != QJsonParseError::NoError || !doc.isObject()) {
        if (error) *error = pe.errorString();
        return false;
    }

    // Partir de los valores de fábrica en cada recarga: si alguien borra una
    // línea del archivo, esa propiedad tiene que volver a su valor original, no
    // quedarse con el de la lectura anterior.
    aplicarDefectos();

    const QJsonObject o = doc.object();
    const auto color = [&o](const char *k, QColor &destino) {
        const auto v = o.value(QLatin1String(k));
        if (v.isString()) {
            const QColor c(v.toString());
            // Un hex mal escrito no debe dejar la ventana en negro sobre negro:
            // se ignora y se conserva el valor de fábrica.
            if (c.isValid()) destino = c;
        }
    };
    const auto num = [&o](const char *k, qreal &destino) {
        const auto v = o.value(QLatin1String(k));
        if (v.isDouble()) destino = v.toDouble();
    };
    color("fondo", m_fondo);
    color("panel", m_panel);
    color("texto", m_texto);
    color("textoTenue", m_textoTenue);
    color("seleccion", m_seleccion);
    color("marcador", m_marcador);
    color("borde", m_borde);
    color("ejeX", m_ejeX);
    color("ejeY", m_ejeY);
    color("ejeZ", m_ejeZ);
    color("pieza3d", m_pieza3d);
    num("radio", m_radio);
    num("margen", m_margen);
    num("hueco", m_hueco);
    num("celda", m_celda);
    num("realce", m_realce);
    num("realcePx", m_realcePx);
    num("aparicionS", m_aparicionS);
    num("scrollTau", m_scrollTau);
    num("lateral", m_lateral);
    num("inspector", m_inspector);
    num("fuente", m_fuente);
    if (o.value(QLatin1String("subidasPorFotograma")).isDouble())
        m_subidas = o.value(QLatin1String("subidasPorFotograma")).toInt();

    m_archivo = ruta;
    emit cambio();
    return true;
}

void Tema::aplicarDefectos()
{
    Tema fabrica;
    m_fondo = fabrica.m_fondo;
    m_panel = fabrica.m_panel;
    m_texto = fabrica.m_texto;
    m_textoTenue = fabrica.m_textoTenue;
    m_seleccion = fabrica.m_seleccion;
    m_marcador = fabrica.m_marcador;
    m_borde = fabrica.m_borde;
    m_ejeX = fabrica.m_ejeX;
    m_ejeY = fabrica.m_ejeY;
    m_ejeZ = fabrica.m_ejeZ;
    m_pieza3d = fabrica.m_pieza3d;
    m_radio = fabrica.m_radio;
    m_margen = fabrica.m_margen;
    m_hueco = fabrica.m_hueco;
    m_celda = fabrica.m_celda;
    m_realce = fabrica.m_realce;
    m_realcePx = fabrica.m_realcePx;
    m_aparicionS = fabrica.m_aparicionS;
    m_scrollTau = fabrica.m_scrollTau;
    m_subidas = fabrica.m_subidas;
    m_lateral = fabrica.m_lateral;
    m_inspector = fabrica.m_inspector;
    m_fuente = fabrica.m_fuente;
}

void Tema::setEscala(qreal e)
{
    if (qFuzzyCompare(m_escala, e)) return;
    m_escala = e;
    emit cambio();
}

void Tema::vigilar()
{
    // Los temas de dentro del programa no cambian: no hay nada que vigilar.
    if (m_archivo.isEmpty() || m_archivo.startsWith(QLatin1Char(':'))) return;
    if (!m_vigia) {
        m_vigia = new QFileSystemWatcher(this);
        connect(m_vigia, &QFileSystemWatcher::fileChanged, this, [this](const QString &ruta) {
            // Casi todos los editores guardan escribiendo un archivo temporal y
            // renombrándolo, así que el original desaparece un instante y el
            // vigía pierde la pista. Por eso se espera un poco y se vuelve a
            // vigilar en vez de fiarse del aviso a secas.
            QTimer::singleShot(80, this, [this, ruta] {
                // Se cambió de tema mientras tanto: el viejo ya no manda.
                if (ruta != m_archivo) return;
                QString err;
                if (cargar(ruta, &err) && m_vigia && !m_vigia->files().contains(ruta))
                    m_vigia->addPath(ruta);
            });
        });
    }
    if (!m_vigia->files().contains(m_archivo)) m_vigia->addPath(m_archivo);
}
