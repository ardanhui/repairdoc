//! Struktur data "Bersihkan File Sampah": jenis sampah, pratinjau, catatan, hasil.

use serde::{Deserialize, Serialize};

use crate::preset::{Failure, PresetError};
use crate::probe::cleanup::JunkKind;

/// Jenis sampah — sama persis dengan `probe::cleanup::JunkKind` (yang hanya mengukur).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Jenis {
    Temp,
    BrowserCache,
    UpdateCache,
    RecycleBin,
    /// Tidak pernah masuk daftar hapus biasa — hanya lewat konfirmasi terpisah.
    WindowsOld,
}

impl Jenis {
    /// Jenis yang boleh ada di daftar hapus biasa, dalam urutan penghapusan.
    /// Windows.old SENGAJA tidak ada di sini.
    pub const NORMAL: [Jenis; 4] = [Jenis::Temp, Jenis::BrowserCache, Jenis::UpdateCache, Jenis::RecycleBin];

    pub fn from_kind(k: JunkKind) -> Self {
        match k {
            JunkKind::Temp => Jenis::Temp,
            JunkKind::BrowserCache => Jenis::BrowserCache,
            JunkKind::RecycleBin => Jenis::RecycleBin,
            JunkKind::UpdateCache => Jenis::UpdateCache,
            JunkKind::WindowsOld => Jenis::WindowsOld,
        }
    }

    pub fn kind(self) -> JunkKind {
        match self {
            Jenis::Temp => JunkKind::Temp,
            Jenis::BrowserCache => JunkKind::BrowserCache,
            Jenis::RecycleBin => JunkKind::RecycleBin,
            Jenis::UpdateCache => JunkKind::UpdateCache,
            Jenis::WindowsOld => JunkKind::WindowsOld,
        }
    }

    pub fn label(self) -> &'static str {
        self.kind().label()
    }

    /// Satu kalimat bahasa awam.
    pub fn description(self) -> &'static str {
        match self {
            Jenis::Temp => "File yang ditinggalkan program dan installer setelah selesai dipakai.",
            Jenis::BrowserCache => "Salinan gambar dan halaman web yang disimpan browser. Akan dibuat ulang otomatis saat browsing.",
            Jenis::UpdateCache => "Unduhan Windows Update yang sudah dipasang. Kalau masih dibutuhkan, Windows mengunduhnya lagi.",
            Jenis::RecycleBin => "File yang sudah kamu hapus dan masih menunggu di Recycle Bin.",
            Jenis::WindowsOld => "Salinan Windows sebelum upgrade.",
        }
    }
}

/// Satu jenis sampah di pratinjau.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KindPreview {
    pub jenis: Jenis,
    pub label: String,
    pub description: String,
    pub bytes: u64,
    pub files: u64,
    /// Angkanya batas bawah (penghitungan terpotong waktu atau ada folder tak terbaca).
    pub approximate: bool,
    /// Folder yang akan dibersihkan isinya.
    pub locations: Vec<String>,
    /// Tercentang saat layar dibuka. Recycle Bin TIDAK.
    pub selected_by_default: bool,
    pub notes: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowsOldPreview {
    pub bytes: u64,
    pub files: u64,
    pub approximate: bool,
    pub location: String,
    /// Kapan folder Windows.old dibuat (≈ tanggal upgrade), bila terbaca.
    pub created_ms: Option<u64>,
    pub warning: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanPreview {
    pub kinds: Vec<KindPreview>,
    pub windows_old: Option<WindowsOldPreview>,
    /// File sistem — HANYA info, tidak pernah dihapus.
    pub hiberfil: Option<u64>,
    pub pagefile: Option<u64>,
    /// Program browser yang sedang terbuka (cache-nya akan dilewati).
    pub running_browsers: Vec<String>,
    pub notes: Vec<String>,
}

/// Pilihan pengguna di layar Bersihkan.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CleanRequest {
    /// Jenis yang dicentang. `windowsOld` di sini DIABAIKAN.
    pub kinds: Vec<Jenis>,
    /// Centang konfirmasi terpisah untuk Windows.old.
    pub windows_old: bool,
}

// ---------------------------------------------------------------- catatan (cleanup-log.json)

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KindStatus {
    /// Semua yang boleh dihapus terhapus.
    Berhasil,
    /// Sebagian terhapus, sebagian dilewati (terkunci, masih baru, browser terbuka…).
    Sebagian,
    /// Tidak ada yang terhapus karena galat.
    Gagal,
}

/// Satu jenis di catatan. Nama kolom mengikuti spesifikasi:
/// `jenis, lokasi, fileTerhapus, byteDibebaskan, dilewati, status`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KindRecord {
    pub jenis: Jenis,
    pub label: String,
    pub lokasi: Vec<String>,
    pub file_terhapus: u64,
    /// Ukuran NYATA file yang terhapus (bukan perkiraan).
    pub byte_dibebaskan: u64,
    /// File yang dilewati (sedang dipakai, masih baru, browser terbuka, tidak ada izin).
    pub dilewati: u64,
    pub status: KindStatus,
    #[serde(default)]
    pub catatan: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanEntry {
    pub id: String,
    pub mesin: String,
    pub nama_pc: String,
    /// Milidetik sejak 1970.
    pub waktu: u64,
    pub jenis: Vec<KindRecord>,
}

impl CleanEntry {
    pub fn freed(&self) -> u64 {
        self.jenis.iter().map(|k| k.byte_dibebaskan).sum()
    }

    pub fn deleted(&self) -> u64 {
        self.jenis.iter().map(|k| k.file_terhapus).sum()
    }

    pub fn skipped(&self) -> u64 {
        self.jenis.iter().map(|k| k.dilewati).sum()
    }
}

/// Isi `cleanup-log.json`. CATATAN, bukan tombol undo: file yang dihapus tidak bisa kembali.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanLog {
    #[serde(default = "log_version")]
    pub versi: u32,
    #[serde(default)]
    pub entri: Vec<CleanEntry>,
}

fn log_version() -> u32 {
    1
}

impl CleanLog {
    pub fn new() -> Self {
        CleanLog { versi: 1, entri: Vec::new() }
    }

    pub fn last_for(&self, machine_id: &str) -> Option<&CleanEntry> {
        self.entri.iter().rev().find(|e| e.mesin == machine_id)
    }
}

// ---------------------------------------------------------------- jawaban untuk tampilan

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanOutcome {
    pub ok: bool,
    pub error: Option<PresetError>,
    pub entry: Option<CleanEntry>,
    pub freed_bytes: u64,
    pub deleted_files: u64,
    pub skipped_files: u64,
    pub failures: Vec<Failure>,
}

impl CleanOutcome {
    pub fn error(e: PresetError) -> Self {
        CleanOutcome { ok: false, error: Some(e), entry: None, freed_bytes: 0, deleted_files: 0, skipped_files: 0, failures: Vec::new() }
    }
}

/// Kabar langkah saat membersihkan.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", tag = "step")]
pub enum CleanEvent {
    Preparing,
    Cleaning { jenis: Jenis, label: String, index: usize, total: usize },
    Saving,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanStatus {
    pub is_windows: bool,
    pub is_admin: bool,
    /// Pembersihan terakhir di PC ini.
    pub last: Option<CleanEntry>,
    pub log_path: String,
    pub error: Option<String>,
}
