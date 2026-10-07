//! Tokens de tema.
//!
//! Aquí se prueba la regla del proyecto: en la capa visual no hay ni un color ni
//! un número mágico escrito a mano. Todo sale de un `theme.json` que se puede
//! cambiar sin recompilar. Los valores por defecto son la identidad de Grimorio.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Tema {
    /// Fondo de la malla, en sRGB "#rrggbb".
    pub fondo: String,
    /// Color del anillo de selección.
    pub seleccion: String,
    /// Color de la celda que aún no tiene ni miniatura ni color dominante.
    pub marcador: String,
    /// Radio de las esquinas de la celda, en píxeles.
    pub radio: f32,
    /// Margen alrededor de la malla, en píxeles.
    pub margen: f32,
    /// Separación entre celdas, en píxeles.
    pub hueco: f32,
    /// Lado objetivo de la celda al arrancar.
    pub celda: f32,
    /// Cuánta luz gana la celda bajo el puntero (0 a 1).
    pub realce: f32,
    /// Cuánto crece la celda bajo el puntero, en píxeles.
    pub realce_px: f32,
    /// Cuánto tarda una miniatura en aparecer sobre su color dominante.
    pub aparicion_s: f32,
    /// Constante de tiempo del suavizado del scroll, en segundos.
    pub scroll_tau: f32,
    /// Tope de subidas a GPU por fotograma, para no comerse el presupuesto.
    pub subidas_por_fotograma: usize,
}

impl Default for Tema {
    fn default() -> Self {
        Tema {
            fondo: "#12100f".into(),
            seleccion: "#63b6bb".into(),
            marcador: "#282c30".into(),
            radio: 6.0,
            margen: 20.0,
            hueco: 10.0,
            celda: 200.0,
            realce: 0.06,
            realce_px: 2.0,
            aparicion_s: 0.12,
            scroll_tau: 0.055,
            subidas_por_fotograma: 24,
        }
    }
}

impl Tema {
    pub fn desde_archivo(p: &Path) -> Result<Tema, String> {
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    /// Un color del tema, ya en espacio lineal para el shader.
    pub fn color(&self, hex: &str) -> [f32; 3] {
        hex_a_lineal(hex).unwrap_or([0.5, 0.5, 0.5])
    }

    pub fn fondo_wgpu(&self) -> wgpu::Color {
        let [r, g, b] = hex_a_lineal(&self.fondo).unwrap_or([0.07, 0.06, 0.06]);
        wgpu::Color {
            r: r as f64,
            g: g as f64,
            b: b as f64,
            a: 1.0,
        }
    }
}

/// El color de limpieza de wgpu está en espacio lineal, no en sRGB.
fn hex_a_lineal(s: &str) -> Option<[f32; 3]> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let c = |i: usize| -> Option<f32> {
        let v = u8::from_str_radix(&s[i..i + 2], 16).ok()? as f32 / 255.0;
        Some(if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        })
    };
    Some([c(0)?, c(2)?, c(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_negro_y_el_blanco_salen_donde_deben() {
        let n = hex_a_lineal("#000000").unwrap();
        let b = hex_a_lineal("#ffffff").unwrap();
        assert!(n.iter().all(|v| *v < 0.001));
        assert!(b.iter().all(|v| (*v - 1.0).abs() < 0.001));
    }

    #[test]
    fn hex_invalido_no_revienta() {
        assert!(hex_a_lineal("nope").is_none());
        assert!(hex_a_lineal("#12345").is_none());
    }
}
