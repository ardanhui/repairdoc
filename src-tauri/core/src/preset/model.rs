//! Struktur data Mode Performa: target perubahan, nilai, catatan (log), dan
//! jawaban untuk tampilan.

use serde::{Deserialize, Serialize};

/// Tiga mode, seperti pengaturan grafis di game.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Low — performa maksimal, tampilan dikorbankan.
    Kentang,
    /// Medium — sebagian efek dimatikan.
    Seimbang,
    /// High — kembali ke bawaan Windows / membatalkan perubahan RepairDoc.
    Normal,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Kentang, Mode::Seimbang, Mode::Normal];

    pub fn title(self) -> &'static str {
        match self {
            Mode::Kentang => "Mode Kentang",
            Mode::Seimbang => "Mode Seimbang",
            Mode::Normal => "Mode Normal",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hive {
    #[serde(rename = "HKCU")]
    Hkcu,
    #[serde(rename = "HKLM")]
    Hklm,
}

impl Hive {
    pub fn short(self) -> &'static str {
        match self {
            Hive::Hkcu => "HKCU",
            Hive::Hklm => "HKLM",
        }
    }
}

/// Apa yang diubah.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Target {
    /// Satu nilai registry.
    Registry { hive: Hive, path: String, name: String },
    /// Jenis start sebuah Windows service (lewat `sc.exe`).
    Service { name: String },
    /// Power plan yang aktif (lewat `powercfg`).
    PowerPlan,
}

impl Target {
    pub fn reg(hive: Hive, path: &str, name: &str) -> Self {
        Target::Registry { hive, path: path.to_string(), name: name.to_string() }
    }

    /// Bentuk yang bisa dibaca teknisi, mis. `HKCU\Control Panel\Desktop\WindowMetrics → MinAnimate`.
    pub fn text(&self) -> String {
        match self {
            Target::Registry { hive, path, name } => format!("{}\\{path} → {name}", hive.short()),
            Target::Service { name } => format!("Service {name}"),
            Target::PowerPlan => "Power plan aktif".to_string(),
        }
    }
}

fn yes() -> bool {
    true
}

/// Nilai sebuah target, sebelum atau sesudah diubah.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Value {
    /// Nilai registry tidak ada. `key_existed = false` berarti kuncinya pun belum ada,
    /// sehingga saat dikembalikan kunci kosong itu ikut dihapus.
    Absent {
        #[serde(rename = "keyExisted", default = "yes")]
        key_existed: bool,
    },
    Dword {
        value: u32,
    },
    String {
        value: String,
    },
    /// Data biner sebagai teks heksadesimal huruf kecil, mis. "03000000…".
    Binary {
        hex: String,
    },
    /// Jenis start service: "auto", "delayed-auto", "demand", "disabled", "boot", "system".
    Service {
        start: String,
    },
    /// GUID power plan (huruf kecil).
    Power {
        guid: String,
    },
    /// Service / power plan tidak ada di PC ini.
    Missing,
    /// Jenis nilai registry yang tidak ditangani RepairDoc (tidak akan diubah).
    Unsupported {
        #[serde(default)]
        kind: String,
    },
}

impl Value {
    pub fn absent() -> Self {
        Value::Absent { key_existed: true }
    }

    pub fn dword(v: u32) -> Self {
        Value::Dword { value: v }
    }

    pub fn string(s: &str) -> Self {
        Value::String { value: s.to_string() }
    }

    pub fn binary(bytes: &[u8]) -> Self {
        Value::Binary { hex: bytes.iter().map(|b| format!("{b:02x}")).collect() }
    }

    pub fn service(start: &str) -> Self {
        Value::Service { start: start.to_string() }
    }

    pub fn power(guid: &str) -> Self {
        Value::Power { guid: guid.to_ascii_lowercase() }
    }

    /// Sama secara makna (dipakai untuk "sudah sesuai"). `Absent` dianggap sama
    /// walau `key_existed` berbeda; heksadesimal & GUID tidak peka huruf besar.
    pub fn same_as(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Absent { .. }, Value::Absent { .. }) => true,
            (Value::Dword { value: a }, Value::Dword { value: b }) => a == b,
            (Value::String { value: a }, Value::String { value: b }) => a == b,
            (Value::Binary { hex: a }, Value::Binary { hex: b }) => a.eq_ignore_ascii_case(b),
            (Value::Service { start: a }, Value::Service { start: b }) => a == b,
            (Value::Power { guid: a }, Value::Power { guid: b }) => a.eq_ignore_ascii_case(b),
            (Value::Missing, Value::Missing) => true,
            _ => false,
        }
    }

    /// Byte pertama data biner (untuk status StartupApproved).
    pub fn first_byte(&self) -> Option<u8> {
        match self {
            Value::Binary { hex } if hex.len() >= 2 => u8::from_str_radix(&hex[..2], 16).ok(),
            _ => None,
        }
    }
}

/// Satu perubahan yang dikirim ke eksekutor (`preset.ps1`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Op {
    /// Kunci unik di dalam satu rencana.
    pub key: String,
    pub target: Target,
    /// Nilai yang dituju.
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpStatus {
    /// Diubah dan sudah dicek ulang.
    Changed,
    /// Nilainya sudah sama dengan tujuan — tidak diubah.
    Same,
    /// (Simulasi) akan diubah.
    Planned,
    /// Target tidak ada di PC ini (service tidak terpasang, power plan tidak tersedia).
    NotPresent,
    /// Ditolak pengaman (di luar daftar izin / service terlindung).
    Refused,
    Failed,
}

/// Jawaban eksekutor untuk satu `Op`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub key: String,
    #[serde(default)]
    pub before: Option<Value>,
    #[serde(default)]
    pub after: Option<Value>,
    pub status: OpStatus,
    #[serde(default)]
    pub error: Option<String>,
}

// ---------------------------------------------------------------- inventaris

/// Identitas PC. Catatan perubahan disimpan di flashdisk dan dipakai di banyak PC,
/// jadi setiap entri ditandai PC-nya supaya "Kembalikan Semua" tidak pernah
/// mengembalikan perubahan milik PC lain.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MachineInfo {
    pub machine_id: String,
    pub computer_name: String,
}

/// Satu program di kunci `Run` beserta status StartupApproved-nya.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    /// "HKCU", "HKLM", atau "HKLM32" (WOW6432Node).
    pub source: String,
    pub name: String,
    pub command: String,
    /// Data biner StartupApproved (heksadesimal) atau `None` bila belum ada (= aktif).
    pub approved_hex: Option<String>,
}

/// Satu aplikasi Microsoft Store milik pengguna.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppItem {
    pub name: String,
    pub family_name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    /// "Store", "System", "Developer", …
    pub signature_kind: String,
    pub non_removable: bool,
    /// Nilai `Disabled` di BackgroundAccessApplications\<familyName> (None = tidak ada).
    pub disabled: Option<u32>,
    pub disabled_by_user: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub machine: MachineInfo,
    pub startup: Vec<StartupItem>,
    pub apps: Vec<AppItem>,
    /// Nilai `GlobalUserDisabled` (dipasang alat lain); hanya dipakai Mode Normal.
    pub global_bg_disabled: Option<u32>,
}

// ---------------------------------------------------------------- catatan (log)

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeStatus {
    Berhasil,
    Gagal,
    Dilewati,
}

/// Satu baris perubahan di catatan. Nama kolom mengikuti spesifikasi:
/// `tweakId, target, nilaiLama, nilaiBaru, status`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRecord {
    pub tweak_id: String,
    /// Satu item yang dikenali pengguna (mis. satu aplikasi) bisa terdiri dari beberapa nilai.
    #[serde(default)]
    pub item_key: String,
    pub label: String,
    pub target: Target,
    pub target_teks: String,
    pub nilai_lama: Option<Value>,
    pub nilai_baru: Value,
    pub status: ChangeStatus,
    #[serde(default)]
    pub keterangan: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// Menerapkan mode (termasuk Mode Normal saat belum ada catatan).
    Terapkan,
    /// Mode Normal yang membatalkan catatan sebelumnya.
    Kembalikan,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    /// MachineGuid PC tempat perubahan dilakukan.
    pub mesin: String,
    pub nama_pc: String,
    /// Milidetik sejak 1970.
    pub waktu: u64,
    pub mode: Mode,
    pub jenis: EntryKind,
    pub restore_point: Option<String>,
    pub perubahan: Vec<ChangeRecord>,
    #[serde(default)]
    pub dikembalikan: bool,
    #[serde(default)]
    pub waktu_dikembalikan: Option<u64>,
    /// Hasil "Kembalikan Semua" terakhir untuk entri ini.
    #[serde(default)]
    pub hasil_pengembalian: Vec<ChangeRecord>,
    /// Untuk entri jenis `kembalikan`: id entri yang dibatalkan.
    #[serde(default)]
    pub membatalkan: Vec<String>,
}

/// Status per item: gagal mengalahkan berhasil, berhasil mengalahkan dilewati.
pub fn item_statuses(records: &[ChangeRecord]) -> Vec<(String, ChangeStatus)> {
    let rank = |s: ChangeStatus| match s {
        ChangeStatus::Gagal => 2,
        ChangeStatus::Berhasil => 1,
        ChangeStatus::Dilewati => 0,
    };
    let mut out: Vec<(String, ChangeStatus)> = Vec::new();
    for r in records {
        let key = if r.item_key.is_empty() { r.target_teks.clone() } else { r.item_key.clone() };
        match out.iter_mut().find(|(k, _)| *k == key) {
            Some((_, s)) if rank(r.status) > rank(*s) => *s = r.status,
            Some(_) => {}
            None => out.push((key, r.status)),
        }
    }
    out
}

impl LogEntry {
    /// Jumlah item (bukan nilai registry) dengan status ini.
    pub fn count(&self, status: ChangeStatus) -> usize {
        item_statuses(&self.perubahan).iter().filter(|(_, s)| *s == status).count()
    }

    /// Entri ini punya perubahan sungguhan yang bisa dibatalkan.
    pub fn revertible(&self) -> bool {
        self.jenis == EntryKind::Terapkan && !self.dikembalikan && self.count(ChangeStatus::Berhasil) > 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresetLog {
    #[serde(default = "log_version")]
    pub versi: u32,
    #[serde(default)]
    pub entri: Vec<LogEntry>,
}

fn log_version() -> u32 {
    1
}

impl PresetLog {
    pub fn new() -> Self {
        PresetLog { versi: 1, entri: Vec::new() }
    }

    /// Entri milik PC ini, urut dari yang terlama.
    pub fn for_machine(&self, machine_id: &str) -> Vec<&LogEntry> {
        self.entri.iter().filter(|e| e.mesin == machine_id).collect()
    }

    /// Entri terakhir yang bisa dibatalkan (target tombol "Kembalikan Semua").
    pub fn revert_target(&self, machine_id: &str) -> Option<&LogEntry> {
        self.for_machine(machine_id).into_iter().rev().find(|e| e.revertible())
    }

    /// Mode yang sedang berlaku menurut catatan: penerapan terakhir yang belum
    /// dibatalkan. Kalau tidak ada, berarti Normal (bawaan).
    pub fn active_mode(&self, machine_id: &str) -> (Mode, Option<u64>) {
        for e in self.for_machine(machine_id).into_iter().rev() {
            match e.jenis {
                EntryKind::Kembalikan => return (Mode::Normal, Some(e.waktu)),
                EntryKind::Terapkan if !e.dikembalikan => return (e.mode, Some(e.waktu)),
                EntryKind::Terapkan => continue,
            }
        }
        (Mode::Normal, None)
    }
}

// ---------------------------------------------------------------- jawaban untuk tampilan

/// Jalur yang ditempuh sebuah rencana.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlanPath {
    /// Menerapkan tweak mode Kentang/Seimbang.
    Apply,
    /// Mode Normal: membatalkan semua perubahan RepairDoc yang tercatat.
    RevertLog,
    /// Mode Normal tanpa catatan: memasang nilai bawaan Windows.
    Defaults,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewChange {
    pub key: String,
    /// Kunci untuk mencentang/tidak mencentang satu item (program startup, aplikasi latar).
    pub item_key: String,
    pub label: String,
    pub target_text: String,
    pub current: String,
    pub new: String,
    pub will_change: bool,
    /// Bisa dikecualikan satu per satu oleh pengguna.
    pub optional: bool,
    pub note: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewGroup {
    pub tweak_id: String,
    pub title: String,
    pub description: String,
    /// Dimatikan lewat toggle — akan dilewati.
    pub skipped_by_user: bool,
    pub changes: Vec<PreviewChange>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub mode: Mode,
    pub path: PlanPath,
    pub groups: Vec<PreviewGroup>,
    /// Jumlah nilai yang akan benar-benar berubah.
    pub change_count: usize,
    pub same_count: usize,
    pub notes: Vec<String>,
    /// Contoh nama restore point yang akan dibuat.
    pub restore_point_name: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub label: String,
    pub reason: String,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    NotAdmin,
    NotWindows,
    RestorePoint,
    NothingToRevert,
    Other,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PresetError {
    pub kind: ErrorKind,
    pub message: String,
    /// Langkah yang bisa dilakukan pengguna.
    pub help: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub ok: bool,
    pub mode: Option<Mode>,
    pub error: Option<PresetError>,
    pub entry: Option<LogEntry>,
    pub changed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub failures: Vec<Failure>,
    pub restore_point: Option<String>,
    pub needs_restart: bool,
}

impl Outcome {
    pub fn error(mode: Option<Mode>, kind: ErrorKind, message: impl Into<String>, help: &[&str]) -> Self {
        Outcome {
            ok: false,
            mode,
            error: Some(PresetError { kind, message: message.into(), help: help.iter().map(|s| s.to_string()).collect() }),
            entry: None,
            changed: 0,
            skipped: 0,
            failed: 0,
            failures: Vec::new(),
            restore_point: None,
            needs_restart: false,
        }
    }
}

/// Kabar langkah saat menerapkan / mengembalikan.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", tag = "step")]
pub enum PresetEvent {
    Reading,
    RestorePoint,
    Applying { total: usize },
    Saving,
}
