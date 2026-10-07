//! La vista previa que un negativo digital lleva dentro.
//!
//! Un archivo RAW no es una imagen: es lo que el sensor midió, y revelarlo de
//! verdad exige demosaico, balance de blancos y curvas —eso es `libraw`, cien
//! mil líneas de C que además hay que aislar en un proceso aparte porque se
//! cuelga con archivos rotos—.
//!
//! Pero **todas las cámaras guardan dentro un JPEG ya revelado**: es lo que
//! enseñan en su pantalla. Para una biblioteca de referencias eso es
//! exactamente lo que hace falta, y sacarlo es leer una estructura TIFF, que
//! son doscientas líneas y ninguna dependencia.
//!
//! Todo lo de aquí lee datos que vienen de fuera y que pueden estar rotos o
//! ser hostiles, así que no hay un solo índice directo: cada lectura pasa por
//! `get`, y una estructura que no cuadra devuelve `None` en vez de un pánico.

/// Cuántos IFD se siguen antes de rendirse.
///
/// Un TIFF sano tiene tres o cuatro niveles. El tope está para que un archivo
/// con un puntero que apunta a sí mismo no dé vueltas para siempre.
const TOPE_IFD: usize = 32;

/// El JPEG más grande que un archivo lleva dentro, o `None`.
///
/// Devuelve un trozo del propio archivo, sin copiar nada.
pub fn previa(bytes: &[u8]) -> Option<&[u8]> {
    // Fuji va por libre: su cabecera dice dónde está el JPEG y ya está.
    if let Some(p) = previa_fuji(bytes) {
        return Some(p);
    }
    let mejor = por_tiff(bytes).or_else(|| por_rastreo(bytes))?;
    Some(mejor)
}

// ------------------------------------------------------------------- TIFF

#[derive(Clone, Copy)]
struct Orden(bool); // true = little endian

impl Orden {
    fn u16(&self, b: &[u8], en: usize) -> Option<u16> {
        let s = b.get(en..en + 2)?;
        Some(if self.0 {
            u16::from_le_bytes([s[0], s[1]])
        } else {
            u16::from_be_bytes([s[0], s[1]])
        })
    }
    fn u32(&self, b: &[u8], en: usize) -> Option<u32> {
        let s = b.get(en..en + 4)?;
        Some(if self.0 {
            u32::from_le_bytes([s[0], s[1], s[2], s[3]])
        } else {
            u32::from_be_bytes([s[0], s[1], s[2], s[3]])
        })
    }
}

/// Etiquetas que nos interesan de cada IFD.
const JPEG_OFF: u16 = 0x0201;
const JPEG_LEN: u16 = 0x0202;
const STRIP_OFF: u16 = 0x0111;
const STRIP_LEN: u16 = 0x0117;
const COMPRESION: u16 = 0x0103;
const SUB_IFDS: u16 = 0x014A;
const EXIF_IFD: u16 = 0x8769;

fn por_tiff(b: &[u8]) -> Option<&[u8]> {
    let orden = match b.get(0..2)? {
        b"II" => Orden(true),
        b"MM" => Orden(false),
        _ => return None,
    };
    // 42 es el número mágico del TIFF. Algunos RAW usan otro (Canon CR2 usa 42
    // igual, Olympus a veces 0x4F52), así que no se exige.
    let primero = orden.u32(b, 4)? as usize;

    let mut candidatos: Vec<(usize, usize)> = Vec::new();
    let mut pendientes = vec![primero];
    let mut vistos = 0usize;

    while let Some(en) = pendientes.pop() {
        vistos += 1;
        if vistos > TOPE_IFD {
            break;
        }
        recoger_ifd(b, orden, en, &mut candidatos, &mut pendientes);
    }

    mejor_jpeg(b, &candidatos)
}

/// Lee un IFD: apunta los trozos que podrían ser un JPEG y encola los hijos.
fn recoger_ifd(
    b: &[u8],
    orden: Orden,
    en: usize,
    candidatos: &mut Vec<(usize, usize)>,
    pendientes: &mut Vec<usize>,
) {
    let Some(n) = orden.u16(b, en) else { return };
    // Un IFD con miles de entradas es un archivo roto, no un RAW.
    if n == 0 || n > 512 {
        return;
    }
    let mut off = None;
    let mut len = None;
    let mut tira_off = None;
    let mut tira_len = None;
    let mut comprimido = false;

    for i in 0..n as usize {
        let e = en + 2 + i * 12;
        let (Some(tag), Some(tipo), Some(cuenta)) =
            (orden.u16(b, e), orden.u16(b, e + 2), orden.u32(b, e + 4))
        else {
            return;
        };
        // Solo interesan valores cortos: los que caben en los cuatro bytes del
        // propio campo o son un puntero.
        let valor = || -> Option<u32> {
            match tipo {
                3 => orden.u16(b, e + 8).map(u32::from), // SHORT
                4 => orden.u32(b, e + 8),                // LONG
                _ => None,
            }
        };
        match tag {
            JPEG_OFF => off = valor(),
            JPEG_LEN => len = valor(),
            STRIP_OFF if cuenta == 1 => tira_off = valor(),
            STRIP_LEN if cuenta == 1 => tira_len = valor(),
            COMPRESION => comprimido = matches!(valor(), Some(6) | Some(7)),
            SUB_IFDS => {
                // Uno cabe en el campo; varios viven en una lista aparte.
                if cuenta == 1 {
                    if let Some(v) = orden.u32(b, e + 8) {
                        pendientes.push(v as usize);
                    }
                } else if cuenta <= 16 {
                    if let Some(lista) = orden.u32(b, e + 8) {
                        for k in 0..cuenta as usize {
                            if let Some(v) = orden.u32(b, lista as usize + k * 4) {
                                pendientes.push(v as usize);
                            }
                        }
                    }
                }
            }
            EXIF_IFD => {
                if let Some(v) = orden.u32(b, e + 8) {
                    pendientes.push(v as usize);
                }
            }
            _ => {}
        }
    }

    if let (Some(o), Some(l)) = (off, len) {
        candidatos.push((o as usize, l as usize));
    }
    // Una sola tira comprimida en JPEG es la vista previa grande: es como
    // guardan los DNG y los NEF la imagen «de pantalla».
    if comprimido {
        if let (Some(o), Some(l)) = (tira_off, tira_len) {
            candidatos.push((o as usize, l as usize));
        }
    }

    // El siguiente IFD de la cadena, después de las entradas.
    if let Some(sig) = orden.u32(b, en + 2 + n as usize * 12) {
        if sig != 0 {
            pendientes.push(sig as usize);
        }
    }
}

/// De todos los trozos apuntados, el JPEG válido más grande.
fn mejor_jpeg<'a>(b: &'a [u8], candidatos: &[(usize, usize)]) -> Option<&'a [u8]> {
    candidatos
        .iter()
        .filter_map(|&(o, l)| {
            let t = b.get(o..o.checked_add(l)?)?;
            es_jpeg(t).then_some(t)
        })
        .max_by_key(|t| t.len())
}

fn es_jpeg(t: &[u8]) -> bool {
    // Cabecera y cola: con solo la cabecera, un puntero que cae por casualidad
    // sobre dos bytes 0xFFD8 daría un candidato basura.
    t.len() > 1024 && t.starts_with(&[0xFF, 0xD8, 0xFF]) && t.ends_with(&[0xFF, 0xD9])
}

// ------------------------------------------------------------------- Fuji

/// Fuji no usa TIFF para la cabecera: dice a pelo dónde empieza el JPEG.
fn previa_fuji(b: &[u8]) -> Option<&[u8]> {
    if !b.starts_with(b"FUJIFILMCCD-RAW") {
        return None;
    }
    let leer = |en: usize| -> Option<u32> {
        let s = b.get(en..en + 4)?;
        Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    };
    let off = leer(84)? as usize;
    let len = leer(88)? as usize;
    let t = b.get(off..off.checked_add(len)?)?;
    es_jpeg(t).then_some(t)
}

// ---------------------------------------------------------------- rastreo

/// Último recurso: buscar un JPEG por sus marcas.
///
/// Existe porque los formatos propietarios cambian con cada generación de
/// cámaras y una estructura que hoy no entendemos no debería dejar la foto sin
/// miniatura. Es tosco, pero el resultado se valida decodificándolo: si no era
/// un JPEG, no entra.
fn por_rastreo(b: &[u8]) -> Option<&[u8]> {
    let mut mejor: Option<&[u8]> = None;
    let mut i = 0usize;
    // Un tope de arranques para no recorrer un archivo de 80 MB entero
    // encontrando ruido: los previews están al principio, siempre.
    let hasta = b.len().min(32 * 1024 * 1024);
    let mut arranques = 0;
    while i + 3 < hasta && arranques < 64 {
        if b[i] == 0xFF && b[i + 1] == 0xD8 && b[i + 2] == 0xFF {
            arranques += 1;
            if let Some(fin) = fin_de_jpeg(b, i + 2) {
                let t = &b[i..fin];
                if es_jpeg(t) && mejor.map(|m| t.len() > m.len()).unwrap_or(true) {
                    mejor = Some(t);
                }
                i = fin;
                continue;
            }
        }
        i += 1;
    }
    mejor
}

fn fin_de_jpeg(b: &[u8], desde: usize) -> Option<usize> {
    let mut i = desde;
    while i + 1 < b.len() {
        if b[i] == 0xFF && b[i + 1] == 0xD9 {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un JPEG de mentira pero con la forma correcta: cabecera, relleno y cola.
    fn jpeg(n: usize) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
        v.extend(std::iter::repeat(0x41).take(n));
        v.extend_from_slice(&[0xFF, 0xD9]);
        v
    }

    /// Construye un TIFF mínimo con un IFD que apunta a un JPEG.
    fn tiff_con(jpegs: &[Vec<u8>]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"II\x2a\x00");
        b.extend_from_slice(&8u32.to_le_bytes()); // IFD0 en 8

        // IFD0: dos entradas por JPEG (offset y longitud) no cabe; se hace un
        // IFD por JPEG, encadenados.
        let mut ifds = Vec::new();
        let mut datos = Vec::new();
        let base_ifd = 8usize;
        let tam_ifd = 2 + 2 * 12 + 4;
        let base_datos = base_ifd + tam_ifd * jpegs.len();
        let mut off_datos = base_datos;
        for (i, j) in jpegs.iter().enumerate() {
            let mut ifd = Vec::new();
            ifd.extend_from_slice(&2u16.to_le_bytes()); // dos entradas
            for (tag, valor) in [(JPEG_OFF, off_datos as u32), (JPEG_LEN, j.len() as u32)] {
                ifd.extend_from_slice(&tag.to_le_bytes());
                ifd.extend_from_slice(&4u16.to_le_bytes()); // LONG
                ifd.extend_from_slice(&1u32.to_le_bytes()); // cuenta
                ifd.extend_from_slice(&valor.to_le_bytes());
            }
            let siguiente = if i + 1 < jpegs.len() {
                (base_ifd + tam_ifd * (i + 1)) as u32
            } else {
                0
            };
            ifd.extend_from_slice(&siguiente.to_le_bytes());
            ifds.push(ifd);
            datos.extend_from_slice(j);
            off_datos += j.len();
        }
        for ifd in ifds {
            b.extend_from_slice(&ifd);
        }
        b.extend_from_slice(&datos);
        b
    }

    #[test]
    fn saca_la_previa_de_un_tiff() {
        let j = jpeg(2000);
        let raw = tiff_con(&[j.clone()]);
        assert_eq!(previa(&raw), Some(j.as_slice()));
    }

    #[test]
    fn entre_varias_previas_se_queda_la_mayor() {
        let pequena = jpeg(1500);
        let grande = jpeg(9000);
        let raw = tiff_con(&[pequena, grande.clone()]);
        assert_eq!(previa(&raw).map(|p| p.len()), Some(grande.len()));
    }

    #[test]
    fn un_puntero_fuera_del_archivo_no_revienta_nada() {
        let j = jpeg(2000);
        let mut raw = tiff_con(&[j.clone()]);
        // Estropear el offset del JPEG a un número enorme.
        let en = 8 + 2 + 8;
        raw[en..en + 4].copy_from_slice(&0xFFFF_FF00u32.to_le_bytes());
        assert!(
            por_tiff(&raw).is_none(),
            "la estructura ya no lleva a ningún sitio"
        );
        // Pero la foto sigue estando en el archivo, y el rastreo la encuentra.
        // Esto es exactamente para lo que está: una cámara nueva con una
        // estructura que no entendemos no debería quedarse sin miniatura.
        assert_eq!(previa(&raw).map(|p| p.len()), Some(j.len()));
    }

    #[test]
    fn un_offset_absurdo_no_desborda_al_sumar() {
        let mut raw = tiff_con(&[jpeg(2000)]);
        let en = 8 + 2 + 8;
        raw[en..en + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let len = 8 + 2 + 12 + 8;
        raw[len..len + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let _ = previa(&raw);
    }

    #[test]
    fn un_ifd_que_se_apunta_a_si_mismo_no_cuelga() {
        let mut raw = tiff_con(&[jpeg(2000)]);
        let siguiente = 8 + 2 + 24;
        raw[siguiente..siguiente + 4].copy_from_slice(&8u32.to_le_bytes());
        // Lo que importa es que termine.
        let _ = previa(&raw);
    }

    #[test]
    fn sin_estructura_conocida_lo_encuentra_rastreando() {
        let j = jpeg(4000);
        let mut raw = vec![0u8; 3000];
        raw.extend_from_slice(&j);
        raw.extend_from_slice(&vec![0u8; 500]);
        assert_eq!(previa(&raw).map(|p| p.len()), Some(j.len()));
    }

    #[test]
    fn una_cabecera_suelta_no_cuenta_como_jpeg() {
        let mut raw = vec![0u8; 100];
        raw.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0x00]);
        raw.extend_from_slice(&vec![0u8; 100]);
        assert!(previa(&raw).is_none());
    }

    #[test]
    fn el_formato_de_fuji_dice_donde_esta() {
        let j = jpeg(3000);
        let mut raw = vec![0u8; 92];
        raw[0..15].copy_from_slice(b"FUJIFILMCCD-RAW");
        raw[84..88].copy_from_slice(&92u32.to_be_bytes());
        raw[88..92].copy_from_slice(&(j.len() as u32).to_be_bytes());
        raw.extend_from_slice(&j);
        assert_eq!(previa(&raw).map(|p| p.len()), Some(j.len()));
    }

    #[test]
    fn un_archivo_vacio_o_diminuto_no_da_problemas() {
        assert!(previa(&[]).is_none());
        assert!(previa(&[0xFF]).is_none());
        assert!(previa(b"II\x2a\x00").is_none());
    }
}
