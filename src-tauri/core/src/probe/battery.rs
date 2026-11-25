//! Data baterai laptop.

use std::time::Duration;

use super::json;
use crate::platform;

/// Jenis casing menurut SMBIOS (Win32_SystemEnclosure.ChassisTypes).
pub const LAPTOP_CHASSIS: &[u32] = &[8, 9, 10, 11, 14, 30, 31, 32];
pub const DESKTOP_CHASSIS: &[u32] = &[3, 4, 5, 6, 7, 13, 15, 16, 17, 23, 24, 35, 36];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BatteryRaw {
    pub chassis: Vec<u32>,
    pub has_battery: bool,
    pub charge_pct: Option<u32>,
    pub on_ac: Option<bool>,
    /// Kapasitas penuh sekarang (mWh).
    pub full_capacity: Option<u64>,
    /// Kapasitas saat baru (mWh).
    pub design_capacity: Option<u64>,
    pub cycles: Option<u64>,
    pub notes: Vec<String>,
}

impl BatteryRaw {
    pub fn is_laptop(&self) -> Option<bool> {
        if self.chassis.iter().any(|c| LAPTOP_CHASSIS.contains(c)) {
            Some(true)
        } else if self.chassis.iter().any(|c| DESKTOP_CHASSIS.contains(c)) {
            Some(false)
        } else {
            None
        }
    }

    /// Kesehatan baterai dalam persen (kapasitas sekarang / saat baru).
    pub fn health_pct(&self) -> Option<f64> {
        match (self.full_capacity, self.design_capacity) {
            (Some(f), Some(d)) if d > 0 && f > 0 => Some((f as f64 / d as f64 * 100.0).min(100.0)),
            _ => None,
        }
    }
}

pub fn probe() -> BatteryRaw {
    let mut raw = BatteryRaw::default();
    if !platform::IS_WINDOWS {
        return raw;
    }
    match platform::run_powershell(include_str!("../scripts/battery.ps1"), Duration::from_secs(30)) {
        Ok(out) => apply_wmi(&mut raw, &out),
        Err(e) => raw.notes.push(format!("Data baterai tidak terbaca: {e}")),
    }
    raw
}

pub fn apply_wmi(raw: &mut BatteryRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    raw.chassis = json::list_of(v.get("chassis")).into_iter().filter_map(|c| json::u64_of(Some(c))).map(|c| c as u32).collect();
    raw.has_battery = v.get("hasBattery").and_then(|b| b.as_bool()).unwrap_or(false);
    raw.charge_pct = json::u64_of(v.get("chargePct")).map(|p| p.min(100) as u32);
    raw.on_ac = v.get("onAc").and_then(|b| b.as_bool()).or_else(|| {
        // Cadangan dari kode status Win32_Battery: 1 = memakai baterai, selain itu tersambung listrik.
        json::u64_of(v.get("statusCode")).map(|c| c != 1)
    });
    raw.full_capacity = json::u64_of(v.get("fullCapacity")).filter(|c| *c > 0);
    raw.design_capacity = json::u64_of(v.get("designCapacity")).filter(|c| *c > 0);
    raw.cycles = json::u64_of(v.get("cycles")).filter(|c| *c > 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laptop_with_worn_battery() {
        let mut raw = BatteryRaw::default();
        apply_wmi(&mut raw, r#"{"chassis":[10],"hasBattery":true,"chargePct":64,"statusCode":2,"onAc":true,"fullCapacity":21000,"designCapacity":41000,"cycles":812}"#);
        assert_eq!(raw.is_laptop(), Some(true));
        assert!((raw.health_pct().unwrap() - 51.2).abs() < 0.1);
        assert_eq!(raw.on_ac, Some(true));
    }

    #[test]
    fn desktop_without_battery() {
        let mut raw = BatteryRaw::default();
        apply_wmi(&mut raw, r#"{"chassis":3,"hasBattery":false,"chargePct":null,"statusCode":null,"onAc":null,"fullCapacity":null,"designCapacity":null,"cycles":null}"#);
        assert_eq!(raw.is_laptop(), Some(false));
        assert!(!raw.has_battery && raw.health_pct().is_none());
    }

    #[test]
    fn status_code_fallback() {
        let mut raw = BatteryRaw::default();
        apply_wmi(&mut raw, r#"{"chassis":[9],"hasBattery":true,"chargePct":40,"statusCode":1,"onAc":null}"#);
        assert_eq!(raw.on_ac, Some(false));
    }
}
