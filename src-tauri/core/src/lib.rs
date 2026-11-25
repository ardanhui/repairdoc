//! RepairDoc core: pemeriksaan kesehatan PC.
//!
//! Dipisah dari aplikasi Tauri supaya logika diagnosa bisa dites tanpa
//! jendela aplikasi (`cargo test -p repairdoc-core`) dan bisa dipakai ulang
//! nanti (misalnya versi bootable di fase 2).
//!
//! - `probe`   : membaca data mentah dari PC (tidak mengubah apa pun)
//! - `analyze` : menilai data mentah → temuan berbahasa awam
//! - `scan`    : alur scan lengkap dengan kabar progres

pub mod analyze;
pub mod dates;
pub mod format;
pub mod model;
pub mod platform;
pub mod probe;
pub mod scan;

pub use model::*;
