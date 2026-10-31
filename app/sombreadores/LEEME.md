English · [Español](LEEME.es.md)

# Shaders

They are shipped compiled (`.qsb`) next to their source, because the Qt 6.4
in Ubuntu comes without `qsb` or QtShaderTools and the rest of the app does
not need them. A `.qsb` built with the 6.4 `qsb` can also be read by newer Qt
versions, such as the 6.8 used on Windows.

To rebuild one after changing its source, use the 6.4 `qsb` (Ubuntu package
`qt6-shader-baker`; without installing it: `apt-get download` that package and
`libqt6shadertools6`, `dpkg-deb -x`, and point `LD_LIBRARY_PATH` at its `lib`):

```sh
qsb --glsl "100 es,120,150" --hlsl 50 --msl 12 -o color.frag.qsb color.frag
```

- `color.frag`: corrects video colors on Qt 6.4. The reason is explained in
  `Reproductor.qml`, next to `verdeCambiado`.
