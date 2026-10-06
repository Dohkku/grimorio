//! El lenguaje del buscador: `tipografía etiqueta:rótulo estrellas:>=4 ext:png`.
//!
//! Existe en vez de un panel con quince desplegables por una razón concreta: un
//! panel se recorre con el ratón y hay que volver a recorrerlo entero para
//! cambiar una cosa; una línea de texto se edita, se copia, se pega y —el día
//! que existan las carpetas inteligentes— **es** la carpeta inteligente. La
//! barra de filtros de la ventana escribe aquí dentro; no hay dos verdades.
//!
//! Regla de diseño: **lo que no se entiende es texto**. Si alguien pega una URL
//! o escribe `nota:` esperando algo que no existe, el buscador busca esas
//! palabras en vez de tragárselas o dar un error. Un buscador que castiga por
//! escribir es un buscador que no se usa.

use crate::medio::Familia;
use crate::query::{Orientation, Query, SortBy};

/// Trocea respetando las comillas: `etiqueta:"arte generativo"` es un trozo.
fn trocear(entrada: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut actual = String::new();
    let mut comillas = false;
    for c in entrada.chars() {
        match c {
            '"' => comillas = !comillas,
            c if c.is_whitespace() && !comillas => {
                if !actual.is_empty() {
                    out.push(std::mem::take(&mut actual));
                }
            }
            c => actual.push(c),
        }
    }
    if !actual.is_empty() {
        out.push(actual);
    }
    out
}

/// Un valor numérico con su comparador: `>=4`, `<500k`, `1920`.
enum Rango {
    Min(u64),
    Max(u64),
    Exacto(u64),
}

fn rango(v: &str) -> Option<Rango> {
    let (op, resto) = if let Some(r) = v.strip_prefix(">=") {
        (">=", r)
    } else if let Some(r) = v.strip_prefix("<=") {
        ("<=", r)
    } else if let Some(r) = v.strip_prefix('>') {
        (">", r)
    } else if let Some(r) = v.strip_prefix('<') {
        ("<", r)
    } else {
        ("=", v)
    };
    let n = numero(resto)?;
    Some(match op {
        // `>4` y `>=5` son lo mismo en enteros, y escribir el caso aparte
        // evitaba tener que llevar dos formas de decir lo mismo hasta el SQL.
        ">" => Rango::Min(n.saturating_add(1)),
        ">=" => Rango::Min(n),
        "<" => Rango::Max(n.saturating_sub(1)),
        "<=" => Rango::Max(n),
        _ => Rango::Exacto(n),
    })
}

/// Números con unidad: `500`, `10k`, `2mb`, `1.5gb`.
fn numero(s: &str) -> Option<u64> {
    let s = s.trim().to_ascii_lowercase();
    let (cuerpo, mult) = if let Some(c) = s.strip_suffix("gb").or_else(|| s.strip_suffix('g')) {
        (c, 1024u64 * 1024 * 1024)
    } else if let Some(c) = s.strip_suffix("mb").or_else(|| s.strip_suffix('m')) {
        (c, 1024 * 1024)
    } else if let Some(c) = s.strip_suffix("kb").or_else(|| s.strip_suffix('k')) {
        (c, 1024)
    } else {
        (s.as_str(), 1)
    };
    let cuerpo = cuerpo.trim();
    if cuerpo.is_empty() {
        return None;
    }
    let valor: f64 = cuerpo.parse().ok()?;
    if valor < 0.0 || !valor.is_finite() {
        return None;
    }
    Some((valor * mult as f64) as u64)
}

fn color(v: &str) -> Option<[u8; 3]> {
    let h = v.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

fn si_o_no(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "si" | "sí" | "s" | "yes" | "y" | "1" | "true" => Some(true),
        "no" | "n" | "0" | "false" => Some(false),
        _ => None,
    }
}

/// Lee una fecha de `fecha:` y devuelve desde y hasta, en ms, incluidos.
///
/// Las relativas —`hoy`, `7d`, `mes`— se calculan con `ahora` cada vez que se
/// busca, y no al escribirlas: una carpeta inteligente de «los últimos siete
/// días» tiene que seguir siéndolo mañana. Las absolutas son días, meses o
/// años enteros en hora local, con comparador si se quiere: `2026-10`,
/// `>=2026-01-01`, `<2026`.
fn fecha(v: &str, ahora: u64, desfase_s: i64) -> Option<(Option<u64>, Option<u64>)> {
    use crate::time::{fecha_local, inicio_dia_local_ms};
    let v = v.to_lowercase();
    const HORA: u64 = 3_600_000;
    const DIA: u64 = 24 * HORA;
    let (y, m, d) = fecha_local(ahora, desfase_s);
    let hoy = inicio_dia_local_ms(y, m, d, desfase_s);
    let hace = |ms: u64| Some((Some(ahora.saturating_sub(ms)), None));
    match v.as_str() {
        "hoy" | "today" => return Some((Some(hoy), None)),
        "ayer" | "yesterday" => {
            return Some((Some(inicio_dia_local_ms(y, m, d - 1, desfase_s)), Some(hoy - 1)))
        }
        "semana" | "week" => return hace(7 * DIA),
        "mes" | "month" => return hace(30 * DIA),
        "año" | "ano" | "year" => return hace(365 * DIA),
        _ => {}
    }
    // `7d`, `12h`, `2s` (semanas), `3m` (meses), `1a` (años).
    if let Some(u) = v.chars().last().filter(|c| c.is_ascii_alphabetic()) {
        if let Ok(n) = v[..v.len() - 1].parse::<u64>() {
            let unidad = match u {
                'h' => HORA,
                'd' => DIA,
                's' | 'w' => 7 * DIA,
                'm' => 30 * DIA,
                'a' | 'y' => 365 * DIA,
                _ => return None,
            };
            return hace(n * unidad);
        }
    }

    let (op, resto) = [">=", "<=", ">", "<"]
        .iter()
        .find_map(|op| v.strip_prefix(op).map(|r| (*op, r)))
        .unwrap_or(("=", v.as_str()));
    let partes: Vec<&str> = resto.split('-').collect();
    let num = |i: usize| partes.get(i).and_then(|p| p.parse::<i64>().ok());
    let (inicio, fin) = match partes.len() {
        1 => {
            let a = num(0).filter(|a| (1900..3000).contains(a))?;
            (inicio_dia_local_ms(a, 1, 1, desfase_s), inicio_dia_local_ms(a + 1, 1, 1, desfase_s))
        }
        2 => {
            let (a, me) = (num(0)?, num(1).filter(|me| (1..=12).contains(me))? as u32);
            (inicio_dia_local_ms(a, me, 1, desfase_s), inicio_dia_local_ms(a, me + 1, 1, desfase_s))
        }
        3 => {
            let (a, me) = (num(0)?, num(1).filter(|me| (1..=12).contains(me))? as u32);
            let di = num(2).filter(|di| (1..=31).contains(di))? as u32;
            (inicio_dia_local_ms(a, me, di, desfase_s), inicio_dia_local_ms(a, me, di + 1, desfase_s))
        }
        _ => return None,
    };
    Some(match op {
        ">=" => (Some(inicio), None),
        ">" => (Some(fin), None),
        "<" => (None, Some(inicio.saturating_sub(1))),
        "<=" => (None, Some(fin - 1)),
        _ => (Some(inicio), Some(fin - 1)),
    })
}

/// Lee la línea del buscador sobre una consulta de partida.
///
/// La consulta de partida es la que trae lo que **no** se escribe: la carpeta
/// elegida en la barra lateral, el límite, el orden por defecto. Lo escrito
/// manda sobre ella.
///
/// Devuelve también los avisos: valores que se entienden a medias, como
/// `estrellas:muchas`. Van a la barra de estado en vez de bloquear la
/// búsqueda, porque quien escribe está a media palabra casi todo el rato.
pub fn parsear(entrada: &str, base: Query) -> (Query, Vec<String>) {
    let mut q = base;
    let mut avisos = Vec::new();
    let mut libres: Vec<String> = Vec::new();

    for trozo in trocear(entrada) {
        // Atajo: `#rótulo` es lo mismo que `etiqueta:rótulo`. Se escribe solo.
        if let Some(t) = trozo.strip_prefix('#') {
            if !t.is_empty() {
                q.tags.push(t.to_string());
                continue;
            }
        }
        let Some((campo, valor)) = trozo.split_once(':') else {
            libres.push(trozo);
            continue;
        };
        let campo_l = campo.to_ascii_lowercase();
        let v = valor.trim();
        if v.is_empty() {
            // `etiqueta:` a medio escribir no es un error, es alguien tecleando.
            continue;
        }
        let mut mal = || avisos.push(format!("no entiendo «{campo}:{valor}»"));
        match campo_l.as_str() {
            "etiqueta" | "etiquetas" | "tag" | "tags" => q.tags.push(v.to_string()),
            // `tipo:` sirve para las dos cosas y elige sola: `tipo:video` es una
            // familia y `tipo:png` una extensión. Obligar a acordarse de cuál
            // es cada campo sería hacerle recordar a la persona una distinción
            // que solo existe por dentro.
            "ext" | "tipo" | "formato" | "familia" => match Familia::parse(v) {
                Some(f) => q.familias.push(f),
                None => q.exts.push(v.trim_start_matches('.').to_string()),
            },
            "estrellas" | "stars" => match rango(v) {
                Some(Rango::Min(n)) => q.min_stars = Some(n.min(5) as u8),
                Some(Rango::Max(n)) => q.max_stars = Some(n.min(5) as u8),
                Some(Rango::Exacto(n)) => {
                    q.min_stars = Some(n.min(5) as u8);
                    q.max_stars = Some(n.min(5) as u8);
                }
                None => mal(),
            },
            "ancho" | "width" | "w" => match rango(v) {
                Some(Rango::Min(n)) => q.min_width = Some(n as u32),
                Some(Rango::Max(n)) => q.max_width = Some(n as u32),
                Some(Rango::Exacto(n)) => {
                    q.min_width = Some(n as u32);
                    q.max_width = Some(n as u32);
                }
                None => mal(),
            },
            "alto" | "height" | "h" => match rango(v) {
                Some(Rango::Min(n)) => q.min_height = Some(n as u32),
                Some(Rango::Max(n)) => q.max_height = Some(n as u32),
                Some(Rango::Exacto(n)) => {
                    q.min_height = Some(n as u32);
                    q.max_height = Some(n as u32);
                }
                None => mal(),
            },
            "peso" | "size" | "tamaño" | "tamano" => match rango(v) {
                Some(Rango::Min(n)) => q.min_size = Some(n),
                Some(Rango::Max(n)) => q.max_size = Some(n),
                // Un peso exacto en bytes no se lo sabe nadie: escribir
                // `peso:10mb` quiere decir «de diez megas para arriba».
                Some(Rango::Exacto(n)) => q.min_size = Some(n),
                None => mal(),
            },
            "orientacion" | "orientación" | "orient" => match Orientation::parse(v) {
                Some(o) => q.orientation = Some(o),
                None => mal(),
            },
            "color" => match color(v) {
                Some(c) => q.color = Some(c),
                None => mal(),
            },
            "orden" | "sort" => match SortBy::parse(v) {
                Some(s) => q.sort = s,
                None => mal(),
            },
            "fecha" | "date" | "importado" | "cuando" => {
                let ahora = crate::time::now_ms();
                match fecha(v, ahora, crate::time::desfase_local_s(ahora)) {
                    Some((desde, hasta)) => {
                        q.desde_ms = desde;
                        q.hasta_ms = hasta;
                    }
                    None => mal(),
                }
            }
            "repetidos" | "repetido" | "duplicados" | "dupes" => match si_o_no(v) {
                Some(b) => q.repetidos = b,
                None => mal(),
            },
            "papelera" | "trash" => match si_o_no(v) {
                Some(b) => q.papelera = b,
                None => mal(),
            },
            // Campo aparte y no `etiqueta:no`: eso buscaría una etiqueta que se
            // llamase «no», y alguien la tendrá.
            "etiquetado" | "etiquetada" | "tagged" => match si_o_no(v) {
                Some(b) => q.etiquetado = Some(b),
                None => mal(),
            },
            // `adulto:si` y `+18:si` son lo mismo. El segundo porque es como se
            // llama por ahí fuera, y quien lo escriba no debería tener que
            // acertar con la palabra que usamos por dentro.
            "adulto" | "+18" | "adult" => match si_o_no(v) {
                Some(b) => q.adulto = Some(b),
                None => mal(),
            },
            // Un campo que no conocemos no es un error: es texto. Así una URL
            // pegada busca por la URL en vez de dar un mensaje inútil.
            _ => libres.push(trozo),
        }
    }

    let texto = libres.join(" ");
    q.text = if texto.trim().is_empty() {
        None
    } else {
        Some(texto)
    };
    (q, avisos)
}

/// Escribe la consulta como se escribiría a mano.
///
/// Es la vuelta del viaje, y existe para que la barra de filtros y la línea de
/// texto no puedan discrepar: tocar un desplegable reescribe la línea, y la
/// línea es lo único que se ejecuta.
pub fn escribir(q: &Query) -> String {
    let mut partes: Vec<String> = Vec::new();
    let entrecomilla = |s: &str| {
        if s.contains(' ') {
            format!("\"{s}\"")
        } else {
            s.to_string()
        }
    };
    if let Some(t) = q.text.as_ref().filter(|t| !t.trim().is_empty()) {
        partes.push(t.trim().to_string());
    }
    for t in &q.tags {
        partes.push(format!("etiqueta:{}", entrecomilla(t)));
    }
    for f in &q.familias {
        partes.push(format!("tipo:{}", f.as_str()));
    }
    for e in &q.exts {
        partes.push(format!("ext:{e}"));
    }
    match (q.min_stars, q.max_stars) {
        (Some(a), Some(b)) if a == b => partes.push(format!("estrellas:{a}")),
        (Some(a), _) => partes.push(format!("estrellas:>={a}")),
        (None, Some(b)) => partes.push(format!("estrellas:<={b}")),
        _ => {}
    }
    if let Some(w) = q.min_width {
        partes.push(format!("ancho:>={w}"));
    }
    if let Some(h) = q.min_height {
        partes.push(format!("alto:>={h}"));
    }
    if let Some(s) = q.min_size {
        partes.push(format!("peso:>={s}"));
    }
    if let Some(o) = q.orientation {
        partes.push(format!(
            "orientacion:{}",
            match o {
                Orientation::Landscape => "apaisada",
                Orientation::Portrait => "vertical",
                Orientation::Square => "cuadrada",
            }
        ));
    }
    if let Some(c) = q.color {
        partes.push(format!("color:#{:02x}{:02x}{:02x}", c[0], c[1], c[2]));
    }
    // De vuelta se escriben como fechas absolutas: lo relativo que se tecleó
    // ya no se sabe aquí, y un día concreto se entiende igual de bien.
    let desfase = crate::time::desfase_local_s(crate::time::now_ms());
    let dia = |ms: u64| {
        let (y, m, d) = crate::time::fecha_local(ms, desfase);
        format!("{y:04}-{m:02}-{d:02}")
    };
    if let Some(t) = q.desde_ms {
        partes.push(format!("fecha:>={}", dia(t)));
    }
    if let Some(t) = q.hasta_ms {
        partes.push(format!("fecha:<={}", dia(t)));
    }
    if q.repetidos {
        partes.push("repetidos:si".into());
    }
    if q.papelera {
        partes.push("papelera:si".into());
    }
    if let Some(a) = q.adulto {
        partes.push(format!("adulto:{}", if a { "si" } else { "no" }));
    }
    if let Some(e) = q.etiquetado {
        partes.push(format!("etiquetado:{}", if e { "si" } else { "no" }));
    }
    partes.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Query {
        parsear(s, Query::default()).0
    }

    #[test]
    fn las_fechas_relativas_cuentan_desde_ahora_y_en_hora_local() {
        use crate::time::inicio_dia_local_ms;
        // 5 de octubre de 2026, 18:30 en Madrid (UTC+2).
        let desfase = 2 * 3600;
        let ahora = inicio_dia_local_ms(2026, 10, 5, desfase) + (18 * 60 + 30) * 60_000;
        let medianoche = inicio_dia_local_ms(2026, 10, 5, desfase);
        assert_eq!(fecha("hoy", ahora, desfase), Some((Some(medianoche), None)));
        let (d, h) = fecha("ayer", ahora, desfase).unwrap();
        assert_eq!(d, Some(medianoche - 86_400_000));
        assert_eq!(h, Some(medianoche - 1));
        assert_eq!(fecha("7d", ahora, desfase), Some((Some(ahora - 7 * 86_400_000), None)));
        assert_eq!(fecha("semana", ahora, desfase), fecha("7d", ahora, desfase));
        assert_eq!(fecha("12h", ahora, desfase), Some((Some(ahora - 12 * 3_600_000), None)));
    }

    #[test]
    fn las_fechas_absolutas_son_dias_meses_o_años_enteros() {
        use crate::time::inicio_dia_local_ms;
        let desfase = 3600;
        let ahora = inicio_dia_local_ms(2026, 10, 5, desfase);
        let oct = inicio_dia_local_ms(2026, 10, 1, desfase);
        let nov = inicio_dia_local_ms(2026, 11, 1, desfase);
        assert_eq!(fecha("2026-10", ahora, desfase), Some((Some(oct), Some(nov - 1))));
        assert_eq!(fecha(">=2026-10", ahora, desfase), Some((Some(oct), None)));
        assert_eq!(fecha("<2026-10", ahora, desfase), Some((None, Some(oct - 1))));
        // Diciembre pasa bien al año siguiente.
        let dic = fecha("2026-12", ahora, desfase).unwrap();
        assert_eq!(dic.1, Some(inicio_dia_local_ms(2027, 1, 1, desfase) - 1));
        assert_eq!(fecha("2026-10-05", ahora, desfase).unwrap().0, Some(ahora));
        assert_eq!(fecha("mañana por la tarde", ahora, desfase), None);
        assert_eq!(fecha("2026-13", ahora, desfase), None);
    }

    #[test]
    fn fecha_entra_en_la_consulta() {
        let q = p("gato fecha:2026");
        assert!(q.desde_ms.is_some() && q.hasta_ms.is_some());
        assert_eq!(q.text.as_deref(), Some("gato"));
    }

    #[test]
    fn el_texto_suelto_es_texto() {
        let q = p("gato azul");
        assert_eq!(q.text.as_deref(), Some("gato azul"));
        assert!(q.tags.is_empty());
    }

    #[test]
    fn los_campos_salen_del_texto_y_no_se_buscan_como_palabras() {
        let q = p("tipografía etiqueta:rótulo ext:png");
        assert_eq!(q.text.as_deref(), Some("tipografía"));
        assert_eq!(q.tags, vec!["rótulo".to_string()]);
        assert_eq!(q.exts, vec!["png".to_string()]);
    }

    #[test]
    fn las_comillas_mantienen_juntas_las_palabras_de_una_etiqueta() {
        let q = p(r#"etiqueta:"arte generativo" azul"#);
        assert_eq!(q.tags, vec!["arte generativo".to_string()]);
        assert_eq!(q.text.as_deref(), Some("azul"));
    }

    #[test]
    fn la_almohadilla_es_un_atajo_de_etiqueta() {
        let q = p("#nocturno #azul");
        assert_eq!(q.tags.len(), 2);
        assert!(q.text.is_none());
    }

    #[test]
    fn los_comparadores_de_estrellas() {
        assert_eq!(p("estrellas:>=4").min_stars, Some(4));
        assert_eq!(
            p("estrellas:>3").min_stars,
            Some(4),
            "«más de 3» es «4 o más»"
        );
        assert_eq!(p("estrellas:<2").max_stars, Some(1));
        let q = p("estrellas:0");
        assert_eq!(
            (q.min_stars, q.max_stars),
            (Some(0), Some(0)),
            "sin puntuar"
        );
    }

    #[test]
    fn las_estrellas_no_se_salen_de_cinco() {
        assert_eq!(p("estrellas:>=9").min_stars, Some(5));
    }

    #[test]
    fn el_peso_entiende_unidades() {
        assert_eq!(p("peso:>2mb").min_size, Some(2 * 1024 * 1024 + 1));
        assert_eq!(p("peso:<500k").max_size, Some(500 * 1024 - 1));
        assert_eq!(p("peso:1.5gb").min_size, Some(1610612736));
    }

    #[test]
    fn el_color_se_escribe_como_en_cualquier_sitio() {
        assert_eq!(p("color:#ff0055").color, Some([255, 0, 85]));
        assert_eq!(p("color:ff0055").color, Some([255, 0, 85]));
    }

    #[test]
    fn un_campo_desconocido_se_busca_como_texto() {
        let q = p("https://ejemplo.com/foto.png");
        assert_eq!(q.text.as_deref(), Some("https://ejemplo.com/foto.png"));
        assert!(
            parsear("https://x", Query::default()).1.is_empty(),
            "y sin regañar"
        );
    }

    #[test]
    fn un_valor_que_no_se_entiende_avisa_pero_no_rompe() {
        let (q, avisos) = parsear("estrellas:muchas gato", Query::default());
        assert_eq!(q.text.as_deref(), Some("gato"));
        assert_eq!(avisos.len(), 1);
        assert!(avisos[0].contains("estrellas:muchas"));
    }

    #[test]
    fn un_campo_a_medio_escribir_no_es_un_error() {
        let (q, avisos) = parsear("etiqueta:", Query::default());
        assert!(avisos.is_empty());
        assert!(q.tags.is_empty());
    }

    #[test]
    fn la_consulta_de_partida_aporta_lo_que_no_se_escribe() {
        let base = Query {
            folder: Some("f1".into()),
            limit: 0,
            ..Default::default()
        };
        let q = parsear("gato", base).0;
        assert_eq!(q.folder.as_deref(), Some("f1"));
        assert_eq!(q.limit, 0);
    }

    #[test]
    fn escribir_y_volver_a_leer_da_la_misma_consulta() {
        let original = p(
            r#"gato etiqueta:"arte generativo" ext:png estrellas:>=4 orientacion:vertical color:#112233"#,
        );
        let vuelta = p(&escribir(&original));
        assert_eq!(vuelta.text, original.text);
        assert_eq!(vuelta.tags, original.tags);
        assert_eq!(vuelta.exts, original.exts);
        assert_eq!(vuelta.min_stars, original.min_stars);
        assert_eq!(vuelta.orientation, original.orientation);
        assert_eq!(vuelta.color, original.color);
    }

    #[test]
    fn tipo_distingue_solo_entre_familia_y_extension() {
        let q = p("tipo:video tipo:png");
        assert_eq!(q.familias, vec![Familia::Video]);
        assert_eq!(q.exts, vec!["png".to_string()]);
        assert!(q.text.is_none(), "ninguno de los dos se busca como palabra");
    }

    #[test]
    fn la_marca_de_adulto_se_pide_de_las_dos_maneras() {
        assert_eq!(p("adulto:si").adulto, Some(true));
        assert_eq!(p("+18:si").adulto, Some(true));
        assert_eq!(p("adulto:no").adulto, Some(false));
        assert_eq!(p("").adulto, None, "sin poner nada se ve todo");
        // Y da la vuelta: lo escrito y lo leído tienen que ser lo mismo.
        assert_eq!(escribir(&p("adulto:si")), "adulto:si");
    }

    #[test]
    fn sin_etiquetar_es_un_campo_y_no_una_etiqueta() {
        assert_eq!(p("etiquetado:no").etiquetado, Some(false));
        assert_eq!(p("etiquetado:si").etiquetado, Some(true));
        // `etiqueta:no` sigue siendo la etiqueta «no».
        let q = p("etiqueta:no");
        assert_eq!(q.tags, vec!["no".to_string()]);
        assert_eq!(q.etiquetado, None);
        assert_eq!(escribir(&p("etiquetado:no")), "etiquetado:no");
    }

    #[test]
    fn la_papelera_se_pide_por_su_nombre() {
        assert!(p("papelera:si").papelera);
        assert!(!p("papelera:no").papelera);
    }
}
