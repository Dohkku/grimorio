[English](LEEME.md) · Español

# Sombreadores

Van compilados (`.qsb`) junto a su fuente, porque el Qt 6.4 de Ubuntu viene sin
`qsb` ni QtShaderTools y el resto de la app no los necesita. Un `.qsb` hecho
con el `qsb` de 6.4 lo leen también los Qt más nuevos, como el 6.8 de Windows.

Para rehacer uno después de tocar su fuente, con el `qsb` de 6.4 (paquete
`qt6-shader-baker` de Ubuntu; sin instalarlo, `apt-get download` de ese y de
`libqt6shadertools6`, `dpkg-deb -x` y `LD_LIBRARY_PATH` a su `lib`):

```sh
qsb --glsl "100 es,120,150" --hlsl 50 --msl 12 -o color.frag.qsb color.frag
```

- `color.frag`: corrige los colores del vídeo en Qt 6.4. El porqué, en
  `Reproductor.qml`, junto a `verdeCambiado`.
