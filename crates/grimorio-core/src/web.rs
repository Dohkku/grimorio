//! Páginas web guardadas: un solo `.html` con todo dentro.
//!
//! La extensión del navegador guarda la página como la estaba viendo quien la
//! guardó: el HTML ya pintado, sin scripts, con las hojas de estilo, las
//! imágenes y las tipografías metidas dentro como `data:`. Así se abre sin
//! conexión con cualquier navegador y sigue igual aunque la web cambie o
//! desaparezca. Es un archivo que se puede leer y copiar tal cual, como todo
//! lo demás de la biblioteca.
//!
//! Grimorio no lleva un navegador dentro (ver el README), así que no la pinta:
//! la miniatura es la captura de la pestaña que la extensión deja en una
//! etiqueta `<meta>` al principio del `<head>`:
//!
//! ```html
//! <meta name="grimorio:captura" content="data:image/jpeg;base64,…">
//! ```
//!
//! Un `.html` de otro sitio, sin esa etiqueta, entra igual, a ciegas.

/// Cuánto del principio del archivo se lee buscando la captura. La extensión
/// la pone la primera del `<head>`; una captura en JPEG ronda unos cientos de
/// kilobytes, y en base64 crece un tercio.
pub const PRINCIPIO: usize = 8 << 20;

const NOMBRE: &str = "grimorio:captura";

/// Los bytes de la imagen de la captura, si la página la lleva.
pub fn captura(html: &[u8]) -> Option<Vec<u8>> {
    let texto = String::from_utf8_lossy(&html[..html.len().min(PRINCIPIO)]);
    let mut desde = 0;
    while let Some(i) = texto[desde..].find(NOMBRE) {
        let pos = desde + i;
        desde = pos + NOMBRE.len();
        // La etiqueta entera en la que está el nombre.
        let Some(abre) = texto[..pos].rfind('<') else {
            continue;
        };
        let Some(cierra) = texto[pos..].find('>') else {
            break;
        };
        let etiqueta = &texto[abre..pos + cierra];
        let b = etiqueta.as_bytes();
        if b.len() < 6 || !b[..5].eq_ignore_ascii_case(b"<meta") || !b[5].is_ascii_whitespace() {
            continue;
        }
        if let Some(bytes) = contenido(etiqueta).and_then(|c| datos_de_url(&c)) {
            return Some(bytes);
        }
    }
    None
}

/// El valor de `content="…"` (o con comillas simples) de una etiqueta.
fn contenido(etiqueta: &str) -> Option<String> {
    let minus = etiqueta.to_ascii_lowercase();
    let i = minus.find("content")?;
    let resto = etiqueta[i + "content".len()..].trim_start();
    let resto = resto.strip_prefix('=')?.trim_start();
    let comilla = resto.chars().next()?;
    if comilla != '"' && comilla != '\'' {
        return None;
    }
    let cuerpo = &resto[1..];
    let fin = cuerpo.find(comilla)?;
    Some(cuerpo[..fin].to_string())
}

/// Los bytes de una URL `data:image/…;base64,…`.
fn datos_de_url(url: &str) -> Option<Vec<u8>> {
    let resto = url.trim().strip_prefix("data:")?;
    let (cabecera, datos) = resto.split_once(',')?;
    let cabecera = cabecera.to_ascii_lowercase();
    if !cabecera.starts_with("image/") || !cabecera.ends_with(";base64") {
        return None;
    }
    base64(datos)
}

/// Base64 estándar, sin saltarse nada que no sea espacio o relleno.
fn base64(s: &str) -> Option<Vec<u8>> {
    fn valor(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    }
    let mut salida = Vec::with_capacity(s.len() / 4 * 3);
    let mut acumulado = 0u32;
    let mut bits = 0;
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        acumulado = (acumulado << 6) | valor(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            salida.push((acumulado >> bits) as u8);
            acumulado &= (1 << bits) - 1;
        }
    }
    Some(salida)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_basico() {
        assert_eq!(base64("aG9sYQ==").unwrap(), b"hola");
        assert_eq!(base64("aG9sYSE").unwrap(), b"hola!");
        assert_eq!(base64("aG9s\nYQ==").unwrap(), b"hola");
        assert!(base64("a*b").is_none());
    }

    #[test]
    fn encuentra_la_captura() {
        let html = br#"<!DOCTYPE html><html><head><meta charset="utf-8">
<meta name="grimorio:captura" content="data:image/png;base64,aG9sYQ==">
<title>x</title></head><body></body></html>"#;
        assert_eq!(captura(html).unwrap(), b"hola");
    }

    #[test]
    fn comillas_simples_y_orden_cambiado() {
        let html = b"<head><META content='data:image/jpeg;base64,aG9sYQ==' name='grimorio:captura'></head>";
        assert_eq!(captura(html).unwrap(), b"hola");
    }

    #[test]
    fn sin_captura_o_que_no_es_imagen() {
        assert!(captura(b"<html><head><title>grimorio:captura</title></head></html>").is_none());
        assert!(captura(
            br#"<meta name="grimorio:captura" content="data:text/html;base64,aG9sYQ==">"#
        )
        .is_none());
        assert!(captura(b"").is_none());
    }
}
