// Corrige los colores de un vídeo ya pintado: cada píxel por una matriz que
// llega de `Reproductor.qml`. El porqué y de dónde sale la matriz, allí.
//
// Los píxeles llegan premultiplicados. Se quita el alfa antes de corregir y se
// vuelve a poner después: si no, la parte fija de la corrección teñiría las
// franjas transparentes de alrededor del vídeo.
//
// Compilado a mano a `color.frag.qsb`: ver LEEME.md de esta carpeta.
#version 440

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    mat4 correccion;
};

layout(binding = 1) uniform sampler2D source;

void main()
{
    vec4 c = texture(source, qt_TexCoord0);
    vec3 rgb = c.a > 0.0 ? c.rgb / c.a : vec3(0.0);
    rgb = (correccion * vec4(rgb, 1.0)).rgb;
    fragColor = vec4(clamp(rgb, 0.0, 1.0) * c.a, c.a) * qt_Opacity;
}
