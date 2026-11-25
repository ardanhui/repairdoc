//! Data kartu grafis.

use std::time::Duration;

use super::json;
use crate::platform;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdapterRaw {
    pub name: String,
    pub vram: Option<u64>,
    pub driver_version: String,
    /// "YYYY-MM-DD"
    pub driver_date: Option<String>,
    pub status: String,
    /// Kode error Device Manager (0 = normal).
    pub error_code: Option<u32>,
    pub resolution: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NvidiaRaw {
    pub name: String,
    pub temp_c: Option<f64>,
    pub utilization_pct: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct GpuRaw {
    pub adapters: Vec<AdapterRaw>,
    pub nvidia: Option<NvidiaRaw>,
    pub notes: Vec<String>,
}

pub fn probe() -> GpuRaw {
    let mut raw = GpuRaw::default();
    if !platform::IS_WINDOWS {
        return raw;
    }
    match platform::run_powershell(include_str!("../scripts/gpu.ps1"), Duration::from_secs(30)) {
        Ok(out) => apply_wmi(&mut raw, &out),
        Err(e) => raw.notes.push(format!("Data kartu grafis tidak terbaca: {e}")),
    }
    raw
}

pub fn apply_wmi(raw: &mut GpuRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    raw.adapters = json::list_of(v.get("adapters"))
        .into_iter()
        .map(|a| AdapterRaw {
            name: json::str_of(a.get("name")),
            vram: json::u64_of(a.get("vramBytes")).filter(|b| *b > 0),
            driver_version: json::str_of(a.get("driverVersion")),
            driver_date: Some(json::str_of(a.get("driverDate"))).filter(|s| !s.is_empty()),
            status: json::str_of(a.get("status")),
            error_code: json::u64_of(a.get("errorCode")).map(|c| c as u32),
            resolution: match (json::u64_of(a.get("width")), json::u64_of(a.get("height"))) {
                (Some(w), Some(h)) if w > 0 && h > 0 => Some((w as u32, h as u32)),
                _ => None,
            },
        })
        .collect();
    if let Some(n) = v.get("nvidia").filter(|n| !n.is_null()) {
        raw.nvidia = Some(NvidiaRaw {
            name: json::str_of(n.get("name")),
            temp_c: json::f64_of(n.get("temperatureC")).filter(|t| *t > 0.0 && *t < 130.0),
            utilization_pct: json::f64_of(n.get("utilizationPct")),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_adapters_and_nvidia() {
        let mut raw = GpuRaw::default();
        apply_wmi(
            &mut raw,
            r#"{"adapters":{"name":"NVIDIA GeForce GT 710","vramBytes":2147483648,"driverVersion":"31.0.15.3598",
                "driverDate":"2023-07-21","status":"OK","errorCode":0,"width":1366,"height":768},
                "nvidia":{"name":"NVIDIA GeForce GT 710","temperatureC":47,"utilizationPct":3,"driver":"535.98"}}"#,
        );
        assert_eq!(raw.adapters.len(), 1);
        assert_eq!(raw.adapters[0].resolution, Some((1366, 768)));
        assert_eq!(raw.adapters[0].error_code, Some(0));
        assert_eq!(raw.nvidia.as_ref().unwrap().temp_c, Some(47.0));
    }

    #[test]
    fn tolerates_nulls() {
        let mut raw = GpuRaw::default();
        apply_wmi(&mut raw, r#"{"adapters":[{"name":"Microsoft Basic Display Adapter","vramBytes":null,"driverVersion":"10.0.19041.1","driverDate":null,"status":"OK","errorCode":null,"width":null,"height":null}],"nvidia":null}"#);
        assert_eq!(raw.adapters[0].vram, None);
        assert!(raw.nvidia.is_none());
    }
}
