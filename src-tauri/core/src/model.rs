//! Struktur data hasil diagnosa yang dikirim ke tampilan.
//!
//! Prinsip: semua teks yang dilihat pengguna awam ada di `Finding`
//! (judul, ringkasan, alasan, langkah). Angka-angka teknis hanya masuk
//! ke `TechSection` ("Info untuk teknisi") yang tertutup secara default.

use serde::Serialize;

/// Status per temuan. Urutan penting: dipakai untuk mengurutkan
/// temuan dari yang paling bermasalah.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Sehat,
    Perhatian,
    Bermasalah,
}

/// Seberapa parah sebuah masalah. `None` untuk temuan yang sehat.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Ringan,
    Sedang,
    Berat,
}

/// Kelompok besar yang dilihat pengguna di layar scan (5 pil progres).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Hardware,
    Windows,
    Cleanup,
    Security,
    Network,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Hardware => "Hardware",
            Group::Windows => "Windows",
            Group::Cleanup => "Kebersihan",
            Group::Security => "Keamanan",
            Group::Network => "Jaringan",
        }
    }
}

/// Kategori pemeriksaan (urutan = urutan scan).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Cpu,
    Memory,
    Storage,
    Gpu,
    Battery,
    System,
    Programs,
    Cleanup,
    Security,
    Network,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Cpu,
        Category::Memory,
        Category::Storage,
        Category::Gpu,
        Category::Battery,
        Category::System,
        Category::Programs,
        Category::Cleanup,
        Category::Security,
        Category::Network,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Cpu => "Prosesor",
            Category::Memory => "Memori (RAM)",
            Category::Storage => "Penyimpanan",
            Category::Gpu => "Kartu grafis",
            Category::Battery => "Baterai",
            Category::System => "Windows",
            Category::Programs => "Program",
            Category::Cleanup => "Kebersihan",
            Category::Security => "Keamanan",
            Category::Network => "Jaringan",
        }
    }

    pub fn group(self) -> Group {
        match self {
            Category::Cpu | Category::Memory | Category::Storage | Category::Gpu | Category::Battery => Group::Hardware,
            Category::System | Category::Programs => Group::Windows,
            Category::Cleanup => Group::Cleanup,
            Category::Security => Group::Security,
            Category::Network => Group::Network,
        }
    }
}

/// Satu temuan diagnosa, ditulis untuk orang awam.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// ID stabil, misalnya "storage.volume.C" — berguna untuk laporan & tahap preset.
    pub id: String,
    pub category: Category,
    pub status: Status,
    pub severity: Option<Severity>,
    /// Judul singkat: "Drive C: hampir penuh".
    pub title: String,
    /// Apa yang terjadi, satu-dua kalimat bahasa sehari-hari.
    pub summary: String,
    /// Kenapa ini jadi masalah. Kosong untuk temuan sehat.
    pub why: Option<String>,
    /// Langkah yang disarankan, berurutan.
    pub steps: Vec<String>,
}

impl Finding {
    pub fn healthy(id: impl Into<String>, category: Category, title: impl Into<String>, summary: impl Into<String>) -> Self {
        Finding {
            id: id.into(),
            category,
            status: Status::Sehat,
            severity: None,
            title: title.into(),
            summary: summary.into(),
            why: None,
            steps: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn problem(
        id: impl Into<String>,
        category: Category,
        status: Status,
        severity: Severity,
        title: impl Into<String>,
        summary: impl Into<String>,
        why: impl Into<String>,
        steps: &[&str],
    ) -> Self {
        Finding {
            id: id.into(),
            category,
            status,
            severity: Some(severity),
            title: title.into(),
            summary: summary.into(),
            why: Some(why.into()),
            steps: steps.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Satu baris info teknis ("Model prosesor" → "Intel Core i3-4005U").
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TechItem {
    pub label: String,
    pub value: String,
}

/// Kelompok info teknis per kategori, untuk teknisi.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TechSection {
    pub title: String,
    pub items: Vec<TechItem>,
}

impl TechSection {
    pub fn new(title: impl Into<String>) -> Self {
        TechSection { title: title.into(), items: Vec::new() }
    }

    pub fn push(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.items.push(TechItem { label: label.into(), value: value.into() });
    }
}

/// Hasil satu kategori pemeriksaan.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryResult {
    pub category: Category,
    pub findings: Vec<Finding>,
    pub tech: TechSection,
    /// Hal yang tidak bisa diperiksa tanpa izin administrator, mis. "umur dan suhu disk".
    pub needs_admin: Vec<String>,
}

impl CategoryResult {
    pub fn new(category: Category, findings: Vec<Finding>, tech: TechSection) -> Self {
        CategoryResult { category, findings, tech, needs_admin: Vec::new() }
    }

    /// Status terburuk di kategori ini.
    pub fn worst(&self) -> Status {
        self.findings.iter().map(|f| f.status).max().unwrap_or(Status::Sehat)
    }
}

/// Kesimpulan besar di bagian atas hasil.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub status: Status,
    /// "PC ini perlu sedikit perhatian"
    pub headline: String,
    /// "Ada 1 masalah dan 2 hal yang perlu diperhatikan."
    pub detail: String,
    /// Temuan terpenting, kalau ada: "Yang paling penting: Drive C: hampir penuh."
    pub top_priority: Option<String>,
    pub problem_count: usize,
    pub attention_count: usize,
    pub healthy_count: usize,
}

/// Laporan lengkap satu kali scan.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub verdict: Verdict,
    /// Diurutkan: bermasalah → perhatian → sehat, lalu dari yang paling berat.
    pub findings: Vec<Finding>,
    pub tech: Vec<TechSection>,
    /// Apakah scan berjalan dengan izin administrator.
    pub is_admin: bool,
    /// Yang belum bisa diperiksa karena tanpa izin administrator (kosong bila admin).
    pub needs_admin: Vec<String>,
    /// Waktu selesai scan (milidetik sejak 1970), diformat di tampilan.
    pub finished_at_ms: u64,
    pub duration_ms: u64,
}

/// Kabar progres yang dikirim ke tampilan selama scan.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ScanEvent {
    CategoryStarted { category: Category },
    CategoryFinished { category: Category, status: Status },
    CategoryFailed { category: Category },
    /// Kategori yang memang tidak berlaku di PC ini, mis. baterai di PC desktop.
    CategorySkipped { category: Category, reason: String },
}
