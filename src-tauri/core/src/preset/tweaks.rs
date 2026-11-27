//! Definisi tweak sebagai DATA. Tampilan dan eksekutor tidak menyimpan aksi
//! sendiri — semuanya dibaca dari daftar di file ini.
//!
//! Cara baca nilai lama & cara kembali sama untuk semua tweak sejenis:
//! - registry : nilai dibaca dulu (atau dicatat "tidak ada"), lalu saat dikembalikan
//!   ditulis ulang persis (atau dihapus bila sebelumnya tidak ada);
//! - service  : `sc.exe qc` dicatat, dikembalikan dengan `sc.exe config … start= <lama>`;
//! - power    : GUID power plan aktif dicatat, dikembalikan dengan `powercfg /setactive <lama>`.

use serde::Serialize;

use super::model::{FileLocation, Hive, Mode, Target, Value};

/// Nilai registry di definisi (bentuk statis dari `Value`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegVal {
    Dword(u32),
    Str(&'static str),
    Bin(&'static [u8]),
    Absent,
}

impl RegVal {
    pub fn to_value(self) -> Value {
        match self {
            RegVal::Dword(v) => Value::dword(v),
            RegVal::Str(s) => Value::string(s),
            RegVal::Bin(b) => Value::binary(b),
            RegVal::Absent => Value::absent(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RegSetting {
    pub hive: Hive,
    pub path: &'static str,
    pub name: &'static str,
    /// Nilai yang dipasang tweak.
    pub value: RegVal,
    /// Nilai bawaan Windows (dipakai Mode Normal bila belum ada catatan). `None` = tidak disentuh.
    pub windows_default: Option<RegVal>,
    /// Penjelasan singkat untuk daftar pratinjau.
    pub label: &'static str,
}

impl RegSetting {
    pub fn target(&self) -> Target {
        Target::reg(self.hive, self.path, self.name)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TweakKind {
    Registry(&'static [RegSetting]),
    /// Program startup non-Microsoft & non-keamanan (daftarnya dibaca saat itu juga).
    Startup,
    /// Aplikasi Store non-sistem boleh/tidak jalan di latar (dibaca saat itu juga).
    BackgroundApps,
    /// Service di daftar putih → Manual.
    Services(&'static [&'static str]),
    /// Power plan (GUID).
    Power(&'static str),
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    /// Terasa langsung.
    Langsung,
    /// Terasa setelah keluar-masuk akun (sign out) atau restart.
    SetelahSignOut,
    /// Terasa setelah PC dinyalakan ulang.
    SetelahRestart,
}

#[derive(Clone, Copy, Debug)]
pub struct TweakDef {
    pub id: &'static str,
    pub title: &'static str,
    /// Satu kalimat bahasa awam.
    pub description: &'static str,
    pub modes: &'static [Mode],
    pub kind: TweakKind,
    pub effect: Effect,
    /// Cara nilai lama dibaca (untuk teknisi).
    pub read_how: &'static str,
    /// Cara perubahan dikembalikan (untuk teknisi).
    pub revert_how: &'static str,
}

// ---------------------------------------------------------------- konstanta Windows

/// High performance — sama dengan alias `SCHEME_MIN` di `powercfg`.
pub const SCHEME_MIN: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
/// Balanced — sama dengan alias `SCHEME_BALANCED`.
pub const SCHEME_BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
/// Power saver — alias `SCHEME_MAX`.
pub const SCHEME_MAX: &str = "a1841308-3541-4fab-bc81-f71556f20b4a";

pub const KEY_VISUAL_EFFECTS: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects";
pub const KEY_DESKTOP: &str = r"Control Panel\Desktop";
pub const KEY_WINDOW_METRICS: &str = r"Control Panel\Desktop\WindowMetrics";
pub const KEY_EXPLORER_ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
pub const KEY_PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
pub const KEY_GAME_CONFIG: &str = r"System\GameConfigStore";
pub const KEY_GAMEDVR_USER: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
pub const KEY_GAMEDVR_POLICY: &str = r"SOFTWARE\Policies\Microsoft\Windows\GameDVR";
pub const KEY_BACKGROUND_APPS: &str = r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications";
pub const KEY_STARTUP_APPROVED_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
pub const KEY_STARTUP_APPROVED_RUN32: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32";
/// Kunci khusus tes otomatis. Tidak pernah dipakai tweak sungguhan.
pub const KEY_TEST_ROOT: &str = r"Software\RepairDocTest";

/// Folder cadangan shortcut startup, di dalam `RepairDoc-data` (tahap 4).
pub const STARTUP_BACKUP_DIR: &str = "startup-cadangan";
/// Tugas terjadwal di folder ini (dan anak-anaknya) tidak pernah diubah — milik Windows.
/// Daftar yang sama ada di `scripts/preset.ps1` — dicek tes.
pub const TASK_PATH_FORBIDDEN: &str = r"\Microsoft\";
/// Folder tugas khusus tes otomatis (dibuat & dihapus oleh tesnya sendiri).
pub const TASK_TEST_PATH: &str = r"\RepairDocTest\";

/// Isi `UserPreferencesMask` untuk "Adjust for best performance" (semua animasi menu,
/// tooltip, bayangan kursor, dsb. mati). Nilai yang sama dipakai WinUtil.
pub const USER_PREFERENCES_BEST_PERFORMANCE: &[u8] = &[0x90, 0x12, 0x03, 0x80, 0x10, 0x00, 0x00, 0x00];

// ---------------------------------------------------------------- daftar aman

/// Service yang boleh dijadikan Manual. Daftar putih — tidak boleh lebih.
pub const MANUAL_SERVICES: &[&str] =
    &["XblGameSave", "XboxGipSvc", "XboxNetApiSvc", "XblAuthManager", "Fax", "RemoteRegistry", "MapsBroker", "WMPNetworkSvc"];

/// Service yang TIDAK BOLEH disentuh tweak mana pun (dicek tes & ditolak eksekutor).
pub const PROTECTED_SERVICES: &[&str] = &[
    "wuauserv",
    "WinDefend",
    "MpsSvc",
    "WlanSvc",
    "Audiosrv",
    "Spooler",
    "WSearch",
    "SysMain",
    "BITS",
    "Dhcp",
    "Dnscache",
    "EventLog",
    "PlugPlay",
    "Power",
    "RpcSs",
    // Tambahan pengaman: pendukung yang dibutuhkan service di atas.
    "AudioEndpointBuilder",
    "SecurityHealthService",
    "wscsvc",
    "Winmgmt",
    "TrustedInstaller",
    "UsoSvc",
    "WaaSMedicSvc",
];

pub fn is_protected_service(name: &str) -> bool {
    PROTECTED_SERVICES.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// Kata yang membuat sebuah program startup TIDAK PERNAH dimatikan ("aplikasi penting"
/// menurut pemilik, tahap 4): Microsoft, Windows, keamanan, dan antivirus.
/// Dicek pada nama DAN perintah/jalur program, jadi program yang file-nya ada di folder
/// `C:\Windows` juga ikut terlindung (jalurnya mengandung "windows").
pub const PROTECTED_WORDS: &[&str] = &[
    "microsoft",
    "windows",
    "security",
    // Antivirus & keamanan
    "defender",
    "antivirus",
    "anti-virus",
    "virus",
    "smadav",
    "avast",
    "avg",
    "kaspersky",
    "eset",
    "mcafee",
    "norton",
    "bitdefender",
    "avira",
    "sophos",
    "malwarebytes",
    "trend micro",
    "panda",
    "comodo",
    "360 total",
];

/// Aplikasi tambahan vendor hardware (audio, touchpad, grafis, Bluetooth, tombol Fn).
/// Tahap 3 melindunginya; sejak tahap 4 aplikasi ini BOLEH jadi kandidat startup yang
/// dimatikan (keputusan pemilik), karena yang dimatikan hanya APLIKASI PEMBUKA / ikon
/// tray-nya, bukan drivernya: driver audio, touchpad, dan kartu grafis tetap jalan
/// sebagai driver/service Windows. Efeknya ditampilkan di pratinjau
/// (`startup_effect_note`) dan tiap item bisa dibatalkan centangnya.
///
/// Daftar ini TETAP dipakai untuk aplikasi latar (tweak `background-apps-off`), supaya
/// perilaku tahap 3 di sana tidak berubah.
pub const HARDWARE_HELPER_WORDS: &[&str] = &[
    "realtek",
    "rtkaud",
    "rtkngui",
    "ravcpl",
    "waves",
    "maxxaudio",
    "dolby",
    "conexant",
    "synaptics",
    "syntp",
    "elan",
    "etdctrl",
    "alps",
    "touchpad",
    "igfx",
    "hkcmd",
    "persistence",
    "intel",
    "nvidia",
    "nvcpl",
    "amd",
    "radeon",
    "bluetooth",
    "btray",
    "hotkey",
    "atkkey",
    "fnkey",
    "onekey",
    "dptf",
];

pub fn has_hardware_word(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    HARDWARE_HELPER_WORDS.iter().any(|w| t.contains(w))
}

/// Ada kata `word` yang berdiri di awal kata (huruf/angka sebelumnya tidak ada).
/// `whole` = setelahnya juga tidak boleh huruf (supaya "intel" tidak cocok dengan "IntelliJ").
fn has_word(text: &str, word: &str, whole: bool) -> bool {
    let t = text.to_ascii_lowercase();
    let b = t.as_bytes();
    let mut from = 0;
    while let Some(pos) = t[from..].find(word) {
        let i = from + pos;
        let before_ok = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        let j = i + word.len();
        let after_ok = !whole || j >= b.len() || !b[j].is_ascii_alphabetic();
        if before_ok && after_ok {
            return true;
        }
        from = i + 1;
    }
    false
}

/// Efek mematikan program startup, untuk ditampilkan di pratinjau. Hanya untuk yang
/// efeknya diketahui; pembantu hardware yang efeknya tidak jelas ditulis apa adanya
/// ("efeknya tidak diketahui"). `None` = program biasa (efeknya: tidak ikut menyala).
///
/// `label` = nama program, `detail` = perintah/jalurnya. Vendor dikenali dari keduanya;
/// "pembaru otomatis" hanya dari nama (perintah seperti `Discord\Update.exe --processStart`
/// hanya peluncur, bukan pembaru).
pub fn startup_effect_note(label: &str, detail: &str) -> Option<&'static str> {
    let both = format!("{label} {detail}");
    let text = both.as_str();
    let any = |words: &[&str]| words.iter().any(|w| has_word(text, w, false));
    if any(&["realtek", "rtkaud", "rtkngui", "ravcpl", "waves", "maxxaudio", "dolby", "conexant"]) {
        return Some("Aplikasi pengatur suara dari vendor. Suara tetap keluar karena drivernya tetap jalan, tapi efek suara tambahan (equalizer, Dolby/MaxxAudio) atau pemberitahuan saat headset dicolok bisa tidak jalan.");
    }
    if any(&["synaptics", "syntp", "etdctrl", "touchpad"]) || has_word(text, "elan", true) || has_word(text, "alps", true) {
        return Some("Aplikasi pembantu touchpad. Gerak kursor dan klik tetap jalan, tapi gestur lanjutan (mis. scroll dua jari, zoom) bisa mati sampai dikembalikan.");
    }
    if any(&["nvidia", "nvcpl", "geforce"]) {
        return Some("Aplikasi tambahan NVIDIA (mis. GeForce Experience). Driver kartu grafis tetap jalan, tampilan tidak terpengaruh.");
    }
    if any(&["radeon"]) || has_word(text, "amd", false) {
        return Some("Aplikasi tambahan AMD (mis. Radeon Software). Driver kartu grafis tetap jalan, tampilan tidak terpengaruh.");
    }
    if any(&["igfx", "hkcmd", "persistence"]) {
        return Some("Aplikasi tambahan grafis Intel. Driver kartu grafis tetap jalan; tombol pintas grafis Intel (mis. memutar layar) bisa tidak jalan.");
    }
    if any(&["dptf"]) {
        return Some("Bagian dari pengatur suhu Intel (DPTF). Efeknya tidak diketahui.");
    }
    if has_word(text, "intel", true) {
        return Some("Aplikasi tambahan Intel. Drivernya tetap jalan; efek lainnya tidak diketahui.");
    }
    if any(&["bluetooth", "btray"]) {
        return Some("Aplikasi Bluetooth tambahan dari vendor. Bluetooth bawaan Windows tetap jalan; efek lainnya tidak diketahui.");
    }
    if any(&["hotkey", "atkkey", "fnkey", "onekey"]) {
        return Some("Pembantu tombol Fn / tombol pintas laptop. Tombol Fn (kecerahan, volume, Wi-Fi) bisa tidak berfungsi sampai dikembalikan.");
    }
    if label.to_ascii_lowercase().contains("update") {
        return Some("Pembaru otomatis. Kalau dimatikan, programnya bisa telat mendapat perbaikan keamanan.");
    }
    None
}

pub fn has_protected_word(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    PROTECTED_WORDS.iter().any(|w| t.contains(w))
}

/// Lokasi registry yang boleh DITULIS eksekutor (kunci itu dan anak-anaknya).
/// Daftar yang sama ada di `scripts/preset.ps1` (`$Allowed`) — dicek tes.
pub const REGISTRY_ALLOWED: &[(Hive, &str)] = &[
    (Hive::Hkcu, KEY_VISUAL_EFFECTS),
    (Hive::Hkcu, KEY_DESKTOP),
    (Hive::Hkcu, KEY_EXPLORER_ADVANCED),
    (Hive::Hkcu, KEY_PERSONALIZE),
    (Hive::Hkcu, KEY_GAME_CONFIG),
    (Hive::Hkcu, KEY_GAMEDVR_USER),
    (Hive::Hklm, KEY_GAMEDVR_POLICY),
    (Hive::Hkcu, KEY_BACKGROUND_APPS),
    (Hive::Hkcu, KEY_STARTUP_APPROVED_RUN),
    (Hive::Hklm, KEY_STARTUP_APPROVED_RUN),
    (Hive::Hklm, KEY_STARTUP_APPROVED_RUN32),
    (Hive::Hkcu, KEY_TEST_ROOT),
];

pub fn registry_allowed(hive: Hive, path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    REGISTRY_ALLOWED.iter().any(|(h, a)| {
        let a = a.to_ascii_lowercase();
        *h == hive && (p == a || p.starts_with(&format!("{a}\\")))
    })
}

/// Pengaman terakhir sebelum sebuah target dikirim ke eksekutor.
pub fn validate_target(t: &Target) -> Result<(), String> {
    match t {
        Target::Registry { hive, path, .. } => {
            if registry_allowed(*hive, path) {
                Ok(())
            } else {
                Err(format!("{} tidak ada di daftar izin RepairDoc", t.text()))
            }
        }
        Target::Service { name } => {
            if is_protected_service(name) {
                Err(format!("service {name} dilindungi dan tidak boleh diubah"))
            } else {
                Ok(())
            }
        }
        Target::PowerPlan => Ok(()),
        Target::StartupFile { scope, name, backup } => {
            let bad_name = name.is_empty() || name.contains(['\\', '/', ':']) || name.starts_with('.') || !name.to_ascii_lowercase().ends_with(".lnk");
            let rel = backup.replace('\\', "/");
            let bad_backup = !rel.starts_with(&format!("{STARTUP_BACKUP_DIR}/"))
                || rel.split('/').any(|c| c.is_empty() || c == "." || c == "..")
                || rel.contains(':');
            if scope != "user" && scope != "common" {
                Err(format!("folder Startup '{scope}' tidak dikenal"))
            } else if bad_name {
                Err(format!("'{name}' bukan shortcut (.lnk) yang boleh dipindah"))
            } else if bad_backup {
                Err(format!("lokasi cadangan '{backup}' tidak sah"))
            } else {
                Ok(())
            }
        }
        Target::ScheduledTask { path, name } => {
            let p = path.to_ascii_lowercase();
            if name.is_empty() || !path.starts_with('\\') || !path.ends_with('\\') {
                Err(format!("tugas terjadwal '{path}{name}' tidak sah"))
            } else if p.starts_with(&TASK_PATH_FORBIDDEN.to_ascii_lowercase()) {
                Err(format!("tugas terjadwal {path}{name} milik Windows dan tidak boleh diubah"))
            } else {
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------- definisi tweak

const VISUAL: &[RegSetting] = &[
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_VISUAL_EFFECTS,
        name: "VisualFXSetting",
        value: RegVal::Dword(2),
        windows_default: Some(RegVal::Dword(0)),
        label: "Pilihan di Performance Options: \"Adjust for best performance\"",
    },
    // VisualFXSetting hanya mengingat pilihan di dialog; efeknya sendiri disimpan di
    // nilai-nilai berikut. Tanpa ini, tweak efek visual tidak berpengaruh apa-apa.
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_DESKTOP,
        name: "UserPreferencesMask",
        value: RegVal::Bin(USER_PREFERENCES_BEST_PERFORMANCE),
        windows_default: None,
        label: "Animasi menu, tooltip, dan bayangan",
    },
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_EXPLORER_ADVANCED,
        name: "TaskbarAnimations",
        value: RegVal::Dword(0),
        windows_default: None,
        label: "Animasi taskbar",
    },
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_EXPLORER_ADVANCED,
        name: "ListviewAlphaSelect",
        value: RegVal::Dword(0),
        windows_default: None,
        label: "Kotak pilih transparan di File Explorer",
    },
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_EXPLORER_ADVANCED,
        name: "ListviewShadow",
        value: RegVal::Dword(0),
        windows_default: None,
        label: "Bayangan teks ikon desktop",
    },
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_DESKTOP,
        name: "DragFullWindows",
        value: RegVal::Str("0"),
        windows_default: None,
        label: "Isi jendela saat digeser",
    },
];

const TRANSPARENCY: &[RegSetting] = &[RegSetting {
    hive: Hive::Hkcu,
    path: KEY_PERSONALIZE,
    name: "EnableTransparency",
    value: RegVal::Dword(0),
    windows_default: Some(RegVal::Dword(1)),
    label: "Efek transparan taskbar & menu Start",
}];

/// `MinAnimate` bertipe teks (REG_SZ) "0"/"1", bukan DWORD.
const ANIMATIONS: &[RegSetting] = &[RegSetting {
    hive: Hive::Hkcu,
    path: KEY_WINDOW_METRICS,
    name: "MinAnimate",
    value: RegVal::Str("0"),
    windows_default: Some(RegVal::Str("1")),
    label: "Animasi jendela saat dikecilkan/dibesarkan",
}];

const GAME_DVR: &[RegSetting] = &[
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_GAME_CONFIG,
        name: "GameDVR_Enabled",
        value: RegVal::Dword(0),
        windows_default: Some(RegVal::Dword(1)),
        label: "Perekam game (Game DVR)",
    },
    RegSetting {
        hive: Hive::Hkcu,
        path: KEY_GAMEDVR_USER,
        name: "AppCaptureEnabled",
        value: RegVal::Dword(0),
        windows_default: Some(RegVal::Dword(1)),
        label: "Rekam klip & screenshot lewat Game Bar",
    },
    RegSetting {
        hive: Hive::Hklm,
        path: KEY_GAMEDVR_POLICY,
        name: "AllowGameDVR",
        value: RegVal::Dword(0),
        windows_default: Some(RegVal::Absent),
        label: "Kebijakan Game DVR untuk semua pengguna PC",
    },
];

const READ_REG: &str = "Nilai registry dibaca dulu (atau dicatat \"tidak ada\") sebelum ditulis.";
const REVERT_REG: &str = "Nilai lama dari catatan ditulis ulang persis; kalau dulu tidak ada, nilainya dihapus.";

pub static TWEAKS: &[TweakDef] = &[
    TweakDef {
        id: "visual-best-performance",
        title: "Efek visual dimatikan",
        description: "Animasi menu, bayangan, dan efek halus Windows dimatikan supaya tampilan lebih ringan.",
        modes: &[Mode::Kentang],
        kind: TweakKind::Registry(VISUAL),
        effect: Effect::SetelahSignOut,
        read_how: READ_REG,
        revert_how: REVERT_REG,
    },
    TweakDef {
        id: "transparency-off",
        title: "Efek transparan mati",
        description: "Taskbar dan menu Start jadi tidak tembus pandang. Hemat kerja kartu grafis.",
        modes: &[Mode::Kentang, Mode::Seimbang],
        kind: TweakKind::Registry(TRANSPARENCY),
        effect: Effect::Langsung,
        read_how: READ_REG,
        revert_how: REVERT_REG,
    },
    TweakDef {
        id: "animations-off",
        title: "Animasi jendela mati",
        description: "Jendela langsung muncul/hilang tanpa animasi saat dikecilkan atau dibesarkan.",
        modes: &[Mode::Kentang, Mode::Seimbang],
        kind: TweakKind::Registry(ANIMATIONS),
        effect: Effect::SetelahSignOut,
        read_how: READ_REG,
        revert_how: REVERT_REG,
    },
    TweakDef {
        id: "game-dvr-off",
        title: "Perekam game (Game Bar) mati",
        description: "Fitur rekam layar game yang diam-diam memakai tenaga dimatikan.",
        modes: &[Mode::Kentang, Mode::Seimbang],
        kind: TweakKind::Registry(GAME_DVR),
        effect: Effect::Langsung,
        read_how: READ_REG,
        revert_how: REVERT_REG,
    },
    TweakDef {
        id: "background-apps-off",
        title: "Aplikasi latar dibatasi",
        description: "Aplikasi dari Microsoft Store buatan pihak lain tidak boleh jalan diam-diam di latar.",
        modes: &[Mode::Kentang],
        kind: TweakKind::BackgroundApps,
        effect: Effect::SetelahRestart,
        read_how: "Status Disabled/DisabledByUser tiap aplikasi dibaca dari BackgroundAccessApplications.",
        revert_how: REVERT_REG,
    },
    TweakDef {
        id: "startup-off",
        title: "Program startup dikurangi",
        description: "Program pihak lain tidak ikut jalan saat PC menyala. Programnya tetap ada dan bisa dibuka manual.",
        modes: &[Mode::Kentang],
        kind: TweakKind::Startup,
        effect: Effect::SetelahRestart,
        read_how: "Tiga sumber: kunci Run (status dari StartupApproved, sama dengan tab Startup Task Manager), shortcut di folder Startup pengguna & semua pengguna, dan tugas terjadwal pihak lain yang jalan saat masuk Windows.",
        revert_how: "Kunci Run: data StartupApproved lama ditulis ulang (entri Run tidak pernah dihapus). Folder Startup: shortcut dipindah balik dari RepairDoc-data\\startup-cadangan. Tugas terjadwal: diaktifkan lagi dengan Enable-ScheduledTask (tidak pernah dihapus).",
    },
    TweakDef {
        id: "services-manual",
        title: "Service tidak penting jadi Manual",
        description: "Service Xbox, Fax, peta offline, dan sejenisnya tidak lagi menyala sendiri, tapi tetap bisa dipakai bila dibutuhkan.",
        modes: &[Mode::Kentang],
        kind: TweakKind::Services(MANUAL_SERVICES),
        effect: Effect::SetelahRestart,
        read_how: "Jenis start tiap service dibaca dengan `sc.exe qc`.",
        revert_how: "Jenis start lama dipasang lagi dengan `sc.exe config <nama> start= <lama>`.",
    },
    TweakDef {
        id: "power-high-performance",
        title: "Power plan (pengaturan daya Windows): High performance",
        description: "Prosesor dibiarkan bekerja penuh, tidak ditahan untuk hemat daya.",
        modes: &[Mode::Kentang],
        kind: TweakKind::Power(SCHEME_MIN),
        effect: Effect::Langsung,
        read_how: "GUID power plan aktif dibaca dengan `powercfg /getactivescheme`.",
        revert_how: "Power plan lama diaktifkan lagi dengan `powercfg /setactive <GUID lama>`.",
    },
    TweakDef {
        id: "power-balanced",
        title: "Power plan (pengaturan daya Windows): Balanced",
        description: "Windows mengatur kecepatan prosesor sesuai kebutuhan (bawaan Windows).",
        modes: &[Mode::Seimbang],
        kind: TweakKind::Power(SCHEME_BALANCED),
        effect: Effect::Langsung,
        read_how: "GUID power plan aktif dibaca dengan `powercfg /getactivescheme`.",
        revert_how: "Power plan lama diaktifkan lagi dengan `powercfg /setactive <GUID lama>`.",
    },
];

pub fn tweak(id: &str) -> Option<&'static TweakDef> {
    TWEAKS.iter().find(|t| t.id == id)
}

pub fn tweaks_for(mode: Mode) -> Vec<&'static TweakDef> {
    TWEAKS.iter().filter(|t| t.modes.contains(&mode)).collect()
}

// ---------------------------------------------------------------- teks nilai

pub fn service_label(name: &str) -> &'static str {
    match name {
        "XblGameSave" => "Penyimpanan game Xbox",
        "XboxGipSvc" => "Aksesori Xbox",
        "XboxNetApiSvc" => "Jaringan Xbox Live",
        "XblAuthManager" => "Login Xbox Live",
        "Fax" => "Fax",
        "RemoteRegistry" => "Remote Registry (akses registry dari jarak jauh)",
        "MapsBroker" => "Peta offline (Downloaded Maps Manager)",
        "WMPNetworkSvc" => "Berbagi media Windows Media Player",
        _ => "Service Windows",
    }
}

pub fn power_name(guid: &str) -> String {
    match guid.to_ascii_lowercase().as_str() {
        SCHEME_MIN => "High performance".into(),
        SCHEME_BALANCED => "Balanced".into(),
        SCHEME_MAX => "Power saver".into(),
        "e9a42b02-d5df-448d-aa00-03f14749eb61" => "Ultimate performance".into(),
        g => format!("Power plan lain ({g})"),
    }
}

/// Nilai dalam kata-kata, untuk pratinjau "sekarang → baru".
pub fn value_text(target: &Target, v: &Value) -> String {
    let reg_name = match target {
        Target::Registry { name, .. } => name.as_str(),
        _ => "",
    };
    let startup = matches!(target, Target::Registry { path, .. } if path.contains("StartupApproved"));
    let on_off = |on: bool| if on { "Nyala" } else { "Mati" }.to_string();
    match v {
        Value::Absent { .. } if startup => "Nyala saat PC menyala".into(),
        Value::Binary { .. } if startup => {
            if v.first_byte().is_some_and(|b| b % 2 == 1) {
                "Tidak ikut menyala".into()
            } else {
                "Nyala saat PC menyala".into()
            }
        }
        Value::Absent { .. } => match reg_name {
            "Disabled" | "DisabledByUser" => "Boleh jalan di latar (bawaan)".into(),
            "AllowGameDVR" => "Tidak diatur (bawaan)".into(),
            _ => "Belum diatur (bawaan Windows)".into(),
        },
        Value::Dword { value } => match reg_name {
            "VisualFXSetting" => match value {
                0 => "Biar Windows yang memilih".into(),
                1 => "Tampilan terbaik".into(),
                2 => "Performa terbaik".into(),
                3 => "Diatur sendiri".into(),
                n => n.to_string(),
            },
            "Disabled" | "DisabledByUser" => {
                if *value == 0 {
                    "Boleh jalan di latar".into()
                } else {
                    "Tidak boleh jalan di latar".into()
                }
            }
            "AllowGameDVR" => {
                if *value == 0 {
                    "Dilarang".into()
                } else {
                    "Diizinkan".into()
                }
            }
            "EnableTransparency" | "GameDVR_Enabled" | "AppCaptureEnabled" | "TaskbarAnimations" | "ListviewAlphaSelect"
            | "ListviewShadow" | "GlobalUserDisabled" => {
                if reg_name == "GlobalUserDisabled" {
                    on_off(*value == 0)
                } else {
                    on_off(*value != 0)
                }
            }
            _ => value.to_string(),
        },
        Value::String { value } => match reg_name {
            "MinAnimate" | "DragFullWindows" => on_off(value != "0"),
            _ => format!("\"{value}\""),
        },
        // Angka heksa tidak ditampilkan ke pengguna; nilai lengkapnya tetap tersimpan di log.
        Value::Binary { .. } => {
            if reg_name == "UserPreferencesMask" && v.same_as(&Value::binary(USER_PREFERENCES_BEST_PERFORMANCE)) {
                "Semua efek mati".into()
            } else if reg_name == "UserPreferencesMask" {
                "Sebagian efek nyala".into()
            } else {
                "Setelan khusus".into()
            }
        }
        Value::Shortcut { location: FileLocation::Startup } => "Nyala saat masuk Windows".into(),
        Value::Shortcut { location: FileLocation::Backup } => "Tidak ikut menyala (shortcut di folder cadangan)".into(),
        Value::Task { enabled: true } => "Nyala saat masuk Windows".into(),
        Value::Task { enabled: false } => "Dinonaktifkan".into(),
        Value::Service { start } => match start.as_str() {
            "auto" => "Otomatis".into(),
            "delayed-auto" => "Otomatis (tertunda)".into(),
            "demand" => "Manual".into(),
            "disabled" => "Dimatikan".into(),
            other => other.to_string(),
        },
        Value::Power { guid } => power_name(guid),
        Value::Missing => "Tidak ada di PC ini".into(),
        Value::Unsupported { kind } => format!("Jenis nilai {kind} (tidak diubah)"),
    }
}
