//! Data contoh Mode Performa untuk mode pratinjau (`npm run dev` tanpa Tauri),
//! dibuat dari alur yang asli dengan backend tiruan (`preset::fake`), supaya
//! teks di tampilan persis sama dengan yang dihasilkan aplikasi.
//! Tahap 4: PC tiruan memakai keempat sumber startup, dan ada bagian `cleanup` untuk
//! layar Bersihkan File Sampah (pratinjau dari aturan yang asli, angka contoh).
//!     cargo run -p repairdoc-core --example mock_preset > ../src/lib/mock-preset.json
use std::path::PathBuf;

use repairdoc_core::cleaner::{self, CleanEntry, CleanStatus, Jenis, KindRecord, KindStatus};
use repairdoc_core::preset::fake::FakeBackend;
use repairdoc_core::preset::*;
use repairdoc_core::probe::cleanup::{CleanupRaw, FolderSize, JunkEntry, JunkKind};
use serde_json::{json, Value as Json};

/// 26 November 2026, 13.50 WIB.
const T: u64 = 1_795_675_800_000;
const SHOWN_LOG_PATH: &str = r"E:\RepairDoc\RepairDoc-data\preset-log.json";

fn status_json(b: &FakeBackend, log: &std::path::Path) -> Json {
    let mut st = status(b, log);
    st.log_path = SHOWN_LOG_PATH.into();
    serde_json::to_value(st).unwrap()
}

fn main() {
    let dir = std::env::temp_dir().join(format!("repairdoc-mock-preset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let opts = PlanOptions::default();

    let fresh = |name: &str| {
        let p = dir.join(format!("{name}.json"));
        (FakeBackend::potato_pc_lengkap(), p)
    };

    let (b0, l0) = fresh("awal");
    let initial = status_json(&b0, &l0);
    let mut previews = serde_json::Map::new();
    for m in Mode::ALL {
        let key = serde_json::to_value(m).unwrap().as_str().unwrap().to_string();
        previews.insert(key, serde_json::to_value(preview(&b0, &l0, m, &opts, T).unwrap()).unwrap());
    }

    let mut after = serde_json::Map::new();
    for m in Mode::ALL {
        let key = serde_json::to_value(m).unwrap().as_str().unwrap().to_string();
        let (b, l) = fresh(&key);
        let outcome = apply(&b, &l, m, &opts, T, &mut |_| {});
        let st = status_json(&b, &l);
        let normal = preview(&b, &l, Mode::Normal, &opts, T + 60_000).unwrap();
        let revert = revert_last(&b, &l, T + 120_000, &mut |_| {});
        after.insert(
            key,
            json!({
                "outcome": outcome,
                "status": st,
                "previewNormal": normal,
                "revert": revert,
                "statusAfterRevert": status_json(&b, &l),
            }),
        );
    }

    let mut broken = FakeBackend::potato_pc_lengkap();
    broken.restore_ok = false;
    let restore_failed = apply(&broken, &dir.join("gagal.json"), Mode::Kentang, &opts, T, &mut |_| {});

    let out = json!({
        "status": initial,
        "preview": previews,
        "after": after,
        "restoreFailed": restore_failed,
        "cleanup": cleanup_mock(),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------- Bersihkan File Sampah

const MB: u64 = 1024 * 1024;

fn entry(kind: JunkKind, path: &str, mb: u64, files: u64) -> JunkEntry {
    JunkEntry { kind, path: PathBuf::from(path), size: FolderSize { bytes: mb * MB, files, denied: 0, truncated: false } }
}

fn rec(jenis: Jenis, lokasi: &[&str], files: u64, mb: u64, skipped: u64, status: KindStatus, notes: &[&str]) -> KindRecord {
    KindRecord {
        jenis,
        label: jenis.label().to_string(),
        lokasi: lokasi.iter().map(|s| s.to_string()).collect(),
        file_terhapus: files,
        byte_dibebaskan: mb * MB,
        dilewati: skipped,
        status,
        catatan: notes.iter().map(|s| s.to_string()).collect(),
    }
}

fn cleanup_mock() -> Json {
    let temp = [r"C:\Users\Budi\AppData\Local\Temp", r"C:\Windows\Temp"];
    let browser = [
        r"C:\Users\Budi\AppData\Local\Microsoft\Windows\INetCache",
        r"C:\Users\Budi\AppData\Local\Google\Chrome\User Data\Default\Cache",
        r"C:\Users\Budi\AppData\Local\Google\Chrome\User Data\Default\Code Cache",
        r"C:\Users\Budi\AppData\Local\Microsoft\Edge\User Data\Default\Cache",
    ];
    let update = [r"C:\Windows\SoftwareDistribution\Download"];
    let recycle = [r"C:\$Recycle.Bin", r"D:\$Recycle.Bin"];
    let raw = CleanupRaw {
        entries: vec![
            entry(JunkKind::Temp, temp[0], 1270, 8412),
            entry(JunkKind::Temp, temp[1], 312, 1203),
            entry(JunkKind::UpdateCache, update[0], 1946, 213),
            entry(JunkKind::WindowsOld, r"C:\Windows.old", 19_046, 104_522),
            entry(JunkKind::BrowserCache, browser[0], 96, 2210),
            entry(JunkKind::BrowserCache, browser[1], 640, 3120),
            entry(JunkKind::BrowserCache, browser[2], 112, 960),
            entry(JunkKind::BrowserCache, browser[3], 288, 1544),
            entry(JunkKind::RecycleBin, recycle[0], 2355, 47),
            entry(JunkKind::RecycleBin, recycle[1], 410, 12),
        ],
        hiberfil: Some(3277 * MB),
        pagefile: Some(2560 * MB),
        notes: Vec::new(),
    };
    // Windows.old dibuat 20 November 2026 (6 hari sebelum contoh).
    let preview = cleaner::plan::preview_from(&raw, &["Edge".to_string()], Some(T - 6 * 86_400_000));
    let records = json!({
        "temp": rec(Jenis::Temp, &temp, 7904, 1418, 1711, KindStatus::Sebagian, &[
            "96 file dilewati karena sedang dipakai.",
            "1.615 file dilewati karena masih baru (24 jam terakhir).",
        ]),
        "browserCache": rec(Jenis::BrowserCache, &browser, 6187, 838, 1647, KindStatus::Sebagian, &[
            "Cache Edge dilewati karena browsernya sedang terbuka.",
            "103 file dilewati karena sedang dipakai.",
        ]),
        "updateCache": rec(Jenis::UpdateCache, &update, 211, 1931, 2, KindStatus::Sebagian, &["2 file dilewati karena sedang dipakai."]),
        "recycleBin": rec(Jenis::RecycleBin, &recycle, 59, 2765, 0, KindStatus::Berhasil, &[]),
        "windowsOld": rec(Jenis::WindowsOld, &[r"C:\Windows.old"], 104_522, 19_046, 0, KindStatus::Berhasil, &[]),
    });
    let status = CleanStatus { is_windows: true, is_admin: true, last: None, log_path: r"E:\RepairDoc\RepairDoc-data\cleanup-log.json".into(), error: None };
    let sample = CleanEntry { id: format!("{T}-bersihkan"), mesin: "0b3c5e2a-1111-4c1d-9e0f-REPAIRDOC".into(), nama_pc: "LAPTOP-BUDI".into(), waktu: T, jenis: Vec::new() };
    json!({ "status": status, "preview": preview, "records": records, "entryTemplate": sample })
}
