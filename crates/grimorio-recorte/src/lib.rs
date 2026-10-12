//! Quitar el fondo de una imagen con un modelo de segmentación.
//!
//! Vive en un crate y un binario propios (`grim-recorte`), que el resto llama
//! como a ffmpeg o a Blender: en su propio proceso, con tope de tiempo. Un
//! modelo que se atasca o se queda sin memoria tira ese proceso y no la
//! ventana, y el núcleo no carga nada de esto. El modelo viene con el
//! programa, en `modelos/` (ver herramientas/modelo/preparar.py).
//!
//! El modelo es ISNet (`isnet-general-use`, Apache-2.0): segmentación del
//! objeto principal. No sabe qué es «una persona» o «una taza»; sabe qué es lo
//! que la foto está enseñando, y eso es lo que se queda. En un retrato, la
//! persona; en una foto de producto, el producto.
//!
//! Se ejecuta con `tract`, en Rust puro: sin onnxruntime ni nada de C que
//! haya que empaquetar para cada sistema.

use image::{DynamicImage, GenericImageView, ImageBuffer, Luma, RgbaImage};
use std::path::Path;
use tract_onnx::prelude::*;

/// El lado de la entrada que espera el modelo. ISNet trabaja a 1024×1024 y
/// la máscara se vuelve a llevar al tamaño de la foto.
pub const LADO: usize = 1024;

/// La media que se resta a cada canal, la misma con la que se entrenó. La
/// desviación es 1: ISNet no divide.
const MEDIA: [f32; 3] = [0.485, 0.456, 0.406];

pub struct Recortador {
    plan: std::sync::Arc<TypedRunnableModel>,
}

impl Recortador {
    /// Carga y optimiza el modelo. Tarda unos segundos: se hace una vez por
    /// proceso, no una por imagen.
    pub fn cargar(modelo: &Path) -> TractResult<Recortador> {
        // Todos los núcleos: es un programa que se lanza para esto y acaba.
        // Con uno solo, una foto tardaba seis segundos.
        let hilos = std::thread::available_parallelism().map_or(4, |n| n.get());
        tract_linalg::multithread::set_default_executor(
            tract_linalg::multithread::Executor::multithread(hilos),
        );
        let plan = tract_onnx::onnx()
            .model_for_path(modelo)?
            .with_input_fact(0, f32::fact([1, 3, LADO, LADO]).into())?
            .into_optimized()?
            .into_runnable()?;
        Ok(Recortador { plan })
    }

    /// La máscara del sujeto, del tamaño de la imagen: 255 es sujeto, 0 fondo.
    pub fn mascara(&self, img: &DynamicImage) -> TractResult<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let (w, h) = img.dimensions();
        let rgb = img
            .resize_exact(LADO as u32, LADO as u32, image::imageops::FilterType::Lanczos3)
            .to_rgb8();
        // Se divide por el valor más alto de la imagen y no por 255: es como se
        // preparaban las fotos al entrenar, y una foto oscura sale distinta si
        // se hace de otra forma.
        let tope = rgb.as_raw().iter().copied().max().unwrap_or(255).max(1) as f32;
        let entrada: Tensor =
            tract_ndarray::Array4::from_shape_fn((1, 3, LADO, LADO), |(_, c, y, x)| {
                rgb.get_pixel(x as u32, y as u32)[c] as f32 / tope - MEDIA[c]
            })
            .into();
        let salida = self.plan.run(tvec!(entrada.into()))?;
        let pred = salida[0].to_plain_array_view::<f32>()?;

        // La salida no va de 0 a 1 exactos: se estira entre su mínimo y su
        // máximo, que es lo que la deja como una máscara limpia.
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for v in pred.iter() {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
        let rango = (hi - lo).max(1e-6);
        let pequena = ImageBuffer::from_fn(LADO as u32, LADO as u32, |x, y| {
            let v = (pred[[0, 0, y as usize, x as usize]] - lo) / rango;
            Luma([(v * 255.0).round().clamp(0.0, 255.0) as u8])
        });
        Ok(image::imageops::resize(&pequena, w, h, image::imageops::FilterType::Lanczos3))
    }

    /// La imagen con el fondo transparente.
    pub fn recortar(&self, img: &DynamicImage) -> TractResult<RgbaImage> {
        let mascara = self.mascara(img)?;
        let mut rgba = img.to_rgba8();
        for (p, m) in rgba.pixels_mut().zip(mascara.pixels()) {
            // Lo que ya era transparente sigue siéndolo: la máscara solo quita.
            p[3] = ((p[3] as u16 * m[0] as u16) / 255) as u8;
        }
        Ok(rgba)
    }
}
