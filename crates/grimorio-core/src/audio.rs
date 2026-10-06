//! La carátula que un archivo de audio lleva dentro.
//!
//! Mismo espíritu que [`crate::raw`]: la imagen ya está en el archivo, y
//! sacarla es leer una estructura documentada. Un MP3 la guarda en un marco
//! `APIC` de ID3v2 y un FLAC en un bloque `PICTURE`; las dos son cabeceras de
//! longitudes explícitas, así que no hace falta decodificar ni un solo byte de
//! sonido.
//!
//! Como en el RAW, todo lo de aquí lee datos que pueden estar rotos: cada
//! lectura pasa por `get` y una estructura que no cuadra devuelve `None`.

/// La carátula más grande que el archivo lleve dentro, o `None`.
pub fn caratula(bytes: &[u8]) -> Option<&[u8]> {
    de_id3(bytes).or_else(|| de_flac(bytes))
}

fn es_imagen(t: &[u8]) -> bool {
    // JPEG o PNG, que es lo que las etiquetas admiten en la práctica.
    t.len() > 256
        && (t.starts_with(&[0xFF, 0xD8, 0xFF]) || t.starts_with(&[0x89, b'P', b'N', b'G']))
}

// ------------------------------------------------------------------- ID3v2

/// Los tamaños de ID3v2 son «sincroseguros»: siete bits por byte, para que
/// nunca aparezca un 0xFF seguido de bits altos y un reproductor viejo no
/// confunda la etiqueta con el principio del sonido.
fn sincroseguro(b: &[u8]) -> Option<usize> {
    let s = b.get(0..4)?;
    if s.iter().any(|x| *x & 0x80 != 0) {
        return None;
    }
    Some(((s[0] as usize) << 21) | ((s[1] as usize) << 14) | ((s[2] as usize) << 7) | s[3] as usize)
}

fn u32be(b: &[u8]) -> Option<usize> {
    let s = b.get(0..4)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

fn de_id3(b: &[u8]) -> Option<&[u8]> {
    if !b.starts_with(b"ID3") {
        return None;
    }
    let version = *b.get(3)?;
    let tam = sincroseguro(b.get(6..10)?)?;
    let fin = 10usize.checked_add(tam)?.min(b.len());

    let mut en = 10usize;
    let mut mejor: Option<&[u8]> = None;
    // Un ID3v2.2 usa marcos de tres letras y tamaños de tres bytes; del 2.3 en
    // adelante son cuatro y cuatro. Es la única diferencia que importa aquí.
    let (nombre, cabecera) = if version <= 2 {
        (3usize, 6usize)
    } else {
        (4, 10)
    };

    while en + cabecera <= fin {
        let etiqueta = b.get(en..en + nombre)?;
        if etiqueta[0] == 0 {
            break; // relleno hasta el final de la etiqueta
        }
        let tam_marco = if version <= 2 {
            let s = b.get(en + 3..en + 6)?;
            ((s[0] as usize) << 16) | ((s[1] as usize) << 8) | s[2] as usize
        } else if version == 4 {
            sincroseguro(b.get(en + 4..en + 8)?)?
        } else {
            u32be(b.get(en + 4..en + 8)?)?
        };
        let cuerpo = b.get(en + cabecera..(en + cabecera).checked_add(tam_marco)?)?;

        if etiqueta == b"APIC" || etiqueta == b"PIC" {
            if let Some(img) = imagen_de_apic(cuerpo, etiqueta == b"PIC") {
                if mejor.map(|m| img.len() > m.len()).unwrap_or(true) {
                    mejor = Some(img);
                }
            }
        }
        en = en.checked_add(cabecera)?.checked_add(tam_marco)?;
    }
    mejor
}

/// El cuerpo de un `APIC`: codificación, tipo MIME, qué clase de imagen es,
/// una descripción, y por fin los bytes.
fn imagen_de_apic(c: &[u8], corto: bool) -> Option<&[u8]> {
    let codificacion = *c.first()?;
    let mut i = 1usize;
    if corto {
        // En ID3v2.2 el tipo son tres letras fijas («JPG», «PNG») en vez de un
        // MIME terminado en cero.
        i += 3;
    } else {
        i += hasta_cero(c.get(i..)?)? + 1;
    }
    i += 1; // el byte de «qué clase de imagen es»
            // La descripción termina en uno o dos ceros según la codificación: las
            // UTF-16 (1 y 2) usan dos bytes por carácter.
    let doble = matches!(codificacion, 1 | 2);
    i += fin_de_texto(c.get(i..)?, doble)?;
    let img = c.get(i..)?;
    es_imagen(img).then_some(img)
}

fn hasta_cero(s: &[u8]) -> Option<usize> {
    s.iter().position(|b| *b == 0)
}

fn fin_de_texto(s: &[u8], doble: bool) -> Option<usize> {
    if !doble {
        return Some(hasta_cero(s)? + 1);
    }
    let mut i = 0;
    while i + 1 < s.len() {
        if s[i] == 0 && s[i + 1] == 0 {
            return Some(i + 2);
        }
        i += 2;
    }
    None
}

// -------------------------------------------------------------------- FLAC

fn de_flac(b: &[u8]) -> Option<&[u8]> {
    if !b.starts_with(b"fLaC") {
        return None;
    }
    let mut en = 4usize;
    let mut mejor: Option<&[u8]> = None;
    // Cada bloque: un byte con «es el último» y el tipo, y tres de longitud.
    loop {
        let cabecera = b.get(en..en + 4)?;
        let ultimo = cabecera[0] & 0x80 != 0;
        let tipo = cabecera[0] & 0x7F;
        let largo =
            ((cabecera[1] as usize) << 16) | ((cabecera[2] as usize) << 8) | cabecera[3] as usize;
        let cuerpo = b.get(en + 4..(en + 4).checked_add(largo)?)?;
        if tipo == 6 {
            if let Some(img) = imagen_de_picture(cuerpo) {
                if mejor.map(|m| img.len() > m.len()).unwrap_or(true) {
                    mejor = Some(img);
                }
            }
        }
        if ultimo {
            return mejor;
        }
        en = en.checked_add(4)?.checked_add(largo)?;
    }
}

/// El bloque `PICTURE`: todo con longitudes explícitas de cuatro bytes.
fn imagen_de_picture(c: &[u8]) -> Option<&[u8]> {
    let mut i = 4usize; // qué clase de imagen es
    let mime = u32be(c.get(i..)?)?;
    i = i.checked_add(4)?.checked_add(mime)?;
    let desc = u32be(c.get(i..)?)?;
    i = i.checked_add(4)?.checked_add(desc)?;
    // Ancho, alto, profundidad y número de colores: cuatro campos de cuatro.
    i = i.checked_add(16)?;
    let datos = u32be(c.get(i..)?)?;
    i = i.checked_add(4)?;
    let img = c.get(i..i.checked_add(datos)?)?;
    es_imagen(img).then_some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(n: usize) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
        v.extend(std::iter::repeat(0x41).take(n));
        v.extend_from_slice(&[0xFF, 0xD9]);
        v
    }

    fn sincro(n: usize) -> [u8; 4] {
        [
            ((n >> 21) & 0x7F) as u8,
            ((n >> 14) & 0x7F) as u8,
            ((n >> 7) & 0x7F) as u8,
            (n & 0x7F) as u8,
        ]
    }

    fn mp3_con(img: &[u8], version: u8) -> Vec<u8> {
        let mut cuerpo = Vec::new();
        cuerpo.push(0u8); // codificación latin1
        cuerpo.extend_from_slice(b"image/jpeg\0");
        cuerpo.push(3); // portada
        cuerpo.extend_from_slice(b"tapa\0");
        cuerpo.extend_from_slice(img);

        let mut marco = Vec::from(&b"APIC"[..]);
        if version == 4 {
            marco.extend_from_slice(&sincro(cuerpo.len()));
        } else {
            marco.extend_from_slice(&(cuerpo.len() as u32).to_be_bytes());
        }
        marco.extend_from_slice(&[0, 0]); // banderas
        marco.extend_from_slice(&cuerpo);

        let mut b = Vec::from(&b"ID3"[..]);
        b.push(version);
        b.push(0);
        b.push(0);
        b.extend_from_slice(&sincro(marco.len()));
        b.extend_from_slice(&marco);
        b.extend_from_slice(b"aqui empezaria el sonido");
        b
    }

    fn flac_con(img: &[u8]) -> Vec<u8> {
        let mut cuerpo = Vec::new();
        cuerpo.extend_from_slice(&3u32.to_be_bytes());
        cuerpo.extend_from_slice(&(10u32).to_be_bytes());
        cuerpo.extend_from_slice(b"image/jpeg");
        cuerpo.extend_from_slice(&0u32.to_be_bytes()); // sin descripción
        for _ in 0..4 {
            cuerpo.extend_from_slice(&0u32.to_be_bytes());
        }
        cuerpo.extend_from_slice(&(img.len() as u32).to_be_bytes());
        cuerpo.extend_from_slice(img);

        let mut b = Vec::from(&b"fLaC"[..]);
        b.push(0x80 | 6); // último bloque, tipo PICTURE
        b.extend_from_slice(&[
            ((cuerpo.len() >> 16) & 0xFF) as u8,
            ((cuerpo.len() >> 8) & 0xFF) as u8,
            (cuerpo.len() & 0xFF) as u8,
        ]);
        b.extend_from_slice(&cuerpo);
        b
    }

    #[test]
    fn saca_la_caratula_de_un_mp3() {
        let j = jpeg(2000);
        for version in [3u8, 4u8] {
            let mp3 = mp3_con(&j, version);
            assert_eq!(
                caratula(&mp3).map(|c| c.len()),
                Some(j.len()),
                "ID3v2.{version}"
            );
        }
    }

    #[test]
    fn saca_la_caratula_de_un_flac() {
        let j = jpeg(3000);
        assert_eq!(caratula(&flac_con(&j)).map(|c| c.len()), Some(j.len()));
    }

    #[test]
    fn un_mp3_sin_caratula_no_inventa_ninguna() {
        let mut b = Vec::from(&b"ID3\x03\x00\x00"[..]);
        b.extend_from_slice(&sincro(0));
        b.extend_from_slice(b"solo sonido");
        assert!(caratula(&b).is_none());
    }

    #[test]
    fn una_longitud_mentirosa_no_revienta_nada() {
        let j = jpeg(2000);
        let mut mp3 = mp3_con(&j, 3);
        // Decir que el marco mide muchísimo más de lo que hay.
        mp3[14..18].copy_from_slice(&0x7FFF_FFFFu32.to_be_bytes());
        assert!(caratula(&mp3).is_none());
    }

    #[test]
    fn un_archivo_que_no_es_ni_una_cosa_ni_la_otra() {
        assert!(caratula(&[]).is_none());
        assert!(caratula(b"esto es otra cosa").is_none());
        assert!(caratula(b"fLaC").is_none());
    }

    #[test]
    fn una_caratula_diminuta_no_cuenta() {
        // Cuatro bytes que empiezan por la marca de JPEG no son una portada.
        let mp3 = mp3_con(&jpeg(4), 3);
        assert!(caratula(&mp3).is_none());
    }
}
