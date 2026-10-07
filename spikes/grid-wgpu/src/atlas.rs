//! Atlas de texturas con desalojo LRU.
//!
//! Una textura por miniatura significa miles de cambios de estado por
//! fotograma. Aquí todas las miniaturas visibles viven en unas pocas capas de
//! un `texture_2d_array`, así que la malla entera se dibuja de una vez.

use std::collections::HashMap;

pub const LADO_RANURA: u32 = 320;
const RANURAS_LADO: u32 = 8; // 8×8 ranuras por capa

#[derive(Debug, Clone, Copy)]
pub struct Ranura {
    pub slot: u32,
    pub w: u32,
    pub h: u32,
}

pub struct Atlas {
    pub textura: wgpu::Texture,
    pub vista: wgpu::TextureView,
    capas: u32,
    por_capa: u32,
    lado_capa: u32,
    mapa: HashMap<u32, Ranura>,
    dueno: Vec<Option<u32>>,
    ultimo_uso: Vec<u64>,
    pub subidas: u64,
    pub desalojos: u64,
}

impl Atlas {
    pub fn nuevo(device: &wgpu::Device, capas: u32) -> Atlas {
        let lado_capa = LADO_RANURA * RANURAS_LADO;
        let textura = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas de miniaturas"),
            size: wgpu::Extent3d {
                width: lado_capa,
                height: lado_capa,
                depth_or_array_layers: capas,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let vista = textura.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let total = (capas * RANURAS_LADO * RANURAS_LADO) as usize;
        Atlas {
            textura,
            vista,
            capas,
            por_capa: RANURAS_LADO * RANURAS_LADO,
            lado_capa,
            mapa: HashMap::with_capacity(total),
            dueno: vec![None; total],
            ultimo_uso: vec![0; total],
            subidas: 0,
            desalojos: 0,
        }
    }

    pub fn capacidad(&self) -> usize {
        self.dueno.len()
    }

    pub fn ocupadas(&self) -> usize {
        self.mapa.len()
    }

    /// Consulta marcando uso, para que el LRU sepa qué está vivo.
    pub fn obtener(&mut self, item: u32, fotograma: u64) -> Option<Ranura> {
        let r = *self.mapa.get(&item)?;
        self.ultimo_uso[r.slot as usize] = fotograma;
        Some(r)
    }

    pub fn insertar(
        &mut self,
        queue: &wgpu::Queue,
        item: u32,
        w: u32,
        h: u32,
        rgba: &[u8],
        fotograma: u64,
    ) -> Ranura {
        if let Some(r) = self.mapa.get(&item).copied() {
            self.ultimo_uso[r.slot as usize] = fotograma;
            return r;
        }
        let slot = self.elegir_ranura(fotograma);
        if let Some(anterior) = self.dueno[slot as usize].take() {
            self.mapa.remove(&anterior);
            self.desalojos += 1;
        }

        let w = w.min(LADO_RANURA);
        let h = h.min(LADO_RANURA);
        let (cx, cy, capa) = self.posicion(slot);

        // `write_texture` exige que la fila esté alineada a 256 bytes.
        let stride = alinear(w * 4, 256);
        let mut datos = vec![0u8; (stride * h) as usize];
        for y in 0..h as usize {
            let origen = y * (w as usize) * 4;
            let destino = y * stride as usize;
            datos[destino..destino + w as usize * 4]
                .copy_from_slice(&rgba[origen..origen + w as usize * 4]);
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.textura,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: cx,
                    y: cy,
                    z: capa,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &datos,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let r = Ranura { slot, w, h };
        self.mapa.insert(item, r);
        self.dueno[slot as usize] = Some(item);
        self.ultimo_uso[slot as usize] = fotograma;
        self.subidas += 1;
        r
    }

    /// Coordenadas de textura de una ranura, ya normalizadas.
    pub fn uv(&self, r: &Ranura) -> ([f32; 4], u32) {
        let (cx, cy, capa) = self.posicion(r.slot);
        let l = self.lado_capa as f32;
        (
            [
                cx as f32 / l,
                cy as f32 / l,
                (cx + r.w) as f32 / l,
                (cy + r.h) as f32 / l,
            ],
            capa,
        )
    }

    fn posicion(&self, slot: u32) -> (u32, u32, u32) {
        let capa = slot / self.por_capa;
        let dentro = slot % self.por_capa;
        let cx = (dentro % RANURAS_LADO) * LADO_RANURA;
        let cy = (dentro / RANURAS_LADO) * LADO_RANURA;
        (cx, cy, capa.min(self.capas - 1))
    }

    fn elegir_ranura(&self, fotograma: u64) -> u32 {
        // Primero cualquier ranura libre.
        if let Some(i) = self.dueno.iter().position(|d| d.is_none()) {
            return i as u32;
        }
        // Si no, la que lleve más tiempo sin verse. Nunca se desaloja algo
        // usado en este mismo fotograma: eso sería pelearse consigo mismo.
        let mut mejor = 0usize;
        let mut mejor_uso = u64::MAX;
        for (i, uso) in self.ultimo_uso.iter().enumerate() {
            if *uso < mejor_uso && *uso != fotograma {
                mejor_uso = *uso;
                mejor = i;
            }
        }
        mejor as u32
    }
}

fn alinear(v: u32, a: u32) -> u32 {
    v.div_ceil(a) * a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alineacion() {
        assert_eq!(alinear(1280, 256), 1280);
        assert_eq!(alinear(1281, 256), 1536);
        assert_eq!(alinear(4, 256), 256);
    }
}
