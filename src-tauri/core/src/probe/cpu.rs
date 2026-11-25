//! Data prosesor: nama, jumlah inti, pemakaian, dan suhu (bila ada sensornya).

use std::time::Duration;

use super::json;
use crate::platform;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TempSource {
    /// Penghitung kinerja "Thermal Zone Information" (tanpa admin).
    ThermalZone,
    /// WMI MSAcpi_ThermalZoneTemperature (biasanya butuh admin).
    Acpi,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TempReading {
    pub celsius: f64,
    pub source: TempSource,
}

#[derive(Clone, Debug, Default)]
pub struct CpuRaw {
    pub name: String,
    pub physical_cores: Option<usize>,
    pub logical_cores: usize,
    /// Rata-rata pemakaian selama jendela sampling, 0–100.
    pub usage_avg: Option<f64>,
    pub usage_peak: Option<f64>,
    pub temp: Option<TempReading>,
    /// Catatan untuk teknisi bila ada yang tidak bisa dibaca.
    pub notes: Vec<String>,
}

/// Baca suhu lewat PowerShell dan simpan ke `raw`. Data pemakaian/nama CPU
/// diisi oleh `probe::live` (satu jendela pengukuran bersama proses-proses).
pub fn probe_temp(raw: &mut CpuRaw) {
    if !platform::IS_WINDOWS {
        return;
    }
    match platform::run_powershell(include_str!("../scripts/cpu_temp.ps1"), Duration::from_secs(25)) {
        Ok(out) => match parse_temp(&out) {
            Some(t) => raw.temp = Some(t),
            None => raw.notes.push("PC ini tidak menyediakan sensor suhu yang bisa dibaca Windows.".into()),
        },
        Err(e) => raw.notes.push(format!("Suhu tidak terbaca: {e}")),
    }
}

/// Pilih pembacaan suhu yang masuk akal dari keluaran `cpu_temp.ps1`.
pub fn parse_temp(out: &str) -> Option<TempReading> {
    let v = json::parse(out).ok()?;
    let plausible = |c: f64| (5.0..=120.0).contains(&c);
    if let Some(c) = json::f64_of(v.get("perf")).filter(|c| plausible(*c)) {
        return Some(TempReading { celsius: c, source: TempSource::ThermalZone });
    }
    if let Some(c) = json::f64_of(v.get("acpi")).filter(|c| plausible(*c)) {
        return Some(TempReading { celsius: c, source: TempSource::Acpi });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_prefers_thermal_zone() {
        let t = parse_temp(r#"{"perf":61.9,"acpi":55}"#).unwrap();
        assert_eq!(t.source, TempSource::ThermalZone);
        assert!((t.celsius - 61.9).abs() < 0.01);
    }

    #[test]
    fn temp_falls_back_and_rejects_nonsense() {
        assert_eq!(parse_temp(r#"{"perf":null,"acpi":48.5}"#).unwrap().source, TempSource::Acpi);
        assert!(parse_temp(r#"{"perf":-273.1,"acpi":null}"#).is_none());
        assert!(parse_temp("bukan json").is_none());
    }
}
