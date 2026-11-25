//! Data keamanan: Defender / antivirus lain, firewall, UAC, program terpasang.

use std::time::Duration;

use super::json;
use crate::platform;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DefenderRaw {
    pub service_enabled: bool,
    pub antivirus_enabled: bool,
    pub real_time: bool,
    pub signature_age_days: Option<u64>,
    pub signature_date: Option<String>,
    /// `None` = belum pernah scan (Windows melaporkan angka raksasa).
    pub quick_scan_age_days: Option<u64>,
    pub full_scan_age_days: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AvProductRaw {
    pub name: String,
    pub enabled: bool,
    pub up_to_date: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgramRaw {
    pub name: String,
    pub publisher: String,
    /// "20240314"
    pub install_date: String,
    pub size_kb: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct SecurityRaw {
    pub defender: Option<DefenderRaw>,
    pub av_products: Vec<AvProductRaw>,
    pub firewall_domain: Option<bool>,
    pub firewall_private: Option<bool>,
    pub firewall_public: Option<bool>,
    pub uac_enabled: Option<bool>,
    pub programs: Vec<ProgramRaw>,
    pub notes: Vec<String>,
}

impl SecurityRaw {
    pub fn firewall_all_known(&self) -> bool {
        self.firewall_private.is_some() && self.firewall_public.is_some()
    }

    /// Ada profil firewall (privat/publik) yang mati?
    pub fn firewall_off_somewhere(&self) -> bool {
        self.firewall_private == Some(false) || self.firewall_public == Some(false)
    }
}

pub fn probe() -> SecurityRaw {
    let mut raw = SecurityRaw::default();
    if !platform::IS_WINDOWS {
        return raw;
    }
    match platform::run_powershell(include_str!("../scripts/security.ps1"), Duration::from_secs(60)) {
        Ok(out) => apply_wmi(&mut raw, &out),
        Err(e) => raw.notes.push(format!("Data keamanan tidak terbaca: {e}")),
    }
    raw
}

/// Umur scan: Windows memberi angka raksasa (4294967295) bila belum pernah.
fn age(v: Option<&serde_json::Value>) -> Option<u64> {
    json::u64_of(v).filter(|d| *d < 36_500)
}

pub fn apply_wmi(raw: &mut SecurityRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    let b = |x: Option<&serde_json::Value>| x.and_then(|b| b.as_bool());
    if let Some(d) = v.get("defender").filter(|d| !d.is_null()) {
        raw.defender = Some(DefenderRaw {
            service_enabled: b(d.get("serviceEnabled")).unwrap_or(false),
            antivirus_enabled: b(d.get("antivirusEnabled")).unwrap_or(false),
            real_time: b(d.get("realTime")).unwrap_or(false),
            signature_age_days: age(d.get("signatureAgeDays")),
            signature_date: Some(json::str_of(d.get("signatureDate"))).filter(|s| !s.is_empty()),
            quick_scan_age_days: age(d.get("quickScanAgeDays")),
            full_scan_age_days: age(d.get("fullScanAgeDays")),
        });
    }
    raw.av_products = json::list_of(v.get("avProducts"))
        .into_iter()
        .map(|a| AvProductRaw {
            name: json::str_of(a.get("name")),
            enabled: b(a.get("enabled")).unwrap_or(false),
            up_to_date: b(a.get("upToDate")).unwrap_or(true),
        })
        .filter(|a| !a.name.is_empty())
        .collect();
    if let Some(f) = v.get("firewall") {
        raw.firewall_domain = b(f.get("domain"));
        raw.firewall_private = b(f.get("private"));
        raw.firewall_public = b(f.get("public"));
    }
    raw.uac_enabled = b(v.get("uacEnabled"));
    raw.programs = json::list_of(v.get("programs"))
        .into_iter()
        .map(|p| ProgramRaw {
            name: json::str_of(p.get("name")),
            publisher: json::str_of(p.get("publisher")),
            install_date: json::str_of(p.get("installDate")),
            size_kb: json::u64_of(p.get("sizeKb")),
        })
        .filter(|p| !p.name.is_empty())
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_output() {
        let mut raw = SecurityRaw::default();
        apply_wmi(
            &mut raw,
            r#"{"defender":{"serviceEnabled":true,"antivirusEnabled":true,"realTime":false,"signatureAgeDays":45,"signatureDate":"2026-10-11","quickScanAgeDays":4294967295,"fullScanAgeDays":12},
                "avProducts":[{"name":"Windows Defender","state":"061000","enabled":true,"upToDate":true},{"name":"Smadav Antivirus","state":"266000","enabled":true,"upToDate":true}],
                "firewall":{"domain":true,"private":false,"public":true},"uacEnabled":false,
                "programs":{"name":"KMSpico","publisher":"","installDate":"20240314","sizeKb":12000}}"#,
        );
        let d = raw.defender.as_ref().unwrap();
        assert!(!d.real_time);
        assert_eq!(d.quick_scan_age_days, None, "angka raksasa = belum pernah scan");
        assert_eq!(d.full_scan_age_days, Some(12));
        assert_eq!(raw.av_products.len(), 2);
        assert!(raw.firewall_off_somewhere());
        assert_eq!(raw.uac_enabled, Some(false));
        assert_eq!(raw.programs.len(), 1);
    }

    #[test]
    fn missing_defender_cmdlet() {
        let mut raw = SecurityRaw::default();
        apply_wmi(&mut raw, r#"{"defender":null,"avProducts":[],"firewall":{"domain":null,"private":null,"public":null},"uacEnabled":null,"programs":[]}"#);
        assert!(raw.defender.is_none() && !raw.firewall_all_known());
    }
}
