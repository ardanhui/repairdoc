//! Catatan perubahan: `RepairDoc-data/preset-log.json` di samping exe.
//! Ditulis atomik (file sementara lalu rename) supaya tidak pernah setengah jadi.

use std::path::{Path, PathBuf};

use super::model::PresetLog;

pub const LOG_FILE: &str = "preset-log.json";

/// Lokasi catatan: `<folder exe>\RepairDoc-data\preset-log.json`, sama dengan folder
/// data yang sudah dipakai tampilan. Kalau flashdisk tidak bisa ditulisi, pakai TEMP.
pub fn default_path() -> PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let mut candidates = Vec::new();
    if let Some(d) = exe_dir {
        candidates.push(d.join("RepairDoc-data"));
    }
    candidates.push(std::env::temp_dir().join("RepairDoc"));
    for dir in candidates {
        if std::fs::create_dir_all(&dir).is_ok() && writable(&dir) {
            return dir.join(LOG_FILE);
        }
    }
    std::env::temp_dir().join(LOG_FILE)
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(".tulis-tes-preset");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Baca catatan. File tidak ada → catatan kosong. File rusak → disimpan dengan
/// nama lain (tidak dihapus, supaya teknisi masih bisa membacanya) lalu mulai baru.
pub fn load(path: &Path) -> PresetLog {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return PresetLog::new(),
    };
    match serde_json::from_str::<PresetLog>(&text) {
        Ok(log) => log,
        Err(_) => {
            let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
            let _ = std::fs::rename(path, path.with_file_name(format!("preset-log.rusak-{ms}.json")));
            PresetLog::new()
        }
    }
}

pub fn save(path: &Path, log: &PresetLog) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("folder catatan tidak bisa dibuat: {e}"))?;
    }
    let text = serde_json::to_string_pretty(log).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("catatan tidak bisa ditulis: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("catatan tidak bisa disimpan: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::model::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Folder uji unik per tes (tes berjalan paralel), dihapus otomatis.
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp(test: &str) -> TempDir {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!("repairdoc-presetlog-{test}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        TempDir(d)
    }

    fn entry(id: &str) -> LogEntry {
        LogEntry {
            id: id.into(),
            mesin: "PC-1".into(),
            nama_pc: "BUDI".into(),
            waktu: 1,
            mode: Mode::Kentang,
            jenis: EntryKind::Terapkan,
            restore_point: Some("RepairDoc Mode Kentang 2026-11-26 13:50".into()),
            perubahan: vec![ChangeRecord {
                tweak_id: "transparency-off".into(),
                item_key: "transparency-off:EnableTransparency".into(),
                label: "x".into(),
                target: Target::reg(Hive::Hkcu, "A", "B"),
                target_teks: "HKCU\\A → B".into(),
                nilai_lama: Some(Value::Absent { key_existed: false }),
                nilai_baru: Value::dword(0),
                status: ChangeStatus::Berhasil,
                keterangan: None,
            }],
            dikembalikan: false,
            waktu_dikembalikan: None,
            hasil_pengembalian: vec![],
            membatalkan: vec![],
        }
    }

    #[test]
    fn roundtrip_and_spec_field_names() {
        let t = temp("roundtrip");
        let p = t.0.join("sub").join(LOG_FILE);
        let mut log = PresetLog::new();
        log.entri.push(entry("a"));
        save(&p, &log).unwrap();
        assert_eq!(load(&p), log);
        let text = std::fs::read_to_string(&p).unwrap();
        for field in ["\"tweakId\"", "\"target\"", "\"nilaiLama\"", "\"nilaiBaru\"", "\"status\": \"berhasil\"", "\"restorePoint\"", "\"keyExisted\": false"] {
            assert!(text.contains(field), "kolom {field} harus ada di catatan");
        }
        assert!(!t.0.join("sub").join("preset-log.json.tmp").exists(), "file sementara tidak boleh tertinggal");
    }

    #[test]
    fn missing_file_is_empty_log() {
        let t = temp("missing");
        assert_eq!(load(&t.0.join(LOG_FILE)), PresetLog::new());
    }

    #[test]
    fn corrupt_file_is_kept_aside() {
        let t = temp("corrupt");
        let p = t.0.join(LOG_FILE);
        std::fs::write(&p, "{ bukan json").unwrap();
        assert_eq!(load(&p), PresetLog::new());
        let kept = std::fs::read_dir(&t.0).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("preset-log.rusak-"));
        assert!(kept, "file rusak harus disimpan dengan nama lain, bukan dihapus");
    }

    #[test]
    fn per_machine_queries() {
        let mut log = PresetLog::new();
        let mut other = entry("lain");
        other.mesin = "PC-2".into();
        log.entri.push(entry("a"));
        log.entri.push(other);
        assert_eq!(log.revert_target("PC-1").unwrap().id, "a");
        assert_eq!(log.revert_target("PC-2").unwrap().id, "lain");
        assert!(log.revert_target("PC-3").is_none());
        assert_eq!(log.active_mode("PC-1").0, Mode::Kentang);
        assert_eq!(log.active_mode("PC-3").0, Mode::Normal);
    }
}
