//! En qué idioma habla lo que el núcleo le dice a una persona: los avisos de
//! «hecho», lo que se deshace con Ctrl+Z, los errores que llegan a la pantalla.
//!
//! Un interruptor global y no un parámetro en cada llamada: el idioma lo elige
//! quien está delante al abrir el programa y no cambia mientras corre. El
//! español es el de siempre; el inglés se enciende desde la interfaz
//! (`grim_idioma` en el puente).

use std::sync::atomic::{AtomicBool, Ordering};

static INGLES: AtomicBool = AtomicBool::new(false);

pub fn poner_ingles(si: bool) {
    INGLES.store(si, Ordering::Relaxed);
}

pub fn ingles() -> bool {
    INGLES.load(Ordering::Relaxed)
}

/// El texto en el idioma puesto: `t("hecho", "done")`.
pub fn t<'a>(es: &'a str, en: &'a str) -> &'a str {
    if ingles() {
        en
    } else {
        es
    }
}
