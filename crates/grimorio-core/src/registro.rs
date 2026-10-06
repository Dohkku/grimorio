//! El registro de operaciones: lo que hace posible deshacer.
//!
//! Una regla del plan que aquí se cumple: **el deshacer se implementa una vez y
//! cubre toda operación**. No hay un deshacer de estrellas y otro de etiquetas;
//! hay una sola máquina que sabe volver a poner los campos como estaban.
//!
//! La pieza que lo consigue es [`Parche`]: aplicar un parche a un elemento
//! devuelve *el parche contrario*, con los valores de antes y solo de los campos
//! que de verdad cambiaron. Deshacer es aplicar lo contrario, y rehacer es
//! aplicar lo de ida. Ninguna operación tiene que escribir su propio inverso a
//! mano, que es donde estos sistemas se rompen en silencio.

use crate::folder::Folders;
use crate::item::Item;
use crate::time;
use serde::{Deserialize, Serialize};

/// Los campos de un elemento que una operación puede cambiar.
///
/// Un `None` es «esto no se toca», no «esto se pone a vacío». Esa distinción es
/// lo que deja que poner una estrella y quitar una etiqueta sean la misma
/// máquina sin pisarse la una a la otra.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Parche {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estrellas: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etiquetas: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carpetas: Option<Vec<String>>,
    /// La nota entera. Una cadena en blanco borra la nota: una nota hecha solo
    /// de espacios no es una nota.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nota: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nombre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub papelera: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adulto: Option<bool>,
    /// El mapa de orden entero (ver `Item::orden`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orden: Option<std::collections::BTreeMap<String, f64>>,
}

impl Parche {
    pub fn estrellas(v: u8) -> Self {
        Parche {
            estrellas: Some(v.min(5)),
            ..Default::default()
        }
    }
    pub fn etiquetas(v: Vec<String>) -> Self {
        Parche {
            etiquetas: Some(v),
            ..Default::default()
        }
    }
    pub fn carpetas(v: Vec<String>) -> Self {
        Parche {
            carpetas: Some(v),
            ..Default::default()
        }
    }
    pub fn nota(t: &str) -> Self {
        Parche {
            nota: Some(t.to_string()),
            ..Default::default()
        }
    }
    pub fn nombre(n: &str) -> Self {
        Parche {
            nombre: Some(n.to_string()),
            ..Default::default()
        }
    }
    pub fn papelera(v: bool) -> Self {
        Parche {
            papelera: Some(v),
            ..Default::default()
        }
    }
    pub fn adulto(v: bool) -> Self {
        Parche {
            adulto: Some(v),
            ..Default::default()
        }
    }
    pub fn orden(v: std::collections::BTreeMap<String, f64>) -> Self {
        Parche {
            orden: Some(v),
            ..Default::default()
        }
    }

    /// Toma de `item` los mismos campos que menciona `molde`.
    ///
    /// Es lo que convierte «lo que se pidió» en «lo que de verdad pasó»:
    /// rehacer necesita el parche efectivo, porque el que se pidió puede
    /// mencionar campos que ya estaban como se pedían.
    pub fn tomar(item: &Item, molde: &Parche) -> Parche {
        Parche {
            estrellas: molde.estrellas.map(|_| item.stars),
            etiquetas: molde.etiquetas.as_ref().map(|_| item.tags.clone()),
            carpetas: molde.carpetas.as_ref().map(|_| item.folders.clone()),
            nota: molde
                .nota
                .as_ref()
                .map(|_| item.note.clone().unwrap_or_default()),
            nombre: molde.nombre.as_ref().map(|_| item.name.clone()),
            papelera: molde.papelera.map(|_| item.trashed),
            adulto: molde.adulto.map(|_| item.adult),
            orden: molde.orden.as_ref().map(|_| item.orden.clone()),
        }
    }

    pub fn vacio(&self) -> bool {
        *self == Parche::default()
    }

    /// Aplica el parche y devuelve **el contrario**: los valores de antes, y
    /// solo de los campos que han cambiado de verdad.
    ///
    /// Que el inverso salga de aquí y no de quien llama es lo que hace que
    /// deshacer no pueda quedarse desincronizado de la operación de ida.
    pub fn aplicar(&self, item: &mut Item) -> Parche {
        let mut atras = Parche::default();

        if let Some(v) = self.estrellas {
            let v = v.min(5);
            if item.stars != v {
                atras.estrellas = Some(item.stars);
                item.stars = v;
            }
        }
        if let Some(v) = &self.etiquetas {
            let limpias = limpiar_lista(v);
            if item.tags != limpias {
                atras.etiquetas = Some(item.tags.clone());
                item.tags = limpias;
            }
        }
        if let Some(v) = &self.carpetas {
            let limpias = limpiar_lista(v);
            if item.folders != limpias {
                atras.carpetas = Some(item.folders.clone());
                item.folders = limpias;
            }
        }
        if let Some(t) = &self.nota {
            let nueva = if t.trim().is_empty() {
                None
            } else {
                Some(t.trim().to_string())
            };
            if item.note != nueva {
                // El contrario de «sin nota» es la cadena vacía, no un `None`:
                // así el inverso de borrar una nota vuelve a borrarla.
                atras.nota = Some(item.note.clone().unwrap_or_default());
                item.note = nueva;
            }
        }
        if let Some(n) = &self.nombre {
            let nuevo = n.trim();
            if !nuevo.is_empty() && item.name != nuevo {
                atras.nombre = Some(item.name.clone());
                item.name = nuevo.to_string();
            }
        }
        if let Some(v) = self.papelera {
            if item.trashed != v {
                atras.papelera = Some(item.trashed);
                item.trashed = v;
            }
        }
        if let Some(v) = self.adulto {
            if item.adult != v {
                atras.adulto = Some(item.adult);
                item.adult = v;
            }
        }

        if !atras.vacio() {
            item.touch();
        }
        if let Some(v) = &self.orden {
            if item.orden != *v {
                atras.orden = Some(item.orden.clone());
                item.orden = v.clone();
            }
        }
        atras
    }
}

/// Quita vacíos y repetidos conservando el orden en que se escribieron.
///
/// El orden importa para que el inverso sea exacto: si al deshacer se
/// devolviera la misma lista ordenada de otra forma, el `item.json` cambiaría
/// sin que nada haya cambiado.
fn limpiar_lista(v: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(v.len());
    for s in v {
        let s = s.trim();
        if !s.is_empty() && !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    }
    out
}

/// Un cambio completo, con lo necesario para volver atrás y para repetirlo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operacion {
    /// Lo que se le enseña a la persona: «Deshacer poner 4 estrellas».
    pub titulo: String,
    pub cuando: String,
    /// Qué había antes en cada elemento tocado. Deshacer es aplicar esto.
    pub antes: Vec<(String, Parche)>,
    /// Qué se puso. Rehacer es aplicar esto.
    pub despues: Vec<(String, Parche)>,
    /// El árbol de carpetas antes y después, entero.
    ///
    /// Entero y no en diferencias porque son decenas de filas, y porque el
    /// inverso de «borrar una carpeta con tres hijas» escrito a mano es
    /// exactamente la clase de código que falla el día que nadie mira.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carpetas_antes: Option<Folders>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carpetas_despues: Option<Folders>,
}

impl Operacion {
    pub fn nueva(titulo: impl Into<String>) -> Self {
        Operacion {
            titulo: titulo.into(),
            cuando: time::to_rfc3339(time::now_ms()),
            antes: Vec::new(),
            despues: Vec::new(),
            carpetas_antes: None,
            carpetas_despues: None,
        }
    }

    pub fn vacia(&self) -> bool {
        self.antes.is_empty() && self.carpetas_antes.is_none()
    }

    pub fn tocados(&self) -> usize {
        self.antes.len()
    }
}

/// La pila de deshacer y la de rehacer.
///
/// Vive en memoria y muere con el programa: un deshacer que sobrevive al cierre
/// promete más de lo que puede cumplir, porque entre una sesión y otra el
/// archivo pudo cambiar por fuera.
#[derive(Debug)]
pub struct Registro {
    hechas: Vec<Operacion>,
    deshechas: Vec<Operacion>,
    tope_ops: usize,
    tope_parches: usize,
}

impl Default for Registro {
    fn default() -> Self {
        // Cien pasos son muchos más de los que nadie deshace seguidos, y el
        // tope de parches es el que de verdad manda: etiquetar diez mil
        // elementos son diez mil parches, y sin este número la memoria del
        // programa la decidiría el usuario sin saberlo.
        Registro {
            hechas: Vec::new(),
            deshechas: Vec::new(),
            tope_ops: 100,
            tope_parches: 250_000,
        }
    }
}

impl Registro {
    pub fn empujar(&mut self, op: Operacion) {
        if op.vacia() {
            return;
        }
        // Hacer algo nuevo después de deshacer parte la historia en dos: lo
        // deshecho ya no se puede rehacer porque el mundo cambió por debajo.
        self.deshechas.clear();
        self.hechas.push(op);
        self.recortar();
    }

    fn recortar(&mut self) {
        while self.hechas.len() > self.tope_ops {
            self.hechas.remove(0);
        }
        let mut total: usize = self.hechas.iter().map(|o| o.tocados()).sum();
        while total > self.tope_parches && self.hechas.len() > 1 {
            total -= self.hechas.remove(0).tocados();
        }
    }

    pub fn ultima(&self) -> Option<&Operacion> {
        self.hechas.last()
    }

    pub fn ultima_deshecha(&self) -> Option<&Operacion> {
        self.deshechas.last()
    }

    /// Mueve la cima de «hechas» a «deshechas». Se llama **después** de aplicar
    /// el inverso: si aplicarlo falla, la pila se queda como estaba y el botón
    /// sigue ofreciendo lo mismo.
    pub fn marcar_deshecha(&mut self) {
        if let Some(op) = self.hechas.pop() {
            self.deshechas.push(op);
        }
    }

    pub fn marcar_rehecha(&mut self) {
        if let Some(op) = self.deshechas.pop() {
            self.hechas.push(op);
        }
    }

    /// Tira toda la historia. Lo llama lo irreversible: si vaciar la papelera
    /// borró los archivos, ofrecer «deshacer» sería mentir.
    pub fn limpiar(&mut self) {
        self.hechas.clear();
        self.deshechas.clear();
    }

    pub fn cuantas(&self) -> (usize, usize) {
        (self.hechas.len(), self.deshechas.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::OriginMode;

    fn item() -> Item {
        Item::new(
            "01JKX7Q2M8V3N4P5R6S7T8XYZ0".into(),
            "gato.jpg".into(),
            "jpg".into(),
            10,
            OriginMode::Copy,
        )
    }

    #[test]
    fn aplicar_devuelve_el_camino_de_vuelta() {
        let mut it = item();
        it.stars = 2;
        let atras = Parche::estrellas(5).aplicar(&mut it);
        assert_eq!(it.stars, 5);
        assert_eq!(atras.estrellas, Some(2));
        atras.aplicar(&mut it);
        assert_eq!(it.stars, 2, "deshacer deja el elemento como estaba");
    }

    #[test]
    fn un_campo_que_no_cambia_no_entra_en_el_inverso() {
        let mut it = item();
        it.stars = 3;
        let atras = Parche::estrellas(3).aplicar(&mut it);
        assert!(atras.vacio(), "poner lo que ya había no es un cambio");
        assert_eq!(it.modified_at_ms(), it.imported_at_ms(), "ni toca la fecha");
    }

    #[test]
    fn los_campos_que_el_parche_no_menciona_se_quedan_quietos() {
        let mut it = item();
        it.tags = vec!["felino".into()];
        it.note = Some("para el moodboard".into());
        Parche::estrellas(4).aplicar(&mut it);
        assert_eq!(it.tags, vec!["felino".to_string()]);
        assert_eq!(it.note.as_deref(), Some("para el moodboard"));
    }

    #[test]
    fn la_nota_en_blanco_borra_y_el_inverso_la_devuelve() {
        let mut it = item();
        it.note = Some("algo".into());
        let atras = Parche::nota("   ").aplicar(&mut it);
        assert!(it.note.is_none());
        atras.aplicar(&mut it);
        assert_eq!(it.note.as_deref(), Some("algo"));
    }

    #[test]
    fn el_inverso_de_poner_nota_donde_no_habia_es_quitarla() {
        let mut it = item();
        let atras = Parche::nota("apunte").aplicar(&mut it);
        assert_eq!(atras.nota.as_deref(), Some(""));
        atras.aplicar(&mut it);
        assert!(
            it.note.is_none(),
            "vuelve a no tener nota, no a tener una vacía"
        );
    }

    #[test]
    fn las_etiquetas_se_limpian_pero_conservan_su_orden() {
        let mut it = item();
        Parche::etiquetas(vec!["  b ".into(), "a".into(), "b".into(), "".into()]).aplicar(&mut it);
        assert_eq!(it.tags, vec!["b".to_string(), "a".to_string()]);
    }

    #[test]
    fn un_nombre_en_blanco_no_deja_el_elemento_sin_nombre() {
        let mut it = item();
        let atras = Parche::nombre("   ").aplicar(&mut it);
        assert!(atras.vacio());
        assert_eq!(it.name, "gato.jpg");
    }

    #[test]
    fn hacer_algo_nuevo_despues_de_deshacer_corta_el_rehacer() {
        let mut r = Registro::default();
        let mut op = Operacion::nueva("uno");
        op.antes.push(("x".into(), Parche::estrellas(1)));
        r.empujar(op.clone());
        r.marcar_deshecha();
        assert_eq!(r.cuantas(), (0, 1));
        r.empujar(op);
        assert_eq!(r.cuantas(), (1, 0), "lo deshecho ya no se puede rehacer");
    }

    #[test]
    fn una_operacion_que_no_cambio_nada_no_entra_en_la_pila() {
        let mut r = Registro::default();
        r.empujar(Operacion::nueva("nada"));
        assert_eq!(r.cuantas(), (0, 0));
    }

    #[test]
    fn la_pila_tiene_tope_y_lo_respeta() {
        let mut r = Registro::default();
        r.tope_ops = 3;
        for i in 0..10 {
            let mut op = Operacion::nueva(format!("op {i}"));
            op.antes.push(("x".into(), Parche::estrellas(1)));
            r.empujar(op);
        }
        assert_eq!(r.cuantas().0, 3);
        assert_eq!(
            r.ultima().unwrap().titulo,
            "op 9",
            "se tira lo viejo, no lo nuevo"
        );
    }

    #[test]
    fn el_tope_cuenta_parches_y_no_solo_operaciones() {
        let mut r = Registro::default();
        r.tope_parches = 100;
        for i in 0..5 {
            let mut op = Operacion::nueva(format!("op {i}"));
            for k in 0..60 {
                op.antes.push((format!("{i}-{k}"), Parche::estrellas(1)));
            }
            r.empujar(op);
        }
        let total: usize = r.hechas.iter().map(|o| o.tocados()).sum();
        assert!(total <= 120, "no se guardan cinco lotes enteros: {total}");
        assert!(!r.hechas.is_empty(), "pero nunca se queda sin el último");
    }
}
