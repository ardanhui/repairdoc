// Sembunyikan jendela console hitam di versi rilis (Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use repairdoc_core::{platform, scan, ScanEvent, ScanReport};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::window::Color;
use tauri::{AppHandle, State, WebviewUrl, WebviewWindowBuilder};

/// Info yang diketahui saat aplikasi mulai.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    is_admin: bool,
    version: String,
    is_windows: bool,
}

/// Folder tempat RepairDoc.exe berada (biasanya di flashdisk).
fn portable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".tulis-tes");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Folder data WebView. Disimpan di flashdisk (samping exe) supaya tidak
/// meninggalkan jejak di PC yang diperiksa. Kalau flashdisk tidak bisa ditulisi,
/// pakai folder TEMP. Admin dan non-admin memakai folder berbeda supaya tidak bentrok
/// saat aplikasi dijalankan ulang sebagai admin.
fn webview_data_dir(is_admin: bool) -> PathBuf {
    let name = if is_admin { "webview-admin" } else { "webview" };
    let candidates = [
        portable_dir().join("RepairDoc-data").join(name),
        std::env::temp_dir().join("RepairDoc").join(name),
    ];
    for dir in candidates {
        if std::fs::create_dir_all(&dir).is_ok() && is_writable(&dir) {
            return dir;
        }
    }
    std::env::temp_dir().join("RepairDoc")
}

/// Untuk PC tanpa WebView2 (Windows 10 lama yang tidak pernah update):
/// kalau ada folder `WebView2` berisi "Fixed Version Runtime" di samping exe, pakai itu.
fn use_bundled_webview2() {
    let dir = portable_dir().join("WebView2");
    if dir.join("msedgewebview2.exe").exists() {
        std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", &dir);
    }
}

#[tauri::command]
fn app_info(info: State<'_, AppInfo>) -> AppInfo {
    info.inner().clone()
}

/// Jalankan scan (hanya membaca). Progres dikirim lewat `on_event`.
#[tauri::command]
async fn start_scan(on_event: Channel<ScanEvent>) -> Result<ScanReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scan::run(|ev| {
            let _ = on_event.send(ev);
        })
    })
    .await
    .map_err(|e| format!("Scan gagal: {e}"))
}

/// Minta izin admin (jendela UAC). Kalau disetujui, RepairDoc versi admin dibuka
/// dan jendela ini ditutup.
#[tauri::command]
async fn relaunch_as_admin(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(platform::relaunch_elevated)
        .await
        .map_err(|e| e.to_string())??;
    app.exit(0);
    Ok(())
}

fn main() {
    use_bundled_webview2();
    let is_admin = platform::is_elevated();
    let info = AppInfo { is_admin, version: env!("CARGO_PKG_VERSION").to_string(), is_windows: platform::IS_WINDOWS };

    tauri::Builder::default()
        .manage(info)
        .invoke_handler(tauri::generate_handler![app_info, start_scan, relaunch_as_admin])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("RepairDoc — Dokter PC")
                .inner_size(1120.0, 780.0)
                .min_inner_size(900.0, 620.0)
                .center()
                // Warna langit, supaya tidak ada kilatan putih saat membuka di PC lambat.
                .background_color(Color(199, 233, 250, 255))
                .data_directory(webview_data_dir(is_admin))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("RepairDoc gagal dijalankan");
}
