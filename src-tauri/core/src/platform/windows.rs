//! Implementasi khusus Windows.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use base64::Engine;

/// Supaya jendela hitam (console) tidak berkedip saat PowerShell dijalankan.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn system32(rel: &str) -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(root).join("System32").join(rel)
}

/// Jalankan skrip PowerShell dan ambil keluarannya (biasanya JSON).
///
/// Skrip dikirim lewat `-EncodedCommand` supaya aman dari masalah tanda kutip,
/// dan PowerShell dipanggil dengan path lengkap (bukan dari PATH) supaya tidak
/// tertukar dengan program lain di PC yang diperiksa.
pub fn run_powershell(script: &str, timeout: Duration) -> Result<String, String> {
    let full = format!(
        "$ErrorActionPreference='SilentlyContinue'\n$ProgressPreference='SilentlyContinue'\n\
         [Console]::OutputEncoding=[System.Text.Encoding]::UTF8\n{script}"
    );
    let utf16: Vec<u8> = full.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);

    let child = Command::new(system32(r"WindowsPowerShell\v1.0\powershell.exe"))
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &encoded])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("PowerShell tidak bisa dijalankan: {e}"))?;

    super::wait_with_timeout(child, timeout, "PowerShell")
}

/// Apakah RepairDoc berjalan dengan izin Administrator?
/// Dicek dari tingkat integritas proses (High = S-1-16-12288, System = S-1-16-16384).
pub fn is_elevated() -> bool {
    let child = Command::new(system32("whoami.exe"))
        .args(["/groups", "/fo", "csv", "/nh"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    match child {
        Ok(child) => match super::wait_with_timeout(child, Duration::from_secs(10), "whoami") {
            Ok(out) => out.contains("S-1-16-12288") || out.contains("S-1-16-16384"),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Jalankan ulang RepairDoc dengan izin Administrator (memunculkan jendela UAC).
/// Mengembalikan Ok bila jendela UAC disetujui dan program baru sudah dijalankan.
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let q = |s: &str| s.replace('\'', "''");
    let script = format!(
        "try {{ Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs -ErrorAction Stop; exit 0 }} catch {{ exit 1 }}",
        q(&exe.to_string_lossy()),
        q(&dir.to_string_lossy())
    );
    run_powershell(&script, Duration::from_secs(120))
        .map(|_| ())
        .map_err(|_| "Izin administrator tidak diberikan.".to_string())
}

pub const IS_WINDOWS: bool = true;
