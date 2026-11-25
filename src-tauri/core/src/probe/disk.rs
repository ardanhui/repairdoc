//! Data penyimpanan: ruang kosong per drive (C:, D:, …) dan kondisi disk fisik.

use std::time::Duration;

use sysinfo::{DiskKind, Disks};

use super::json;
use crate::platform;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Hdd,
    Ssd,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusKind {
    Usb,
    Sata,
    Nvme,
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskHealth {
    Healthy,
    Warning,
    Unhealthy,
    Unknown,
}

/// Satu drive yang terlihat di File Explorer (C:, D:, …).
#[derive(Clone, Debug)]
pub struct VolumeRaw {
    pub mount: String,
    pub letter: Option<char>,
    pub label: String,
    pub file_system: String,
    pub total: u64,
    pub available: u64,
    pub removable: bool,
    pub kind: MediaKind,
}

/// Satu disk fisik (keping HDD/SSD).
#[derive(Clone, Debug)]
pub struct PhysDiskRaw {
    pub name: String,
    pub media: MediaKind,
    pub bus: BusKind,
    pub health: DiskHealth,
    pub size: u64,
    pub letters: Vec<char>,
    /// Persentase umur pakai SSD yang sudah terpakai (0 = baru, 100 = habis).
    pub wear: Option<u64>,
    pub temp_c: Option<f64>,
    pub read_errors_uncorrected: Option<u64>,
    pub power_on_hours: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct DiskRaw {
    pub volumes: Vec<VolumeRaw>,
    pub physical: Vec<PhysDiskRaw>,
    /// "storage" (lengkap) atau "win32" (cadangan, data terbatas).
    pub physical_source: Option<String>,
    /// Apakah data umur/suhu/error disk bisa dibaca (butuh admin).
    pub reliability_readable: bool,
    pub system_letter: Option<char>,
    /// Drive tempat RepairDoc dijalankan — dilewati saat diagnosa.
    pub exe_letter: Option<char>,
    pub notes: Vec<String>,
}

pub fn probe() -> DiskRaw {
    let mut raw = DiskRaw {
        system_letter: platform::system_drive_letter(),
        exe_letter: platform::exe_drive_letter(),
        ..Default::default()
    };

    match Disks::new_with_refreshed_list() {
        Ok(disks) => {
            for d in disks.iter() {
                let mount = d.mount_point().to_string_lossy().to_string();
                raw.volumes.push(VolumeRaw {
                    letter: platform::drive_letter_of(&mount),
                    mount,
                    label: d.name().to_string_lossy().trim().to_string(),
                    file_system: d.file_system().to_string_lossy().to_string(),
                    total: d.total_space(),
                    available: d.available_space(),
                    removable: d.is_removable(),
                    kind: match d.kind() {
                        DiskKind::HDD => MediaKind::Hdd,
                        DiskKind::SSD => MediaKind::Ssd,
                        _ => MediaKind::Unknown,
                    },
                });
            }
        }
        Err(e) => raw.notes.push(format!("Daftar drive tidak terbaca: {e}")),
    }

    if platform::IS_WINDOWS {
        match platform::run_powershell(include_str!("../scripts/disks.ps1"), Duration::from_secs(40)) {
            Ok(out) => apply_wmi(&mut raw, &out),
            Err(e) => raw.notes.push(format!("Kondisi disk fisik tidak terbaca: {e}")),
        }
    }

    raw
}

pub fn media_from(s: &str) -> MediaKind {
    match s.trim().to_ascii_uppercase().as_str() {
        "3" | "HDD" => MediaKind::Hdd,
        "4" | "SSD" => MediaKind::Ssd,
        _ => MediaKind::Unknown,
    }
}

pub fn bus_from(s: &str) -> BusKind {
    match s.trim().to_ascii_uppercase().as_str() {
        "7" | "USB" => BusKind::Usb,
        "11" | "SATA" | "3" | "ATA" | "IDE" => BusKind::Sata,
        "17" | "NVME" => BusKind::Nvme,
        other => BusKind::Other(other.to_string()),
    }
}

/// Peta status kesehatan dari MSFT_PhysicalDisk (angka/teks) maupun Win32_DiskDrive.
pub fn health_from(s: &str) -> DiskHealth {
    match s.trim().to_ascii_uppercase().as_str() {
        "0" | "HEALTHY" | "OK" => DiskHealth::Healthy,
        "1" | "WARNING" | "DEGRADED" | "STRESSED" => DiskHealth::Warning,
        "2" | "UNHEALTHY" | "PRED FAIL" | "ERROR" | "NONRECOVER" => DiskHealth::Unhealthy,
        _ => DiskHealth::Unknown,
    }
}

/// Gabungkan keluaran `disks.ps1` ke data mentah.
pub fn apply_wmi(raw: &mut DiskRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    let source = json::str_of(v.get("source"));
    raw.physical_source = (!source.is_empty()).then_some(source);
    raw.reliability_readable = v.get("reliabilityReadable").and_then(|b| b.as_bool()).unwrap_or(false);
    raw.physical = json::list_of(v.get("disks"))
        .into_iter()
        .map(|d| PhysDiskRaw {
            name: json::str_of(d.get("name")),
            media: media_from(&json::str_of(d.get("mediaType"))),
            bus: bus_from(&json::str_of(d.get("busType"))),
            health: health_from(&json::str_of(d.get("health"))),
            size: json::u64_of(d.get("sizeBytes")).unwrap_or(0),
            letters: json::list_of(d.get("letters"))
                .into_iter()
                .filter_map(|l| json::str_of(Some(l)).chars().next())
                .map(|c| c.to_ascii_uppercase())
                .collect(),
            wear: json::u64_of(d.get("wear")).filter(|w| *w <= 100),
            temp_c: json::f64_of(d.get("temperatureC")).filter(|t| *t > 0.0 && *t < 120.0),
            read_errors_uncorrected: json::u64_of(d.get("readErrorsUncorrected")),
            power_on_hours: json::u64_of(d.get("powerOnHours")).filter(|h| *h > 0),
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_storage_output_numeric_codes() {
        let mut raw = DiskRaw::default();
        apply_wmi(
            &mut raw,
            r#"{"source":"storage","reliabilityReadable":true,"disks":{"id":"0","name":"ST500LM012","mediaType":"3",
                "busType":"11","health":"0","sizeBytes":500107862016,"letters":"C",
                "wear":0,"temperatureC":41,"readErrorsUncorrected":0,"powerOnHours":15123}}"#,
        );
        assert_eq!(raw.physical.len(), 1);
        let d = &raw.physical[0];
        assert_eq!(d.media, MediaKind::Hdd);
        assert_eq!(d.bus, BusKind::Sata);
        assert_eq!(d.health, DiskHealth::Healthy);
        assert_eq!(d.letters, vec!['C']);
        assert_eq!(d.temp_c, Some(41.0));
        assert!(raw.reliability_readable);
    }

    #[test]
    fn parses_text_codes_and_win32_fallback() {
        assert_eq!(media_from("SSD"), MediaKind::Ssd);
        assert_eq!(bus_from("NVMe"), BusKind::Nvme);
        assert_eq!(health_from("Pred Fail"), DiskHealth::Unhealthy);
        assert_eq!(health_from("Warning"), DiskHealth::Warning);
        assert_eq!(health_from("5"), DiskHealth::Unknown);
    }

    #[test]
    fn zero_temperature_means_unknown() {
        let mut raw = DiskRaw::default();
        apply_wmi(&mut raw, r#"{"source":"storage","disks":[{"name":"X","health":"0","temperatureC":0,"letters":[]}]}"#);
        assert_eq!(raw.physical[0].temp_c, None);
    }
}
