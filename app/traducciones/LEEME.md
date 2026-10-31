English · [Español](LEEME.es.md)

# Translations

The interface is written in Spanish (`qsTr("…")` in QML, `tr("…")` in C++) and
translated to English here. `grimorio_en.ts` is the source of the translation;
`grimorio_en.qm` is its compiled form, committed so that building does not
need the Linguist tools. `main.cpp` loads it when the language in settings is
English, or when it is «system default» and the system is not in Spanish.

Text built in JavaScript (plurals, the names of search filters) does not go
through `qsTr`: `textos.js` and `consulta.js` check `Qt.uiLanguage` and write
both languages there.

After adding or changing texts, with the Qt 6 `lupdate` and `lrelease`
(Ubuntu package `qt6-l10n-tools`), from `app/`:

```sh
lupdate -locations none -no-obsolete -source-language es -target-language en \
    qml src -ts traducciones/grimorio_en.ts
# translate the new entries (Qt Linguist, or edit the .ts by hand)
lrelease traducciones/grimorio_en.ts -qm traducciones/grimorio_en.qm
```

The search language (`tipo:`, `estrellas:`…) is shared by both languages: the
core understands the Spanish and the English field names (`filtro.rs`), and the
buttons write the Spanish ones.
