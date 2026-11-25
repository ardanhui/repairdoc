//! Program yang ikut jalan saat Windows menyala (startup).

use std::time::Duration;

use super::json;
use crate::platform;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StartupEntry {
    pub name: String,
    pub command: String,
    /// "HKLM\\...\\Run", "Startup", "Common Startup", "StoreApp", …
    pub location: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct StartupRaw {
    pub entries: Vec<StartupEntry>,
    pub notes: Vec<String>,
}

impl StartupRaw {
    pub fn enabled(&self) -> Vec<&StartupEntry> {
        self.entries.iter().filter(|e| e.enabled).collect()
    }
}

pub fn probe() -> StartupRaw {
    let mut raw = StartupRaw::default();
    if !platform::IS_WINDOWS {
        return raw;
    }
    match platform::run_powershell(include_str!("../scripts/programs.ps1"), Duration::from_secs(40)) {
        Ok(out) => apply_wmi(&mut raw, &out),
        Err(e) => raw.notes.push(format!("Daftar program startup tidak terbaca: {e}")),
    }
    raw
}

pub fn apply_wmi(raw: &mut StartupRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    raw.entries = json::list_of(v.get("startup"))
        .into_iter()
        .map(|e| StartupEntry {
            name: json::str_of(e.get("name")),
            command: json::str_of(e.get("command")),
            location: json::str_of(e.get("location")),
            enabled: e.get("enabled").and_then(|b| b.as_bool()).unwrap_or(true),
        })
        .filter(|e| !e.name.is_empty())
        .collect();
    // Satu program bisa terdaftar dua kali (HKLM dan HKCU); hitung sekali.
    raw.entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(b.enabled.cmp(&a.enabled)));
    raw.entries.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name) && a.command.eq_ignore_ascii_case(&b.command));
}

/// Nama yang enak dibaca dari entri startup: "OneDrive" dari "OneDrive",
/// "CCleaner64" dari `"C:\\Program Files\\CCleaner\\CCleaner64.exe" /MONITOR`, dsb.
pub fn display_name(e: &StartupEntry) -> String {
    let n = e.name.trim();
    if !n.is_empty() && !n.contains('\\') {
        return n.to_string();
    }
    exe_name(&e.command).or_else(|| exe_name(n)).unwrap_or_else(|| n.to_string())
}

/// Nama file program (tanpa .exe) dari sebuah baris perintah.
pub fn exe_name(command: &str) -> Option<String> {
    let c = command.trim();
    let path = match c.strip_prefix('"') {
        Some(rest) => rest.split('"').next().unwrap_or(rest),
        None => c.split_whitespace().next().unwrap_or(c),
    };
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path).trim();
    if file.is_empty() {
        return None;
    }
    let lower = file.to_ascii_lowercase();
    let base = if lower.ends_with(".exe") { &file[..file.len() - 4] } else { file };
    Some(base.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_dedups() {
        let mut raw = StartupRaw::default();
        apply_wmi(
            &mut raw,
            r#"{"startup":[
                {"name":"OneDrive","command":"\"C:\\Users\\x\\OneDrive.exe\" /background","location":"HKU\\S-1-5-21\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run","enabled":true},
                {"name":"OneDrive","command":"\"C:\\Users\\x\\OneDrive.exe\" /background","location":"HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run","enabled":true},
                {"name":"Spotify","command":"","location":"StoreApp","enabled":false},
                {"name":"","command":"x","location":"Startup","enabled":true}
            ]}"#,
        );
        assert_eq!(raw.entries.len(), 2);
        assert_eq!(raw.enabled().len(), 1);
    }

    #[test]
    fn display_names() {
        let e = StartupEntry { name: "C:\\Program Files\\CCleaner\\CCleaner64.exe".into(), command: "\"C:\\Program Files\\CCleaner\\CCleaner64.exe\" /MONITOR".into(), ..Default::default() };
        assert_eq!(display_name(&e), "CCleaner64");
        let e = StartupEntry { name: "Discord".into(), ..Default::default() };
        assert_eq!(display_name(&e), "Discord");
        assert_eq!(exe_name("C:\\Users\\x\\AppData\\Local\\Spotify\\Spotify.EXE --minimized"), Some("Spotify".into()));
        assert_eq!(exe_name(""), None);
    }
}
