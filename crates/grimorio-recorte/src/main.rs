//! `grim-recorte --modelo MODELO.onnx ENTRADA SALIDA.png`
//!
//! Quita el fondo de ENTRADA y escribe SALIDA como PNG con transparencia. Es un
//! programa aparte para que Grimorio lo llame como llama a ffmpeg: con tope de
//! tiempo, en su propio proceso, y sin que un modelo que se atasque pueda
//! llevarse la ventana por delante. Sale con 0 si todo fue bien y con 1 y una
//! línea de error si no.

use std::path::PathBuf;
use std::time::Instant;

fn main() {
    if let Err(e) = correr() {
        eprintln!("grim-recorte: {e}");
        std::process::exit(1);
    }
}

fn correr() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut modelo: Option<PathBuf> = None;
    let mut sueltos: Vec<PathBuf> = Vec::new();
    while let Some(a) = args.next() {
        if a == "--modelo" {
            modelo = args.next().map(PathBuf::from);
        } else if a == "--version" {
            println!("grim-recorte {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        } else {
            sueltos.push(PathBuf::from(a));
        }
    }
    let (Some(modelo), [entrada, salida]) = (modelo, sueltos.as_slice()) else {
        return Err("uso: grim-recorte --modelo MODELO.onnx ENTRADA SALIDA.png".into());
    };

    let t = Instant::now();
    let recortador = grimorio_recorte::Recortador::cargar(&modelo)?;
    let carga = t.elapsed();
    let img = image::open(entrada)?;
    let t = Instant::now();
    let recortada = recortador.recortar(&img)?;
    let calculo = t.elapsed();
    recortada.save_with_format(salida, image::ImageFormat::Png)?;
    eprintln!(
        "modelo cargado en {:.1} s, recorte en {:.1} s",
        carga.as_secs_f32(),
        calculo.as_secs_f32()
    );
    Ok(())
}
