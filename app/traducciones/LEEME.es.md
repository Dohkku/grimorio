[English](LEEME.md) · Español

# Traducciones

La interfaz está escrita en español (`qsTr("…")` en QML, `tr("…")` en C++) y
aquí se traduce al inglés. `grimorio_en.ts` es la fuente de la traducción;
`grimorio_en.qm` es su versión compilada, que va en el repositorio para que
compilar no necesite las herramientas de Linguist. `main.cpp` la carga cuando
el idioma de los ajustes es inglés, o cuando es «el del sistema» y el sistema
no está en español.

Los textos que se montan en JavaScript (plurales, los nombres de los filtros
de búsqueda) no pasan por `qsTr`: `textos.js` y `consulta.js` miran
`Qt.uiLanguage` y llevan ahí los dos idiomas.

Después de añadir o cambiar textos, con el `lupdate` y el `lrelease` de Qt 6
(paquete `qt6-l10n-tools` de Ubuntu), desde `app/`:

```sh
lupdate -locations none -no-obsolete -source-language es -target-language en \
    qml src -ts traducciones/grimorio_en.ts
# traducir las entradas nuevas (Qt Linguist, o a mano en el .ts)
lrelease traducciones/grimorio_en.ts -qm traducciones/grimorio_en.qm
```

El lenguaje de búsqueda (`tipo:`, `estrellas:`…) es el mismo en los dos
idiomas: el núcleo entiende los nombres de campo en español y en inglés
(`filtro.rs`), y los botones escriben los de español.
