#include "ajustes.h"

Ajustes::Ajustes(QObject *padre) : QObject(padre)
{
    m_anchoLateral = m_disco.value(QStringLiteral("paneles/lateral"), 0.0).toReal();
    m_anchoInspector = m_disco.value(QStringLiteral("paneles/inspector"), 0.0).toReal();
    m_videoAlPasar = m_disco.value(QStringLiteral("video/alPasar"), true).toBool();
    m_modoSeguro = m_disco.value(QStringLiteral("seguro/puesto"), true).toBool();
    m_verNombres = m_disco.value(QStringLiteral("malla/nombres"), false).toBool();
    m_verInspector = m_disco.value(QStringLiteral("paneles/verInspector"), true).toBool();
    m_vista = m_disco.value(QStringLiteral("malla/vista"), -1).toInt();
    m_escala = qBound(0.7, m_disco.value(QStringLiteral("interfaz/escala"), 1.0).toReal(), 1.6);
    m_tema = m_disco.value(QStringLiteral("interfaz/tema"), QStringLiteral("oscuro")).toString();
    m_buscarVersiones = m_disco.value(QStringLiteral("novedades/buscar"), true).toBool();
}

void Ajustes::guardar(const char *clave, const QVariant &valor)
{
    m_disco.setValue(QString::fromLatin1(clave), valor);
    // Al vuelo y no al cerrar: un programa que se va abajo no puede llevarse
    // por delante los anchos que alguien acaba de colocar.
    m_disco.sync();
    emit cambio();
}

void Ajustes::setAnchoLateral(qreal v)
{
    if (qFuzzyCompare(m_anchoLateral, v)) return;
    m_anchoLateral = v;
    guardar("paneles/lateral", v);
}

void Ajustes::setAnchoInspector(qreal v)
{
    if (qFuzzyCompare(m_anchoInspector, v)) return;
    m_anchoInspector = v;
    guardar("paneles/inspector", v);
}

void Ajustes::setVideoAlPasar(bool v)
{
    if (m_videoAlPasar == v) return;
    m_videoAlPasar = v;
    guardar("video/alPasar", v);
}


void Ajustes::setModoSeguro(bool v)
{
    if (m_modoSeguro == v) return;
    m_modoSeguro = v;
    guardar("seguro/puesto", v);
}

void Ajustes::setVerNombres(bool v)
{
    if (m_verNombres == v) return;
    m_verNombres = v;
    guardar("malla/nombres", v);
}

void Ajustes::setVerInspector(bool v)
{
    if (m_verInspector == v) return;
    m_verInspector = v;
    guardar("paneles/verInspector", v);
}

void Ajustes::setVista(int v)
{
    if (m_vista == v) return;
    m_vista = v;
    guardar("malla/vista", v);
}

void Ajustes::setEscala(qreal v)
{
    // Más allá de estos topes la ventana deja de caber o la letra de leerse.
    v = qBound(0.7, v, 1.6);
    if (qFuzzyCompare(m_escala, v)) return;
    m_escala = v;
    guardar("interfaz/escala", v);
}

void Ajustes::setTema(const QString &v)
{
    if (m_tema == v) return;
    m_tema = v;
    guardar("interfaz/tema", v);
}

void Ajustes::setBuscarVersiones(bool v)
{
    if (m_buscarVersiones == v) return;
    m_buscarVersiones = v;
    guardar("novedades/buscar", v);
}

int Ajustes::orientacion3d(const QString &id) const
{
    return m_disco.value(QStringLiteral("modelos3d/") + id, 0).toInt();
}

void Ajustes::ponerOrientacion3d(const QString &id, int v)
{
    const QString clave = QStringLiteral("modelos3d/") + id;
    // Sin corrección no se guarda nada: el archivo no se llena de ceros.
    if (v == 0) m_disco.remove(clave);
    else m_disco.setValue(clave, v);
    m_disco.sync();
}
