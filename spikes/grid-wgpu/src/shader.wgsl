// Una sola llamada de dibujo para toda la malla: seis vértices por celda,
// el resto son datos de instancia. La celda se dibuja con esquinas redondeadas
// mediante SDF, así no hacen falta ni máscaras ni texturas de borde.

struct Globals {
    viewport: vec2<f32>,
    radius: f32,
    highlight: f32,
    selection: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct Instance {
    @location(0) rect: vec4<f32>,   // x, y, ancho, alto (en píxeles)
    @location(1) uv: vec4<f32>,     // u0, v0, u1, v1
    @location(2) tint: vec4<f32>,   // color dominante + alfa de la textura
    @location(3) extra: vec4<f32>,   // capa del atlas, selección, aparición, _
};

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local: vec2<f32>,  // posición dentro de la celda, en píxeles
    @location(2) size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) extra: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VertexOut {
    // Dos triángulos: (0,0) (1,0) (0,1) / (1,0) (1,1) (0,1)
    var esquinas = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let c = esquinas[vi];
    let px = inst.rect.xy + c * inst.rect.zw;

    // Píxeles a espacio de recorte, con el origen arriba a la izquierda.
    let ndc = vec2<f32>(
        px.x / globals.viewport.x * 2.0 - 1.0,
        1.0 - px.y / globals.viewport.y * 2.0,
    );

    var out: VertexOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = mix(inst.uv.xy, inst.uv.zw, c);
    out.local = c * inst.rect.zw;
    out.size = inst.rect.zw;
    out.tint = inst.tint;
    out.extra = inst.extra;
    return out;
}

// Distancia con signo a un rectángulo de esquinas redondeadas.
fn sdf_caja(p: vec2<f32>, medio: vec2<f32>, r: f32) -> f32 {
    let d = abs(p) - medio + vec2<f32>(r, r);
    return length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let medio = in.size * 0.5;
    let p = in.local - medio;
    let r = min(globals.radius, min(medio.x, medio.y));
    let d = sdf_caja(p, medio, r);

    // Antialias de un píxel en el borde: sin esto las esquinas dentellean.
    let alpha = 1.0 - smoothstep(-1.0, 0.0, d);
    if (alpha <= 0.001) {
        discard;
    }

    let tex = textureSampleLevel(atlas, atlas_sampler, in.uv, i32(in.extra.x), 0.0);
    // Mientras la miniatura no ha llegado se pinta el color dominante del
    // elemento: la malla nunca enseña huecos grises.
    var color = mix(in.tint.rgb, tex.rgb, in.tint.a * in.extra.z);

    // Celda señalada por el puntero: un punto más de luz, nada más.
    if (in.extra.w > 0.5) {
        color = color + vec3<f32>(globals.highlight);
    }

    // Anillo de selección por dentro del borde.
    if (in.extra.y > 0.5) {
        let borde = 1.0 - smoothstep(-2.5, -1.0, d);
        color = mix(color, globals.selection.rgb, borde * 0.9);
    }

    return vec4<f32>(color, alpha);
}
