//! Carpetas.
//!
//! Un elemento puede estar en varias a la vez, como en Eagle: la carpeta es una
//! etiqueta con jerarquía, no un sitio donde vive el archivo. El archivo vive
//! siempre en `items/<shard>/<id>/`, y eso no lo cambia ninguna carpeta.
//!
//! La verdad está repartida en dos sitios y a propósito:
//!   * `folders.json` guarda **qué carpetas existen** (nombre, padre, color).
//!   * cada `item.json` guarda **en cuáles está** ese elemento.
//!
//! Así, borrar `index.sqlite` no pierde nada, y copiar un elemento a otra
//! biblioteca se lleva su pertenencia dentro del propio archivo.

use crate::error::{Error, Result};
use crate::id;
use crate::item::write_atomic;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FOLDERS_FILE: &str = "folders.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Color de la pestaña en la barra lateral, en "#rrggbb". Lo elige quien usa
    /// el programa; el tema solo pone el color por defecto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Orden entre hermanas. Se deja hueco al insertar para no renumerar todo.
    #[serde(default)]
    pub pos: i64,
}

impl Folder {
    pub fn nueva(name: &str, parent: Option<&str>) -> Folder {
        Folder {
            id: id::new_id(),
            name: name.trim().to_string(),
            parent: parent.map(|s| s.to_string()),
            color: None,
            pos: 0,
        }
    }
}

/// El árbol entero, en memoria. Son decenas o cientos de carpetas, nunca
/// millones: mantenerlo en un vector y recorrerlo es más rápido y mucho más
/// simple que cualquier estructura con punteros.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folders {
    #[serde(default = "esquema_actual")]
    pub schema: u32,
    #[serde(default)]
    pub folders: Vec<Folder>,
}

fn esquema_actual() -> u32 {
    crate::SCHEMA_VERSION
}

impl Folders {
    pub fn cargar(root: &Path) -> Result<Folders> {
        let p = root.join(FOLDERS_FILE);
        if !p.exists() {
            return Ok(Folders {
                schema: crate::SCHEMA_VERSION,
                folders: Vec::new(),
            });
        }
        let bytes = std::fs::read(&p).map_err(|e| Error::io(&p, e))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Invalid(format!("{} no se puede leer: {e}", p.display())))
    }

    pub fn guardar(&self, root: &Path) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| Error::Invalid(format!("no pude serializar las carpetas: {e}")))?;
        write_atomic(&root.join(FOLDERS_FILE), &bytes)
    }

    pub fn get(&self, id: &str) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }

    pub fn existe(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Hijas directas, ya ordenadas como se van a pintar.
    pub fn hijas(&self, padre: Option<&str>) -> Vec<&Folder> {
        let mut v: Vec<&Folder> = self
            .folders
            .iter()
            .filter(|f| f.parent.as_deref() == padre)
            .collect();
        v.sort_by(|a, b| a.pos.cmp(&b.pos).then_with(|| alfabetico(&a.name, &b.name)));
        v
    }

    /// La carpeta y todo lo que cuelga de ella. Es lo que hace falta para que
    /// pinchar en una carpeta enseñe también lo de sus hijas.
    pub fn con_descendientes(&self, id: &str) -> Vec<String> {
        let mut salida = Vec::new();
        if !self.existe(id) {
            return salida;
        }
        salida.push(id.to_string());
        let mut i = 0;
        // Anchura, sin recursión: un `folders.json` corrupto con un ciclo no
        // debe desbordar la pila. El `contains` corta el ciclo.
        while i < salida.len() {
            let actual = salida[i].clone();
            for f in &self.folders {
                if f.parent.as_deref() == Some(actual.as_str()) && !salida.contains(&f.id) {
                    salida.push(f.id.clone());
                }
            }
            i += 1;
        }
        salida
    }

    /// ¿Meter `id` dentro de `nuevo_padre` crearía un ciclo?
    pub fn seria_ciclo(&self, id: &str, nuevo_padre: Option<&str>) -> bool {
        let Some(mut actual) = nuevo_padre else {
            return false;
        };
        if actual == id {
            return true;
        }
        // Subir por los padres hasta la raíz. El contador es el seguro contra un
        // ciclo que ya estuviera en el archivo.
        for _ in 0..self.folders.len() + 1 {
            match self.get(actual).and_then(|f| f.parent.as_deref()) {
                Some(p) if p == id => return true,
                Some(p) => actual = p,
                None => return false,
            }
        }
        true
    }

    /// Ruta legible, de la raíz a la hoja: "Referencias / Tipografía / Rótulos".
    pub fn ruta(&self, id: &str) -> String {
        let mut partes = Vec::new();
        let mut actual = Some(id.to_string());
        for _ in 0..self.folders.len() + 1 {
            let Some(a) = actual else { break };
            let Some(f) = self.get(&a) else { break };
            partes.push(f.name.clone());
            actual = f.parent.clone();
        }
        partes.reverse();
        partes.join(" / ")
    }

    pub fn crear(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("una carpeta necesita nombre".into()));
        }
        if let Some(p) = parent {
            if !self.existe(p) {
                return Err(Error::Invalid(format!("no existe la carpeta padre {p}")));
            }
        }
        let mut f = Folder::nueva(name, parent);
        f.pos = self.hijas(parent).last().map(|u| u.pos + 10).unwrap_or(0);
        let id = f.id.clone();
        self.folders.push(f);
        Ok(id)
    }

    pub fn renombrar(&mut self, id: &str, name: &str) -> Result<()> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(Error::Invalid("una carpeta necesita nombre".into()));
        }
        match self.folders.iter_mut().find(|f| f.id == id) {
            Some(f) => {
                f.name = name;
                Ok(())
            }
            None => Err(Error::Invalid(format!("no existe la carpeta {id}"))),
        }
    }

    /// Le pone color a la carpeta, o se lo quita con `None`.
    ///
    /// Se valida aquí y no en la interfaz: `folders.json` es legible y editable
    /// a mano, y un «rojo» escrito en él tiene que fallar al entrar por la API
    /// igual que fallaría al leerlo un tema.
    pub fn colorear(&mut self, id: &str, color: Option<&str>) -> Result<()> {
        let color = match color.map(str::trim).filter(|c| !c.is_empty()) {
            None => None,
            Some(c) if c.len() == 7 && c.starts_with('#')
                && c[1..].chars().all(|x| x.is_ascii_hexdigit()) => Some(c.to_ascii_lowercase()),
            Some(c) => return Err(Error::Invalid(format!("«{c}» no es un color #rrggbb"))),
        };
        match self.folders.iter_mut().find(|f| f.id == id) {
            Some(f) => {
                f.color = color;
                Ok(())
            }
            None => Err(Error::Invalid(format!("no existe la carpeta {id}"))),
        }
    }

    /// Cuelga la carpeta de otra y la deja la última de sus hermanas.
    pub fn mover(&mut self, id: &str, nuevo_padre: Option<&str>) -> Result<()> {
        self.recolocar(id, nuevo_padre, None)
    }

    /// Cambia de quién cuelga y **en qué sitio** queda entre sus hermanas.
    ///
    /// `antes_de` es la hermana delante de la cual se coloca; `None` la deja la
    /// última. Renumerar el grupo entero es más simple que ir buscando hueco, y
    /// son decenas de filas: el coste no se mide. Lo que sí importa es que al
    /// terminar los `pos` vuelvan a estar separados de diez en diez, porque si
    /// dos hermanas empatan, el desempate alfabético manda sobre lo que acaba
    /// de decidir quien arrastró.
    pub fn recolocar(
        &mut self,
        id: &str,
        nuevo_padre: Option<&str>,
        antes_de: Option<&str>,
    ) -> Result<()> {
        if !self.existe(id) {
            return Err(Error::Invalid(format!("no existe la carpeta {id}")));
        }
        if let Some(p) = nuevo_padre {
            if !self.existe(p) {
                return Err(Error::Invalid(format!("no existe la carpeta padre {p}")));
            }
        }
        if self.seria_ciclo(id, nuevo_padre) {
            return Err(Error::Invalid(
                "una carpeta no puede meterse dentro de sí misma".into(),
            ));
        }
        // Soltar una carpeta justo delante de sí misma no es un error, es no
        // haberla movido. Tratarlo como `None` la mandaría al final, que es
        // exactamente la sorpresa que nadie pidió.
        if antes_de == Some(id) {
            return Ok(());
        }

        // El grupo de destino, ya en el orden en que se pinta y sin la que se
        // mueve: insertarla en esta lista es la única cuenta que hay que hacer.
        let mut hermanas: Vec<String> = self
            .hijas(nuevo_padre)
            .iter()
            .map(|f| f.id.clone())
            .filter(|h| h != id)
            .collect();
        let donde = match antes_de {
            Some(a) => hermanas
                .iter()
                .position(|h| h == a)
                // Una hermana que no está en el grupo de destino no dice nada
                // sobre el sitio: al final, como si no se hubiera pedido.
                .unwrap_or(hermanas.len()),
            None => hermanas.len(),
        };
        hermanas.insert(donde, id.to_string());

        if let Some(f) = self.folders.iter_mut().find(|f| f.id == id) {
            f.parent = nuevo_padre.map(|s| s.to_string());
        }
        for (i, h) in hermanas.iter().enumerate() {
            if let Some(f) = self.folders.iter_mut().find(|f| &f.id == h) {
                f.pos = (i as i64) * 10;
            }
        }
        Ok(())
    }

    /// Quita la carpeta y todas sus descendientes. Devuelve los ids borrados
    /// para que quien llame desvincule los elementos.
    pub fn borrar(&mut self, id: &str) -> Result<Vec<String>> {
        if !self.existe(id) {
            return Err(Error::Invalid(format!("no existe la carpeta {id}")));
        }
        let fuera = self.con_descendientes(id);
        self.folders.retain(|f| !fuera.contains(&f.id));
        Ok(fuera)
    }
}

/// Orden por nombre que no deja "Zócalo" antes que "árbol" ni "Foto10" antes
/// que "Foto2".
fn alfabetico(a: &str, b: &str) -> std::cmp::Ordering {
    let na = normalizar(a);
    let nb = normalizar(b);
    natural(&na, &nb)
}

fn normalizar(s: &str) -> String {
    // Primero minúsculas y luego quitar tildes, no al revés: si se hace al
    // revés, "Álbum" conserva su Á mayúscula, se queda fuera de la tabla y
    // acaba ordenado después de la zeta.
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            otro => otro,
        })
        .collect()
}

/// Comparación natural: los tramos de dígitos se comparan como números.
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut ia = a.chars().peekable();
    let mut ib = b.chars().peekable();
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na: String = tomar_digitos(&mut ia);
                    let nb: String = tomar_digitos(&mut ib);
                    // Comparar por longitud primero evita desbordar con números
                    // absurdamente largos en un nombre de archivo.
                    let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                    match ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb)) {
                        Ordering::Equal => continue,
                        otro => return otro,
                    }
                }
                match ca.cmp(&cb) {
                    Ordering::Equal => {
                        ia.next();
                        ib.next();
                    }
                    otro => return otro,
                }
            }
        }
    }
}

fn tomar_digitos(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut s = String::new();
    while let Some(c) = it.peek().copied() {
        if c.is_ascii_digit() {
            s.push(c);
            it.next();
        } else {
            break;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arbol() -> (Folders, String, String, String) {
        let mut f = Folders::default();
        let raiz = f.crear("Referencias", None).unwrap();
        let tipo = f.crear("Tipografía", Some(&raiz)).unwrap();
        let rotulos = f.crear("Rótulos", Some(&tipo)).unwrap();
        (f, raiz, tipo, rotulos)
    }

    #[test]
    fn una_carpeta_no_puede_meterse_dentro_de_si_misma() {
        let (mut f, raiz, _tipo, rotulos) = arbol();
        assert!(f.mover(&raiz, Some(&rotulos)).is_err());
        assert!(f.mover(&raiz, Some(&raiz)).is_err());
        // Al revés sí: llevar la nieta a la raíz del árbol.
        assert!(f.mover(&rotulos, None).is_ok());
    }

    #[test]
    fn los_descendientes_incluyen_la_propia_carpeta() {
        let (f, raiz, tipo, rotulos) = arbol();
        let d = f.con_descendientes(&raiz);
        assert_eq!(d.len(), 3);
        assert!(d.contains(&raiz) && d.contains(&tipo) && d.contains(&rotulos));
        assert_eq!(f.con_descendientes(&rotulos), vec![rotulos]);
        assert!(f.con_descendientes("no-existe").is_empty());
    }

    #[test]
    fn borrar_se_lleva_lo_que_cuelga() {
        let (mut f, _raiz, tipo, rotulos) = arbol();
        let fuera = f.borrar(&tipo).unwrap();
        assert_eq!(fuera.len(), 2);
        assert!(!f.existe(&tipo) && !f.existe(&rotulos));
        assert_eq!(f.folders.len(), 1, "la raíz se queda");
    }

    #[test]
    fn el_color_de_una_carpeta_se_valida_y_se_quita() {
        let (mut f, raiz, _tipo, _rotulos) = arbol();
        f.colorear(&raiz, Some("#C0392B")).unwrap();
        assert_eq!(f.get(&raiz).unwrap().color.as_deref(), Some("#c0392b"));
        assert!(f.colorear(&raiz, Some("rojo")).is_err());
        assert!(f.colorear(&raiz, Some("#12345")).is_err());
        assert_eq!(f.get(&raiz).unwrap().color.as_deref(), Some("#c0392b"), "un error no lo toca");
        f.colorear(&raiz, None).unwrap();
        assert_eq!(f.get(&raiz).unwrap().color, None);
        assert!(f.colorear("no-existe", Some("#000000")).is_err());
    }

    #[test]
    fn la_ruta_se_lee_de_la_raiz_a_la_hoja() {
        let (f, _raiz, _tipo, rotulos) = arbol();
        assert_eq!(f.ruta(&rotulos), "Referencias / Tipografía / Rótulos");
    }

    #[test]
    fn un_ciclo_en_el_archivo_no_cuelga_el_programa() {
        // Nadie debería poder escribir esto, pero un archivo editado a mano sí.
        let mut f = Folders::default();
        f.folders.push(Folder {
            id: "A".into(),
            name: "a".into(),
            parent: Some("B".into()),
            color: None,
            pos: 0,
        });
        f.folders.push(Folder {
            id: "B".into(),
            name: "b".into(),
            parent: Some("A".into()),
            color: None,
            pos: 0,
        });
        assert_eq!(f.con_descendientes("A").len(), 2);
        assert!(f.seria_ciclo("A", Some("B")));
        assert!(!f.ruta("A").is_empty());
    }

    #[test]
    fn las_hermanas_se_ordenan_como_las_leeria_una_persona() {
        let mut f = Folders::default();
        for n in ["foto10", "Álbum", "foto2", "zócalo", "Foto1"] {
            f.crear(n, None).unwrap();
        }
        // `pos` manda; con el mismo hueco entre ellas, decide el nombre.
        for x in f.folders.iter_mut() {
            x.pos = 0;
        }
        let nombres: Vec<&str> = f.hijas(None).iter().map(|x| x.name.as_str()).collect();
        assert_eq!(nombres, ["Álbum", "Foto1", "foto2", "foto10", "zócalo"]);
    }

    #[test]
    fn sin_nombre_no_hay_carpeta() {
        let mut f = Folders::default();
        assert!(f.crear("   ", None).is_err());
        let id = f.crear("  con espacios  ", None).unwrap();
        assert_eq!(f.get(&id).unwrap().name, "con espacios");
        assert!(f.renombrar(&id, "").is_err());
    }

    /// Los nombres de las hijas de `padre`, en el orden en que se pintan.
    fn orden(f: &Folders, padre: Option<&str>) -> Vec<String> {
        f.hijas(padre).iter().map(|x| x.name.clone()).collect()
    }

    #[test]
    fn una_hermana_se_coloca_donde_se_suelta() {
        let mut f = Folders::default();
        let a = f.crear("A", None).unwrap();
        let b = f.crear("B", None).unwrap();
        let c = f.crear("C", None).unwrap();
        assert_eq!(orden(&f, None), ["A", "B", "C"]);

        // Delante de la primera.
        f.recolocar(&c, None, Some(&a)).unwrap();
        assert_eq!(orden(&f, None), ["C", "A", "B"]);
        // En medio.
        f.recolocar(&c, None, Some(&b)).unwrap();
        assert_eq!(orden(&f, None), ["A", "C", "B"]);
        // Sin hermana delante, la última.
        f.recolocar(&a, None, None).unwrap();
        assert_eq!(orden(&f, None), ["C", "B", "A"]);
    }

    #[test]
    fn colgar_y_colocar_son_el_mismo_gesto() {
        let mut f = Folders::default();
        let padre = f.crear("padre", None).unwrap();
        let x = f.crear("x", Some(&padre)).unwrap();
        let y = f.crear("y", Some(&padre)).unwrap();
        let suelta = f.crear("suelta", None).unwrap();

        // Entra en otra rama y en un sitio concreto de ella, de una vez.
        f.recolocar(&suelta, Some(&padre), Some(&y)).unwrap();
        assert_eq!(orden(&f, Some(&padre)), ["x", "suelta", "y"]);
        assert_eq!(
            f.get(&suelta).unwrap().parent.as_deref(),
            Some(padre.as_str())
        );

        // Volver a la raíz es colgar de nadie.
        f.recolocar(&suelta, None, None).unwrap();
        assert_eq!(orden(&f, None), ["padre", "suelta"]);
        let _ = x;
    }

    #[test]
    fn el_orden_sobrevive_a_que_los_nombres_digan_otra_cosa() {
        // Sin renumerar el grupo, dos hermanas empatadas a `pos` las desempata
        // el nombre, y el alfabeto acaba mandando sobre lo que decidió quien
        // arrastró. Aquí el orden pedido es justo el contrario del alfabético.
        let mut f = Folders::default();
        let a = f.crear("A", None).unwrap();
        let b = f.crear("B", None).unwrap();
        let c = f.crear("C", None).unwrap();
        f.recolocar(&c, None, Some(&a)).unwrap();
        f.recolocar(&b, None, Some(&c)).unwrap();
        assert_eq!(orden(&f, None), ["B", "C", "A"]);
        let sitios: Vec<i64> = f.hijas(None).iter().map(|x| x.pos).collect();
        assert_eq!(sitios, [0, 10, 20], "los huecos se rehacen al colocar");
    }

    #[test]
    fn soltarse_delante_de_si_misma_no_la_manda_al_final() {
        let mut f = Folders::default();
        let a = f.crear("A", None).unwrap();
        let _b = f.crear("B", None).unwrap();
        f.recolocar(&a, None, Some(&a)).unwrap();
        assert_eq!(orden(&f, None), ["A", "B"]);
    }

    #[test]
    fn colocarse_no_abre_la_puerta_a_un_ciclo() {
        let (mut f, raiz, tipo, rotulos) = arbol();
        // Ni dentro de una nieta, ni delante de una hermana que cuelga de ella:
        // las dos son la misma trampa por dos puertas.
        assert!(f.recolocar(&raiz, Some(&rotulos), None).is_err());
        assert!(f.recolocar(&raiz, Some(&tipo), Some(&rotulos)).is_err());
    }

    #[test]
    fn ida_y_vuelta_por_disco() {
        let dir = tempfile::tempdir().unwrap();
        let (f, _, tipo, _) = arbol();
        f.guardar(dir.path()).unwrap();
        let leidas = Folders::cargar(dir.path()).unwrap();
        assert_eq!(leidas.folders, f.folders);
        assert_eq!(leidas.ruta(&tipo), "Referencias / Tipografía");
        // Una biblioteca sin folders.json es una biblioteca sin carpetas, no un
        // error.
        let vacio = tempfile::tempdir().unwrap();
        assert!(Folders::cargar(vacio.path()).unwrap().folders.is_empty());
    }
}
