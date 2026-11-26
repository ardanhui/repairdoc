//! Pengganti untuk sistem selain Windows (dipakai saat pengembangan & tes).

use std::time::Duration;

pub fn run_powershell(_script: &str, _timeout: Duration) -> Result<String, String> {
    Err("PowerShell hanya tersedia di Windows".to_string())
}

pub fn run_powershell_script(_script: &str, _args: &str, _env: &[(&str, &str)], _timeout: Duration) -> Result<String, String> {
    Err("PowerShell hanya tersedia di Windows".to_string())
}

pub fn is_elevated() -> bool {
    false
}

pub fn relaunch_elevated() -> Result<(), String> {
    Err("Hanya tersedia di Windows".to_string())
}

pub const IS_WINDOWS: bool = false;
