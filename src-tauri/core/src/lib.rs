//! RepairDoc core: pemeriksaan kesehatan PC.
//!
//! Dipisah dari aplikasi Tauri supaya logika diagnosa bisa dites tanpa
//! jendela aplikasi (`cargo test -p repairdoc-core`) dan bisa dipakai ulang
//! nanti (misalnya versi bootable di fase 2).
//!
//! - `probe`   : membaca data mentah dari PC (tidak mengubah apa pun)
//! - `analyze` : menilai data mentah → temuan berbahasa awam
//! - `scan`    : alur scan lengkap dengan kabar progres
//! - `preset`  : Mode Performa (tahap 3, startup diperluas di tahap 4) — MENGUBAH pengaturan,
//!   selalu dengan pratinjau, restore point, dan catatan untuk dikembalikan
//! - `cleaner` : Bersihkan File Sampah (tahap 4) — MENGHAPUS file sampah di lokasi yang
//!   diukur scan, selalu dengan pratinjau, centang per jenis, dan catatan
//!
//! Hanya `preset` dan `cleaner` yang mengubah PC; `probe` dan `scan` tetap hanya membaca.

pub mod analyze;
pub mod cleaner;
pub mod dates;
pub mod format;
pub mod model;
pub mod platform;
pub mod preset;
pub mod probe;
pub mod scan;

pub use model::*;
