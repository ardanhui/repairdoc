//! Catatan pembersihan `RepairDoc-data/cleanup-log.json`, di samping `preset-log.json`.
//! Ini CATATAN (riwayat), bukan tombol undo — file yang dihapus tidak bisa kembali.

use std::path::{Path, PathBuf};

use super::model::CleanLog;

pub const LOG_FILE: &str = "cleanup-log.json";

/// Di folder yang sama dengan catatan Mode Performa (flashdisk, atau TEMP bila tidak bisa ditulisi).
pub fn default_path() -> PathBuf {
    crate::preset::log::default_path().with_file_name(LOG_FILE)
}

/// Baca catatan. File rusak disimpan dengan nama lain lalu diganti catatan kosong.
pub fn load(path: &Path) -> CleanLog {
    let Ok(text) = std::fs::read_to_string(path) else { return CleanLog::new() };
    match serde_json::from_str::<CleanLog>(&text) {
        Ok(log) => log,
        Err(_) => {
            let stamp = crate::preset::now_ms();
            let _ = std::fs::rename(path, path.with_file_name(format!("cleanup-log.rusak-{stamp}.json")));
            CleanLog::new()
        }
    }
}

/// Simpan lewat file sementara lalu ganti nama, supaya catatan tidak setengah tertulis.
pub fn save(path: &Path, log: &CleanLog) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Folder catatan tidak bisa dibuat: {e}"))?;
    }
    let text = serde_json::to_string_pretty(log).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Catatan tidak bisa ditulis: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Catatan tidak bisa disimpan: {e}"))
}
