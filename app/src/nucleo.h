// El núcleo visto desde Qt: manda comandos y reparte los eventos que vuelven.
//
// Todo lo que llega del hilo trabajador entra por `recibir`, que se llama desde
// ese hilo, y se reenvía al hilo de interfaz con una conexión en cola. A partir
// de ahí, nada de Qt Quick se toca desde fuera del hilo de la ventana.
#pragma once
#include <QJsonObject>
#include <QHash>
#include <QObject>
#include <QString>
#include <QUrl>
#include <QVariantList>
#include <QVariantMap>

extern "C" {
#include "grimorio.h"
}

class Nucleo : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString nombre READ nombre NOTIFY nombreCambio)
    Q_PROPERTY(QString raiz READ raiz NOTIFY listo)
    Q_PROPERTY(int total READ total NOTIFY totalCambio)
    Q_PROPERTY(QVariantList carpetas READ carpetas NOTIFY carpetasCambiaron)
    /// Las carpetas inteligentes: `{id, nombre, consulta, cuenta}`.
    Q_PROPERTY(QVariantList busquedas READ busquedas NOTIFY busquedasCambiaron)
    /// Las etiquetas que existen, de más usada a menos, para autocompletar.
    /// Llegan solas cada vez que cambian: la interfaz no las pide por cada
    /// tecla, que con diez mil elementos sería una consulta por pulsación.
    Q_PROPERTY(QVariantList etiquetas READ etiquetas NOTIFY etiquetasCambiaron)
    /// Todas, para el gestor de etiquetas: `{nombre, n}`.
    Q_PROPERTY(QVariantList todasEtiquetas READ todasEtiquetas NOTIFY todasEtiquetasCambiaron)
    /// Las carpetas del disco vigiladas: `{id, ruta, carpeta, desde_ms}`.
    Q_PROPERTY(QVariantList vigiladas READ vigiladas NOTIFY vigiladasCambiaron)
    /// Los grupos de etiquetas: `{id, nombre, color, etiquetas: [..]}`.
    Q_PROPERTY(QVariantList grupos READ grupos NOTIFY gruposCambiaron)
    /// Cuántos elementos hay en la papelera.
    Q_PROPERTY(int tirados READ tirados NOTIFY carpetasCambiaron)
    /// Si la biblioteca tiene algo marcado como +18. Sin nada marcado, el botón
    /// del modo seguro sería un interruptor que no enciende nada.
    Q_PROPERTY(bool hayAdultos READ hayAdultos NOTIFY carpetasCambiaron)
    /// Cuánto hay de cada familia: `{familia, n, bytes}`, de más a menos.
    Q_PROPERTY(QVariantList reparto READ reparto NOTIFY carpetasCambiaron)
    /// Cómo se llama lo que se desharía, o vacío si no hay nada.
    ///
    /// Es una cadena y no un booleano porque el menú tiene que poder decir
    /// «Deshacer poner 4 estrellas»: saber qué va a pasar antes de pulsar es
    /// la mitad de para qué sirve un deshacer.
    Q_PROPERTY(QString queSeDeshace READ queSeDeshace NOTIFY registroCambio)
    Q_PROPERTY(QString queSeRehace READ queSeRehace NOTIFY registroCambio)
    /// La ficha del elemento en foco: lo que la vista no lleva porque solo
    /// hace falta de uno cada vez (peso, medidas, fechas, ruta, nota).
    Q_PROPERTY(QVariantMap ficha READ ficha NOTIFY fichaCambio)
    Q_PROPERTY(QString ultimoAviso READ ultimoAviso NOTIFY aviso)

public:
    explicit Nucleo(QObject *padre = nullptr);
    ~Nucleo() override;

    /// Abre la biblioteca. Devuelve false si la ruta no lo es.
    bool abrir(const QString &ruta);

    GrimNucleo *crudo() const { return m_n; }

    QString nombre() const { return m_nombre; }
    QString raiz() const { return m_raiz; }
    int total() const { return m_total; }
    QString ultimoAviso() const { return m_ultimoAviso; }
    QVariantList etiquetas() const { return m_etiquetas; }
    QVariantList todasEtiquetas() const { return m_todasEtiquetas; }
    QVariantList grupos() const { return m_grupos; }
    QVariantList vigiladas() const { return m_vigiladas; }
    QVariantMap ficha() const { return m_ficha; }
    int tirados() const { return m_tirados; }
    bool hayAdultos() const { return m_adultos > 0; }
    QVariantList reparto() const { return m_reparto; }
    QString queSeDeshace() const { return m_deshacer; }
    QString queSeRehace() const { return m_rehacer; }

    /// El árbol de carpetas, aplanado y ya ordenado para pintarlo: cada entrada
    /// lleva su nivel y su cuenta, incluida la de sus descendientes.
    QVariantList carpetas() const { return m_carpetas; }
    QVariantList busquedas() const { return m_busquedas; }

    // ------------------------------------------------------------- comandos

    /// `filtro` es lo que hay escrito en el buscador, tal cual. Se manda sin
    /// tocar: lo entiende el núcleo, que es donde el lenguaje tiene pruebas.
    Q_INVOKABLE void consultar(const QVariantMap &q, const QString &filtro = QString());
    Q_INVOKABLE void crearCarpeta(const QString &nombre, const QString &padre = QString());
    Q_INVOKABLE void guardarBusqueda(const QString &nombre, const QString &consulta);
    Q_INVOKABLE void renombrarBusqueda(const QString &id, const QString &nombre);
    Q_INVOKABLE void borrarBusqueda(const QString &id);
    /// De cada grupo de copias exactas deja la primera; el resto, a la papelera.
    Q_INVOKABLE void quitarCopiasExactas();
    /// Pone `id` entre `antes` y `despues` en el orden a mano de `carpeta`.
    Q_INVOKABLE void reordenar(const QString &id, const QString &carpeta,
                               const QString &antes, const QString &despues);
    /// Renombra con un patrón: {nombre}, {n}, {n:3}, {fecha}.
    Q_INVOKABLE void renombrarEnLote(const QStringList &ids, const QString &patron, int inicio);
    Q_INVOKABLE void renombrarCarpeta(const QString &id, const QString &nombre);
    /// El nombre que se enseña de la biblioteca; la carpeta del disco no cambia.
    Q_INVOKABLE void renombrarBiblioteca(const QString &nombre);
    /// `color` en "#rrggbb"; cadena vacía para quitárselo.
    Q_INVOKABLE void colorCarpeta(const QString &id, const QString &color);
    /// Cuelga la carpeta de otra y, si se dice, la coloca delante de una
    /// hermana. Arrastrar una fila del árbol es un solo gesto: colgar y
    /// colocar viajan juntos.
    Q_INVOKABLE void moverCarpeta(const QString &id, const QString &padre,
                                  const QString &antesDe = QString());
    Q_INVOKABLE void borrarCarpeta(const QString &id);
    Q_INVOKABLE void moverAcarpeta(const QStringList &ids, const QString &carpeta);
    Q_INVOKABLE void sacarDeCarpeta(const QStringList &ids, const QString &carpeta);
    /// Sacar de una y meter en otra, en una sola operación.
    ///
    /// No son dos llamadas seguidas por una razón concreta: serían dos entradas
    /// en el registro, y deshacer un movimiento pediría dos deshaceres —el
    /// primero dejaría el elemento en las dos carpetas a la vez—.
    Q_INVOKABLE void moverEntreCarpetas(const QStringList &ids, const QString &de,
                                        const QString &a);
    Q_INVOKABLE void estrellas(const QStringList &ids, int valor);
    Q_INVOKABLE void etiquetar(const QStringList &ids, const QStringList &anadir,
                               const QStringList &quitar);
    Q_INVOKABLE void nota(const QString &id, const QString &texto);
    Q_INVOKABLE void renombrar(const QString &id, const QString &nombre);
    Q_INVOKABLE void papelera(const QStringList &ids, bool dentro);
    /// Marca o desmarca lo elegido como contenido adulto.
    Q_INVOKABLE void marcarAdulto(const QStringList &ids, bool si);
    Q_INVOKABLE void vaciarPapelera();
    Q_INVOKABLE void deshacer();
    Q_INVOKABLE void rehacer();
    /// Pide las etiquetas que empiezan por un prefijo. Sin prefijo, las más
    /// usadas.
    Q_INVOKABLE void pedirEtiquetas(const QString &prefijo = QString());
    Q_INVOKABLE void pedirTodasEtiquetas();
    void vigilar(const QString &ruta, const QString &carpeta);
    Q_INVOKABLE void dejarDeVigilar(const QString &id);
    /// La ruta del disco que se vigila hacia esa carpeta, o "".
    Q_INVOKABLE QString vigiladaDe(const QString &carpeta) const;
    Q_INVOKABLE void renombrarEtiqueta(const QString &vieja, const QString &nueva);
    Q_INVOKABLE void borrarEtiqueta(const QString &nombre);
    Q_INVOKABLE void crearGrupo(const QString &nombre, const QString &color);
    Q_INVOKABLE void renombrarGrupo(const QString &id, const QString &nombre);
    Q_INVOKABLE void colorGrupo(const QString &id, const QString &color);
    Q_INVOKABLE void borrarGrupo(const QString &id);
    /// `grupo` vacío la deja sin grupo.
    Q_INVOKABLE void agruparEtiqueta(const QString &etiqueta, const QString &grupo);
    /// El color de su grupo, o "" si no tiene.
    Q_INVOKABLE QString colorDeEtiqueta(const QString &etiqueta) const;
    /// Pide la ficha de un elemento. Llamarla con el mismo id dos veces no
    /// cuesta nada: es leer un JSON pequeño en el hilo del núcleo.
    Q_INVOKABLE void pedirFicha(const QString &id);
    Q_INVOKABLE void importar(const QStringList &rutas, const QString &carpeta = QString());
    /// Lo que llega de arrastrar y soltar: URLs, no rutas. La conversión se
    /// hace aquí y no en QML porque quitar el "file://" a mano se rompe con el
    /// primer nombre que lleve un espacio o un acento.
    Q_INVOKABLE void importarUrls(const QList<QUrl> &urls, const QString &carpeta = QString());
    /// Importar con lo que solo sabe quien trae el archivo de fuera: la URL de
    /// la que viene y si es un temporal que se puede mover en vez de copiar.
    void importarDeFuera(const QStringList &rutas, const QString &carpeta,
                         const QString &origen, bool mover);
    Q_INVOKABLE void abrirFuera(const QString &id);
    /// Si `hija` cuelga de `padre`, a cualquier profundidad.
    ///
    /// Lo pregunta la barra lateral antes de dejar soltar una carpeta dentro de
    /// otra. El núcleo también lo rechaza —hay prueba de ello—, pero un sitio
    /// donde no se puede soltar tiene que dejar de iluminarse antes de soltar,
    /// no contestar con un error después.
    Q_INVOKABLE bool cuelgaDe(const QString &hija, const QString &padre) const;

signals:
    void listo();
    void nombreCambio();
    void vistaNueva(int n);
    void carpetasCambiaron();
    void busquedasCambiaron();
    void totalCambio();
    void etiquetasCambiaron();
    void todasEtiquetasCambiaron();
    void gruposCambiaron();
    void vigiladasCambiaron();
    void fichaCambio();
    void registroCambio();
    /// Un elemento cambió, con su estado nuevo: estrellas, nombre, etiquetas,
    /// nota y carpetas. Va con los datos dentro para que la interfaz no tenga
    /// que volver a consultar la biblioteca entera por una estrella.
    void itemCambiado(const QString &id, const QVariantMap &estado);
    void progreso(const QString &que, int hechos, int total);
    /// Un mensaje corto para la barra de estado: fin de una tarea o un error.
    void aviso(const QString &mensaje, bool esError);

private slots:
    /// Corre ya en el hilo de interfaz.
    void procesar(const QString &json);

private:
    void mandar(const QJsonObject &o);
    void reconstruirCarpetas(const QJsonObject &ev);
    static void recibir(void *usuario, const uint8_t *json, uint32_t len);

    GrimNucleo *m_n = nullptr;
    QString m_nombre;
    QString m_raiz;
    int m_total = 0;
    QVariantList m_carpetas;
    QVariantList m_busquedas;
    QVariantList m_etiquetas;
    QVariantList m_todasEtiquetas;
    QVariantList m_grupos;
    QVariantList m_vigiladas;
    QHash<QString, QString> m_colorEtiqueta;
    QVariantMap m_ficha;
    int m_tirados = 0;
    int m_adultos = 0;
    QVariantList m_reparto;
    QString m_deshacer;
    QString m_rehacer;
    QString m_ultimoAviso;
};
