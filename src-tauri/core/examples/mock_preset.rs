//! Data contoh Mode Performa untuk mode pratinjau (`npm run dev` tanpa Tauri),
//! dibuat dari alur yang asli dengan backend tiruan (`preset::fake`), supaya
//! teks di tampilan persis sama dengan yang dihasilkan aplikasi.
//!     cargo run -p repairdoc-core --example mock_preset > ../src/lib/mock-preset.json
use repairdoc_core::preset::fake::FakeBackend;
use repairdoc_core::preset::*;
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
        (FakeBackend::potato_pc(), p)
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

    let mut broken = FakeBackend::potato_pc();
    broken.restore_ok = false;
    let restore_failed = apply(&broken, &dir.join("gagal.json"), Mode::Kentang, &opts, T, &mut |_| {});

    let out = json!({
        "status": initial,
        "preview": previews,
        "after": after,
        "restoreFailed": restore_failed,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}
