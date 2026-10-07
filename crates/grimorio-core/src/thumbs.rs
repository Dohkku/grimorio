//! Almacén de miniaturas de malla: un solo archivo append-only mapeado en memoria.
//!
//! Con 100.000 elementos, tener un archivo por miniatura significa 100.000
//! `open()` durante un scroll. Aquí las miniaturas viven concatenadas en
//! `thumbs/grid.pack`, la UI mapea el archivo una vez y el sistema operativo se
//! encarga del caché de disco.
//!
//! Cada blob lleva delante una cabecera que incluye el id del elemento, así que
//! el pack se puede recorrer solo y sirve para reconstruir el índice.

use crate::error::{Error, Result};
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"GRIMPK01";
/// Dónde guarda la cabecera hasta qué byte el pack está sellado: los ocho
/// bytes que siguen a la firma, que antes iban a cero. Ver `fin_valido`.
const SELLO_OFF: u64 = 8;
/// Marca de inicio de registro, para detectar truncados a medias.
const REC_MAGIC: u8 = 0xA5;
pub const HEADER_LEN: u64 = 16;
const REC_HEADER_LEN: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct ThumbRef {
    /// Desplazamiento de los *datos* (ya después de la cabecera del registro).
    pub offset: u64,
    pub len: u32,
}

pub struct PackWriter {
    out: BufWriter<File>,
    pos: u64,
    path: PathBuf,
    /// El cerrojo (`grid.pack.lock`): abierto mientras viva el escritor.
    _cerrojo: File,
}

impl PackWriter {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| Error::io(path, e))?;

        // Cerrojo exclusivo mientras dure la escritura. Dos procesos añadiendo
        // al mismo pack se pisan las posiciones y dejan la biblioteca con
        // miniaturas cruzadas; es de los pocos daños que `reindex` no arregla.
        // El cerrojo lo suelta el sistema al cerrarse el archivo.
        //
        // Va en un archivo al lado y no en el pack. En Windows los cerrojos
        // son obligatorios: bloquear el pack impediría a la interfaz leer las
        // miniaturas mapeadas mientras se importa. En Linux son de aviso y
        // daría igual, pero así los dos sistemas se portan igual.
        let ruta_cerrojo = path.with_extension("pack.lock");
        let cerrojo = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&ruta_cerrojo)
            .map_err(|e| Error::io(&ruta_cerrojo, e))?;
        if cerrojo.try_lock().is_err() {
            return Err(Error::Invalid(format!(
                "otro proceso está escribiendo en {}\n  \
                 cierra la otra instancia de Grimorio e inténtalo de nuevo",
                path.display()
            )));
        }
        let len = file.metadata().map_err(|e| Error::io(path, e))?.len();
        let pos = if len < HEADER_LEN {
            file.set_len(0).map_err(|e| Error::io(path, e))?;
            file.write_all(MAGIC).map_err(|e| Error::io(path, e))?;
            file.write_all(&[0u8; 8]).map_err(|e| Error::io(path, e))?;
            HEADER_LEN
        } else {
            let mut magic = [0u8; 8];
            file.seek(SeekFrom::Start(0))
                .map_err(|e| Error::io(path, e))?;
            file.read_exact(&mut magic)
                .map_err(|e| Error::io(path, e))?;
            if &magic != MAGIC {
                return Err(Error::Invalid(format!(
                    "{} no es un pack de miniaturas de Grimorio",
                    path.display()
                )));
            }
            // Si el proceso anterior murió a mitad de un registro, el final
            // del archivo es medio registro. Añadir detrás lo dejaba ahí en
            // medio: `scan` se para en el primer registro roto, así que todo
            // lo escrito después —miniaturas buenas— se volvía invisible
            // para reconstruir el índice. Se corta por el último registro
            // entero y se sigue desde ahí.
            let fin = fin_valido(&file, len, path)?;
            if fin < len {
                file.set_len(fin).map_err(|e| Error::io(path, e))?;
            }
            fin
        };
        file.seek(SeekFrom::Start(pos))
            .map_err(|e| Error::io(path, e))?;
        Ok(PackWriter {
            out: BufWriter::with_capacity(1 << 20, file),
            pos,
            path: path.to_path_buf(),
            _cerrojo: cerrojo,
        })
    }

    pub fn append(&mut self, id: &str, data: &[u8]) -> Result<ThumbRef> {
        let idb = id.as_bytes();
        if idb.len() > u16::MAX as usize {
            return Err(Error::Invalid("id demasiado largo".into()));
        }
        let mut head = [0u8; REC_HEADER_LEN];
        head[0] = REC_MAGIC;
        head[1] = 0; // flags: 0 = JPEG
        head[2..4].copy_from_slice(&(idb.len() as u16).to_le_bytes());
        head[4..8].copy_from_slice(&(data.len() as u32).to_le_bytes());
        self.out
            .write_all(&head)
            .map_err(|e| Error::io(&self.path, e))?;
        self.out
            .write_all(idb)
            .map_err(|e| Error::io(&self.path, e))?;
        self.out
            .write_all(data)
            .map_err(|e| Error::io(&self.path, e))?;
        let data_off = self.pos + REC_HEADER_LEN as u64 + idb.len() as u64;
        self.pos = data_off + data.len() as u64;
        Ok(ThumbRef {
            offset: data_off,
            len: data.len() as u32,
        })
    }

    /// Vacía el búfer y sella el pack hasta donde llega.
    ///
    /// El sello va **después** de los datos: si el proceso muere entre medias,
    /// el sello se queda corto, y al reabrir solo se repasa lo que hay detrás
    /// de él. Nunca puede apuntar más allá de lo escrito.
    pub fn flush(&mut self) -> Result<()> {
        self.out.flush().map_err(|e| Error::io(&self.path, e))?;
        let pos = self.pos;
        let f = self.out.get_mut();
        f.seek(SeekFrom::Start(SELLO_OFF))
            .and_then(|_| f.write_all(&pos.to_le_bytes()))
            .and_then(|_| f.seek(SeekFrom::Start(pos)))
            .map_err(|e| Error::io(&self.path, e))?;
        Ok(())
    }

    pub fn len(&self) -> u64 {
        self.pos
    }
}

impl Drop for PackWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Hasta qué byte llega el último registro entero del pack.
///
/// Recorrer el pack entero en cada apertura sería leer de disco una cabecera
/// por miniatura —cien mil con una biblioteca grande— cada vez que se importa
/// algo. Por eso `flush` deja en la cabecera hasta dónde está todo escrito, y
/// aquí solo se repasa lo que hay detrás: normalmente nada.
///
/// Si ese repaso no llega limpio al final —se cortó la luz a mitad de un
/// registro, o el sello no es de fiar porque el pack es de antes de haberlo—
/// se recorre desde el principio. Es lento, pero solo pasa después de un
/// accidente, y es la única forma de no cortar por un sitio equivocado.
fn fin_valido(file: &File, len: u64, path: &Path) -> Result<u64> {
    if len <= HEADER_LEN {
        return Ok(HEADER_LEN.min(len));
    }
    // SAFETY: tenemos el cerrojo exclusivo del pack; nadie más lo escribe
    // mientras se lee, y el mapa se suelta antes de tocar el archivo.
    let map = unsafe { Mmap::map(file).map_err(|e| Error::io(path, e))? };
    let mut sello = [0u8; 8];
    sello.copy_from_slice(&map[SELLO_OFF as usize..HEADER_LEN as usize]);
    let sello = u64::from_le_bytes(sello);
    let recorrer = |desde: u64| {
        let mut scan = PackScan {
            map: &map,
            pos: desde as usize,
        };
        while scan.next().is_some() {}
        scan.pos as u64
    };
    if (HEADER_LEN..=len).contains(&sello) && recorrer(sello) == len {
        return Ok(len);
    }
    Ok(recorrer(HEADER_LEN))
}

/// Lectura: el archivo entero mapeado, sin copias.
pub struct PackReader {
    /// Se guarda abierto solo para poder preguntar el tamaño de verdad del
    /// archivo. Ver `crecio`.
    file: File,
    map: Mmap,
}

impl PackReader {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| Error::io(path, e))?;
        // SAFETY: el pack es append-only; un lector puede ver menos bytes de los
        // que hay, nunca bytes reescritos bajo sus pies.
        let map = unsafe { Mmap::map(&file).map_err(|e| Error::io(path, e))? };
        Ok(PackReader { file, map })
    }

    /// Si el pack ha crecido desde que se mapeó.
    ///
    /// El mapa congela el tamaño del archivo en el momento de abrirlo, y esa es
    /// justo la mitad buena del trato: lo que ya está mapeado no se mueve bajo
    /// los pies de nadie. La otra mitad es que lo que se añade después queda
    /// fuera, y una miniatura recién importada se pide y no está. Quien
    /// reaprovecha un mapa tiene que preguntar esto antes; cuesta un `stat`, y
    /// solo se pregunta al rehacer una vista.
    pub fn crecio(&self) -> bool {
        self.file
            .metadata()
            .map(|m| m.len() as usize > self.map.len())
            .unwrap_or(false)
    }

    pub fn get(&self, r: ThumbRef) -> Option<&[u8]> {
        let start = r.offset as usize;
        let end = start.checked_add(r.len as usize)?;
        self.map.get(start..end)
    }

    pub fn size(&self) -> usize {
        self.map.len()
    }

    /// Recorre el pack de principio a fin. Es lo que permite reconstruir el
    /// índice de miniaturas sin la base de datos.
    pub fn scan(&self) -> PackScan<'_> {
        PackScan {
            map: &self.map,
            pos: HEADER_LEN as usize,
        }
    }
}

pub struct PackScan<'a> {
    map: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for PackScan<'a> {
    type Item = (String, ThumbRef);

    fn next(&mut self) -> Option<Self::Item> {
        let head = self.map.get(self.pos..self.pos + REC_HEADER_LEN)?;
        if head[0] != REC_MAGIC {
            return None;
        }
        let idlen = u16::from_le_bytes([head[2], head[3]]) as usize;
        let datalen = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let id_start = self.pos + REC_HEADER_LEN;
        let id = std::str::from_utf8(self.map.get(id_start..id_start + idlen)?).ok()?;
        let data_off = id_start + idlen;
        if data_off + datalen > self.map.len() {
            return None; // registro truncado: paramos limpiamente
        }
        self.pos = data_off + datalen;
        Some((
            id.to_string(),
            ThumbRef {
                offset: data_off as u64,
                len: datalen as u32,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escribe_y_lee() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        let (r1, r2);
        {
            let mut w = PackWriter::open(&path).unwrap();
            r1 = w.append("A".repeat(26).as_str(), b"primera").unwrap();
            r2 = w
                .append("B".repeat(26).as_str(), b"segunda-mas-larga")
                .unwrap();
            w.flush().unwrap();
        }
        let r = PackReader::open(&path).unwrap();
        assert_eq!(r.get(r1).unwrap(), b"primera");
        assert_eq!(r.get(r2).unwrap(), b"segunda-mas-larga");
    }

    #[test]
    fn el_pack_se_puede_recorrer_solo() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        {
            let mut w = PackWriter::open(&path).unwrap();
            for i in 0..10 {
                w.append(&format!("{:0>26}", i), format!("dato{i}").as_bytes())
                    .unwrap();
            }
            w.flush().unwrap();
        }
        let r = PackReader::open(&path).unwrap();
        let all: Vec<_> = r.scan().collect();
        assert_eq!(all.len(), 10);
        assert_eq!(r.get(all[3].1).unwrap(), b"dato3");
    }

    #[test]
    fn dos_escritores_a_la_vez_no() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        let _primero = PackWriter::open(&path).unwrap();
        let segundo = PackWriter::open(&path);
        assert!(
            segundo.is_err(),
            "el segundo escritor debería encontrarse el cerrojo puesto"
        );
        let msg = match segundo {
            Err(e) => e.to_string(),
            Ok(_) => unreachable!(),
        };
        assert!(msg.contains("otro proceso"), "mensaje poco claro: {msg}");
    }

    #[test]
    fn el_cerrojo_se_suelta_al_cerrar() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        {
            let mut w = PackWriter::open(&path).unwrap();
            w.append(&"A".repeat(26), b"uno").unwrap();
        }
        assert!(PackWriter::open(&path).is_ok());
    }

    #[test]
    fn reabrir_continua_al_final() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        {
            let mut w = PackWriter::open(&path).unwrap();
            w.append(&"A".repeat(26), b"uno").unwrap();
            w.flush().unwrap();
        }
        let r2 = {
            let mut w = PackWriter::open(&path).unwrap();
            let r = w.append(&"B".repeat(26), b"dos").unwrap();
            w.flush().unwrap();
            r
        };
        let r = PackReader::open(&path).unwrap();
        assert_eq!(r.get(r2).unwrap(), b"dos");
        assert_eq!(r.scan().count(), 2);
    }

    /// Un corte a mitad de escribir deja medio registro al final. El
    /// siguiente escritor tiene que cortarlo y seguir desde el último entero:
    /// si añadiera detrás, `scan` se pararía en el roto y no vería lo nuevo.
    #[test]
    fn un_registro_cortado_al_final_se_quita_al_reabrir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        {
            let mut w = PackWriter::open(&path).unwrap();
            w.append(&"A".repeat(26), b"primera-entera").unwrap();
            w.append(&"B".repeat(26), b"segunda-que-se-cortara").unwrap();
            w.flush().unwrap();
        }
        // Se corta la luz a mitad del segundo registro.
        let len = std::fs::metadata(&path).unwrap().len();
        let f = OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(len - 5).unwrap();
        drop(f);

        let (r3, r4) = {
            let mut w = PackWriter::open(&path).unwrap();
            let r3 = w.append(&"C".repeat(26), b"tercera").unwrap();
            let r4 = w.append(&"D".repeat(26), b"cuarta").unwrap();
            w.flush().unwrap();
            (r3, r4)
        };
        let r = PackReader::open(&path).unwrap();
        let ids: Vec<String> = r.scan().map(|(id, _)| id).collect();
        assert_eq!(ids, vec!["A".repeat(26), "C".repeat(26), "D".repeat(26)]);
        assert_eq!(r.get(r3).unwrap(), b"tercera");
        assert_eq!(r.get(r4).unwrap(), b"cuarta");
    }

    /// Lo mismo con un pack de antes del sello —cabecera a cero— y con un
    /// sello que miente: los dos se arreglan recorriendo desde el principio.
    #[test]
    fn sin_sello_o_con_sello_malo_se_recorre_entero() {
        for sello in [0u64, 7, 1 << 40] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("grid.pack");
            {
                let mut w = PackWriter::open(&path).unwrap();
                w.append(&"A".repeat(26), b"uno").unwrap();
                w.append(&"B".repeat(26), b"dos").unwrap();
            }
            let len = std::fs::metadata(&path).unwrap().len();
            {
                let mut f = OpenOptions::new().write(true).open(&path).unwrap();
                f.seek(SeekFrom::Start(SELLO_OFF)).unwrap();
                f.write_all(&sello.to_le_bytes()).unwrap();
                f.set_len(len - 1).unwrap();
            }
            {
                let mut w = PackWriter::open(&path).unwrap();
                w.append(&"C".repeat(26), b"tres").unwrap();
            }
            let r = PackReader::open(&path).unwrap();
            assert_eq!(r.scan().count(), 2, "sello {sello}: A y C, sin el B cortado");
        }
    }

    #[test]
    fn un_mapa_viejo_sabe_que_se_quedo_corto() {
        // Es el caso de importar con el programa abierto: alguien tiene el pack
        // mapeado y llegan miniaturas nuevas al final.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.pack");
        {
            let mut w = PackWriter::open(&path).unwrap();
            w.append(&"A".repeat(26), b"vieja").unwrap();
            w.flush().unwrap();
        }
        let antiguo = PackReader::open(&path).unwrap();
        assert!(!antiguo.crecio());

        let nueva_ref = {
            let mut w = PackWriter::open(&path).unwrap();
            let r = w.append(&"B".repeat(26), b"recien-llegada").unwrap();
            w.flush().unwrap();
            r
        };
        assert!(antiguo.crecio(), "el pack creció y el lector no se entera");
        assert!(
            antiguo.get(nueva_ref).is_none(),
            "el mapa viejo no puede tener bytes que se escribieron después"
        );

        let al_dia = PackReader::open(&path).unwrap();
        assert_eq!(al_dia.get(nueva_ref).unwrap(), b"recien-llegada");
    }
}
