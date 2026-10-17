//! Modelos 3D: leerlos, medirlos y dibujarlos sin tarjeta gráfica.
//!
//! El núcleo no sabe qué es una ventana, así que la miniatura de un modelo se
//! dibuja aquí a mano: un rasterizador de triángulos con búfer de profundidad,
//! luz de dos lados y supermuestreo. No es bonito como un render de Blender,
//! pero se lee de un vistazo en una celda de la malla, y un STL de un millón de
//! triángulos sale en una fracción de segundo.
//!
//! Lo que se lee es lo que se usa de verdad en la mesa de trabajo: STL y 3MF de
//! impresión, OBJ y PLY de intercambio y escaneo, glTF/GLB de motor de juego.
//! Todo en Rust y sin dependencias: cada formato son longitudes y números
//! explícitos. El 3MF es un zip, y para inflarlo se usa `miniz_oxide`, que ya
//! estaba compilado dentro por el decodificador de PNG.
//!
//! `.blend` y `.fbx` no se leen aquí: los abre Blender, si está, y vuelca sus
//! triángulos (ver [`crate::derivado::blender_a_tris`]).
//!
//! Convención interna: **Y hacia arriba**. Los formatos de impresión (STL, 3MF)
//! vienen con Z hacia arriba y se giran al leerlos; si no, una pieza tumbada en
//! la cama de la impresora saldría de canto en la miniatura.

use crate::error::{Error, Result};
use image::RgbImage;
use std::collections::HashMap;
use std::path::Path;

/// Extensiones que este módulo sabe leer sin ayuda.
pub const PROPIAS: &[&str] = &["stl", "obj", "ply", "glb", "gltf", "3mf"];
/// Extensiones que solo se leen convirtiéndolas con Blender.
pub const CON_BLENDER: &[&str] = &["blend", "fbx"];

/// Un modelo hecho triángulos sueltos: cada tres vértices, uno.
///
/// Sin índices a propósito. Lo que se hace con él es dibujarlo una vez y
/// escribirlo para el visor, y en los dos casos los índices solo añaden un
/// salto en memoria por vértice.
///
/// Los materiales son opcionales: un STL no tiene y se pinta entero del color
/// de la pieza. Si los hay, `mat` dice el de cada triángulo y `uv` dónde cae
/// cada vértice en su textura.
#[derive(Debug, Default, Clone)]
pub struct Malla {
    pub v: Vec<[f32; 3]>,
    /// Cuántos milímetros mide una unidad del archivo, si el formato lo dice.
    /// STL no lo dice, pero todo el mundo lo escribe en milímetros; OBJ y PLY
    /// no lo dicen y no hay costumbre, así que ahí no se inventa.
    pub mm_por_unidad: Option<f32>,
    /// Uno por vértice, o vacío si ningún material lleva textura.
    pub uv: Vec<[f32; 2]>,
    /// Uno por triángulo, índice en `materiales`; vacío si no hay materiales.
    pub mat: Vec<u32>,
    pub materiales: Vec<Material>,
    /// Las imágenes tal como venían en el archivo (PNG, JPEG…), sin decodificar:
    /// el visor las decodifica a su manera y aquí solo se miran para la miniatura.
    pub texturas: Vec<Vec<u8>>,
}

/// Cómo se pinta una parte del modelo.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub nombre: String,
    /// En sRGB y con alfa. `None` si el archivo no dice nada: entonces va del
    /// color de la pieza, igual que un modelo sin materiales.
    pub color: Option<[f32; 4]>,
    /// Índice en `Malla::texturas`.
    pub textura: Option<usize>,
    /// Por debajo de este alfa de la textura no se pinta: pelo, pestañas, hojas.
    pub corte: Option<f32>,
    /// glTF dice que por defecto solo se ve la cara de delante. Respetarlo es
    /// lo que hace desaparecer la cáscara del contorno de los personajes de
    /// estilo dibujo animado, que es la misma malla hinchada y del revés.
    pub dos_caras: bool,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            nombre: String::new(),
            color: None,
            textura: None,
            corte: None,
            dos_caras: true,
        }
    }
}

impl Malla {
    pub fn triangulos(&self) -> usize {
        self.v.len() / 3
    }

    /// Esquinas de la caja que lo contiene.
    pub fn caja(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &self.v {
            for k in 0..3 {
                if p[k].is_finite() {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        if !lo[0].is_finite() {
            return ([0.0; 3], [0.0; 3]);
        }
        (lo, hi)
    }

    /// Ancho, fondo y alto en milímetros, en el orden en que se dicen de una
    /// pieza encima de una mesa. `None` si el formato no dice sus unidades.
    pub fn medidas_mm(&self) -> Option<[f32; 3]> {
        let k = self.mm_por_unidad?;
        let (lo, hi) = self.caja();
        // Por dentro Y es la altura; hacia fuera se dice ancho, fondo, alto.
        Some([
            (hi[0] - lo[0]) * k,
            (hi[2] - lo[2]) * k,
            (hi[1] - lo[1]) * k,
        ])
    }
}

fn mal(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}

/// De Z hacia arriba a Y hacia arriba, sin reflejar: girar, no espejar, o la
/// rosca de un tornillo saldría al revés.
fn zup([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

/// Lee un modelo de los bytes de su archivo.
///
/// `junto_a` es la ruta del archivo, para los glTF que guardan su geometría en
/// un `.bin` al lado. Sin ella, un glTF así no se puede leer y se dice.
pub fn leer(bytes: &[u8], ext: &str, junto_a: Option<&Path>) -> Result<Malla> {
    let m = match ext.to_ascii_lowercase().as_str() {
        "stl" => stl(bytes)?,
        "obj" => obj(bytes)?,
        "ply" => ply(bytes)?,
        "glb" | "gltf" => gltf(bytes, junto_a)?,
        "3mf" => tres_mf(bytes)?,
        e => return Err(mal(format!("no sé leer modelos .{e}"))),
    };
    if m.v.is_empty() {
        return Err(mal("el modelo no tiene ningún triángulo"));
    }
    Ok(m)
}

/// Lo que vuelca el guion de Blender (ver [`crate::derivado::blender_a_tris`]),
/// con Z hacia arriba como en Blender. Todo little endian:
/// ```text
/// "GRIMTRI2" · f32 milímetros por unidad · u32 triángulos
/// f32 vértices[triángulos·9] · u32 material[triángulos]
/// u32 con coordenadas de textura (0 o 1), y si 1: f32 uv[triángulos·6]
/// u32 materiales, y por material: u32 bytes · nombre en UTF-8
///     f32 color[4] (sRGB) · i32 textura (-1 sin) · u32 banderas
///     (1: a dos caras, 2: sin color propio)
/// u32 texturas, y por textura: u32 bytes · la imagen tal cual
/// ```
/// `"GRIMTRIS"` es el de antes: los milímetros y los triángulos, sin más.
pub fn leer_tris(b: &[u8]) -> Result<Malla> {
    let corto = || mal("Blender devolvió una malla cortada");
    if b.len() < 12 || (&b[..8] != b"GRIMTRIS" && &b[..8] != b"GRIMTRI2") {
        return Err(mal("Blender no devolvió una malla"));
    }
    let mm = f32_le(b, 8);
    let mut m = Malla {
        mm_por_unidad: (mm.is_finite() && mm > 0.0).then_some(mm),
        ..Default::default()
    };
    if &b[..8] == b"GRIMTRIS" {
        let n = (b.len() - 12) / 36 * 9;
        m.v = (0..n / 3)
            .map(|k| {
                let o = 12 + k * 12;
                zup([f32_le(b, o), f32_le(b, o + 4), f32_le(b, o + 8)])
            })
            .collect();
    } else {
        // Un cursor que no se sale: un volcado cortado es un error, no un pánico.
        let mut o = 12usize;
        let toma = |o: &mut usize, n: usize| -> Result<std::ops::Range<usize>> {
            let r = *o..o.checked_add(n).filter(|f| *f <= b.len()).ok_or_else(corto)?;
            *o = r.end;
            Ok(r)
        };
        let n = u32_le(b, toma(&mut o, 4)?.start) as usize;
        let r = toma(&mut o, n.checked_mul(36).ok_or_else(corto)?)?;
        m.v = (0..n * 3)
            .map(|k| {
                let p = r.start + k * 12;
                zup([f32_le(b, p), f32_le(b, p + 4), f32_le(b, p + 8)])
            })
            .collect();
        let r = toma(&mut o, n * 4)?;
        m.mat = (0..n).map(|k| u32_le(b, r.start + k * 4)).collect();
        if u32_le(b, toma(&mut o, 4)?.start) == 1 {
            let r = toma(&mut o, n * 24)?;
            m.uv = (0..n * 3)
                .map(|k| [f32_le(b, r.start + k * 8), f32_le(b, r.start + k * 8 + 4)])
                .collect();
        }
        let nm = u32_le(b, toma(&mut o, 4)?.start) as usize;
        for _ in 0..nm.min(1 << 16) {
            let largo = u32_le(b, toma(&mut o, 4)?.start) as usize;
            let nombre = String::from_utf8_lossy(&b[toma(&mut o, largo)?]).into_owned();
            let r = toma(&mut o, 24)?;
            let c = |k: usize| f32_le(b, r.start + k * 4);
            let textura = i32::from_le_bytes(b[r.start + 16..r.start + 20].try_into().unwrap());
            let banderas = u32_le(b, r.start + 20);
            m.materiales.push(Material {
                nombre,
                color: (banderas & 2 == 0).then(|| [c(0), c(1), c(2), c(3)]),
                textura: usize::try_from(textura).ok(),
                corte: None,
                dos_caras: banderas & 1 != 0,
            });
        }
        let nt = u32_le(b, toma(&mut o, 4)?.start) as usize;
        for _ in 0..nt.min(1 << 16) {
            let largo = u32_le(b, toma(&mut o, 4)?.start) as usize;
            m.texturas.push(b[toma(&mut o, largo)?].to_vec());
        }
        if m.materiales.is_empty() || m.mat.iter().any(|x| *x as usize >= m.materiales.len()) {
            m.mat.clear();
            m.materiales.clear();
        }
    }
    if m.v.is_empty() {
        return Err(mal("la escena no tiene nada visible con superficie"));
    }
    Ok(m)
}

// ------------------------------------------------------------------- STL

fn f32_le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn u32_le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn stl(b: &[u8]) -> Result<Malla> {
    // Hay STL binarios cuya cabecera empieza por «solid»: lo que decide es si
    // el tamaño cuadra con el número de triángulos que declaran.
    let binario = b.len() >= 84 && {
        let n = u32_le(b, 80) as usize;
        n > 0 && 84 + n.saturating_mul(50) <= b.len()
    };
    let mut v = Vec::new();
    if binario {
        let n = u32_le(b, 80) as usize;
        v.reserve(n * 3);
        for t in 0..n {
            let o = 84 + t * 50 + 12;
            for k in 0..3 {
                let p = o + k * 12;
                v.push(zup([f32_le(b, p), f32_le(b, p + 4), f32_le(b, p + 8)]));
            }
        }
    } else {
        let texto = String::from_utf8_lossy(b);
        let mut it = texto.split_ascii_whitespace();
        while let Some(w) = it.next() {
            if w == "vertex" {
                let mut p = [0f32; 3];
                for c in &mut p {
                    *c = it
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| mal("STL de texto con un vértice roto"))?;
                }
                v.push(zup(p));
            }
        }
        v.truncate(v.len() / 3 * 3);
    }
    Ok(Malla {
        v,
        mm_por_unidad: Some(1.0),
        ..Default::default()
    })
}

// ------------------------------------------------------------------- OBJ

fn obj(b: &[u8]) -> Result<Malla> {
    let texto = String::from_utf8_lossy(b);
    let mut vs: Vec<[f32; 3]> = Vec::new();
    let mut v = Vec::new();
    let mut cara: Vec<usize> = Vec::with_capacity(8);
    for linea in texto.lines() {
        let mut it = linea.split_ascii_whitespace();
        match it.next() {
            Some("v") => {
                let mut p = [0f32; 3];
                for c in &mut p {
                    *c = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                }
                vs.push(p);
            }
            Some("f") => {
                cara.clear();
                for tok in it {
                    let i: i64 = match tok.split('/').next().and_then(|s| s.parse().ok()) {
                        Some(i) => i,
                        None => continue,
                    };
                    // Uno-basados, y los negativos cuentan desde el final.
                    let i = if i < 0 { vs.len() as i64 + i } else { i - 1 };
                    if i >= 0 && (i as usize) < vs.len() {
                        cara.push(i as usize);
                    }
                }
                // Abanico: vale para cualquier polígono convexo, que es lo que
                // escribe todo exportador que se precie.
                for k in 1..cara.len().saturating_sub(1) {
                    v.push(vs[cara[0]]);
                    v.push(vs[cara[k]]);
                    v.push(vs[cara[k + 1]]);
                }
            }
            _ => {}
        }
    }
    Ok(Malla {
        v,
        mm_por_unidad: None,
        ..Default::default()
    })
}

// ------------------------------------------------------------------- PLY

#[derive(Clone, Copy)]
enum Tipo {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl Tipo {
    fn de(s: &str) -> Option<Tipo> {
        Some(match s {
            "char" | "int8" => Tipo::I8,
            "uchar" | "uint8" => Tipo::U8,
            "short" | "int16" => Tipo::I16,
            "ushort" | "uint16" => Tipo::U16,
            "int" | "int32" => Tipo::I32,
            "uint" | "uint32" => Tipo::U32,
            "float" | "float32" => Tipo::F32,
            "double" | "float64" => Tipo::F64,
            _ => return None,
        })
    }
    fn bytes(self) -> usize {
        match self {
            Tipo::I8 | Tipo::U8 => 1,
            Tipo::I16 | Tipo::U16 => 2,
            Tipo::I32 | Tipo::U32 | Tipo::F32 => 4,
            Tipo::F64 => 8,
        }
    }
}

enum Prop {
    Escalar(Tipo, String),
    Lista(Tipo, Tipo, String),
}

struct Elemento {
    nombre: String,
    n: usize,
    props: Vec<Prop>,
}

/// Lee números de un PLY, sea de texto o binario, con el mismo interfaz.
struct Lector<'a> {
    b: &'a [u8],
    o: usize,
    formato: u8, // 0 texto, 1 binario LE, 2 binario BE
}

impl Lector<'_> {
    fn num(&mut self, t: Tipo) -> Result<f64> {
        if self.formato == 0 {
            while self.o < self.b.len() && self.b[self.o].is_ascii_whitespace() {
                self.o += 1;
            }
            let ini = self.o;
            while self.o < self.b.len() && !self.b[self.o].is_ascii_whitespace() {
                self.o += 1;
            }
            return std::str::from_utf8(&self.b[ini..self.o])
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| mal("PLY de texto con un número roto"));
        }
        let n = t.bytes();
        if self.o + n > self.b.len() {
            return Err(mal("PLY cortado antes de tiempo"));
        }
        let mut x = [0u8; 8];
        x[..n].copy_from_slice(&self.b[self.o..self.o + n]);
        if self.formato == 2 {
            x[..n].reverse();
        }
        self.o += n;
        Ok(match t {
            Tipo::I8 => x[0] as i8 as f64,
            Tipo::U8 => x[0] as f64,
            Tipo::I16 => i16::from_le_bytes([x[0], x[1]]) as f64,
            Tipo::U16 => u16::from_le_bytes([x[0], x[1]]) as f64,
            Tipo::I32 => i32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f64,
            Tipo::U32 => u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f64,
            Tipo::F32 => f32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f64,
            Tipo::F64 => f64::from_le_bytes(x),
        })
    }
}

fn ply(b: &[u8]) -> Result<Malla> {
    let fin = b
        .windows(10)
        .position(|w| w == b"end_header")
        .ok_or_else(|| mal("PLY sin cabecera"))?;
    let cabecera = String::from_utf8_lossy(&b[..fin]);
    let mut cuerpo = fin + 10;
    // Tras «end_header» va un salto de línea, \n o \r\n.
    while cuerpo < b.len() && (b[cuerpo] == b'\r' || b[cuerpo] == b'\n') {
        cuerpo += 1;
        if b[cuerpo - 1] == b'\n' {
            break;
        }
    }

    let mut formato = 255u8;
    let mut elems: Vec<Elemento> = Vec::new();
    for l in cabecera.lines() {
        let w: Vec<&str> = l.split_ascii_whitespace().collect();
        match w.as_slice() {
            ["format", f, ..] => {
                formato = match *f {
                    "ascii" => 0,
                    "binary_little_endian" => 1,
                    "binary_big_endian" => 2,
                    _ => return Err(mal(format!("PLY con formato desconocido: {f}"))),
                }
            }
            ["element", nombre, n] => elems.push(Elemento {
                nombre: nombre.to_string(),
                n: n.parse().map_err(|_| mal("PLY con un recuento roto"))?,
                props: Vec::new(),
            }),
            ["property", "list", tn, ti, nombre] => {
                let (tn, ti) = Tipo::de(tn)
                    .zip(Tipo::de(ti))
                    .ok_or_else(|| mal("PLY con un tipo desconocido"))?;
                if let Some(e) = elems.last_mut() {
                    e.props.push(Prop::Lista(tn, ti, nombre.to_string()));
                }
            }
            ["property", t, nombre] => {
                let t = Tipo::de(t).ok_or_else(|| mal("PLY con un tipo desconocido"))?;
                if let Some(e) = elems.last_mut() {
                    e.props.push(Prop::Escalar(t, nombre.to_string()));
                }
            }
            _ => {}
        }
    }
    if formato == 255 {
        return Err(mal("PLY sin formato"));
    }

    let mut lec = Lector {
        b,
        o: cuerpo,
        formato,
    };
    let mut vs: Vec<[f32; 3]> = Vec::new();
    let mut v = Vec::new();
    let mut lista: Vec<usize> = Vec::new();
    for e in &elems {
        let es_vertice = e.nombre == "vertex";
        let es_cara = e.nombre == "face";
        for _ in 0..e.n {
            let mut p = [0f32; 3];
            for prop in &e.props {
                match prop {
                    Prop::Escalar(t, nombre) => {
                        let x = lec.num(*t)? as f32;
                        if es_vertice {
                            match nombre.as_str() {
                                "x" => p[0] = x,
                                "y" => p[1] = x,
                                "z" => p[2] = x,
                                _ => {}
                            }
                        }
                    }
                    Prop::Lista(tn, ti, nombre) => {
                        let n = lec.num(*tn)? as usize;
                        lista.clear();
                        for _ in 0..n {
                            lista.push(lec.num(*ti)? as usize);
                        }
                        if es_cara && nombre.starts_with("vertex_ind") {
                            for k in 1..lista.len().saturating_sub(1) {
                                for &i in &[lista[0], lista[k], lista[k + 1]] {
                                    v.push(
                                        *vs.get(i).ok_or_else(|| {
                                            mal("PLY con una cara que apunta fuera")
                                        })?,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            if es_vertice {
                vs.push(p);
            }
        }
    }
    Ok(Malla {
        v,
        mm_por_unidad: None,
        ..Default::default()
    })
}

// ------------------------------------------------------------- glTF / GLB

type Mat4 = [f32; 16]; // por columnas, como glTF

const IDENTIDAD: Mat4 = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [0f32; 16];
    for c in 0..4 {
        for f in 0..4 {
            r[c * 4 + f] = (0..4).map(|k| a[k * 4 + f] * b[c * 4 + k]).sum();
        }
    }
    r
}

fn aplicar(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

fn trs(t: [f32; 3], q: [f32; 4], s: [f32; 3]) -> Mat4 {
    let [x, y, z, w] = q;
    let r = [
        1. - 2. * (y * y + z * z),
        2. * (x * y + z * w),
        2. * (x * z - y * w),
        2. * (x * y - z * w),
        1. - 2. * (x * x + z * z),
        2. * (y * z + x * w),
        2. * (x * z + y * w),
        2. * (y * z - x * w),
        1. - 2. * (x * x + y * y),
    ];
    [
        r[0] * s[0],
        r[1] * s[0],
        r[2] * s[0],
        0.,
        r[3] * s[1],
        r[4] * s[1],
        r[5] * s[1],
        0.,
        r[6] * s[2],
        r[7] * s[2],
        r[8] * s[2],
        0.,
        t[0],
        t[1],
        t[2],
        1.,
    ]
}

/// Base64 estándar, el de las URI `data:` de glTF. Son veinte líneas y no
/// merece una dependencia.
fn base64(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err(mal("base64 roto dentro del glTF")),
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

fn gltf(b: &[u8], junto_a: Option<&Path>) -> Result<Malla> {
    use serde_json::Value;
    let (json, bin): (&[u8], Option<&[u8]>) = if b.starts_with(b"glTF") {
        if b.len() < 20 {
            return Err(mal("GLB cortado"));
        }
        let largo_json = u32_le(b, 12) as usize;
        let json = b
            .get(20..20 + largo_json)
            .ok_or_else(|| mal("GLB cortado"))?;
        let o = 20 + largo_json;
        let bin = if b.len() >= o + 8 && &b[o + 4..o + 8] == b"BIN\0" {
            let n = u32_le(b, o) as usize;
            b.get(o + 8..o + 8 + n)
        } else {
            None
        };
        (json, bin)
    } else {
        (b, None)
    };
    let doc: Value =
        serde_json::from_slice(json).map_err(|e| mal(format!("glTF con JSON roto: {e}")))?;

    if let Some(req) = doc["extensionsRequired"].as_array() {
        for e in req {
            let e = e.as_str().unwrap_or("");
            if e.contains("draco") || e.contains("meshopt") {
                return Err(mal(format!(
                    "el glTF va comprimido con {e}, que todavía no sé descomprimir"
                )));
            }
        }
    }

    let mut buffers: Vec<Vec<u8>> = Vec::new();
    for (i, buf) in doc["buffers"].as_array().into_iter().flatten().enumerate() {
        let datos = match buf["uri"].as_str() {
            None if i == 0 => bin.map(|b| b.to_vec()).unwrap_or_default(),
            None => Vec::new(),
            Some(uri) if uri.starts_with("data:") => {
                let coma = uri.find(',').ok_or_else(|| mal("URI data: sin datos"))?;
                base64(&uri[coma + 1..])?
            }
            Some(uri) => {
                let base = junto_a
                    .and_then(|p| p.parent())
                    .ok_or_else(|| mal(format!("el glTF guarda su geometría fuera, en «{uri}»")))?;
                std::fs::read(base.join(uri)).map_err(|_| {
                    mal(format!(
                        "el glTF guarda su geometría en «{uri}», y no está al lado"
                    ))
                })?
            }
        };
        buffers.push(datos);
    }

    // Lee un accessor como una lista de vectores de `comp` componentes.
    let leer_accessor = |i: usize, comp: usize| -> Result<Vec<[f32; 4]>> {
        let a = &doc["accessors"][i];
        let n = a["count"].as_u64().unwrap_or(0) as usize;
        let tipo = a["componentType"].as_u64().unwrap_or(5126);
        let normalizado = a["normalized"].as_bool().unwrap_or(false);
        let tam = match tipo {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            _ => 4,
        };
        let mut out = vec![[0f32; 4]; n];
        let Some(bv) = a["bufferView"].as_u64() else {
            // Sin bufferView el accessor vale todo ceros (o es disperso).
            return Ok(out);
        };
        let vista = &doc["bufferViews"][bv as usize];
        let buf = buffers
            .get(vista["buffer"].as_u64().unwrap_or(0) as usize)
            .ok_or_else(|| mal("glTF con un buffer que no existe"))?;
        let ini = vista["byteOffset"].as_u64().unwrap_or(0) as usize
            + a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let paso = vista["byteStride"]
            .as_u64()
            .map(|s| s as usize)
            .unwrap_or(tam * comp);
        for (k, sal) in out.iter_mut().enumerate() {
            for c in 0..comp {
                let o = ini + k * paso + c * tam;
                let x = buf
                    .get(o..o + tam)
                    .ok_or_else(|| mal("glTF con un accessor que se sale del buffer"))?;
                sal[c] = match tipo {
                    5126 => f32::from_le_bytes([x[0], x[1], x[2], x[3]]),
                    5125 => u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f32,
                    5123 => {
                        let v = u16::from_le_bytes([x[0], x[1]]) as f32;
                        if normalizado {
                            v / 65535.0
                        } else {
                            v
                        }
                    }
                    5122 => {
                        let v = i16::from_le_bytes([x[0], x[1]]) as f32;
                        if normalizado {
                            (v / 32767.0).max(-1.0)
                        } else {
                            v
                        }
                    }
                    5121 => {
                        let v = x[0] as f32;
                        if normalizado {
                            v / 255.0
                        } else {
                            v
                        }
                    }
                    _ => {
                        let v = x[0] as i8 as f32;
                        if normalizado {
                            (v / 127.0).max(-1.0)
                        } else {
                            v
                        }
                    }
                };
            }
        }
        Ok(out)
    };
    // Los índices, aparte y en entero: un u32 grande pasado por f32 pierde
    // precisión a partir de 16 millones.
    let leer_indices = |i: usize| -> Result<Vec<u32>> {
        let a = &doc["accessors"][i];
        let n = a["count"].as_u64().unwrap_or(0) as usize;
        let tipo = a["componentType"].as_u64().unwrap_or(5125);
        let tam = match tipo {
            5121 => 1,
            5123 => 2,
            _ => 4,
        };
        let vista = &doc["bufferViews"][a["bufferView"].as_u64().unwrap_or(0) as usize];
        let buf = buffers
            .get(vista["buffer"].as_u64().unwrap_or(0) as usize)
            .ok_or_else(|| mal("glTF con un buffer que no existe"))?;
        let ini = vista["byteOffset"].as_u64().unwrap_or(0) as usize
            + a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let paso = vista["byteStride"]
            .as_u64()
            .map(|s| s as usize)
            .unwrap_or(tam);
        (0..n)
            .map(|k| {
                let o = ini + k * paso;
                let x = buf
                    .get(o..o + tam)
                    .ok_or_else(|| mal("glTF con índices fuera del buffer"))?;
                Ok(match tam {
                    1 => x[0] as u32,
                    2 => u16::from_le_bytes([x[0], x[1]]) as u32,
                    _ => u32::from_le_bytes([x[0], x[1], x[2], x[3]]),
                })
            })
            .collect()
    };

    // Las imágenes, tal como vienen: dentro del GLB, en una URI `data:` o en
    // un archivo al lado. La que no se encuentra no es motivo para no leer el
    // modelo: esa parte sale sin textura.
    let mut texturas: Vec<Vec<u8>> = Vec::new();
    let mut de_imagen: HashMap<usize, Option<usize>> = HashMap::new();
    let mut imagen = |i: usize| -> Option<usize> {
        if let Some(t) = de_imagen.get(&i) {
            return *t;
        }
        let im = &doc["images"][i];
        let datos = if let Some(bv) = im["bufferView"].as_u64() {
            let vista = &doc["bufferViews"][bv as usize];
            let ini = vista["byteOffset"].as_u64().unwrap_or(0) as usize;
            let n = vista["byteLength"].as_u64().unwrap_or(0) as usize;
            buffers
                .get(vista["buffer"].as_u64().unwrap_or(0) as usize)
                .and_then(|b| b.get(ini..ini + n))
                .map(|b| b.to_vec())
        } else {
            match im["uri"].as_str() {
                Some(uri) if uri.starts_with("data:") => uri
                    .find(',')
                    .and_then(|c| base64(&uri[c + 1..]).ok()),
                Some(uri) => junto_a
                    .and_then(|p| p.parent())
                    .and_then(|d| std::fs::read(d.join(sin_porcentajes(uri))).ok()),
                None => None,
            }
        };
        let t = datos.filter(|d| !d.is_empty()).map(|d| {
            texturas.push(d);
            texturas.len() - 1
        });
        de_imagen.insert(i, t);
        t
    };

    let mut materiales: Vec<Material> = Vec::new();
    // Qué juego de coordenadas usa la textura de cada material: casi siempre
    // el 0, pero glTF deja elegir.
    let mut juego_uv: Vec<usize> = Vec::new();
    for (i, m) in doc["materials"].as_array().into_iter().flatten().enumerate() {
        // El modelo de brillo y especular es anterior a PBR y hay archivos
        // viejos que solo traen ese.
        let sg = &m["extensions"]["KHR_materials_pbrSpecularGlossiness"];
        let pbr = &m["pbrMetallicRoughness"];
        let (factor, tex) = if sg.is_object() && !pbr.is_object() {
            (&sg["diffuseFactor"], &sg["diffuseTexture"])
        } else {
            (&pbr["baseColorFactor"], &pbr["baseColorTexture"])
        };
        let textura = tex["index"].as_u64().and_then(|t| {
            let t = &doc["textures"][t as usize];
            // La fuente normal antes que la de WebP: la normal la entiende
            // cualquiera que vaya a pintar esto.
            t["source"]
                .as_u64()
                .or_else(|| t["extensions"]["EXT_texture_webp"]["source"].as_u64())
                .and_then(|s| imagen(s as usize))
        });
        let color = match factor.as_array() {
            Some(f) if f.len() >= 4 => {
                let c = |k: usize| f[k].as_f64().unwrap_or(1.0) as f32;
                Some([a_srgb(c(0)), a_srgb(c(1)), a_srgb(c(2)), c(3)])
            }
            _ if textura.is_some() => Some([1.0; 4]),
            _ => None,
        };
        let corte = match m["alphaMode"].as_str() {
            Some("MASK") => Some(m["alphaCutoff"].as_f64().unwrap_or(0.5) as f32),
            // Una miniatura no mezcla transparencias. Lo casi invisible se
            // quita y lo demás se pinta entero: el pelo hecho de tarjetas se
            // ve pelo, y un cristal tapa lo de detrás, que es menos grave.
            Some("BLEND") => Some(0.1),
            _ => None,
        };
        materiales.push(Material {
            nombre: m["name"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("material {i}")),
            color,
            textura,
            corte,
            dos_caras: m["doubleSided"].as_bool().unwrap_or(false),
        });
        juego_uv.push(tex["texCoord"].as_u64().unwrap_or(0) as usize);
    }
    // Lo que no dice material va con uno neutro. A dos caras: quien no puso
    // material tampoco cuidó hacia dónde mira cada cara.
    let sin_material = materiales.len();
    materiales.push(Material {
        nombre: "sin material".into(),
        ..Default::default()
    });
    juego_uv.push(0);
    let con_uv = materiales.iter().any(|m| m.textura.is_some());

    let mut out = Malla {
        // glTF va en metros por especificación.
        mm_por_unidad: Some(1000.0),
        materiales,
        texturas,
        ..Default::default()
    };
    let malla_en = |m: usize, mat: &Mat4, out: &mut Malla| -> Result<()> {
        // Una escala negativa da la vuelta a los triángulos: lo que era la cara
        // de delante pasa a ser la de detrás, y hay que deshacerlo.
        let columna = |k: usize| [mat[k], mat[k + 1], mat[k + 2]];
        let espejo = punto(columna(0), cruz(columna(4), columna(8))) < 0.0;
        for prim in doc["meshes"][m]["primitives"]
            .as_array()
            .into_iter()
            .flatten()
        {
            // Solo triángulos. Puntos y líneas no tienen superficie que pintar.
            if prim["mode"].as_u64().unwrap_or(4) != 4 {
                continue;
            }
            let Some(pos) = prim["attributes"]["POSITION"].as_u64() else {
                continue;
            };
            let material = prim["material"]
                .as_u64()
                .map(|x| x as usize)
                .filter(|x| *x < sin_material)
                .unwrap_or(sin_material);
            let ps: Vec<[f32; 3]> = leer_accessor(pos as usize, 3)?
                .into_iter()
                .map(|p| aplicar(mat, [p[0], p[1], p[2]]))
                .collect();
            let uvs: Vec<[f32; 2]> = if con_uv {
                let clave = format!("TEXCOORD_{}", juego_uv[material]);
                match prim["attributes"][clave.as_str()].as_u64() {
                    Some(a) => leer_accessor(a as usize, 2)?
                        .into_iter()
                        .map(|t| [t[0], t[1]])
                        .collect(),
                    None => Vec::new(),
                }
            } else {
                Vec::new()
            };
            let indices: Vec<u32> = match prim["indices"].as_u64() {
                Some(ix) => leer_indices(ix as usize)?,
                None => (0..ps.len() as u32).collect(),
            };
            for t in indices.chunks_exact(3) {
                let orden = if espejo {
                    [t[0], t[2], t[1]]
                } else {
                    [t[0], t[1], t[2]]
                };
                for i in orden {
                    out.v.push(
                        *ps.get(i as usize)
                            .ok_or_else(|| mal("glTF con un índice que apunta fuera"))?,
                    );
                    if con_uv {
                        out.uv.push(uvs.get(i as usize).copied().unwrap_or([0.0; 2]));
                    }
                }
                out.mat.push(material as u32);
            }
        }
        Ok(())
    };

    let nodos = doc["nodes"].as_array().cloned().unwrap_or_default();
    let raices: Vec<usize> = {
        let escena = doc["scene"].as_u64().unwrap_or(0) as usize;
        match doc["scenes"][escena]["nodes"].as_array() {
            Some(r) => r
                .iter()
                .filter_map(|n| n.as_u64())
                .map(|n| n as usize)
                .collect(),
            None => {
                // Sin escenas: las raíces son los nodos que no son hijos de nadie.
                let mut hijo = vec![false; nodos.len()];
                for n in &nodos {
                    for h in n["children"].as_array().into_iter().flatten() {
                        if let Some(h) = h.as_u64() {
                            if let Some(x) = hijo.get_mut(h as usize) {
                                *x = true;
                            }
                        }
                    }
                }
                (0..nodos.len()).filter(|i| !hijo[*i]).collect()
            }
        }
    };

    if nodos.is_empty() {
        // Un glTF sin nodos todavía puede tener mallas: se pintan tal cual.
        for m in 0..doc["meshes"].as_array().map(|a| a.len()).unwrap_or(0) {
            malla_en(m, &IDENTIDAD, &mut out)?;
        }
    } else {
        let mut pila: Vec<(usize, Mat4, u32)> =
            raices.into_iter().map(|r| (r, IDENTIDAD, 0)).collect();
        while let Some((i, padre, hondo)) = pila.pop() {
            // Un glTF con ciclos no es válido, pero no puede colgar la importación.
            if hondo > 64 {
                continue;
            }
            let Some(n) = nodos.get(i) else { continue };
            let local = if let Some(m) = n["matrix"].as_array() {
                let mut r = IDENTIDAD;
                for (k, x) in m.iter().take(16).enumerate() {
                    r[k] = x.as_f64().unwrap_or(0.0) as f32;
                }
                r
            } else {
                let f = |clave: &str, def: &[f32]| -> Vec<f32> {
                    n[clave]
                        .as_array()
                        .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect())
                        .unwrap_or_else(|| def.to_vec())
                };
                let t = f("translation", &[0., 0., 0.]);
                let q = f("rotation", &[0., 0., 0., 1.]);
                let s = f("scale", &[1., 1., 1.]);
                if t.len() < 3 || q.len() < 4 || s.len() < 3 {
                    IDENTIDAD
                } else {
                    trs(
                        [t[0], t[1], t[2]],
                        [q[0], q[1], q[2], q[3]],
                        [s[0], s[1], s[2]],
                    )
                }
            };
            let mat = mul(&padre, &local);
            if let Some(m) = n["mesh"].as_u64() {
                malla_en(m as usize, &mat, &mut out)?;
            }
            for h in n["children"].as_array().into_iter().flatten() {
                if let Some(h) = h.as_u64() {
                    pila.push((h as usize, mat, hondo + 1));
                }
            }
        }
    }

    // El neutro de reserva, si nadie lo usó, sobra: es el último y quitarlo
    // no mueve los índices de los demás.
    if !out.mat.iter().any(|m| *m as usize == sin_material) {
        out.materiales.pop();
    }
    Ok(out)
}

/// De luz lineal, que es como glTF guarda sus colores, a sRGB, que es como se
/// pintan las texturas y como se ve todo lo demás.
fn a_srgb(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.003_130_8 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// `mi%20textura.png` → `mi textura.png`: las URI de glTF van escapadas.
fn sin_porcentajes(uri: &str) -> String {
    let b = uri.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(x) = u8::from_str_radix(&uri[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ------------------------------------------------------------------- 3MF

/// Cuánto se deja crecer, entre todas, a las entradas infladas de un 3MF.
///
/// Un zip de unos pocos kilobytes puede declararse en gigas al inflarse —la
/// «bomba zip»— y sin tope bastaba arrastrar uno a la ventana para que la
/// importación se comiera la memoria de la máquina en un hilo de rayon. Un
/// modelo de verdad con millones de triángulos ronda los cientos de megas de
/// XML; un giga deja sitio de sobra y corta el abuso antes de que haga daño.
const TOPE_INFLADO: usize = 1 << 30;

/// Las entradas de un zip, ya infladas. Solo lo que 3MF necesita: sin zip64 ni
/// cifrado, que ningún laminador escribe.
fn zip_entradas(b: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    zip_entradas_hasta(b, TOPE_INFLADO)
}

fn zip_entradas_hasta(b: &[u8], tope_total: usize) -> Result<Vec<(String, Vec<u8>)>> {
    let desde = b.len().saturating_sub(65_557);
    let fin = (desde..b.len().saturating_sub(21))
        .rev()
        .find(|&o| u32_le(b, o) == 0x0605_4b50)
        .ok_or_else(|| mal("el 3MF no es un zip"))?;
    let n = u16::from_le_bytes([b[fin + 10], b[fin + 11]]) as usize;
    let mut o = u32_le(b, fin + 16) as usize;
    let mut out = Vec::new();
    let mut inflado = 0usize;
    for _ in 0..n {
        if o + 46 > b.len() || u32_le(b, o) != 0x0201_4b50 {
            return Err(mal("el índice del zip está roto"));
        }
        let metodo = u16::from_le_bytes([b[o + 10], b[o + 11]]);
        let comprimido = u32_le(b, o + 20) as usize;
        let ln = u16::from_le_bytes([b[o + 28], b[o + 29]]) as usize;
        let le = u16::from_le_bytes([b[o + 30], b[o + 31]]) as usize;
        let lc = u16::from_le_bytes([b[o + 32], b[o + 33]]) as usize;
        let local = u32_le(b, o + 42) as usize;
        let nombre = String::from_utf8_lossy(&b[o + 46..(o + 46 + ln).min(b.len())]).into_owned();
        o += 46 + ln + le + lc;

        if !nombre.to_ascii_lowercase().ends_with(".model") {
            continue;
        }
        if local + 30 > b.len() {
            return Err(mal("el zip está cortado"));
        }
        let lln = u16::from_le_bytes([b[local + 26], b[local + 27]]) as usize;
        let lle = u16::from_le_bytes([b[local + 28], b[local + 29]]) as usize;
        let ini = local + 30 + lln + lle;
        let datos = b
            .get(ini..ini + comprimido)
            .ok_or_else(|| mal("el zip está cortado"))?;
        let datos = match metodo {
            0 => datos.to_vec(),
            8 => {
                let tope = tope_total.saturating_sub(inflado);
                miniz_oxide::inflate::decompress_to_vec_with_limit(datos, tope).map_err(|e| {
                    if e.status == miniz_oxide::inflate::TINFLStatus::HasMoreOutput {
                        mal(format!(
                            "{nombre} se infla a más de {} MB; no parece un modelo",
                            tope_total >> 20
                        ))
                    } else {
                        mal(format!("no pude inflar {nombre}"))
                    }
                })?
            }
            m => return Err(mal(format!("zip con compresión {m}, que no conozco"))),
        };
        inflado += datos.len();
        out.push((format!("/{}", nombre.trim_start_matches('/')), datos));
    }
    Ok(out)
}

/// Recorre las etiquetas de un XML sin construir ningún árbol: para leer un
/// 3MF basta con saber el nombre de cada una y sus atributos.
fn etiquetas_xml(texto: &str, mut f: impl FnMut(&str, bool, &[(&str, &str)])) {
    let b = texto.as_bytes();
    let mut i = 0;
    let mut attrs: Vec<(&str, &str)> = Vec::with_capacity(8);
    while let Some(p) = texto[i..].find('<') {
        let ini = i + p + 1;
        let Some(q) = texto[ini..].find('>') else {
            break;
        };
        let fin = ini + q;
        i = fin + 1;
        let dentro = &texto[ini..fin];
        if dentro.starts_with('?') || dentro.starts_with('!') {
            continue;
        }
        let cierra = dentro.starts_with('/');
        let dentro = dentro.trim_start_matches('/').trim_end_matches('/');
        let corte = dentro
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(dentro.len());
        let nombre = &dentro[..corte];
        let nombre = nombre.rsplit(':').next().unwrap_or(nombre);
        attrs.clear();
        let resto = &dentro[corte..];
        let rb = resto.as_bytes();
        let mut k = 0;
        while k < rb.len() {
            while k < rb.len() && rb[k].is_ascii_whitespace() {
                k += 1;
            }
            let a0 = k;
            while k < rb.len() && rb[k] != b'=' && !rb[k].is_ascii_whitespace() {
                k += 1;
            }
            let clave = &resto[a0..k];
            while k < rb.len() && rb[k] != b'"' && rb[k] != b'\'' {
                k += 1;
            }
            if k >= rb.len() {
                break;
            }
            let comilla = rb[k];
            k += 1;
            let v0 = k;
            while k < rb.len() && rb[k] != comilla {
                k += 1;
            }
            attrs.push((clave, &resto[v0..k.min(rb.len())]));
            k += 1;
        }
        let _ = b;
        f(nombre, cierra, &attrs);
    }
}

/// Transformación de 3MF: doce números, para vectores fila (`p' = p · M`).
type Mat3x4 = [f32; 12];
const IDENTIDAD_3MF: Mat3x4 = [1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.];

fn mat_3mf(s: Option<&str>) -> Mat3x4 {
    let Some(s) = s else { return IDENTIDAD_3MF };
    let n: Vec<f32> = s
        .split_ascii_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    if n.len() == 12 {
        let mut m = [0f32; 12];
        m.copy_from_slice(&n);
        m
    } else {
        IDENTIDAD_3MF
    }
}

fn aplicar_3mf(m: &Mat3x4, p: [f32; 3]) -> [f32; 3] {
    [
        p[0] * m[0] + p[1] * m[3] + p[2] * m[6] + m[9],
        p[0] * m[1] + p[1] * m[4] + p[2] * m[7] + m[10],
        p[0] * m[2] + p[1] * m[5] + p[2] * m[8] + m[11],
    ]
}

/// Primero `a`, luego `b`.
fn compone_3mf(a: &Mat3x4, b: &Mat3x4) -> Mat3x4 {
    let mut r = [0f32; 12];
    for fila in 0..4 {
        let (x, y, z, t) = if fila < 3 {
            (a[fila * 3], a[fila * 3 + 1], a[fila * 3 + 2], 0.0)
        } else {
            (a[9], a[10], a[11], 1.0)
        };
        for c in 0..3 {
            r[fila * 3 + c] = x * b[c] + y * b[3 + c] + z * b[6 + c] + t * b[9 + c];
        }
    }
    r
}

#[derive(Default)]
struct Objeto3mf {
    v: Vec<[f32; 3]>,
    t: Vec<[u32; 3]>,
    /// (archivo, id, transformación)
    partes: Vec<(String, String, Mat3x4)>,
}

fn tres_mf(b: &[u8]) -> Result<Malla> {
    let entradas = zip_entradas(b)?;
    if entradas.is_empty() {
        return Err(mal("el 3MF no lleva ningún modelo dentro"));
    }
    let mut objetos: HashMap<(String, String), Objeto3mf> = HashMap::new();
    let mut construir: Vec<(String, String, Mat3x4)> = Vec::new();
    let mut unidad = 1.0f32;
    let raiz = entradas
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("/3D/3dmodel.model"))
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| entradas[0].0.clone());

    for (archivo, datos) in &entradas {
        let texto = String::from_utf8_lossy(datos);
        let mut actual: Option<String> = None;
        let mut en_build = false;
        etiquetas_xml(&texto, |nombre, cierra, a| {
            let attr = |k: &str| {
                a.iter()
                    .find(|(c, _)| *c == k || c.rsplit(':').next() == Some(k))
                    .map(|(_, v)| *v)
            };
            match (nombre, cierra) {
                ("model", false) if *archivo == raiz => {
                    unidad = match attr("unit").unwrap_or("millimeter") {
                        "micron" => 0.001,
                        "centimeter" => 10.0,
                        "inch" => 25.4,
                        "foot" => 304.8,
                        "meter" => 1000.0,
                        _ => 1.0,
                    };
                }
                ("object", false) => actual = attr("id").map(String::from),
                ("object", true) => actual = None,
                ("build", false) => en_build = true,
                ("build", true) => en_build = false,
                ("vertex", false) => {
                    if let Some(id) = &actual {
                        let n = |k| attr(k).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        objetos
                            .entry((archivo.clone(), id.clone()))
                            .or_default()
                            .v
                            .push([n("x"), n("y"), n("z")]);
                    }
                }
                ("triangle", false) => {
                    if let Some(id) = &actual {
                        let n = |k| attr(k).and_then(|s| s.parse().ok()).unwrap_or(0u32);
                        objetos
                            .entry((archivo.clone(), id.clone()))
                            .or_default()
                            .t
                            .push([n("v1"), n("v2"), n("v3")]);
                    }
                }
                ("component", false) => {
                    if let (Some(id), Some(obj)) = (&actual, attr("objectid")) {
                        let otro = attr("path").map(String::from).unwrap_or(archivo.clone());
                        objetos
                            .entry((archivo.clone(), id.clone()))
                            .or_default()
                            .partes
                            .push((otro, obj.to_string(), mat_3mf(attr("transform"))));
                    }
                }
                ("item", false) if en_build && *archivo == raiz => {
                    if let Some(obj) = attr("objectid") {
                        let otro = attr("path").map(String::from).unwrap_or(archivo.clone());
                        construir.push((otro, obj.to_string(), mat_3mf(attr("transform"))));
                    }
                }
                _ => {}
            }
        });
    }

    if construir.is_empty() {
        // Sin <build>, todo lo que tenga malla.
        for (k, o) in &objetos {
            if !o.t.is_empty() {
                construir.push((k.0.clone(), k.1.clone(), IDENTIDAD_3MF));
            }
        }
    }

    let mut v = Vec::new();
    let mut pila: Vec<(String, String, Mat3x4, u32)> = construir
        .into_iter()
        .map(|(a, i, m)| (a, i, m, 0))
        .collect();
    while let Some((archivo, id, m, hondo)) = pila.pop() {
        if hondo > 16 {
            continue;
        }
        let Some(o) = objetos.get(&(archivo.clone(), id.clone())) else {
            continue;
        };
        for t in &o.t {
            for &i in t {
                if let Some(p) = o.v.get(i as usize) {
                    v.push(zup(aplicar_3mf(&m, *p)));
                }
            }
            v.truncate(v.len() / 3 * 3);
        }
        for (a, i, mp) in &o.partes {
            // La parte se coloca dentro del objeto, y el objeto en la cama.
            pila.push((a.clone(), i.clone(), compone_3mf(mp, &m), hondo + 1));
        }
    }
    Ok(Malla {
        v,
        mm_por_unidad: Some(unidad),
        ..Default::default()
    })
}

// ------------------------------------------------------------- el dibujo

fn resta(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cruz(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn punto(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn unitario(a: [f32; 3]) -> [f32; 3] {
    let n = punto(a, a).sqrt();
    if n > 0.0 {
        [a[0] / n, a[1] / n, a[2] / n]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// El giro con el que se mira una pieza por primera vez: un poco desde
/// arriba y desde la derecha, como se mira algo encima de una mesa. El visor
/// arranca con el mismo, para que abrir no sea un salto.
pub const GIRO_INICIAL: f32 = -35.0;
pub const INCLINACION_INICIAL: f32 = 25.0;

fn vista(p: [f32; 3], cy: f32, sy: f32, cx: f32, sx: f32) -> [f32; 3] {
    // Giro alrededor de Y y luego inclinación alrededor de X.
    let x = p[0] * cy + p[2] * sy;
    let z = -p[0] * sy + p[2] * cy;
    let y = p[1] * cx - z * sx;
    let z = p[1] * sx + z * cx;
    [x, y, z]
}

/// Dibuja el modelo en un cuadrado de `lado` píxeles.
///
/// Se dibuja al doble y se reduce: sin eso, los bordes de una pieza de
/// impresión —que son casi todo líneas rectas— salen en escalera.
pub fn dibujar(m: &Malla, lado: u32) -> RgbImage {
    const SS: u32 = 2;
    let l = lado * SS;
    let (cy, sy) = (
        GIRO_INICIAL.to_radians().cos(),
        GIRO_INICIAL.to_radians().sin(),
    );
    let (cx, sx) = (
        INCLINACION_INICIAL.to_radians().cos(),
        INCLINACION_INICIAL.to_radians().sin(),
    );

    let (lo, hi) = m.caja();
    let centro = [
        (lo[0] + hi[0]) / 2.0,
        (lo[1] + hi[1]) / 2.0,
        (lo[2] + hi[2]) / 2.0,
    ];
    let vs: Vec<[f32; 3]> =
        m.v.iter()
            .map(|p| vista(resta(*p, centro), cy, sy, cx, sx))
            .collect();

    // Encajar lo que se ve, no la caja: una pieza larga girada ocupa menos.
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in &vs {
        if p[0].is_finite() && p[1].is_finite() {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
    }
    let tam = (x1 - x0).max(y1 - y0).max(1e-9);
    let escala = l as f32 * 0.84 / tam;
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);

    // Fondo en degradado oscuro, y lo que no dice su color en arcilla: un tono
    // neutro deja ver la forma, que en una pieza de impresión es lo que importa.
    let arcilla = [0.86f32, 0.82, 0.76];
    let texturas = texturas_para_dibujar(m, l);
    let con_uv = m.uv.len() == m.v.len();

    let mut prof = vec![f32::NEG_INFINITY; (l * l) as usize];
    let mut color = vec![[0f32; 3]; (l * l) as usize];
    let clave = unitario([-0.45, 0.65, 0.62]);
    let relleno = unitario([0.7, 0.1, 0.5]);

    for (it, t) in vs.chunks_exact(3).enumerate() {
        let n = cruz(resta(t[1], t[0]), resta(t[2], t[0]));
        if !n.iter().all(|c| c.is_finite()) || punto(n, n) == 0.0 {
            continue;
        }
        let material = m.mat.get(it).and_then(|i| m.materiales.get(*i as usize));
        let mut n = unitario(n);
        // Luz de dos lados: muchos modelos traen caras con el sentido cambiado
        // y no por eso son negras por fuera. Salvo que el material diga que
        // solo tiene cara por delante: entonces lo de detrás no se pinta.
        if n[2] < 0.0 {
            if material.is_some_and(|m| !m.dos_caras) {
                continue;
            }
            n = [-n[0], -n[1], -n[2]];
        }
        let i = 0.16
            + 0.70 * punto(n, clave).max(0.0)
            + 0.22 * punto(n, relleno).max(0.0)
            + 0.18 * (1.0 - n[2]).powi(2);
        let i = i.min(1.25);
        let base = material
            .and_then(|m| m.color)
            .unwrap_or([arcilla[0], arcilla[1], arcilla[2], 1.0]);
        let textura = material
            .and_then(|m| m.textura)
            .and_then(|k| texturas.get(k))
            .and_then(|t| t.as_ref())
            .filter(|_| con_uv);
        let corte = material.and_then(|m| m.corte);
        let uv = if textura.is_some() {
            [m.uv[it * 3], m.uv[it * 3 + 1], m.uv[it * 3 + 2]]
        } else {
            [[0.0; 2]; 3]
        };

        let s: Vec<[f32; 3]> = t
            .iter()
            .map(|p| {
                [
                    (p[0] - mx) * escala + l as f32 / 2.0,
                    -(p[1] - my) * escala + l as f32 / 2.0,
                    p[2],
                ]
            })
            .collect();
        let area =
            (s[1][0] - s[0][0]) * (s[2][1] - s[0][1]) - (s[2][0] - s[0][0]) * (s[1][1] - s[0][1]);
        if area.abs() < 1e-12 {
            continue;
        }
        let bx0 = s
            .iter()
            .map(|p| p[0])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as i64;
        let bx1 = s
            .iter()
            .map(|p| p[0])
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(l as f32 - 1.0) as i64;
        let by0 = s
            .iter()
            .map(|p| p[1])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as i64;
        let by1 = s
            .iter()
            .map(|p| p[1])
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(l as f32 - 1.0) as i64;
        for y in by0..=by1 {
            for x in bx0..=bx1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((s[1][0] - px) * (s[2][1] - py) - (s[2][0] - px) * (s[1][1] - py)) / area;
                let w1 = ((s[2][0] - px) * (s[0][1] - py) - (s[0][0] - px) * (s[2][1] - py)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * s[0][2] + w1 * s[1][2] + w2 * s[2][2];
                let k = (y as u32 * l + x as u32) as usize;
                if z <= prof[k] {
                    continue;
                }
                // Proyección ortográfica: interpolar las coordenadas de
                // textura en pantalla es exacto, no hace falta corregir
                // la perspectiva.
                let mut c = base;
                if let Some(tx) = textura {
                    let u = w0 * uv[0][0] + w1 * uv[1][0] + w2 * uv[2][0];
                    let v = w0 * uv[0][1] + w1 * uv[1][1] + w2 * uv[2][1];
                    let p = muestra(tx, u, v);
                    for k in 0..4 {
                        c[k] *= p[k];
                    }
                }
                if corte.is_some_and(|corte| c[3] < corte) {
                    continue;
                }
                prof[k] = z;
                color[k] = [c[0] * i, c[1] * i, c[2] * i];
            }
        }
    }

    let mut img = RgbImage::new(lado, lado);
    for y in 0..lado {
        for x in 0..lado {
            let mut acc = [0f32; 3];
            for dy in 0..SS {
                for dx in 0..SS {
                    let (gx, gy) = (x * SS + dx, y * SS + dy);
                    let k = (gy * l + gx) as usize;
                    let c = if prof[k].is_finite() {
                        color[k]
                    } else {
                        let t = gy as f32 / l as f32;
                        let f = (0.20 - 0.08 * t).max(0.0);
                        [f, f, f * 1.08]
                    };
                    for c2 in 0..3 {
                        acc[c2] += c[c2];
                    }
                }
            }
            let n = (SS * SS) as f32;
            img.put_pixel(
                x,
                y,
                image::Rgb([
                    (acc[0] / n * 255.0).clamp(0.0, 255.0) as u8,
                    (acc[1] / n * 255.0).clamp(0.0, 255.0) as u8,
                    (acc[2] / n * 255.0).clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }
    img
}

/// Las texturas que algún material usa, decodificadas y reducidas a lo que
/// cabe en el dibujo: una de 4096 en una miniatura de 1024 es memoria tirada.
/// La que no se deja decodificar queda en `None` y esa parte va sin textura.
fn texturas_para_dibujar(m: &Malla, lado: u32) -> Vec<Option<image::RgbaImage>> {
    let usadas: std::collections::HashSet<usize> =
        m.materiales.iter().filter_map(|x| x.textura).collect();
    m.texturas
        .iter()
        .enumerate()
        .map(|(k, bytes)| {
            if !usadas.contains(&k) {
                return None;
            }
            let img = image::load_from_memory(bytes).ok()?.to_rgba8();
            let tope = lado.min(1024);
            Some(if img.width() > tope || img.height() > tope {
                let f = tope as f32 / img.width().max(img.height()) as f32;
                image::imageops::thumbnail(
                    &img,
                    ((img.width() as f32 * f) as u32).max(1),
                    ((img.height() as f32 * f) as u32).max(1),
                )
            } else {
                img
            })
        })
        .collect()
}

/// El color de la textura en (u, v), repitiéndola fuera de 0‒1 como hacen
/// glTF y Blender por defecto. El origen es la esquina de arriba a la
/// izquierda, como en glTF.
fn muestra(t: &image::RgbaImage, u: f32, v: f32) -> [f32; 4] {
    let (w, h) = (t.width(), t.height());
    let f = |x: f32, n: u32| -> u32 {
        let x = if x.is_finite() { x - x.floor() } else { 0.0 };
        ((x * n as f32) as u32).min(n - 1)
    };
    let p = t.get_pixel(f(u, w), f(v, h)).0;
    [
        p[0] as f32 / 255.0,
        p[1] as f32 / 255.0,
        p[2] as f32 / 255.0,
        p[3] as f32 / 255.0,
    ]
}

// ------------------------------------------------------- para el visor

/// Versión del formato de [`escribir_malla`]. Lo guardado con otra se rehace.
pub const VERSION_MALLA: u32 = 2;

/// Escribe la malla para el visor: centrada, a escala unidad y en f32.
///
/// Formato (little endian):
/// ```text
/// "GRIMALLA" · u32 versión (2) · u32 triángulos
/// f32 radio (la mitad del lado mayor, en unidades del archivo)
/// f32 medidas_mm[3] (ancho, fondo, alto; -1 si no se saben)
/// f32 vértices[triángulos·9], centrados y divididos por el radio
/// u32 grupos, y por grupo:
///     u32 primer triángulo · u32 triángulos · f32 color[4] (sRGB y alfa)
///     i32 textura (-1 sin) · f32 corte de alfa (0 sin) · u32 banderas
///     (1: a dos caras, 2: sin color propio, va del de la pieza)
/// u32 con coordenadas de textura (0 o 1), y si 1: f32 uv[triángulos·6]
/// u32 texturas, y por textura: u32 bytes · la imagen tal cual (PNG, JPEG…)
/// ```
/// Sin normales: el visor las saca del propio triángulo en el sombreador, y
/// así el archivo es la mitad. Los triángulos van ordenados por material, para
/// pintar cada grupo de una vez. La versión 1 acababa en los vértices: un visor
/// de antes abre esta y la pinta sin color.
pub fn escribir_malla(m: &Malla, salida: &mut impl std::io::Write) -> std::io::Result<()> {
    let (lo, hi) = m.caja();
    let c = [
        (lo[0] + hi[0]) / 2.0,
        (lo[1] + hi[1]) / 2.0,
        (lo[2] + hi[2]) / 2.0,
    ];
    let r = ((hi[0] - lo[0]).max(hi[1] - lo[1]).max(hi[2] - lo[2]) / 2.0).max(1e-12);
    let med = m.medidas_mm().unwrap_or([-1.0; 3]);
    let n = m.triangulos();
    let con_mat = m.mat.len() == n && !m.materiales.is_empty();
    let con_uv = con_mat
        && m.uv.len() == m.v.len()
        && m.materiales.iter().any(|x| x.textura.is_some());

    // Ordenados por material sin perder el orden dentro de cada uno.
    let mut orden: Vec<usize> = (0..n).collect();
    if con_mat {
        orden.sort_by_key(|&t| m.mat[t]);
    }

    let mut b = Vec::with_capacity(48 + n * if con_uv { 60 } else { 36 });
    let f = |b: &mut Vec<u8>, x: f32| b.extend_from_slice(&x.to_le_bytes());
    let u = |b: &mut Vec<u8>, x: u32| b.extend_from_slice(&x.to_le_bytes());
    b.extend_from_slice(b"GRIMALLA");
    u(&mut b, VERSION_MALLA);
    u(&mut b, n as u32);
    f(&mut b, r);
    for x in med {
        f(&mut b, x);
    }
    for &t in &orden {
        for p in &m.v[t * 3..t * 3 + 3] {
            for k in 0..3 {
                let x = (p[k] - c[k]) / r;
                f(&mut b, if x.is_finite() { x } else { 0.0 });
            }
        }
    }

    let mut grupos: Vec<(u32, u32, Option<&Material>)> = Vec::new();
    if con_mat {
        for (k, &t) in orden.iter().enumerate() {
            let mt = m.mat[t];
            match grupos.last_mut() {
                Some(g) if k > 0 && m.mat[orden[k - 1]] == mt => g.1 += 1,
                _ => grupos.push((k as u32, 1, m.materiales.get(mt as usize))),
            }
        }
    } else if n > 0 {
        grupos.push((0, n as u32, None));
    }
    // Solo las texturas que algún grupo pinta: las de un material que ahora
    // va de un color liso no hace falta llevarlas.
    let mut usadas: Vec<usize> = Vec::new();
    if con_uv {
        for (_, _, mat) in &grupos {
            if let Some(t) = mat.and_then(|x| x.textura) {
                if t < m.texturas.len() && !usadas.contains(&t) {
                    usadas.push(t);
                }
            }
        }
    }
    u(&mut b, grupos.len() as u32);
    for (primero, cuantos, mat) in &grupos {
        u(&mut b, *primero);
        u(&mut b, *cuantos);
        let color = mat.and_then(|x| x.color);
        for x in color.unwrap_or([1.0; 4]) {
            f(&mut b, x);
        }
        let textura = mat
            .and_then(|x| x.textura)
            .and_then(|t| usadas.iter().position(|u| *u == t))
            .map(|t| t as i32)
            .unwrap_or(-1);
        b.extend_from_slice(&textura.to_le_bytes());
        f(&mut b, mat.and_then(|x| x.corte).unwrap_or(0.0));
        let dos_caras = mat.is_none_or(|x| x.dos_caras);
        u(&mut b, dos_caras as u32 | if color.is_none() { 2 } else { 0 });
    }
    let con_uv = !usadas.is_empty();
    u(&mut b, con_uv as u32);
    if con_uv {
        for &t in &orden {
            for p in &m.uv[t * 3..t * 3 + 3] {
                f(&mut b, if p[0].is_finite() { p[0] } else { 0.0 });
                f(&mut b, if p[1].is_finite() { p[1] } else { 0.0 });
            }
        }
    }
    u(&mut b, usadas.len() as u32);
    for t in usadas.iter().map(|&k| &m.texturas[k]) {
        u(&mut b, t.len() as u32);
        b.extend_from_slice(t);
    }
    salida.write_all(&b)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Un cubo de 10 mm en STL binario, escrito a mano.
    pub(crate) fn cubo_stl() -> Vec<u8> {
        let p = |x: f32, y: f32, z: f32| [x * 10.0, y * 10.0, z * 10.0];
        let caras: [[[f32; 3]; 4]; 6] = [
            [p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
            [p(0., 0., 1.), p(1., 0., 1.), p(1., 1., 1.), p(0., 1., 1.)],
            [p(0., 0., 0.), p(1., 0., 0.), p(1., 0., 1.), p(0., 0., 1.)],
            [p(0., 1., 0.), p(1., 1., 0.), p(1., 1., 1.), p(0., 1., 1.)],
            [p(0., 0., 0.), p(0., 1., 0.), p(0., 1., 1.), p(0., 0., 1.)],
            [p(1., 0., 0.), p(1., 1., 0.), p(1., 1., 1.), p(1., 0., 1.)],
        ];
        let mut b = vec![0u8; 80];
        b.extend_from_slice(&12u32.to_le_bytes());
        for c in &caras {
            for t in [[c[0], c[1], c[2]], [c[0], c[2], c[3]]] {
                b.extend_from_slice(&[0u8; 12]);
                for v in t {
                    for x in v {
                        b.extend_from_slice(&x.to_le_bytes());
                    }
                }
                b.extend_from_slice(&[0u8; 2]);
            }
        }
        b
    }

    #[test]
    fn un_stl_binario_se_lee_y_se_mide_en_milimetros() {
        let m = leer(&cubo_stl(), "stl", None).unwrap();
        assert_eq!(m.triangulos(), 12);
        let med = m.medidas_mm().unwrap();
        for x in med {
            assert!((x - 10.0).abs() < 1e-4, "{med:?}");
        }
    }

    #[test]
    fn un_stl_de_texto_tambien() {
        let s = "solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 20 0 0\nvertex 0 5 3\nendloop\nendfacet\nendsolid x\n";
        let m = leer(s.as_bytes(), "stl", None).unwrap();
        assert_eq!(m.triangulos(), 1);
        // Z hacia arriba en el archivo: 3 mm de alto, 5 de fondo.
        assert_eq!(m.medidas_mm().unwrap(), [20.0, 5.0, 3.0]);
    }

    #[test]
    fn un_obj_con_un_cuadrado_da_dos_triangulos() {
        let s = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1/1/1 2/2/2 3/3/3 -1\n";
        let m = leer(s.as_bytes(), "obj", None).unwrap();
        assert_eq!(m.triangulos(), 2);
        assert!(m.medidas_mm().is_none(), "OBJ no dice sus unidades");
    }

    #[test]
    fn un_ply_de_texto_y_uno_binario_dicen_lo_mismo() {
        let texto = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 2 0\n3 0 1 2\n";
        let a = leer(texto.as_bytes(), "ply", None).unwrap();
        let mut bin = b"ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n".to_vec();
        for p in [[0f32, 0., 0.], [1., 0., 0.], [0., 2., 0.]] {
            for x in p {
                bin.extend_from_slice(&x.to_le_bytes());
            }
        }
        bin.push(3);
        for i in [0i32, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        let b = leer(&bin, "ply", None).unwrap();
        assert_eq!(a.v, b.v);
        assert_eq!(a.triangulos(), 1);
    }

    /// Un GLB mínimo: un triángulo con índices, colgado de un nodo escalado.
    fn glb_triangulo() -> Vec<u8> {
        let mut bin = Vec::new();
        for p in [[0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
            for x in p {
                bin.extend_from_slice(&x.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2, 0] {
            bin.extend_from_slice(&i.to_le_bytes()); // el último es relleno
        }
        let json = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
            "nodes":[{"mesh":0,"scale":[2,2,2]}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}],
            "buffers":[{"byteLength":44}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},
                         {"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}]}"#;
        let mut j = json.as_bytes().to_vec();
        while j.len() % 4 != 0 {
            j.push(b' ');
        }
        let mut b = Vec::new();
        b.extend_from_slice(b"glTF");
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&((12 + 8 + j.len() + 8 + bin.len()) as u32).to_le_bytes());
        b.extend_from_slice(&(j.len() as u32).to_le_bytes());
        b.extend_from_slice(b"JSON");
        b.extend_from_slice(&j);
        b.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        b.extend_from_slice(b"BIN\0");
        b.extend_from_slice(&bin);
        b
    }

    #[test]
    fn un_glb_respeta_la_escala_de_su_nodo_y_va_en_metros() {
        let m = leer(&glb_triangulo(), "glb", None).unwrap();
        assert_eq!(m.triangulos(), 1);
        assert_eq!(m.v[1], [2.0, 0.0, 0.0]);
        assert_eq!(m.medidas_mm().unwrap(), [2000.0, 0.0, 2000.0]);
    }

    #[test]
    fn el_base64_de_las_uri_data() {
        assert_eq!(base64("R3JpbW9yaW8=").unwrap(), b"Grimorio");
    }

    /// Un zip con una sola entrada guardada sin comprimir.
    fn zip_de(nombre: &str, datos: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        let local = b.len() as u32;
        b.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        b.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        b.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        b.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        b.extend_from_slice(&(nombre.len() as u16).to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(nombre.as_bytes());
        b.extend_from_slice(datos);
        let cd = b.len() as u32;
        b.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        b.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        b.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        b.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        b.extend_from_slice(&(nombre.len() as u16).to_le_bytes());
        b.extend_from_slice(&[0u8; 12]);
        b.extend_from_slice(&local.to_le_bytes());
        b.extend_from_slice(nombre.as_bytes());
        let tam = b.len() as u32 - cd;
        b.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        b.extend_from_slice(&tam.to_le_bytes());
        b.extend_from_slice(&cd.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b
    }

    /// Una bomba zip no se infla entera: al pasar del tope se para y lo dice.
    #[test]
    fn un_3mf_que_se_infla_de_mas_se_corta() {
        let plano = vec![b' '; 4 << 20];
        let comprimido = miniz_oxide::deflate::compress_to_vec(&plano, 6);
        let mut zip = zip_de("3D/3dmodel.model", &comprimido);
        // El método de compresión del directorio central: 8 es deflate.
        let cd = zip.len() - 22 - (46 + "3D/3dmodel.model".len());
        zip[cd + 10] = 8;

        let e = zip_entradas_hasta(&zip, 1 << 20).unwrap_err().to_string();
        assert!(e.contains("se infla a más de 1 MB"), "{e}");
        let bien = zip_entradas_hasta(&zip, 8 << 20).unwrap();
        assert_eq!(bien[0].1.len(), 4 << 20);
    }

    #[test]
    fn un_3mf_con_un_componente_movido_se_coloca_donde_dice() {
        let xml = r#"<?xml version="1.0"?>
<model unit="centimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
 <resources>
  <object id="1" type="model"><mesh>
   <vertices><vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="1"/></vertices>
   <triangles><triangle v1="0" v2="1" v3="2"/></triangles>
  </mesh></object>
  <object id="2"><components><component objectid="1" transform="1 0 0 0 1 0 0 0 1 5 0 0"/></components></object>
 </resources>
 <build><item objectid="2"/></build>
</model>"#;
        let m = leer(&zip_de("3D/3dmodel.model", xml.as_bytes()), "3mf", None).unwrap();
        assert_eq!(m.triangulos(), 1);
        assert_eq!(m.v[0][0], 5.0, "el componente va desplazado 5 unidades");
        assert_eq!(
            m.medidas_mm().unwrap(),
            [10.0, 10.0, 10.0],
            "en centímetros"
        );
    }

    #[test]
    fn un_modelo_vacio_es_un_error_y_lo_dice() {
        let e = leer(b"solid nada\nendsolid nada\n", "stl", None).unwrap_err();
        assert!(e.to_string().contains("triángulo"));
    }

    #[test]
    fn la_miniatura_tiene_pieza_en_medio_y_fondo_en_las_esquinas() {
        let m = leer(&cubo_stl(), "stl", None).unwrap();
        let img = dibujar(&m, 128);
        let centro = img.get_pixel(64, 64).0;
        let esquina = img.get_pixel(1, 1).0;
        let brillo = |p: [u8; 3]| p.iter().map(|x| *x as u32).sum::<u32>();
        assert!(
            brillo(centro) > brillo(esquina) * 2,
            "{centro:?} contra {esquina:?}"
        );
    }

    #[test]
    fn la_malla_del_visor_va_centrada_y_a_escala_unidad() {
        let m = leer(&cubo_stl(), "stl", None).unwrap();
        let mut b = Vec::new();
        escribir_malla(&m, &mut b).unwrap();
        assert_eq!(&b[..8], b"GRIMALLA");
        assert_eq!(u32_le(&b, 8), VERSION_MALLA);
        assert_eq!(u32_le(&b, 12), 12);
        let n = 12 * 9;
        for k in 0..n {
            let x = f32_le(&b, 32 + k * 4);
            assert!((-1.0001..=1.0001).contains(&x));
        }
        // Un STL no tiene materiales: un solo grupo, del color de la pieza,
        // sin coordenadas de textura ni texturas.
        let o = 32 + n * 4;
        assert_eq!(u32_le(&b, o), 1);
        assert_eq!(u32_le(&b, o + 8), 12, "el grupo son los doce triángulos");
        assert_eq!(u32_le(&b, o + 36), 1 | 2, "a dos caras y sin color propio");
        assert_eq!(u32_le(&b, o + 40), 0);
        assert_eq!(u32_le(&b, o + 44), 0);
        assert_eq!(b.len(), o + 48);
    }

    /// Un GLB con su JSON y su búfer binario.
    fn glb_de(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut j = json.as_bytes().to_vec();
        while j.len() % 4 != 0 {
            j.push(b' ');
        }
        let mut bin = bin.to_vec();
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let mut b = Vec::new();
        b.extend_from_slice(b"glTF");
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&((12 + 8 + j.len() + 8 + bin.len()) as u32).to_le_bytes());
        b.extend_from_slice(&(j.len() as u32).to_le_bytes());
        b.extend_from_slice(b"JSON");
        b.extend_from_slice(&j);
        b.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        b.extend_from_slice(b"BIN\0");
        b.extend_from_slice(&bin);
        b
    }

    /// Un cuadrado de cara a +Z con una textura roja de 2×2 dentro. `al_reves`
    /// le da la vuelta a los triángulos, y `dos_caras` es el del material.
    fn glb_cuadrado_rojo(al_reves: bool, dos_caras: bool) -> Vec<u8> {
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let mut bin = Vec::new();
        for p in [[0f32, 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]] {
            for x in p {
                bin.extend_from_slice(&x.to_le_bytes());
            }
        }
        for t in [[0f32, 1.], [1., 1.], [1., 0.], [0., 0.]] {
            for x in t {
                bin.extend_from_slice(&x.to_le_bytes());
            }
        }
        let ix: [u16; 6] = if al_reves {
            [0, 2, 1, 0, 3, 2]
        } else {
            [0, 1, 2, 0, 2, 3]
        };
        for i in ix {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        let ini_png = bin.len();
        bin.extend_from_slice(&png);
        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"mesh":0}}],
            "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"TEXCOORD_0":1}},"indices":2,"material":0}}]}}],
            "materials":[{{"name":"Rojo.001","doubleSided":{dos_caras},
                "pbrMetallicRoughness":{{"baseColorTexture":{{"index":0}}}}}}],
            "textures":[{{"source":0}}],
            "images":[{{"bufferView":3,"mimeType":"image/png"}}],
            "buffers":[{{"byteLength":{}}}],
            "bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":48}},
                           {{"buffer":0,"byteOffset":48,"byteLength":32}},
                           {{"buffer":0,"byteOffset":80,"byteLength":12}},
                           {{"buffer":0,"byteOffset":{ini_png},"byteLength":{}}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3"}},
                         {{"bufferView":1,"componentType":5126,"count":4,"type":"VEC2"}},
                         {{"bufferView":2,"componentType":5123,"count":6,"type":"SCALAR"}}]}}"#,
            bin.len(),
            png.len()
        );
        glb_de(&json, &bin)
    }

    #[test]
    fn un_glb_con_textura_trae_su_material_y_la_miniatura_la_pinta() {
        let m = leer(&glb_cuadrado_rojo(false, false), "glb", None).unwrap();
        assert_eq!(m.triangulos(), 2);
        assert_eq!(m.texturas.len(), 1);
        assert_eq!(m.uv.len(), 6);
        assert_eq!(m.mat, vec![0, 0]);
        assert_eq!(m.materiales[0].nombre, "Rojo.001");
        assert_eq!(m.materiales[0].textura, Some(0));
        assert!(!m.materiales[0].dos_caras);
        let p = dibujar(&m, 64).get_pixel(32, 32).0;
        assert!(p[0] > 100 && p[1] < 40 && p[2] < 40, "{p:?}");
    }

    #[test]
    fn lo_que_solo_tiene_cara_por_delante_no_se_ve_por_detras() {
        let fondo = |m: &Malla| {
            let p = dibujar(m, 64).get_pixel(32, 32).0;
            p[0] < 80
        };
        let detras = leer(&glb_cuadrado_rojo(true, false), "glb", None).unwrap();
        assert!(fondo(&detras), "de espaldas y a una cara: no se pinta");
        let dos = leer(&glb_cuadrado_rojo(true, true), "glb", None).unwrap();
        assert!(!fondo(&dos), "a dos caras se ve también de espaldas");
    }

    #[test]
    fn la_malla_del_visor_lleva_grupos_coordenadas_y_textura() {
        let m = leer(&glb_cuadrado_rojo(false, false), "glb", None).unwrap();
        let mut b = Vec::new();
        escribir_malla(&m, &mut b).unwrap();
        let mut o = 32 + 2 * 36;
        assert_eq!(u32_le(&b, o), 1);
        o += 4;
        assert_eq!(i32::from_le_bytes(b[o + 24..o + 28].try_into().unwrap()), 0);
        assert_eq!(u32_le(&b, o + 32), 0, "a una cara y con color");
        o += 36;
        assert_eq!(u32_le(&b, o), 1, "con coordenadas de textura");
        o += 4 + 2 * 24;
        assert_eq!(u32_le(&b, o), 1);
        let largo = u32_le(&b, o + 4) as usize;
        assert_eq!(&b[o + 8..o + 12], b"\x89PNG");
        assert_eq!(b.len(), o + 8 + largo);
    }

    #[test]
    fn lo_que_vuelca_blender_con_materiales_se_lee() {
        let mut b = b"GRIMTRI2".to_vec();
        b.extend_from_slice(&1000f32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        for x in [0f32, 0., 0., 1., 0., 0., 0., 1., 0.] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b.extend_from_slice(&0u32.to_le_bytes()); // material del triángulo
        b.extend_from_slice(&0u32.to_le_bytes()); // sin coordenadas
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"Piel");
        for x in [1f32, 0.5, 0.25, 1.] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b.extend_from_slice(&(-1i32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // a una cara, con color
        b.extend_from_slice(&0u32.to_le_bytes()); // sin texturas
        let m = leer_tris(&b).unwrap();
        assert_eq!(m.triangulos(), 1);
        assert_eq!(m.mat, vec![0]);
        assert_eq!(m.materiales[0].nombre, "Piel");
        assert_eq!(m.materiales[0].color, Some([1.0, 0.5, 0.25, 1.0]));
        assert!(!m.materiales[0].dos_caras);
        // Cortado a la mitad es un error, no un pánico.
        assert!(leer_tris(&b[..b.len() - 10]).is_err());
    }
}
