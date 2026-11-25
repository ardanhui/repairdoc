//! Data Windows: versi, update, driver bermasalah, waktu boot, power plan.

use std::time::Duration;

use super::json;
use crate::platform;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OsRaw {
    /// "Microsoft Windows 10 Pro"
    pub caption: String,
    /// "10.0.19045"
    pub version: String,
    pub build: u32,
    pub ubr: Option<u32>,
    /// "22H2"
    pub display_version: Option<String>,
    pub arch: String,
    pub install_date: Option<String>,
    pub last_boot: Option<String>,
}

impl OsRaw {
    /// Windows 11 memakai build ≥ 22000 walau versinya tetap "10.0".
    pub fn major(&self) -> Option<u32> {
        let caption = self.caption.to_ascii_lowercase();
        if caption.contains("windows 11") || self.build >= 22000 {
            Some(11)
        } else if caption.contains("windows 10") || (10240..22000).contains(&self.build) {
            Some(10)
        } else if caption.contains("windows 8") {
            Some(8)
        } else if caption.contains("windows 7") {
            Some(7)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateRaw {
    pub last_success: Option<String>,
    pub last_hotfix: Option<String>,
    pub last_event: Option<String>,
    /// 2 otomatis, 3 manual, 4 dimatikan.
    pub wu_start: Option<u32>,
    pub reboot_pending: bool,
}

impl UpdateRaw {
    /// Tanggal update terakhir yang paling baru dari ketiga sumber.
    pub fn latest(&self) -> Option<String> {
        [&self.last_success, &self.last_hotfix, &self.last_event]
            .into_iter()
            .flatten()
            .filter(|d| crate::dates::parse_ymd(d).is_some())
            .max_by_key(|d| crate::dates::parse_ymd(d))
            .cloned()
    }

    pub fn service_disabled(&self) -> bool {
        self.wu_start == Some(4)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PnpErrorRaw {
    pub name: String,
    pub code: u32,
    pub class: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BootRaw {
    pub date: String,
    pub boot_ms: Option<u64>,
    pub main_path_ms: Option<u64>,
    pub post_boot_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerPlan {
    Balanced,
    HighPerformance,
    PowerSaver,
    Ultimate,
    Other,
}

pub fn power_plan_from_guid(guid: &str) -> PowerPlan {
    match guid.trim().to_ascii_lowercase().as_str() {
        "381b4222-f694-41f0-9685-ff5bb260df2e" => PowerPlan::Balanced,
        "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c" => PowerPlan::HighPerformance,
        "a1841308-3541-4fab-bc81-f71556f20b4a" => PowerPlan::PowerSaver,
        "e9a42b02-d5df-448d-aa00-03f14749eb61" => PowerPlan::Ultimate,
        _ => PowerPlan::Other,
    }
}

#[derive(Clone, Debug, Default)]
pub struct SystemRaw {
    pub os: Option<OsRaw>,
    pub update: UpdateRaw,
    pub power_plan: Option<PowerPlan>,
    pub power_guid: Option<String>,
    pub fast_startup: Option<bool>,
    pub page_file_auto: Option<bool>,
    pub pc_model: Option<String>,
    pub pnp_errors: Vec<PnpErrorRaw>,
    pub boot: Option<BootRaw>,
    /// Apakah log waktu boot sempat dicoba dan gagal (biasanya karena bukan admin).
    pub boot_unreadable: bool,
    pub uptime_secs: Option<u64>,
    pub notes: Vec<String>,
}

pub fn probe(uptime_secs: Option<u64>) -> SystemRaw {
    let mut raw = SystemRaw { uptime_secs, ..Default::default() };
    if !platform::IS_WINDOWS {
        return raw;
    }
    match platform::run_powershell(include_str!("../scripts/system.ps1"), Duration::from_secs(90)) {
        Ok(out) => apply_wmi(&mut raw, &out),
        Err(e) => raw.notes.push(format!("Data Windows tidak terbaca: {e}")),
    }
    raw
}

fn opt_str(v: Option<&serde_json::Value>) -> Option<String> {
    Some(json::str_of(v)).filter(|s| !s.is_empty())
}

pub fn apply_wmi(raw: &mut SystemRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    if let Some(os) = v.get("os").filter(|o| !o.is_null()) {
        raw.os = Some(OsRaw {
            caption: json::str_of(os.get("caption")),
            version: json::str_of(os.get("version")),
            build: json::u64_of(os.get("build")).unwrap_or(0) as u32,
            ubr: json::u64_of(os.get("ubr")).map(|u| u as u32),
            display_version: opt_str(os.get("displayVersion")),
            arch: json::str_of(os.get("arch")),
            install_date: opt_str(os.get("installDate")),
            last_boot: opt_str(os.get("lastBoot")),
        });
    }
    if let Some(u) = v.get("update") {
        raw.update = UpdateRaw {
            last_success: opt_str(u.get("lastSuccess")),
            last_hotfix: opt_str(u.get("lastHotfix")),
            last_event: opt_str(u.get("lastEvent")),
            wu_start: json::u64_of(u.get("wuStart")).map(|s| s as u32),
            reboot_pending: u.get("rebootPending").and_then(|b| b.as_bool()).unwrap_or(false),
        };
    }
    raw.power_guid = opt_str(v.get("powerScheme"));
    raw.power_plan = raw.power_guid.as_deref().map(power_plan_from_guid);
    raw.fast_startup = v.get("fastStartup").and_then(|b| b.as_bool());
    raw.page_file_auto = v.get("pageFileAuto").and_then(|b| b.as_bool());
    raw.pc_model = opt_str(v.get("pcModel"));
    raw.pnp_errors = json::list_of(v.get("pnpErrors"))
        .into_iter()
        .map(|d| PnpErrorRaw {
            name: json::str_of(d.get("name")),
            code: json::u64_of(d.get("code")).unwrap_or(0) as u32,
            class: json::str_of(d.get("class")),
        })
        .filter(|d| d.code != 0)
        .collect();
    match v.get("boot").filter(|b| !b.is_null()) {
        Some(b) => {
            raw.boot = Some(BootRaw {
                date: json::str_of(b.get("date")),
                boot_ms: json::u64_of(b.get("bootTimeMs")).filter(|m| *m > 0),
                main_path_ms: json::u64_of(b.get("mainPathMs")).filter(|m| *m > 0),
                post_boot_ms: json::u64_of(b.get("postBootMs")).filter(|m| *m > 0),
            })
        }
        None => raw.boot_unreadable = true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"os":{"caption":"Microsoft Windows 10 Pro","version":"10.0.19045","build":19045,"ubr":4046,"displayVersion":"22H2","arch":"64-bit","installDate":"2021-05-02","lastBoot":"2026-11-20 07:12"},
        "update":{"lastSuccess":"2025-09-10","lastHotfix":"2025-10-15","lastEvent":null,"wuStart":4,"rebootPending":true},
        "powerScheme":"a1841308-3541-4fab-bc81-f71556f20b4a","fastStartup":true,"pageFileAuto":true,"pcModel":"ASUSTeK COMPUTER INC. X441SA",
        "pnpErrors":[{"name":"Unknown device","code":28,"class":"Unknown"}],
        "boot":{"date":"2026-11-20","bootTimeMs":143000,"mainPathMs":98000,"postBootMs":45000}}"#;

    #[test]
    fn parses_everything() {
        let mut raw = SystemRaw::default();
        apply_wmi(&mut raw, SAMPLE);
        let os = raw.os.as_ref().unwrap();
        assert_eq!(os.major(), Some(10));
        assert_eq!(os.display_version.as_deref(), Some("22H2"));
        assert_eq!(raw.update.latest().as_deref(), Some("2025-10-15"));
        assert!(raw.update.service_disabled() && raw.update.reboot_pending);
        assert_eq!(raw.power_plan, Some(PowerPlan::PowerSaver));
        assert_eq!(raw.pnp_errors.len(), 1);
        assert_eq!(raw.boot.as_ref().unwrap().main_path_ms, Some(98_000));
        assert!(!raw.boot_unreadable);
    }

    #[test]
    fn windows_11_detected_by_build() {
        let os = OsRaw { caption: "Microsoft Windows 10 Home".into(), build: 22631, ..Default::default() };
        assert_eq!(os.major(), Some(11));
    }

    #[test]
    fn missing_boot_log_is_marked() {
        let mut raw = SystemRaw::default();
        apply_wmi(&mut raw, r#"{"os":null,"update":{},"boot":null,"pnpErrors":[]}"#);
        assert!(raw.boot_unreadable && raw.os.is_none() && raw.update.latest().is_none());
    }

    #[test]
    fn power_guids() {
        assert_eq!(power_plan_from_guid("8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C"), PowerPlan::HighPerformance);
        assert_eq!(power_plan_from_guid("abc"), PowerPlan::Other);
    }
}
