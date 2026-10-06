//! Nombres por patrón, para renombrar muchos de una vez.
//!
//! `{nombre}` el nombre que tenía · `{n}` un contador · `{n:3}` el contador con
//! ceros delante hasta tres cifras · `{fecha}` el día que se importó.
//!
//! Lo que no es una de esas marcas se deja tal cual, llaves incluidas: un
//! patrón con un error de dedo da un nombre raro que se ve en la vista previa,
//! no un fallo.

/// El nombre que sale de aplicar `patron` a un elemento.
///
/// Si el resultado se queda vacío —un patrón de solo espacios—, se devuelve el
/// nombre que tenía: un elemento sin nombre no se encuentra.
pub fn aplicar(patron: &str, nombre: &str, n: u64, importado_ms: u64) -> String {
    let mut out = String::new();
    let mut resto = patron;
    while let Some(i) = resto.find('{') {
        out.push_str(&resto[..i]);
        let tras = &resto[i..];
        let Some(j) = tras.find('}') else {
            out.push_str(tras);
            resto = "";
            break;
        };
        let marca = &tras[1..j];
        match marca {
            "nombre" => out.push_str(nombre),
            "n" => out.push_str(&n.to_string()),
            "fecha" => {
                let desfase = crate::time::desfase_local_s(importado_ms);
                let (y, m, d) = crate::time::fecha_local(importado_ms, desfase);
                out.push_str(&format!("{y:04}-{m:02}-{d:02}"));
            }
            m if m.starts_with("n:") => match m[2..].parse::<usize>() {
                Ok(ancho) if ancho <= 12 => out.push_str(&format!("{n:0ancho$}")),
                _ => out.push_str(&tras[..=j]),
            },
            _ => out.push_str(&tras[..=j]),
        }
        resto = &tras[j + 1..];
    }
    out.push_str(resto);
    let out = out.trim().replace('/', "-");
    if out.is_empty() {
        nombre.to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_marcas_se_sustituyen() {
        assert_eq!(aplicar("{nombre}", "gato", 1, 0), "gato");
        assert_eq!(aplicar("moodboard {n}", "x", 7, 0), "moodboard 7");
        assert_eq!(aplicar("ref-{n:3}-{nombre}", "gato", 7, 0), "ref-007-gato");
        assert!(aplicar("{fecha}", "x", 1, 1_791_000_000_000).starts_with("2026-"));
    }

    #[test]
    fn lo_que_no_se_entiende_se_queda_como_esta() {
        assert_eq!(aplicar("{nombr} {n", "gato", 2, 0), "{nombr} {n");
        assert_eq!(aplicar("{n:x}", "gato", 2, 0), "{n:x}");
    }

    #[test]
    fn un_patron_vacio_no_deja_sin_nombre() {
        assert_eq!(aplicar("   ", "gato", 1, 0), "gato");
        assert_eq!(aplicar("a/b {n}", "gato", 1, 0), "a-b 1");
    }
}
