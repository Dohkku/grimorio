//! Decodificación, miniaturas, paleta y huella perceptual.
//!
//! Todo lo de aquí está detrás de una frontera estrecha a propósito: cuando
//! entren libvips (reducción durante la carga) o los subprocesos de RAW, vídeo y
//! PDF, se sustituye esta implementación sin tocar el resto del núcleo.

use crate::error::Result;
use crate::item::PaletteEntry;
use image::{DynamicImage, GenericImageView, RgbImage};

/// Extensiones que el núcleo sabe decodificar hoy, en Rust puro.
pub const SUPPORTED: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp"];

pub fn is_supported(ext: &str) -> bool {
    let e = ext.to_ascii_lowercase();
    SUPPORTED.contains(&e.as_str())
}

pub fn decode(bytes: &[u8]) -> Result<DynamicImage> {
    Ok(image::load_from_memory(bytes)?)
}

/// Miniatura con el lado mayor a `max`, respetando proporción.
/// No amplía nunca: una imagen pequeña se queda como está.
pub fn make_thumb(img: &DynamicImage, max: u32) -> RgbImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return RgbImage::new(1, 1);
    }
    if w <= max && h <= max {
        return img.to_rgb8();
    }
    let scale = max as f32 / w.max(h) as f32;
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    img.thumbnail(nw, nh).to_rgb8()
}

pub fn encode_jpeg(img: &RgbImage, quality: u8) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(16 * 1024);
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    enc.encode(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(out)
}

// ---------------------------------------------------------------- Oklab

/// sRGB (0-255) a Oklab. Oklab es perceptualmente uniforme: la distancia
/// euclídea entre dos colores se parece a lo distintos que los ve una persona,
/// que es justo lo que hace falta para agrupar y para buscar por color.
pub fn srgb_to_oklab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let f = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (f(r), f(g), f(b));
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

pub fn oklab_to_srgb(lab: [f32; 3]) -> [u8; 3] {
    let l_ = lab[0] + 0.3963377774 * lab[1] + 0.2158037573 * lab[2];
    let m_ = lab[0] - 0.1055613458 * lab[1] - 0.0638541728 * lab[2];
    let s_ = lab[0] - 0.0894841775 * lab[1] - 1.2914855480 * lab[2];
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    let lin = [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ];
    let mut out = [0u8; 3];
    for i in 0..3 {
        let c = lin[i].clamp(0.0, 1.0);
        let c = if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        out[i] = (c * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Paleta dominante por k-medias en Oklab sobre una versión reducida.
/// Determinista: mismos píxeles, misma paleta, siempre.
pub fn palette(img: &DynamicImage, k: usize) -> Vec<PaletteEntry> {
    let small = img.thumbnail(48, 48).to_rgb8();
    let pixels: Vec<[f32; 3]> = small
        .pixels()
        .map(|p| srgb_to_oklab(p[0], p[1], p[2]))
        .collect();
    if pixels.is_empty() {
        return Vec::new();
    }
    let k = k.min(pixels.len()).max(1);

    // Inicialización determinista: muestreo uniforme del conjunto ordenado por
    // luminosidad, que reparte los centroides mejor que coger los k primeros.
    let mut sorted: Vec<usize> = (0..pixels.len()).collect();
    sorted.sort_by(|a, b| {
        pixels[*a][0]
            .partial_cmp(&pixels[*b][0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut centroids: Vec<[f32; 3]> = (0..k)
        .map(|i| pixels[sorted[(i * pixels.len()) / k]])
        .collect();

    let mut assign = vec![0usize; pixels.len()];
    for _ in 0..8 {
        let mut moved = false;
        for (i, px) in pixels.iter().enumerate() {
            let mut best = 0usize;
            let mut best_d = f32::MAX;
            for (c, cen) in centroids.iter().enumerate() {
                let d = sq_dist(px, cen);
                if d < best_d {
                    best_d = d;
                    best = c;
                }
            }
            if assign[i] != best {
                assign[i] = best;
                moved = true;
            }
        }
        let mut sums = vec![[0f32; 3]; k];
        let mut counts = vec![0usize; k];
        for (i, px) in pixels.iter().enumerate() {
            let c = assign[i];
            for d in 0..3 {
                sums[c][d] += px[d];
            }
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                for d in 0..3 {
                    centroids[c][d] = sums[c][d] / counts[c] as f32;
                }
            }
        }
        if !moved {
            break;
        }
    }

    let mut counts = vec![0usize; k];
    for a in &assign {
        counts[*a] += 1;
    }
    let total = pixels.len() as f32;
    let mut out: Vec<PaletteEntry> = (0..k)
        .filter(|c| counts[*c] > 0)
        .map(|c| PaletteEntry {
            rgb: oklab_to_srgb(centroids[c]),
            w: counts[c] as f32 / total,
        })
        .collect();
    out.sort_by(|a, b| b.w.partial_cmp(&a.w).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn sq_dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let (x, y, z) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    x * x + y * y + z * z
}

/// Cubo de color al que pertenece el color dominante: 12×12×12 sobre Oklab.
/// Sirve de preselección barata antes de calcular distancias reales.
pub fn color_bucket(p: &[PaletteEntry]) -> Option<i64> {
    let first = p.first()?;
    let lab = srgb_to_oklab(first.rgb[0], first.rgb[1], first.rgb[2]);
    let q = |v: f32, lo: f32, hi: f32| -> i64 {
        (((v - lo) / (hi - lo) * 12.0).floor() as i64).clamp(0, 11)
    };
    Some(q(lab[0], 0.0, 1.0) * 144 + q(lab[1], -0.4, 0.4) * 12 + q(lab[2], -0.4, 0.4))
}

/// dHash de 64 bits: compara cada píxel con su vecino de la derecha sobre una
/// versión 9×8 en gris. Barato, estable frente a recomprimir y reescalar.
pub fn dhash(img: &DynamicImage) -> u64 {
    let small = img.thumbnail_exact(9, 8).to_luma8();
    let mut bits = 0u64;
    let mut n = 0;
    for y in 0..8u32 {
        for x in 0..8u32 {
            let a = small.get_pixel(x, y)[0];
            let b = small.get_pixel(x + 1, y)[0];
            if a > b {
                bits |= 1 << n;
            }
            n += 1;
        }
    }
    bits
}

pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn img_lleno(w: u32, h: u32, c: [u8; 3]) -> DynamicImage {
        let mut im = RgbImage::new(w, h);
        for p in im.pixels_mut() {
            *p = Rgb(c);
        }
        DynamicImage::ImageRgb8(im)
    }

    #[test]
    fn la_miniatura_respeta_proporcion_y_no_amplia() {
        let big = img_lleno(1000, 500, [10, 20, 30]);
        let t = make_thumb(&big, 320);
        assert_eq!(t.width(), 320);
        assert_eq!(t.height(), 160);

        let small = img_lleno(100, 50, [10, 20, 30]);
        let t2 = make_thumb(&small, 320);
        assert_eq!((t2.width(), t2.height()), (100, 50));
    }

    #[test]
    fn oklab_ida_y_vuelta() {
        for c in [[255u8, 0, 0], [18, 32, 64], [255, 255, 255], [0, 0, 0]] {
            let back = oklab_to_srgb(srgb_to_oklab(c[0], c[1], c[2]));
            for i in 0..3 {
                assert!(
                    (back[i] as i32 - c[i] as i32).abs() <= 1,
                    "{:?} volvió como {:?}",
                    c,
                    back
                );
            }
        }
    }

    #[test]
    fn la_paleta_de_un_color_plano_es_ese_color() {
        let im = img_lleno(64, 64, [200, 40, 60]);
        let p = palette(&im, 5);
        assert!(!p.is_empty());
        assert!((p[0].w - 1.0).abs() < 0.001 || p[0].w > 0.9);
        let d: i32 = (0..3)
            .map(|i| (p[0].rgb[i] as i32 - [200, 40, 60][i]).abs())
            .sum();
        assert!(d < 12, "paleta {:?}", p[0].rgb);
    }

    #[test]
    fn el_jpeg_codificado_se_puede_decodificar() {
        let im = img_lleno(64, 32, [12, 200, 90]);
        let bytes = encode_jpeg(&im.to_rgb8(), 82).unwrap();
        let back = decode(&bytes).unwrap();
        assert_eq!(back.dimensions(), (64, 32));
    }

    #[test]
    fn el_hash_perceptual_aguanta_recomprimir() {
        let mut im = RgbImage::new(64, 64);
        for (x, y, p) in im.enumerate_pixels_mut() {
            *p = Rgb([(x * 4) as u8, (y * 4) as u8, 128]);
        }
        let original = DynamicImage::ImageRgb8(im);
        let h1 = dhash(&original);
        let jpeg = encode_jpeg(&original.to_rgb8(), 70).unwrap();
        let h2 = dhash(&decode(&jpeg).unwrap());
        assert!(hamming(h1, h2) <= 6, "distancia {}", hamming(h1, h2));
    }
}
