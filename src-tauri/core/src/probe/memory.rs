//! Data RAM: total, terpakai, slot, dan hasil tes memori Windows.

use std::time::Duration;

use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use super::json;
use crate::platform;

#[derive(Clone, Debug, Default)]
pub struct MemModule {
    pub capacity: u64,
    pub speed_mhz: Option<u64>,
    pub maker: String,
    pub slot: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemTest {
    pub errors_found: bool,
    /// Format "YYYY-MM-DD".
    pub date: String,
}

#[derive(Clone, Debug, Default)]
pub struct MemRaw {
    /// Total yang bisa dipakai Windows (bisa sedikit di bawah yang terpasang).
    pub total: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    /// Total keping RAM yang terpasang secara fisik (dari WMI).
    pub installed: Option<u64>,
    pub slots_total: Option<u32>,
    pub slots_used: Option<u32>,
    pub modules: Vec<MemModule>,
    pub memtest: Option<MemTest>,
    pub notes: Vec<String>,
}

impl MemRaw {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn used_pct(&self) -> Option<f64> {
        (self.total > 0).then(|| self.used() as f64 / self.total as f64 * 100.0)
    }
}

pub fn probe() -> MemRaw {
    let mut raw = MemRaw::default();

    match System::new_with_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::everything())) {
        Ok(sys) => {
            raw.total = sys.total_memory();
            raw.available = sys.available_memory();
            raw.swap_total = sys.total_swap();
            raw.swap_used = sys.used_swap();
        }
        Err(e) => raw.notes.push(format!("Data memori tidak terbaca: {e}")),
    }

    if platform::IS_WINDOWS {
        match platform::run_powershell(include_str!("../scripts/memory.ps1"), Duration::from_secs(30)) {
            Ok(out) => apply_wmi(&mut raw, &out),
            Err(e) => raw.notes.push(format!("Info keping RAM tidak terbaca: {e}")),
        }
    }

    raw
}

/// Gabungkan keluaran `memory.ps1` ke data mentah.
pub fn apply_wmi(raw: &mut MemRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    raw.installed = json::u64_of(v.get("installedBytes")).filter(|b| *b > 0);
    raw.slots_total = json::u64_of(v.get("slotsTotal")).map(|n| n as u32).filter(|n| *n > 0);
    raw.slots_used = json::u64_of(v.get("slotsUsed")).map(|n| n as u32).filter(|n| *n > 0);
    raw.modules = json::list_of(v.get("modules"))
        .into_iter()
        .map(|m| MemModule {
            capacity: json::u64_of(m.get("capacityBytes")).unwrap_or(0),
            speed_mhz: json::u64_of(m.get("speedMhz")).filter(|s| *s > 0),
            maker: json::str_of(m.get("maker")),
            slot: json::str_of(m.get("slot")),
        })
        .collect();
    if let Some(t) = v.get("memtest").filter(|t| !t.is_null()) {
        match json::u64_of(t.get("eventId")) {
            Some(1201) => raw.memtest = Some(MemTest { errors_found: false, date: json::str_of(t.get("date")) }),
            Some(1202) => raw.memtest = Some(MemTest { errors_found: true, date: json::str_of(t.get("date")) }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wmi_output() {
        let mut raw = MemRaw::default();
        apply_wmi(
            &mut raw,
            r#"{"installedBytes":4294967296,"slotsTotal":2,"slotsUsed":1,
                "modules":{"capacityBytes":4294967296,"speedMhz":1600,"maker":"Samsung","slot":"DIMM 1"},
                "memtest":{"eventId":1202,"date":"2026-03-14"}}"#,
        );
        assert_eq!(raw.installed, Some(4 * 1024 * 1024 * 1024));
        assert_eq!(raw.slots_total, Some(2));
        assert_eq!(raw.modules.len(), 1, "objek tunggal harus dibaca sebagai daftar");
        assert_eq!(raw.memtest, Some(MemTest { errors_found: true, date: "2026-03-14".into() }));
    }

    #[test]
    fn tolerates_missing_fields() {
        let mut raw = MemRaw::default();
        apply_wmi(&mut raw, r#"{"installedBytes":null,"slotsTotal":null,"slotsUsed":null,"modules":[],"memtest":null}"#);
        assert!(raw.installed.is_none() && raw.memtest.is_none() && raw.modules.is_empty());
    }

    #[test]
    fn probe_reads_memory() {
        let raw = probe();
        assert!(raw.total > 0);
        assert!(raw.used_pct().unwrap() <= 100.0);
    }
}
