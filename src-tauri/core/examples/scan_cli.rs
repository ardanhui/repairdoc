//! Jalankan scan tanpa tampilan dan cetak hasilnya sebagai JSON.
//! Berguna untuk mengecek bagian diagnosa di Windows:
//!     cd src-tauri && cargo run -p repairdoc-core --example scan_cli
fn main() {
    let report = repairdoc_core::scan::run(|ev| eprintln!("{}", serde_json::to_string(&ev).unwrap()));
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
