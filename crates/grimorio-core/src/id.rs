//! ULID propio: 48 bits de milisegundos + 80 bits de azar, en Crockford base32.
//!
//! Por qué no una dependencia: son ~80 líneas, no cambia nunca y así el orden
//! monotónico dentro del proceso es *nuestro* invariante, no el de otro.
//!
//! Consecuencia de diseño que usamos en todas partes: ordenar por `id` es
//! ordenar por fecha de importación, así que la vista por defecto no ordena nada.

use std::sync::{Mutex, OnceLock};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const RAND_MASK: u128 = (1u128 << 80) - 1;

pub fn new_id() -> String {
    generator().lock().unwrap().next()
}

/// ULID con marca de tiempo fijada. Solo para tests y para el generador sintético,
/// donde queremos bibliotecas reproducibles.
pub fn id_at(ms: u64, seq: u128) -> String {
    encode(((ms as u128) << 80) | (seq & RAND_MASK))
}

pub fn timestamp_ms(id: &str) -> Option<u64> {
    let v = decode(id)?;
    Some((v >> 80) as u64)
}

fn generator() -> &'static Mutex<UlidGen> {
    static GEN: OnceLock<Mutex<UlidGen>> = OnceLock::new();
    GEN.get_or_init(|| Mutex::new(UlidGen::from_os_entropy()))
}

struct UlidGen {
    state: u128,
    last_ms: u64,
    last_rand: u128,
}

impl UlidGen {
    fn from_os_entropy() -> Self {
        let mut seed = [0u8; 16];
        // Nunca `fs::read` sobre /dev/urandom: no tiene EOF. Solo lectura acotada.
        {
            use std::io::Read;
            if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
                let _ = f.read_exact(&mut seed);
            }
        }
        // Sin /dev/urandom (Windows), el azar del sistema se saca de la
        // biblioteca estándar: cada `RandomState` nace con claves pedidas al
        // sistema operativo, y el hash de nada con esas claves es un número al
        // azar. Sin esto, dos procesos arrancados en el mismo milisegundo —la
        // aplicación y `grim`— sacarían los mismos identificadores.
        if seed == [0u8; 16] {
            use std::hash::{BuildHasher, Hasher};
            let azar = || {
                std::collections::hash_map::RandomState::new()
                    .build_hasher()
                    .finish()
            };
            seed[..8].copy_from_slice(&azar().to_le_bytes());
            seed[8..].copy_from_slice(&azar().to_le_bytes());
        }
        // Si aun así no hubo entropía caemos a la hora: peor azar, pero el id
        // sigue siendo único porque el generador es monotónico dentro del proceso.
        let mut state = u128::from_le_bytes(seed);
        if state == 0 {
            state = crate::time::now_ms() as u128 * 0x2545_F491_4F6C_DD1D;
        }
        UlidGen {
            state,
            last_ms: 0,
            last_rand: 0,
        }
    }

    /// xorshift128: suficiente para la parte de azar de un identificador local.
    fn next_rand(&mut self) -> u128 {
        let mut x = self.state;
        x ^= x << 23;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x & RAND_MASK
    }

    fn next(&mut self) -> String {
        let ms = crate::time::now_ms();
        if ms == self.last_ms {
            self.last_rand = (self.last_rand + 1) & RAND_MASK;
        } else {
            self.last_ms = ms;
            self.last_rand = self.next_rand();
        }
        encode(((ms as u128) << 80) | self.last_rand)
    }
}

/// Si `id` tiene forma de identificador de Grimorio: veintiséis letras o
/// cifras ASCII y nada más.
///
/// Es una comprobación de **forma**, no de alfabeto Crockford, a propósito.
/// Lo que importa es que un id nunca pueda ser una ruta: con él se construye
/// `items/AB/CD/<id>/`, y un `item.json` editado a mano o venido de otra
/// biblioteca con `"id": "../../x"` hacía que vaciar la papelera borrara
/// fuera de la biblioteca. Sin barras, sin puntos y con longitud fija, eso no
/// puede pasar; ser más estricto con las letras solo serviría para dejar
/// fuera bibliotecas que hoy abren bien.
pub fn es_valido(id: &str) -> bool {
    id.len() == 26 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn encode(mut v: u128) -> String {
    let mut out = [0u8; 26];
    for slot in out.iter_mut().rev() {
        *slot = ALPHABET[(v & 31) as usize];
        v >>= 5;
    }
    // 26 caracteres base32 cubren 130 bits; los 2 sobrantes quedan a cero.
    String::from_utf8(out.to_vec()).expect("alfabeto ASCII")
}

fn decode(s: &str) -> Option<u128> {
    if s.len() != 26 {
        return None;
    }
    let mut v: u128 = 0;
    for c in s.bytes() {
        let d = ALPHABET.iter().position(|a| *a == c.to_ascii_uppercase())?;
        v = (v << 5) | d as u128;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_son_de_26_caracteres_y_ordenables() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 26);
        assert_eq!(b.len(), 26);
        assert!(a < b, "{a} debería ordenar antes que {b}");
    }

    #[test]
    fn el_timestamp_va_y_vuelve() {
        let id = id_at(1_756_000_000_000, 7);
        assert_eq!(timestamp_ms(&id), Some(1_756_000_000_000));
    }

    #[test]
    fn no_hay_colisiones_en_rafaga() {
        let ids: std::collections::HashSet<String> = (0..50_000).map(|_| new_id()).collect();
        assert_eq!(ids.len(), 50_000);
    }

    #[test]
    fn un_id_valido_nunca_es_una_ruta() {
        assert!(es_valido(&new_id()));
        assert!(es_valido(&id_at(1_000, 85)));
        // El de las bibliotecas de demostración.
        assert!(es_valido("01M48C5NYN0000000000000085"));
        for malo in ["", "../x", "../../../../../../../../../x", "01M48C5NYN000000000000008/", "01M48C5NYN00000000000000.."] {
            assert!(!es_valido(malo), "{malo:?} no debería pasar");
        }
    }

    #[test]
    fn el_orden_lexicografico_es_orden_temporal() {
        let viejo = id_at(1_000, 0);
        let nuevo = id_at(2_000, 0);
        assert!(viejo < nuevo);
    }
}
