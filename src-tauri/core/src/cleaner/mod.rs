//! Bersihkan File Sampah (tahap 4).
//!
//! Dipisah dari `probe::cleanup`, yang tetap HANYA mengukur: modul ini memakai daftar
//! lokasi & jenis yang sama (`probe::cleanup::locations`, `JunkKind`) dan fungsi ukur
//! yang sama untuk pratinjau, lalu menghapus ISI lokasi itu.
//! - `model`  : jenis sampah, pratinjau, catatan, hasil
//! - `plan`   : aturan murni (lokasi yang sah, Recycle Bin, file baru, browser terbuka)
//! - `exec`   : penghapus (tidak mengikuti symlink/junction, file terkunci dilewati) +
//!   Windows.old lewat Disk Cleanup + backend sungguhan
//! - `engine` : alur periksa izin → pratinjau → hapus per jenis → catat
//! - `log`    : `cleanup-log.json` (catatan, bukan undo)
//!
//! Yang tidak pernah dihapus: Windows.old tanpa centang konfirmasinya sendiri,
//! `hiberfil.sys` & `pagefile.sys` (hanya info), file yang sedang dipakai, apa pun di
//! luar lokasi dari `locations()`.

pub mod engine;
pub mod exec;
pub mod log;
pub mod model;
pub mod plan;

pub use engine::{apply, preview, status, CleanBackend};
pub use model::*;
