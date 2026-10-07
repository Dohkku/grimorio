// Modelo de lista sobre la vista publicada por el núcleo.
//
// No copia nada: los datos viven en la vista y aquí se leen por índice. Con
// 100.000 elementos eso importa — construir 100.000 QVariantMap costaría más
// que todo el arranque junto.
//
// La vista es inmutable. Cuando el núcleo publica otra, el modelo suelta la que
// tenía y toma la nueva; mientras tanto sigue sirviendo la vieja, así que nunca
// hay una fila apuntando a memoria que ya no está.
#pragma once
#include <QAbstractListModel>
#include <QColor>
#include <QDateTime>
#include <QHash>
#include <QMutex>
#include <QSet>
#include <QUrl>
#include <QVariantMap>
#include <QStringList>

extern "C" {
#include "grimorio.h"
}

class Nucleo;

class Modelo : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(int total READ total NOTIFY totalCambio)
    /// La lista de ids **construye** la lista entera en cada lectura, así que
    /// no vale para una condición. Para saber cuántos hay está `elegidos`:
    /// atar `inspector.width` a `seleccion.length` significaba construir cien
    /// mil cadenas en cada evaluación de esa expresión.
    Q_PROPERTY(QStringList seleccion READ seleccion NOTIFY seleccionCambio)
    Q_PROPERTY(int elegidos READ cuantosElegidos NOTIFY seleccionCambio)
    Q_PROPERTY(int actual READ actual WRITE setActual NOTIFY actualCambio)
    /// Las estrellas del elemento en foco.
    ///
    /// Existe como propiedad y no como `estrellasDe(actual)` porque una función
    /// no avisa de nada: el panel las leía una vez y se quedaba con ese número
    /// hasta cambiar de elemento y volver. La malla sí se enteraba, porque ahí
    /// llega por `dataChanged`. Dos caminos y solo uno avisaba.
    Q_PROPERTY(int estrellasFoco READ estrellasFoco NOTIFY focoCambio)
    /// Si el elemento en foco está marcado como +18. Igual que las estrellas:
    /// propiedad y no función, para que el panel se entere cuando cambie.
    Q_PROPERTY(bool adultoFoco READ adultoFoco NOTIFY focoCambio)

public:
    enum Papel {
        Indice = Qt::UserRole + 1,
        Id,
        Ancho,
        Alto,
        Proporcion,
        Dominante,
        Nombre,
        Estrellas,
        TieneMiniatura,
        Elegido,
        Familia,
        Duracion,
        Adulto,
    };

    explicit Modelo(Nucleo *nucleo, QObject *padre = nullptr);
    ~Modelo() override;

    int rowCount(const QModelIndex &padre = QModelIndex()) const override;
    QVariant data(const QModelIndex &idx, int papel) const override;
    QHash<int, QByteArray> roleNames() const override;

    int total() const { return m_n; }
    QStringList seleccion() const;
    int cuantosElegidos() const { return m_elegidos.size(); }
    int actual() const { return m_actual; }
    int estrellasFoco() const { return m_actual >= 0 ? estrellasDe(m_actual) : 0; }
    bool adultoFoco() const { return m_actual >= 0 && adultoDe(m_actual); }
    void setActual(int i);

    /// La vista viva, solo para mirar dentro del mismo fotograma.
    const GrimVista *vista() const { return m_vista; }
    /// Una referencia propia a la vista viva. Hay que soltarla.
    ///
    /// Se llama desde los hilos de decodificación mientras el de la interfaz
    /// puede estar cambiando de vista: sin el cerrojo, clonar la vieja justo
    /// cuando se suelta es tocar memoria ya liberada.
    const GrimVista *tomarVista() const
    {
        QMutexLocker l(&m_cerrojoVista);
        return m_vista ? grim_vista_clonar(m_vista) : nullptr;
    }

public slots:
    /// Recoge la vista que el núcleo acaba de publicar.
    void recoger();
    /// Un solo elemento cambió: se anota su estado nuevo y se repinta esa fila.
    ///
    /// El parche vive hasta que llegue una vista nueva. Sin él, poner una
    /// estrella no se vería hasta la siguiente consulta, porque la vista
    /// publicada es de antes del cambio.
    void refrescarItem(const QString &id, const QVariantMap &estado);

    // ---------------------------------------------------------- selección
    void elegir(int i);
    void elegirHasta(int i);
    void alternar(int i);
    void limpiarSeleccion();
    void elegirTodo();
    /// Los N primeros. Lo usa el banco de edición en lote: «todo» son cien mil
    /// y la puerta de M2 habla de diez mil.
    Q_INVOKABLE void elegirPrimeros(int n);
    /// El id de una fila, para mandarlo al núcleo.
    Q_INVOKABLE QString idDe(int i) const;
    /// Accesos con nombre para QML. Existen para no tener números de rol
    /// escritos a mano en el QML, que es de los errores más difíciles de ver:
    /// un 263 mal puesto no falla, simplemente enseña otra cosa.
    Q_INVOKABLE QString nombreDe(int i) const;
    Q_INVOKABLE int estrellasDe(int i) const;
    /// La familia del elemento, con los números de `GrimFamilia`.
    Q_INVOKABLE int familiaDe(int i) const;
    /// La extensión. Se pide por función y no como papel del modelo porque
    /// solo la mira la insignia, que existe en una celda de cada veinte: como
    /// papel construiría una cadena por celda y por repintado.
    Q_INVOKABLE QString extDe(int i) const;
    /// Duración en segundos, o 0.
    Q_INVOKABLE int duracionDe(int i) const;
    /// Peso del original en bytes, y cuándo se importó. Los pide la vista en
    /// lista, por función y por lo mismo que la extensión.
    Q_INVOKABLE double pesoDe(int i) const;
    Q_INVOKABLE QDateTime importadoDe(int i) const;
    /// La ruta del archivo original, ya como URL.
    ///
    /// La da la vista, así que no hace falta pedir la ficha y esperarla para
    /// poder reproducir: con doce vídeos a la vista serían doce idas y vueltas
    /// al hilo del núcleo para saber doce rutas que ya están aquí.
    Q_INVOKABLE QUrl urlDe(int i) const;
    /// Los originales de lo elegido, en el orden de la vista y con el nombre
    /// de cada elemento. Para copiar y para arrastrar fuera del programa.
    Q_INVOKABLE QList<QUrl> urlsElegidas() const;
    /// Los ids y las posiciones de lo elegido, en el orden de la vista: el
    /// renombrado en lote numera en este orden, que es el que se ve.
    Q_INVOKABLE QStringList idsElegidos() const;
    Q_INVOKABLE QList<int> indicesElegidos() const;
    /// El original con el nombre del elemento (ver modelo.cpp).
    QUrl urlConNombre(int i) const;
    /// La previsualización de 1024 px en disco, o "" si no hay.
    Q_INVOKABLE QString previaDe(int i) const;
    Q_INVOKABLE uint32_t anchoDe(int i) const;
    Q_INVOKABLE uint32_t altoDe(int i) const;
    QColor dominanteDe(int i) const;
    bool tieneMiniaturaDe(int i) const;
    Q_INVOKABLE bool adultoDe(int i) const;
    bool elegidoDe(int i) const;

signals:
    void totalCambio();
    void seleccionCambio();
    void actualCambio();
    /// El elemento en foco cambió: o es otro, o le han cambiado algo.
    void focoCambio();

private:
    void soltarVista();
    QString idEn(size_t i) const;
    const QVariantMap *parcheEn(size_t i) const;

    Nucleo *m_nucleo = nullptr;
    QHash<QString, QVariantMap> m_parche;
    const GrimVista *m_vista = nullptr;
    /// Solo para cambiar `m_vista` y para clonarla desde otro hilo. Leerla en
    /// el hilo de la interfaz no lo necesita: es el único que la cambia.
    mutable QMutex m_cerrojoVista;
    int m_n = 0;
    QSet<int> m_elegidos;
    /// Ancla del rango: shift+clic selecciona desde aquí.
    int m_ancla = -1;
    int m_actual = -1;
};
