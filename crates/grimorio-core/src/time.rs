//! Fechas en RFC 3339 UTC sin dependencias.
//!
//! Solo necesitamos ida y vuelta entre milisegundos Unix y `2026-08-25T10:12:33Z`.
//! El algoritmo de calendario es el de Howard Hinnant (civil_from_days), válido
//! para cualquier fecha proléptica gregoriana.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn to_rfc3339(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

pub fn from_rfc3339(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() < 20 {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { s.get(from..to)?.parse().ok() };
    let y = num(0, 4)?;
    let m = num(5, 7)?;
    let d = num(8, 10)?;
    let hh = num(11, 13)?;
    let mm = num(14, 16)?;
    let ss = num(17, 19)?;
    let days = days_from_civil(y, m as u32, d as u32);
    let secs = days * 86_400 + hh * 3600 + mm * 60 + ss;
    if secs < 0 {
        return None;
    }
    Some(secs as u64 * 1000)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Cuántos segundos va la hora local por delante de UTC en ese instante
/// (con el horario de verano incluido). Lo usa el filtro de fechas: «hoy»
/// empieza a medianoche de aquí, no a la de Greenwich.
#[cfg(unix)]
pub fn desfase_local_s(ms: u64) -> i64 {
    let t = (ms / 1000) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: `t` y `tm` son locales; localtime_r no guarda los punteros.
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff
}

/// En Windows el `tm` no trae `tm_gmtoff`. En vez de preguntar la zona aparte
/// (`_get_timezone` no sabe si ese día concreto era de verano), se convierte
/// el instante a hora local y se mide cuánto se aleja de él: la resta ya lleva
/// dentro el horario de verano de esa fecha, igual que `tm_gmtoff`.
#[cfg(windows)]
pub fn desfase_local_s(ms: u64) -> i64 {
    let segundos = (ms / 1000) as i64;
    let t = segundos as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: `t` y `tm` son locales; localtime_s no guarda los punteros.
    if unsafe { libc::localtime_s(&mut tm, &t) } != 0 {
        return 0;
    }
    let dias = days_from_civil(
        tm.tm_year as i64 + 1900,
        tm.tm_mon as u32 + 1,
        tm.tm_mday as u32,
    );
    let local = dias * 86_400 + tm.tm_hour as i64 * 3600 + tm.tm_min as i64 * 60 + tm.tm_sec as i64;
    local - segundos
}

/// Milisegundos del comienzo de un día en hora local (`desfase_s` por delante
/// de UTC). Acepta meses y días fuera de rango: el 13 del mes 12 es enero.
pub fn inicio_dia_local_ms(y: i64, m: u32, d: u32, desfase_s: i64) -> u64 {
    let (y, m) = (y + (m as i64 - 1).div_euclid(12), (m as i64 - 1).rem_euclid(12) as u32 + 1);
    let dias = days_from_civil(y, m, 1) + d as i64 - 1;
    (dias * 86_400 - desfase_s).max(0) as u64 * 1000
}

/// Año, mes y día locales de un instante.
pub fn fecha_local(ms: u64, desfase_s: i64) -> (i64, u32, u32) {
    let secs = (ms / 1000) as i64 + desfase_s;
    civil_from_days(secs.div_euclid(86_400))
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        for ms in [0u64, 1_000, 1_756_116_753_000, 4_102_444_800_000] {
            let s = to_rfc3339(ms);
            assert_eq!(from_rfc3339(&s), Some(ms - ms % 1000), "fallo con {s}");
        }
    }

    #[test]
    fn fecha_conocida() {
        assert_eq!(to_rfc3339(1_756_116_753_000), "2025-08-25T10:12:33Z");
        assert_eq!(to_rfc3339(1_787_652_753_000), "2026-08-25T10:12:33Z");
    }

    #[test]
    fn anio_bisiesto() {
        // 2024-02-29T00:00:00Z
        assert_eq!(to_rfc3339(1_709_164_800_000), "2024-02-29T00:00:00Z");
    }
}
