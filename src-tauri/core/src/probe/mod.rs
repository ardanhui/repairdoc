//! Pengumpulan data mentah. Modul ini hanya MEMBACA kondisi PC.
//! Penilaian (sehat/bermasalah) dilakukan di `analyze`.

pub mod battery;
pub mod cleanup;
pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod live;
pub mod memory;
pub mod net;
pub mod network;
pub mod programs;
pub mod security;
pub mod system;

use serde_json::Value;

/// Pembacaan JSON yang "pemaaf": keluaran PowerShell 5.1 kadang berupa angka,
/// kadang teks, dan array berisi satu item kadang jadi objek tunggal.
pub(crate) mod json {
    use super::Value;

    pub fn u64_of(v: Option<&Value>) -> Option<u64> {
        match v? {
            Value::Number(n) => n.as_u64().or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn f64_of(v: Option<&Value>) -> Option<f64> {
        match v? {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().replace(',', ".").parse().ok(),
            _ => None,
        }
    }

    pub fn str_of(v: Option<&Value>) -> String {
        match v {
            Some(Value::String(s)) => s.trim().to_string(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            _ => String::new(),
        }
    }

    /// Selalu kembalikan daftar, walau PowerShell mengirim satu objek atau null.
    pub fn list_of(v: Option<&Value>) -> Vec<&Value> {
        match v {
            Some(Value::Array(a)) => a.iter().collect(),
            Some(Value::Null) | None => Vec::new(),
            Some(other) => vec![other],
        }
    }

    pub fn parse(s: &str) -> Result<Value, String> {
        serde_json::from_str(s.trim()).map_err(|e| format!("JSON tidak terbaca: {e}"))
    }
}

/// Tes yang hanya jalan di Windows: menjalankan skrip PowerShell sungguhan
/// dan memastikan keluarannya JSON yang bisa dibaca.
///     cd src-tauri && cargo test -p repairdoc-core
#[cfg(all(test, windows))]
mod windows_tests {
    use super::json;
    use serde_json::Value;
    use crate::platform::run_powershell;
    use std::time::Duration;

    fn run(script: &str) -> Value {
        let out = run_powershell(script, Duration::from_secs(90)).expect("PowerShell gagal dijalankan");
        json::parse(&out).unwrap_or_else(|e| panic!("{e}\nKeluaran: {out}"))
    }

    #[test]
    fn cpu_temp_script_outputs_json() {
        let v = run(include_str!("../scripts/cpu_temp.ps1"));
        assert!(v.get("perf").is_some() && v.get("acpi").is_some(), "{v}");
    }

    #[test]
    fn memory_script_outputs_json() {
        let v = run(include_str!("../scripts/memory.ps1"));
        assert!(v.get("modules").is_some() && v.get("memtest").is_some(), "{v}");
    }

    #[test]
    fn disk_script_finds_a_disk() {
        let v = run(include_str!("../scripts/disks.ps1"));
        assert!(!json::list_of(v.get("disks")).is_empty(), "tidak ada disk terbaca: {v}");
    }

    #[test]
    fn gpu_script_outputs_json() {
        let v = run(include_str!("../scripts/gpu.ps1"));
        assert!(v.get("adapters").is_some(), "{v}");
    }

    #[test]
    fn battery_script_outputs_json() {
        let v = run(include_str!("../scripts/battery.ps1"));
        assert!(v.get("hasBattery").is_some(), "{v}");
    }

    #[test]
    fn system_script_outputs_json() {
        let v = run(include_str!("../scripts/system.ps1"));
        assert!(v.get("os").map(|o| !o.is_null()).unwrap_or(false), "os kosong: {v}");
        assert!(v.get("update").is_some(), "{v}");
    }

    #[test]
    fn programs_script_outputs_json() {
        let v = run(include_str!("../scripts/programs.ps1"));
        assert!(v.get("startup").is_some(), "{v}");
    }

    #[test]
    fn security_script_outputs_json() {
        let v = run(include_str!("../scripts/security.ps1"));
        assert!(v.get("firewall").is_some() && v.get("programs").is_some(), "{v}");
    }

    #[test]
    fn network_script_outputs_json() {
        let v = run(include_str!("../scripts/network.ps1"));
        assert!(v.get("adapters").is_some(), "{v}");
    }

    #[test]
    fn full_scan_produces_findings() {
        let report = crate::scan::run(|_| {});
        assert!(!report.findings.is_empty());
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }
}
