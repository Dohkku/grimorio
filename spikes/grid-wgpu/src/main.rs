//! Spike de la malla: ¿aguanta el pipeline mmap → decodificar → atlas → dibujar
//! una biblioteca de 100.000 miniaturas sin perder fotogramas?
//!
//! No es la interfaz de Grimorio. Es el banco de pruebas del único subsistema
//! que decide si la aplicación se siente bien, medido con datos reales.
//!
//!   grid-spike --lib RUTA                 ventana interactiva
//!   grid-spike --lib RUTA --bench 12      recorrido automático de 12 s
//!   grid-spike --lib RUTA --captura a.png una imagen sin ventana

mod atlas;
mod layout;
mod tema;
mod workers;

use atlas::Atlas;
use grimorio_core::{thumbs::ThumbRef, Library, Query, QueryHit, SortBy};
use layout::{Celda, Disposicion, Modo};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tema::Tema;
use workers::Pool;

// ---------------------------------------------------------------- argumentos

struct Args {
    lib: PathBuf,
    bench: Option<f32>,
    captura: Option<PathBuf>,
    ancho: u32,
    alto: u32,
    modo: Modo,
    objetivo: f32,
    limite: usize,
    vsync: bool,
    tema: Option<PathBuf>,
    /// Diagnóstico: decodifica N miniaturas y sale, sin ventana ni GPU.
    solo_decodificar: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        lib: std::env::var_os("GRIMORIO_LIB")
            .map(PathBuf::from)
            .unwrap_or_default(),
        bench: None,
        captura: None,
        ancho: 1600,
        alto: 1000,
        modo: Modo::Justificado,
        objetivo: 200.0,
        limite: 100_000,
        vsync: true,
        tema: None,
        solo_decodificar: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut siguiente = || it.next().ok_or(format!("falta el valor de {arg}"));
        match arg.as_str() {
            "--lib" | "-L" => a.lib = PathBuf::from(siguiente()?),
            "--bench" => a.bench = Some(siguiente()?.parse().map_err(|_| "segundos inválidos")?),
            "--captura" | "--screenshot" => a.captura = Some(PathBuf::from(siguiente()?)),
            "--ancho" => a.ancho = siguiente()?.parse().map_err(|_| "ancho inválido")?,
            "--alto" => a.alto = siguiente()?.parse().map_err(|_| "alto inválido")?,
            "--celda" => a.objetivo = siguiente()?.parse().map_err(|_| "celda inválida")?,
            "--limite" => a.limite = siguiente()?.parse().map_err(|_| "límite inválido")?,
            "--tema" => a.tema = Some(PathBuf::from(siguiente()?)),
            "--cuadricula" => a.modo = Modo::Cuadricula,
            "--justificado" => a.modo = Modo::Justificado,
            "--sin-vsync" => a.vsync = false,
            "--solo-decodificar" => {
                a.solo_decodificar = Some(siguiente()?.parse().map_err(|_| "número inválido")?)
            }
            "--ayuda" | "-h" | "--help" => {
                println!("{}", AYUDA);
                std::process::exit(0);
            }
            otro => return Err(format!("opción desconocida: {otro}")),
        }
    }
    if a.lib.as_os_str().is_empty() {
        return Err("falta --lib RUTA (o la variable GRIMORIO_LIB)".into());
    }
    Ok(a)
}

const AYUDA: &str = "\
spike de malla de Grimorio

  --lib RUTA          biblioteca a abrir (o GRIMORIO_LIB)
  --bench SEGUNDOS    recorrido automático y estadísticas de fotograma
  --captura ARCHIVO   render de un fotograma a PNG, sin ventana
  --ancho/--alto N    tamaño de la ventana (por defecto 1600×1000)
  --celda N           lado objetivo de la celda en píxeles
  --limite N          cuántos elementos cargar
  --cuadricula        celdas cuadradas recortadas
  --justificado       filas justificadas conservando proporción (por defecto)
  --sin-vsync         no esperar al monitor: mide el techo real
  --tema ARCHIVO      tema en JSON
  --solo-decodificar N  diagnóstico: decodifica N miniaturas y sale

teclado: flechas/AvPág/RePág/Inicio/Fin  ·  +/-  zoom  ·  L  cambia disposición";

// -------------------------------------------------------------------- escena

struct Escena {
    hits: Vec<QueryHit>,
    pool: Pool,
    disp: Disposicion,
    scroll: f32,
    /// Adónde va el scroll. La posición real lo persigue con una exponencial:
    /// una rueda de ratón da saltos de 90 px y sin suavizado la malla parpadea
    /// en vez de moverse.
    scroll_objetivo: f32,
    raton: (f32, f32),
    hover: Option<u32>,
    seleccion: Option<u32>,
    visibles: Vec<Celda>,
    aparicion: std::collections::HashMap<u32, f32>,
    /// Elemento → fotograma en que se pidió. No es un conjunto: una petición
    /// cancelada por el scroll nunca llega, y si esto fuese un `HashSet` ese
    /// elemento no se volvería a pedir jamás y dejaría un hueco permanente.
    en_vuelo: std::collections::HashMap<u32, u64>,
    fotograma: u64,
    tema: Tema,
    ancho: f32,
    alto: f32,
}

impl Escena {
    fn nueva(args: &Args, tema: Tema) -> Result<Escena, String> {
        let t0 = Instant::now();
        let lib = Library::open(&args.lib).map_err(|e| e.to_string())?;
        let q = Query {
            sort: SortBy::ImportedDesc,
            limit: args.limite,
            ..Default::default()
        };
        let hits = lib.index().search(&q).map_err(|e| e.to_string())?;
        let consulta = t0.elapsed();

        let t1 = Instant::now();
        let pack = Arc::new(lib.pack_reader().map_err(|e| e.to_string())?);
        let refs: Arc<Vec<Option<ThumbRef>>> = Arc::new(
            hits.iter()
                .map(|h| match (h.thumb_off, h.thumb_len) {
                    (Some(o), Some(l)) => Some(ThumbRef { offset: o, len: l }),
                    _ => None,
                })
                .collect(),
        );
        let mapeo = t1.elapsed();

        let hilos = std::thread::available_parallelism()
            .map(|n| (n.get().saturating_sub(2)).max(2))
            .unwrap_or(4);
        let pool = Pool::nuevo(pack, Arc::clone(&refs), hilos);

        println!(
            "  biblioteca: {} elementos · consulta {:.1} ms · mmap {:.1} ms · {hilos} hilos de decodificación",
            hits.len(),
            consulta.as_secs_f64() * 1000.0,
            mapeo.as_secs_f64() * 1000.0
        );

        Ok(Escena {
            hits,
            pool,
            disp: Disposicion::nueva(args.modo, args.objetivo, tema.margen, tema.hueco),
            scroll: 0.0,
            scroll_objetivo: 0.0,
            raton: (-1.0, -1.0),
            hover: None,
            seleccion: None,
            visibles: Vec::with_capacity(512),
            aparicion: std::collections::HashMap::new(),
            en_vuelo: std::collections::HashMap::new(),
            fotograma: 0,
            tema,
            ancho: args.ancho as f32,
            alto: args.alto as f32,
        })
    }

    fn redimensionar(&mut self, ancho: f32, alto: f32) {
        self.ancho = ancho;
        self.alto = alto;
        self.disp.set_ancho(ancho, &self.hits);
        self.limitar_scroll();
    }

    fn limitar_scroll(&mut self) {
        let max = (self.disp.alto_total() - self.alto).max(0.0);
        self.scroll_objetivo = self.scroll_objetivo.clamp(0.0, max);
        self.scroll = self.scroll.clamp(0.0, max);
    }

    /// Pide moverse. La posición real llega detrás, suavizada.
    fn desplazar(&mut self, dy: f32) {
        self.scroll_objetivo += dy;
        self.limitar_scroll();
    }

    /// Salta sin suavizar. Lo usa el banco de pruebas, que mueve la malla a
    /// velocidad constante y no quiere que el suavizado le mienta.
    fn desplazar_seco(&mut self, dy: f32) {
        self.scroll_objetivo += dy;
        self.scroll = self.scroll_objetivo;
        self.limitar_scroll();
    }

    fn avanzar_scroll(&mut self, dt: f32) {
        let d = self.scroll_objetivo - self.scroll;
        if d.abs() < 0.35 {
            self.scroll = self.scroll_objetivo;
            return;
        }
        // Se nota el arrastre pero no va por detrás del dedo. Independiente de
        // los fps, gracias al exp(-dt/tau).
        let k = 1.0 - (-dt / self.tema.scroll_tau.max(0.001)).exp();
        self.scroll += d * k;
    }

    /// Qué celda hay bajo un punto de la ventana.
    fn celda_en(&self, x: f32, y: f32) -> Option<u32> {
        self.visibles
            .iter()
            .find(|c| x >= c.x && x <= c.x + c.w && y >= c.y && y <= c.y + c.h)
            .map(|c| c.indice)
    }

    fn zoom(&mut self, delta: f32) {
        let centro = self.scroll + self.alto * 0.5;
        let antes = self.disp.alto_total().max(1.0);
        self.disp
            .set_objetivo(self.disp.objetivo() + delta, &self.hits);
        let despues = self.disp.alto_total().max(1.0);
        // Mantener el punto que se estaba mirando: sin esto, el zoom te
        // teletransporta a otra parte de la biblioteca.
        self.scroll_objetivo = centro * (despues / antes) - self.alto * 0.5;
        self.scroll = self.scroll_objetivo;
        self.limitar_scroll();
    }

    fn cambiar_modo(&mut self) {
        let m = self.disp.modo().alternar();
        self.disp.set_modo(m, &self.hits);
        self.limitar_scroll();
    }

    /// Actualiza el estado del fotograma y devuelve las celdas a dibujar.
    fn preparar(&mut self, dt: f32, atlas: &mut Atlas, queue: &wgpu::Queue) {
        self.fotograma += 1;
        self.avanzar_scroll(dt);
        let margen = self.alto * 0.75; // una pantalla y media de precarga
        self.disp.visibles(
            &self.hits,
            self.scroll,
            self.alto,
            margen,
            &mut self.visibles,
        );

        // Recoger lo decodificado. El tope por fotograma evita que una avalancha
        // de subidas a GPU se coma el presupuesto de tiempo.
        for d in self.pool.cosechar(self.tema.subidas_por_fotograma) {
            self.en_vuelo.remove(&d.indice);
            atlas.insertar(queue, d.indice, d.w, d.h, &d.rgba, self.fotograma);
            self.aparicion.insert(d.indice, 0.0);
        }

        // Lista de deseos, ordenada por cercanía al centro de la pantalla.
        let centro = self.alto * 0.5;
        let mut faltan: Vec<(i64, u32)> = Vec::new();
        // Una petición se da por perdida a los 30 fotogramas: o llegó, o el
        // scroll la canceló. Así el hueco se vuelve a pedir en vez de quedarse.
        let caducidad = self.fotograma.saturating_sub(30);
        self.en_vuelo.retain(|_, f| *f > caducidad);

        for c in &self.visibles {
            let idx = c.indice;
            if atlas.obtener(idx, self.fotograma).is_some() || self.en_vuelo.contains_key(&idx) {
                continue;
            }
            let d = ((c.y + c.h * 0.5) - centro).abs() as i64;
            faltan.push((d, idx));
        }
        faltan.sort_unstable();
        // Pedir mucho más de lo que se puede consumir por fotograma solo genera
        // trabajo que se tira: el tope va acorde al ritmo de subida a GPU.
        faltan.truncate(self.tema.subidas_por_fotograma * 4);
        for (_, i) in &faltan {
            self.en_vuelo.insert(*i, self.fotograma);
        }
        self.pool.pedir(faltan.into_iter().map(|(_, i)| i));

        self.hover = if self.raton.0 >= 0.0 {
            self.celda_en(self.raton.0, self.raton.1)
        } else {
            None
        };

        // Aparición suave: la miniatura entra en ~120 ms en vez de aparecer
        // de golpe encima del color dominante.
        let paso = dt / self.tema.aparicion_s;
        self.aparicion.retain(|_, v| {
            *v = (*v + paso).min(1.0);
            *v < 1.0
        });
    }

    fn instancias(&mut self, atlas: &mut Atlas) -> Vec<Instancia> {
        let mut out = Vec::with_capacity(self.visibles.len());
        let f = self.fotograma;
        for c in &self.visibles {
            if c.y + c.h < 0.0 || c.y > self.alto {
                continue; // precargado pero fuera de pantalla: no se dibuja
            }
            let hit = &self.hits[c.indice as usize];
            let marcador = self.tema.color(&self.tema.marcador);
            let (uv, capa, tiene) = match atlas.obtener(c.indice, f) {
                Some(r) => {
                    let (uv, capa) = atlas.uv(&r);
                    // Recorte centrado: la imagen llena la celda sin deformarse.
                    // En filas justificadas la proporción ya coincide y esto no
                    // hace nada; en cuadrícula es la diferencia entre una malla
                    // digna y una de caras aplastadas.
                    (
                        recortar(uv, r.w as f32 / r.h as f32, c.w / c.h),
                        capa as f32,
                        1.0f32,
                    )
                }
                None => ([0.0, 0.0, 0.0, 0.0], 0.0, 0.0),
            };
            let ap = *self.aparicion.get(&c.indice).unwrap_or(&1.0);
            let señalada = self.hover == Some(c.indice);
            let elegida = self.seleccion == Some(c.indice);
            // La celda señalada crece dos píxeles hacia fuera: se nota que
            // responde sin necesidad de animar nada.
            let crece = if señalada { self.tema.realce_px } else { 0.0 };
            out.push(Instancia {
                rect: [
                    c.x - crece,
                    c.y - crece,
                    c.w + crece * 2.0,
                    c.h + crece * 2.0,
                ],
                uv,
                tint: match hit.dominant {
                    Some(d) => [
                        srgb_lineal(d[0]),
                        srgb_lineal(d[1]),
                        srgb_lineal(d[2]),
                        tiene,
                    ],
                    None => [marcador[0], marcador[1], marcador[2], tiene],
                },
                meta: [
                    capa,
                    if elegida { 1.0 } else { 0.0 },
                    ap,
                    if señalada { 1.0 } else { 0.0 },
                ],
            });
        }
        out
    }
}

/// Ajusta las coordenadas de textura para que la imagen llene la celda
/// recortando por el centro, en vez de deformarse.
fn recortar(uv: [f32; 4], prop_imagen: f32, prop_celda: f32) -> [f32; 4] {
    let (u0, v0, u1, v1) = (uv[0], uv[1], uv[2], uv[3]);
    if !prop_imagen.is_finite() || !prop_celda.is_finite() || prop_imagen <= 0.0 {
        return uv;
    }
    if (prop_imagen - prop_celda).abs() < 0.001 {
        return uv;
    }
    if prop_imagen > prop_celda {
        // Imagen más ancha que la celda: se recortan los lados.
        let quedarse = prop_celda / prop_imagen;
        let sobra = (u1 - u0) * (1.0 - quedarse) * 0.5;
        [u0 + sobra, v0, u1 - sobra, v1]
    } else {
        let quedarse = prop_imagen / prop_celda;
        let sobra = (v1 - v0) * (1.0 - quedarse) * 0.5;
        [u0, v0 + sobra, u1, v1 - sobra]
    }
}

/// El destino es una superficie sRGB: los colores planos hay que llevarlos a
/// lineal a mano o salen lavados respecto a la miniatura.
fn srgb_lineal(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UV: [f32; 4] = [0.0, 0.0, 0.5, 0.25];

    #[test]
    fn misma_proporcion_no_recorta() {
        assert_eq!(recortar(UV, 1.5, 1.5), UV);
    }

    #[test]
    fn imagen_ancha_pierde_los_lados() {
        let r = recortar(UV, 2.0, 1.0);
        assert!(r[0] > UV[0] && r[2] < UV[2], "{r:?}");
        assert_eq!((r[1], r[3]), (UV[1], UV[3]));
        // Se queda con la mitad del ancho y sigue centrada.
        assert!(((r[2] - r[0]) - (UV[2] - UV[0]) * 0.5).abs() < 1e-6);
        assert!(((r[0] + r[2]) * 0.5 - (UV[0] + UV[2]) * 0.5).abs() < 1e-6);
    }

    #[test]
    fn imagen_alta_pierde_arriba_y_abajo() {
        let r = recortar(UV, 0.5, 1.0);
        assert!(r[1] > UV[1] && r[3] < UV[3], "{r:?}");
        assert_eq!((r[0], r[2]), (UV[0], UV[2]));
    }

    #[test]
    fn proporciones_absurdas_no_rompen_nada() {
        for (a, b) in [(0.0, 1.0), (f32::NAN, 1.0), (1.0, f32::INFINITY)] {
            let r = recortar(UV, a, b);
            assert!(r.iter().all(|v| v.is_finite()), "{a} {b} → {r:?}");
        }
    }

    /// Centinela del principio: en la capa visual no hay ni un color escrito a
    /// mano. Si alguien pone un hexadecimal en el código en vez de en el tema,
    /// esta prueba lo caza. Es una regla tonta, y es exactamente lo que evita
    /// que a los seis meses el tema esté a medio funcionar.
    ///
    /// (La primera vez que se ejecutó se cazó a sí misma: el ejemplo que había
    /// en este comentario era un color de verdad.)
    #[test]
    fn ningun_color_escrito_a_mano_fuera_del_tema() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for archivo in ["main.rs", "layout.rs", "atlas.rs", "workers.rs"] {
            let texto = std::fs::read_to_string(raiz.join(archivo)).unwrap();
            if let Some(pos) = buscar_hex(&texto) {
                panic!(
                    "{archivo} tiene un color escrito a mano cerca de: {}",
                    &texto[pos..(pos + 40).min(texto.len())]
                );
            }
        }
        let shader = std::fs::read_to_string(raiz.join("shader.wgsl")).unwrap();
        assert!(
            !shader.contains("vec3<f32>(0."),
            "el shader tiene un color literal; debería venir de `globals`"
        );
        assert!(buscar_hex(&shader).is_none());
    }

    /// Busca `#rrggbb` sin traerse una caja de expresiones regulares.
    fn buscar_hex(texto: &str) -> Option<usize> {
        let b = texto.as_bytes();
        b.iter().enumerate().position(|(i, c)| {
            *c == b'#'
                && b.len() > i + 6
                && b[i + 1..i + 7].iter().all(|d| d.is_ascii_hexdigit())
                && b[i + 1..i + 7].iter().any(|d| d.is_ascii_digit())
        })
    }

    #[test]
    fn el_color_dominante_va_a_lineal() {
        assert!(srgb_lineal(0) < 0.001);
        assert!((srgb_lineal(255) - 1.0).abs() < 0.001);
        // El gris medio sRGB queda cerca de 0,22 en lineal, no de 0,5.
        assert!((srgb_lineal(128) - 0.216).abs() < 0.01);
    }
}

// --------------------------------------------------------------------- datos

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instancia {
    rect: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
    meta: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    viewport: [f32; 2],
    radio: f32,
    realce: f32,
    /// Color del anillo de selección, en lineal. Viene del tema: en el shader
    /// no hay ni un color escrito a mano.
    seleccion: [f32; 4],
}

// ----------------------------------------------------------------------- gpu

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniformes: wgpu::Buffer,
    instancias: wgpu::Buffer,
    capacidad: usize,
    atlas: Atlas,
}

impl Gpu {
    fn nuevo(
        device: wgpu::Device,
        queue: wgpu::Queue,
        formato: wgpu::TextureFormat,
        capas_atlas: u32,
    ) -> Gpu {
        let atlas = Atlas::nuevo(&device, capas_atlas);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("malla"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let uniformes = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let muestreador = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("miniaturas"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("malla"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("malla"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniformes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas.vista),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&muestreador),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("malla"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("malla"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instancia>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: formato,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let instancias = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instancias"),
            size: (std::mem::size_of::<Instancia>() * 4096) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Gpu {
            device,
            queue,
            pipeline,
            bind_group,
            uniformes,
            instancias,
            capacidad: 4096,
            atlas,
        }
    }

    fn subir(&mut self, inst: &[Instancia], viewport: (f32, f32), tema: &Tema) {
        if inst.len() > self.capacidad {
            self.capacidad = inst.len().next_power_of_two();
            self.instancias = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instancias"),
                size: (std::mem::size_of::<Instancia>() * self.capacidad) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue.write_buffer(
            &self.uniformes,
            0,
            bytemuck::bytes_of(&Globals {
                viewport: [viewport.0, viewport.1],
                radio: tema.radio,
                realce: tema.realce,
                seleccion: {
                    let c = tema.color(&tema.seleccion);
                    [c[0], c[1], c[2], 1.0]
                },
            }),
        );
        if !inst.is_empty() {
            self.queue
                .write_buffer(&self.instancias, 0, bytemuck::cast_slice(inst));
        }
    }

    fn dibujar(&self, vista: &wgpu::TextureView, n: u32, fondo: wgpu::Color) {
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("malla"),
            });
        {
            let mut paso = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("malla"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: vista,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(fondo),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if n > 0 {
                paso.set_pipeline(&self.pipeline);
                paso.set_bind_group(0, &self.bind_group, &[]);
                paso.set_vertex_buffer(0, self.instancias.slice(..));
                // Toda la malla visible en una sola llamada de dibujo.
                paso.draw(0..6, 0..n);
            }
        }
        self.queue.submit(Some(enc.finish()));
    }
}

// ------------------------------------------------------------- estadísticas

#[derive(Default)]
struct Fotogramas {
    ms: Vec<f32>,
}

impl Fotogramas {
    fn anotar(&mut self, dt: f32) {
        self.ms.push(dt * 1000.0);
    }

    fn informe(&mut self, titulo: &str, extra: &str) {
        if self.ms.is_empty() {
            println!("  sin fotogramas medidos");
            return;
        }
        // El primer fotograma incluye compilar el pipeline: se anota aparte.
        let primero = self.ms[0];
        let resto = &mut self.ms[1..];
        resto.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |q: f32| resto[((resto.len() - 1) as f32 * q) as usize];
        let media: f32 = resto.iter().sum::<f32>() / resto.len() as f32;
        let sobre = |lim: f32| resto.iter().filter(|v| **v > lim).count();
        println!("\n{titulo}");
        println!("  fotogramas          {}", resto.len() + 1);
        println!("  primero             {primero:.1} ms (incluye compilar el pipeline)");
        println!(
            "  media               {media:.2} ms  ({:.0} fps)",
            1000.0 / media
        );
        println!(
            "  p50 / p95 / p99     {:.2} / {:.2} / {:.2} ms",
            p(0.5),
            p(0.95),
            p(0.99)
        );
        println!("  máximo              {:.2} ms", resto[resto.len() - 1]);
        println!(
            "  por encima de 8,3 ms {:>5}   ({:.2}%)",
            sobre(8.3),
            sobre(8.3) as f32 * 100.0 / resto.len() as f32
        );
        println!(
            "  por encima de 16,6 ms{:>5}   ({:.2}%)",
            sobre(16.6),
            sobre(16.6) as f32 * 100.0 / resto.len() as f32
        );
        if !extra.is_empty() {
            println!("{extra}");
        }
    }
}

/// Memoria propia y memoria de archivos mapeados, por separado.
///
/// Mezclarlas engaña: las páginas del pack de miniaturas son caché del sistema,
/// las devuelve el kernel cuando hace falta y no son "consumo" de la aplicación.
fn memoria() -> (f64, f64) {
    let mut anon = 0.0;
    let mut archivo = 0.0;
    if let Ok(txt) = std::fs::read_to_string("/proc/self/status") {
        for linea in txt.lines() {
            let valor = |l: &str| -> f64 {
                l.split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0)
                    / 1024.0
            };
            if linea.starts_with("RssAnon:") {
                anon = valor(linea);
            } else if linea.starts_with("RssFile:") {
                archivo = valor(linea);
            }
        }
    }
    (anon, archivo)
}

fn rss_mb() -> f64 {
    let (a, f) = memoria();
    a + f
}

// ---------------------------------------------------------------- ejecución

fn main() {
    let mut args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n\n{AYUDA}");
            std::process::exit(2);
        }
    };
    let tema = match args.tema.as_ref() {
        Some(p) => match Tema::desde_archivo(p) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: no pude leer el tema {}: {e}", p.display());
                std::process::exit(2);
            }
        },
        None => Tema::default(),
    };

    // La línea de órdenes gana al tema; el tema gana al valor de fábrica.
    if !std::env::args().any(|a| a == "--celda") {
        args.objetivo = tema.celda;
    }

    let escena = match Escena::nueva(&args, tema) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if let Some(n) = args.solo_decodificar {
        solo_decodificar(escena, n);
        return;
    }

    if let Some(destino) = args.captura.clone() {
        capturar(escena, &args, &destino);
    } else {
        ventana::correr(escena, args);
    }
}

/// Diagnóstico puro de CPU: decodificar N miniaturas del pack y tirarlas.
/// Si aquí la memoria crece, el problema no está en la GPU.
fn solo_decodificar(escena: Escena, n: usize) {
    println!("  RSS al empezar: {:.0} MB", rss_mb());
    let t = Instant::now();
    let mut hechas = 0usize;
    let total = escena.hits.len().min(n);
    let mut siguiente = 0usize;
    while hechas < total {
        let lote: Vec<u32> = (siguiente..(siguiente + 256).min(total))
            .map(|i| i as u32)
            .collect();
        if lote.is_empty() {
            break;
        }
        siguiente += lote.len();
        escena.pool.pedir(lote.iter().copied());
        let esperadas = lote.len();
        let mut recibidas = 0;
        while recibidas < esperadas {
            let lote = escena.pool.esperar(2000);
            if lote.is_empty() {
                break;
            }
            recibidas += lote.len();
            hechas += lote.len();
        }
        if hechas % 4096 < 256 {
            println!("  {hechas:>7} decodificadas · RSS {:.0} MB", rss_mb());
        }
    }
    println!(
        "  {hechas} miniaturas en {:.2} s ({:.0}/s) · RSS final {:.0} MB",
        t.elapsed().as_secs_f64(),
        hechas as f64 / t.elapsed().as_secs_f64(),
        rss_mb()
    );
}

/// Render sin ventana: sirve para revisar el diseño y para entornos sin
/// escritorio (CI, servidores).
fn capturar(mut escena: Escena, args: &Args, destino: &std::path::Path) {
    let instancia =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adaptador =
        pollster::block_on(instancia.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("no hay adaptador gráfico disponible");
    let (device, queue) = pollster::block_on(adaptador.request_device(&wgpu::DeviceDescriptor {
        label: Some("captura"),
        ..Default::default()
    }))
    .expect("no pude crear el dispositivo");

    let formato = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut gpu = Gpu::nuevo(device, queue, formato, 8);
    escena.redimensionar(args.ancho as f32, args.alto as f32);

    let destino_tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("captura"),
        size: wgpu::Extent3d {
            width: args.ancho,
            height: args.alto,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: formato,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let vista = destino_tex.create_view(&Default::default());

    // Varias pasadas para que las miniaturas visibles lleguen a estar dentro.
    for _ in 0..40 {
        escena.preparar(0.016, &mut gpu.atlas, &gpu.queue);
        let recibidas = escena.pool.esperar(120);
        for d in recibidas {
            escena.en_vuelo.remove(&d.indice);
            gpu.atlas
                .insertar(&gpu.queue, d.indice, d.w, d.h, &d.rgba, escena.fotograma);
            escena.aparicion.insert(d.indice, 1.0);
        }
        if escena.pool.pendientes() == 0 && escena.en_vuelo.is_empty() {
            break;
        }
    }
    escena.aparicion.clear();
    escena.preparar(0.016, &mut gpu.atlas, &gpu.queue);
    let inst = escena.instancias(&mut gpu.atlas);
    gpu.subir(&inst, (args.ancho as f32, args.alto as f32), &escena.tema);
    gpu.dibujar(&vista, inst.len() as u32, escena.tema.fondo_wgpu());

    // Copia a CPU y PNG.
    let stride = (args.ancho * 4).div_ceil(256) * 256;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("lectura"),
        size: (stride * args.alto) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &destino_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(args.alto),
            },
        },
        wgpu::Extent3d {
            width: args.ancho,
            height: args.alto,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit(Some(enc.finish()));

    let rodaja = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    rodaja.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = gpu.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    rx.recv().unwrap().expect("no pude leer la textura");

    let datos = rodaja.get_mapped_range().expect("mapear la lectura");
    let mut pixeles = Vec::with_capacity((args.ancho * args.alto * 4) as usize);
    for y in 0..args.alto {
        let fila = (y * stride) as usize;
        pixeles.extend_from_slice(&datos[fila..fila + (args.ancho * 4) as usize]);
    }
    drop(datos);
    buffer.unmap();

    image::RgbaImage::from_raw(args.ancho, args.alto, pixeles)
        .expect("dimensiones de la captura")
        .save(destino)
        .expect("no pude guardar el PNG");
    println!(
        "  captura guardada en {} ({} celdas, {} miniaturas en el atlas)",
        destino.display(),
        inst.len(),
        gpu.atlas.ocupadas()
    );
}

mod ventana {
    use super::*;
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{Key, NamedKey};
    use winit::window::{Window, WindowId};

    struct App {
        escena: Escena,
        args: Args,
        estado: Option<Estado>,
        stats: Fotogramas,
        ultimo: Instant,
        arranque: Instant,
        primer_fotograma: Option<f32>,
    }

    struct Estado {
        ventana: Arc<Window>,
        superficie: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        gpu: Gpu,
    }

    pub fn correr(escena: Escena, args: Args) {
        let event_loop = EventLoop::new().expect("no pude crear el bucle de eventos");
        event_loop.set_control_flow(ControlFlow::Poll);
        let mut app = App {
            escena,
            args,
            estado: None,
            stats: Fotogramas::default(),
            ultimo: Instant::now(),
            arranque: Instant::now(),
            primer_fotograma: None,
        };
        event_loop.run_app(&mut app).expect("fallo del bucle");
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.estado.is_some() {
                return;
            }
            let atributos = Window::default_attributes()
                .with_title("Grimorio · spike de malla")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.args.ancho as f64,
                    self.args.alto as f64,
                ));
            let ventana = Arc::new(event_loop.create_window(atributos).expect("ventana"));

            let instancia = wgpu::Instance::new(
                wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
            );
            let superficie = instancia
                .create_surface(Arc::clone(&ventana))
                .expect("superficie");
            let adaptador =
                pollster::block_on(instancia.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&superficie),
                    force_fallback_adapter: false,
                    ..Default::default()
                }))
                .expect("adaptador");
            let info = adaptador.get_info();
            println!("  gpu: {} ({:?})", info.name, info.backend);

            let (device, queue) =
                pollster::block_on(adaptador.request_device(&wgpu::DeviceDescriptor {
                    label: Some("grimorio"),
                    ..Default::default()
                }))
                .expect("dispositivo");

            let caps = superficie.get_capabilities(&adaptador);
            let formato = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let modo = if self.args.vsync {
                wgpu::PresentMode::AutoVsync
            } else if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
                wgpu::PresentMode::Immediate
            } else {
                wgpu::PresentMode::AutoNoVsync
            };
            let tam = ventana.inner_size();
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: formato,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: tam.width.max(1),
                height: tam.height.max(1),
                present_mode: modo,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            superficie.configure(&device, &config);
            println!("  presentación: {modo:?} · formato {formato:?}");

            let gpu = Gpu::nuevo(device, queue, formato, 8);
            self.escena
                .redimensionar(config.width as f32, config.height as f32);
            self.estado = Some(Estado {
                ventana,
                superficie,
                config,
                gpu,
            });
            self.ultimo = Instant::now();
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            let Some(estado) = self.estado.as_mut() else {
                return;
            };
            match event {
                WindowEvent::CloseRequested => {
                    self.stats
                        .informe("resultados", &resumen(&self.escena, &estado.gpu));
                    event_loop.exit();
                }
                WindowEvent::Resized(tam) => {
                    estado.config.width = tam.width.max(1);
                    estado.config.height = tam.height.max(1);
                    estado
                        .superficie
                        .configure(&estado.gpu.device, &estado.config);
                    self.escena
                        .redimensionar(tam.width as f32, tam.height as f32);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.escena.raton = (position.x as f32, position.y as f32);
                }
                WindowEvent::CursorLeft { .. } => {
                    self.escena.raton = (-1.0, -1.0);
                }
                WindowEvent::MouseInput { state, button, .. }
                    if state == ElementState::Pressed
                        && button == winit::event::MouseButton::Left =>
                {
                    let (x, y) = self.escena.raton;
                    self.escena.seleccion = self.escena.celda_en(x, y);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y * 90.0,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32,
                    };
                    self.escena.desplazar(-dy);
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed =>
                {
                    let alto = self.escena.alto;
                    match event.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            self.stats
                                .informe("resultados", &resumen(&self.escena, &estado.gpu));
                            event_loop.exit();
                        }
                        Key::Named(NamedKey::ArrowDown) => self.escena.desplazar(90.0),
                        Key::Named(NamedKey::ArrowUp) => self.escena.desplazar(-90.0),
                        Key::Named(NamedKey::PageDown) => self.escena.desplazar(alto * 0.9),
                        Key::Named(NamedKey::PageUp) => self.escena.desplazar(-alto * 0.9),
                        Key::Named(NamedKey::Home) => {
                            self.escena.scroll_objetivo = 0.0;
                            self.escena.limitar_scroll();
                        }
                        Key::Named(NamedKey::End) => {
                            self.escena.scroll_objetivo = self.escena.disp.alto_total();
                            self.escena.limitar_scroll();
                        }
                        Key::Character("+") | Key::Character("=") => self.escena.zoom(20.0),
                        Key::Character("-") => self.escena.zoom(-20.0),
                        Key::Character("l") | Key::Character("L") => self.escena.cambiar_modo(),
                        _ => {}
                    }
                }
                WindowEvent::RedrawRequested => self.dibujar(event_loop),
                _ => {}
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(e) = &self.estado {
                e.ventana.request_redraw();
            }
        }
    }

    impl App {
        fn dibujar(&mut self, event_loop: &ActiveEventLoop) {
            let Some(estado) = self.estado.as_mut() else {
                return;
            };
            let ahora = Instant::now();
            let dt = (ahora - self.ultimo).as_secs_f32().min(0.1);
            self.ultimo = ahora;

            // Modo banco de pruebas: recorrido automático a velocidad constante.
            if let Some(segundos) = self.args.bench {
                let t = self.arranque.elapsed().as_secs_f32();
                if t > segundos {
                    self.stats
                        .informe("banco de pruebas", &resumen(&self.escena, &estado.gpu));
                    event_loop.exit();
                    return;
                }
                let recorrido = self.escena.disp.alto_total() - self.escena.alto;
                // Un recorrido completo de la biblioteca en la mitad del tiempo,
                // ida y vuelta: fuerza carga y desalojo continuos.
                let v = (recorrido / segundos) * 2.0;
                let dir = if (t / (segundos / 2.0)) as i32 % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                self.escena.desplazar_seco(v * dt * dir);
            }

            self.escena
                .preparar(dt, &mut estado.gpu.atlas, &estado.gpu.queue);
            let inst = self.escena.instancias(&mut estado.gpu.atlas);
            estado.gpu.subir(
                &inst,
                (estado.config.width as f32, estado.config.height as f32),
                &self.escena.tema,
            );

            use wgpu::CurrentSurfaceTexture as Cst;
            let marco = match estado.superficie.get_current_texture() {
                Cst::Success(m) | Cst::Suboptimal(m) => m,
                Cst::Outdated | Cst::Lost => {
                    // La ventana cambió de tamaño o el compositor se reinició:
                    // reconfigurar y seguir, no es un error.
                    estado
                        .superficie
                        .configure(&estado.gpu.device, &estado.config);
                    return;
                }
                Cst::Timeout | Cst::Occluded => return,
                otro => {
                    eprintln!("superficie: {otro:?}");
                    return;
                }
            };
            let vista = marco.texture.create_view(&Default::default());
            estado
                .gpu
                .dibujar(&vista, inst.len() as u32, self.escena.tema.fondo_wgpu());
            estado.gpu.queue.present(marco);

            // Un `poll` sin bloquear por fotograma: deja que wgpu recicle los
            // recursos de las subidas ya terminadas sin frenar la CPU.
            let _ = estado.gpu.device.poll(wgpu::PollType::Poll);

            if self.primer_fotograma.is_none() {
                let ms = self.arranque.elapsed().as_secs_f32() * 1000.0;
                self.primer_fotograma = Some(ms);
                println!("  primer fotograma a los {ms:.0} ms desde el arranque");
            }
            self.stats.anotar(dt);
        }
    }

    fn resumen(escena: &Escena, gpu: &Gpu) -> String {
        format!(
            "  celdas visibles     {}\n  \
             atlas               {} / {} ranuras · {} subidas · {} desalojos\n  \
             miniaturas decodif. {}\n  \
             memoria propia      {:.0} MB  (+ {:.0} MB de archivos mapeados)\n  \
             disposición         {} · celda {:.0} px · {} filas",
            escena.visibles.len(),
            gpu.atlas.ocupadas(),
            gpu.atlas.capacidad(),
            gpu.atlas.subidas,
            gpu.atlas.desalojos,
            escena.pool.recibidas(),
            memoria().0,
            memoria().1,
            escena.disp.modo().nombre(),
            escena.disp.objetivo(),
            escena.disp.filas(),
        )
    }
}
