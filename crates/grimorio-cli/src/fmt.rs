//! Formato de salida. Regla: legible por una persona a simple vista y por
//! `grep` sin pelearse. Los colores se apagan solos si la salida no es un tty
//! o si existe NO_COLOR.

use std::io::IsTerminal;
use std::sync::OnceLock;

pub fn color_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal())
}

pub fn dim(s: &str) -> String {
    if color_on() {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn bold(s: &str) -> String {
    if color_on() {
        format!("\x1b[1m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn accent(s: &str) -> String {
    if color_on() {
        format!("\x1b[36m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn warn(s: &str) -> String {
    if color_on() {
        format!("\x1b[33m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// Cuadradito con el color dominante, para ver la paleta en la terminal.
pub fn swatch(rgb: [u8; 3]) -> String {
    if color_on() {
        format!("\x1b[48;2;{};{};{}m  \x1b[0m", rgb[0], rgb[1], rgb[2])
    } else {
        format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
    }
}

pub fn bytes(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else if v < 10.0 {
        format!("{v:.1} {}", U[i])
    } else {
        format!("{v:.0} {}", U[i])
    }
}

pub fn millis(ms: u128) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms < 60_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else {
        format!("{} min {} s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

pub fn stars(n: u8) -> String {
    let s: String = (0..5).map(|i| if i < n { '★' } else { '·' }).collect();
    if n > 0 {
        warn(&s)
    } else {
        dim(&s)
    }
}

/// Recorta a `max` caracteres visibles añadiendo elipsis. Cuenta caracteres,
/// no bytes: los nombres con acentos no se parten a la mitad.
pub fn ellipsis(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        format!("{s}{}", " ".repeat(max - n))
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Barra de progreso de una línea, reescribible con \r.
pub fn progress_bar(done: usize, total: usize, width: usize) -> String {
    let frac = if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    };
    let filled = (frac * width as f64).round() as usize;
    let bar: String = std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(width - filled))
        .collect();
    format!("{bar} {done}/{total}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tamanos_legibles() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(2048), "2.0 KB");
        assert_eq!(bytes(15 * 1024 * 1024), "15 MB");
    }

    #[test]
    fn elipsis_cuenta_caracteres_no_bytes() {
        assert_eq!(ellipsis("añañaña", 4), "aña…");
        assert_eq!(ellipsis("ab", 4), "ab  ");
    }

    #[test]
    fn la_barra_nunca_se_pasa_de_ancho() {
        for d in [0, 3, 7, 10] {
            let b = progress_bar(d, 10, 20);
            assert_eq!(b.chars().filter(|c| *c == '█' || *c == '░').count(), 20);
        }
    }
}
