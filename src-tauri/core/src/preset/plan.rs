//! Perencana: mengubah definisi tweak + kondisi PC + catatan menjadi daftar
//! perubahan (`PlannedOp`). Logika murni — tidak menyentuh sistem, bisa dites di mana saja.

use super::model::*;
use super::tweaks::{self, has_hardware_word, has_protected_word, startup_effect_note, TweakDef, TweakKind, KEY_BACKGROUND_APPS, SCHEME_BALANCED, STARTUP_BACKUP_DIR};
use crate::dates;

/// Satu perubahan dalam rencana, lengkap dengan konteks untuk tampilan & catatan.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedOp {
    pub op: Op,
    pub tweak_id: String,
    /// Kunci item yang bisa dikecualikan pengguna (sama untuk beberapa op satu aplikasi).
    pub item_key: String,
    pub label: String,
    pub optional: bool,
    /// Sumber program startup ("Registry", "Folder Startup", "Tugas Terjadwal"); tahap 4.
    pub source: Option<String>,
    /// Perintah / jalur program, untuk pratinjau.
    pub detail: Option<String>,
    /// Efek mematikan item ini (pembantu hardware, pembaru), untuk pratinjau.
    pub effect: Option<String>,
}

pub const SOURCE_REGISTRY: &str = "Registry";
pub const SOURCE_FOLDER: &str = "Folder Startup";
pub const SOURCE_TASK: &str = "Tugas Terjadwal";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanOptions {
    /// Tweak yang toggle-nya dimatikan pengguna.
    pub disabled_tweaks: Vec<String>,
    /// Item (program startup / aplikasi latar) yang tidak dicentang pengguna.
    pub skip_items: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub mode: Mode,
    pub path: PlanPath,
    /// Yang akan dikirim ke eksekutor (urutan = urutan penerapan).
    pub ops: Vec<PlannedOp>,
    /// Dilewati karena pilihan pengguna: (op, alasan).
    pub user_skipped: Vec<(PlannedOp, String)>,
    /// Untuk `RevertLog`: id entri yang dibatalkan.
    pub reverts: Vec<String>,
}

// ---------------------------------------------------------------- program startup

/// Kunci StartupApproved untuk sumber program startup.
pub fn approved_target(item: &StartupItem) -> Option<Target> {
    match item.source.as_str() {
        "HKCU" => Some(Target::reg(Hive::Hkcu, tweaks::KEY_STARTUP_APPROVED_RUN, &item.name)),
        "HKLM" => Some(Target::reg(Hive::Hklm, tweaks::KEY_STARTUP_APPROVED_RUN, &item.name)),
        "HKLM32" => Some(Target::reg(Hive::Hklm, tweaks::KEY_STARTUP_APPROVED_RUN32, &item.name)),
        _ => None,
    }
}

/// Aktif = belum ada data StartupApproved, atau byte pertamanya genap (02/06).
pub fn startup_enabled(item: &StartupItem) -> bool {
    match &item.approved_hex {
        None => true,
        Some(hex) => Value::Binary { hex: hex.clone() }.first_byte().map(|b| b % 2 == 0).unwrap_or(true),
    }
}

/// Program yang tidak pernah dimatikan (Microsoft, Windows, keamanan, pembantu hardware).
pub fn startup_protected(item: &StartupItem) -> bool {
    has_protected_word(&item.name) || has_protected_word(&item.command)
}

/// Program startup yang akan dimatikan oleh tweak `startup-off`.
/// Hanya tiga kunci Run (HKCU, HKLM, HKLM 32-bit): sumber lain seperti `RunOnce`,
/// `Winlogon`/`Userinit`/`Shell` tidak pernah dibaca, dan sumber yang tidak dikenal
/// tidak punya `approved_target` sehingga tidak pernah jadi kandidat.
pub fn startup_candidates(inv: &Inventory) -> Vec<&StartupItem> {
    inv.startup.iter().filter(|i| startup_enabled(i) && !startup_protected(i) && approved_target(i).is_some()).collect()
}

// ---------------------------------------------------------------- folder Startup (tahap 4)

/// Nama program dari nama shortcut: "Spotify.lnk" → "Spotify".
pub fn shortcut_label(name: &str) -> String {
    let n = name.trim();
    if n.len() > 4 && n.to_ascii_lowercase().ends_with(".lnk") {
        n[..n.len() - 4].to_string()
    } else {
        n.to_string()
    }
}

/// Shortcut yang aktif: belum dimatikan lewat Task Manager (`StartupApproved\StartupFolder`
/// tidak ada, atau byte pertamanya genap).
pub fn startup_file_enabled(f: &StartupFile) -> bool {
    match &f.approved_hex {
        None => true,
        Some(hex) => Value::Binary { hex: hex.clone() }.first_byte().map(|b| b % 2 == 0).unwrap_or(true),
    }
}

pub fn startup_file_protected(f: &StartupFile) -> bool {
    has_protected_word(&shortcut_label(&f.name)) || has_protected_word(&f.target)
}

/// Shortcut startup yang akan dipindah ke folder cadangan.
pub fn startup_file_candidates(inv: &Inventory) -> Vec<&StartupFile> {
    inv.startup_files
        .iter()
        .filter(|f| (f.scope == "user" || f.scope == "common") && f.name.to_ascii_lowercase().ends_with(".lnk"))
        .filter(|f| startup_file_enabled(f) && !startup_file_protected(f))
        .collect()
}

/// Nama folder yang aman untuk dipakai sebagai nama folder: huruf, angka, '-', '_'.
fn safe_dir_name(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if t.trim_matches('_').is_empty() {
        "PC".into()
    } else {
        t
    }
}

/// Lokasi cadangan shortcut, relatif terhadap `RepairDoc-data`:
/// `startup-cadangan/<nama-pc>/<tanggal-jam UTC>/<pengguna|semua-pengguna>/<nama>.lnk`.
/// Folder bertanggal per penerapan, supaya cadangan lama tidak pernah tertimpa.
pub fn backup_rel(computer_name: &str, scope: &str, name: &str, now_ms: u64) -> String {
    let secs = now_ms / 1000;
    let (y, m, d) = dates::civil_from_days((secs / 86_400) as i64);
    let t = secs % 86_400;
    let stamp = format!("{y:04}-{m:02}-{d:02}_{:02}{:02}{:02}", t / 3600, (t / 60) % 60, t % 60);
    let who = if scope == "common" { "semua-pengguna" } else { "pengguna" };
    format!("{STARTUP_BACKUP_DIR}/{}/{stamp}/{who}/{name}", safe_dir_name(computer_name))
}

// ---------------------------------------------------------------- tugas terjadwal (tahap 4)

/// Jalur program pihak ketiga: di Program Files atau AppData pengguna, dan bukan di
/// folder Windows. Jalur relatif (mis. "cmd.exe") tidak dianggap pihak ketiga.
pub fn is_third_party_path(path: &str) -> bool {
    let p = path.trim().trim_matches('"').replace('/', "\\").to_ascii_lowercase();
    let in_allowed = p.contains(":\\program files\\") || p.contains(":\\program files (x86)\\") || p.contains("\\appdata\\local\\") || p.contains("\\appdata\\roaming\\");
    in_allowed && !p.contains(":\\windows\\")
}

pub fn task_protected(t: &LogonTask) -> bool {
    has_protected_word(&t.name)
        || has_protected_word(&t.path)
        || has_protected_word(&t.author)
        || t.actions.iter().any(|a| has_protected_word(a))
        || t.arguments.iter().any(|a| has_protected_word(a))
}

/// Tugas terjadwal yang akan dinonaktifkan: aktif (Ready/Running), punya pemicu logon
/// (inventaris hanya mengirim yang begitu), SEMUA aksinya menjalankan program pihak
/// ketiga, tidak di folder `\Microsoft\`, dan tidak mengandung kata terlindung.
pub fn task_candidates(inv: &Inventory) -> Vec<&LogonTask> {
    inv.logon_tasks
        .iter()
        .filter(|t| t.state.eq_ignore_ascii_case("Ready") || t.state.eq_ignore_ascii_case("Running"))
        .filter(|t| !t.actions.is_empty() && t.actions.iter().all(|a| is_third_party_path(a)))
        .filter(|t| tweaks::validate_target(&Target::ScheduledTask { path: t.path.clone(), name: t.name.clone() }).is_ok())
        .filter(|t| !task_protected(t))
        .collect()
}

fn task_detail(t: &LogonTask) -> String {
    t.actions
        .iter()
        .zip(t.arguments.iter().chain(std::iter::repeat(&String::new())))
        .map(|(a, g)| if g.trim().is_empty() { a.clone() } else { format!("{a} {}", g.trim()) })
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// Data StartupApproved "dimatikan" seperti yang ditulis Task Manager:
/// 03 00 00 00 + waktu (FILETIME, 8 byte little-endian).
pub fn startup_disabled_hex(now_ms: u64) -> String {
    let filetime: u64 = (now_ms / 1000 + 11_644_473_600) * 10_000_000 + (now_ms % 1000) * 10_000;
    let mut bytes = vec![0x03u8, 0, 0, 0];
    bytes.extend_from_slice(&filetime.to_le_bytes());
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------- aplikasi latar

/// Aplikasi Store "non-sistem": ditandatangani Store (bukan System), bisa dihapus,
/// bukan buatan Microsoft/Windows, dan bukan pembantu hardware/keamanan.
pub fn app_is_candidate(app: &AppItem) -> bool {
    let name = app.name.to_ascii_lowercase();
    let display = app.display_name.clone().unwrap_or_default();
    // Pembantu hardware tetap terlindung di sini, sama seperti tahap 3 — pelonggaran
    // tahap 4 hanya berlaku untuk program startup.
    app.signature_kind.eq_ignore_ascii_case("Store")
        && !app.non_removable
        && !name.starts_with("microsoft")
        && !name.starts_with("windows")
        && !name.starts_with("appup.") // Intel Graphics Command Center, dsb.
        && !has_protected_word(&app.name)
        && !has_protected_word(&display)
        && !has_hardware_word(&app.name)
        && !has_hardware_word(&display)
}

pub fn app_disabled(app: &AppItem) -> bool {
    app.disabled == Some(1)
}

/// Nama aplikasi yang enak dibaca: nama tampilan bila ada, kalau tidak bagian
/// setelah titik pertama ("SpotifyAB.SpotifyMusic" → "SpotifyMusic").
pub fn app_label(app: &AppItem) -> String {
    if let Some(d) = app.display_name.as_ref().filter(|d| !d.trim().is_empty() && !d.starts_with("ms-resource:")) {
        return d.trim().to_string();
    }
    app.name.split_once('.').map(|(_, r)| r).unwrap_or(&app.name).to_string()
}

fn bg_target(app: &AppItem, value_name: &str) -> Target {
    Target::reg(Hive::Hkcu, &format!("{KEY_BACKGROUND_APPS}\\{}", app.family_name), value_name)
}

// ---------------------------------------------------------------- penyusun rencana

fn planned(op: Op, tweak_id: &str, item_key: &str, label: String, optional: bool) -> PlannedOp {
    PlannedOp { op, tweak_id: tweak_id.to_string(), item_key: item_key.to_string(), label, optional, source: None, detail: None, effect: None }
}

/// Item startup dengan sumber, perintah, dan catatan efek untuk pratinjau.
fn startup_op(op: Op, tweak_id: &str, label: String, source: &str, detail: &str) -> PlannedOp {
    let key = op.key.clone();
    let effect = startup_effect_note(&label, detail).map(|e| e.to_string());
    PlannedOp { source: Some(source.to_string()), detail: Some(detail.to_string()).filter(|d| !d.trim().is_empty()), effect, ..planned(op, tweak_id, &key, label, true) }
}

/// Perubahan yang dibuat satu tweak untuk mencapai nilai tujuannya.
pub fn ops_for_tweak(def: &TweakDef, inv: &Inventory, now_ms: u64) -> Vec<PlannedOp> {
    let id = def.id;
    match def.kind {
        TweakKind::Registry(settings) => settings
            .iter()
            .map(|s| {
                let key = format!("{id}:{}", s.name);
                planned(Op { key: key.clone(), target: s.target(), value: s.value.to_value() }, id, &key, s.label.to_string(), false)
            })
            .collect(),
        TweakKind::Startup => {
            let hex = startup_disabled_hex(now_ms);
            // 1) Kunci Run: lewat StartupApproved (tidak berubah dari tahap 3).
            let mut out: Vec<PlannedOp> = startup_candidates(inv)
                .into_iter()
                .filter_map(|item| {
                    let target = approved_target(item)?;
                    let key = format!("startup:{}:{}", item.source, item.name);
                    Some(startup_op(Op { key, target, value: Value::Binary { hex: hex.clone() } }, id, item.name.clone(), SOURCE_REGISTRY, &item.command))
                })
                .collect();
            // 2) Folder Startup: shortcut DIPINDAH ke folder cadangan (bukan dihapus).
            for f in startup_file_candidates(inv) {
                let target = Target::StartupFile {
                    scope: f.scope.clone(),
                    name: f.name.clone(),
                    backup: backup_rel(&inv.machine.computer_name, &f.scope, &f.name, now_ms),
                };
                let key = format!("startup:folder:{}:{}", f.scope, f.name);
                out.push(startup_op(Op { key, target, value: Value::Shortcut { location: FileLocation::Backup } }, id, shortcut_label(&f.name), SOURCE_FOLDER, &f.target));
            }
            // 3) Tugas terjadwal: DINONAKTIFKAN (bukan dihapus).
            for t in task_candidates(inv) {
                let key = format!("startup:task:{}{}", t.path, t.name);
                let target = Target::ScheduledTask { path: t.path.clone(), name: t.name.clone() };
                out.push(startup_op(Op { key, target, value: Value::Task { enabled: false } }, id, t.name.clone(), SOURCE_TASK, &task_detail(t)));
            }
            out
        }
        TweakKind::BackgroundApps => inv
            .apps
            .iter()
            .filter(|a| app_is_candidate(a) && !app_disabled(a))
            .flat_map(|a| {
                let item = format!("bg:{}", a.family_name);
                let label = app_label(a);
                ["Disabled", "DisabledByUser"].map(|n| {
                    planned(Op { key: format!("{item}:{n}"), target: bg_target(a, n), value: Value::dword(1) }, id, &item, label.clone(), true)
                })
            })
            .collect(),
        TweakKind::Services(names) => names
            .iter()
            .map(|n| {
                let key = format!("service:{n}");
                planned(
                    Op { key: key.clone(), target: Target::Service { name: n.to_string() }, value: Value::service("demand") },
                    id,
                    &key,
                    format!("{} ({n})", tweaks::service_label(n)),
                    false,
                )
            })
            .collect(),
        TweakKind::Power(guid) => {
            vec![planned(Op { key: "power".into(), target: Target::PowerPlan, value: Value::power(guid) }, id, "power", "Power plan aktif".into(), false)]
        }
    }
}

/// Nilai bawaan Windows untuk Mode Normal saat belum ada catatan.
pub fn default_ops(inv: &Inventory) -> Vec<PlannedOp> {
    let mut out = Vec::new();
    for def in tweaks::TWEAKS {
        match def.kind {
            TweakKind::Registry(settings) => {
                for s in settings {
                    if let Some(d) = s.windows_default {
                        let key = format!("default:{}:{}", def.id, s.name);
                        out.push(planned(Op { key: key.clone(), target: s.target(), value: d.to_value() }, def.id, &key, s.label.to_string(), false));
                    }
                }
            }
            TweakKind::BackgroundApps => {
                // Aplikasi latar "nyala": hanya yang sekarang dilarang.
                if inv.global_bg_disabled == Some(1) {
                    out.push(planned(
                        Op { key: "default:bg:global".into(), target: Target::reg(Hive::Hkcu, KEY_BACKGROUND_APPS, "GlobalUserDisabled"), value: Value::dword(0) },
                        def.id,
                        "default:bg:global",
                        "Saklar semua aplikasi latar".into(),
                        false,
                    ));
                }
                for a in inv.apps.iter().filter(|a| app_is_candidate(a) && app_disabled(a)) {
                    let item = format!("default:bg:{}", a.family_name);
                    for n in ["Disabled", "DisabledByUser"] {
                        out.push(planned(Op { key: format!("{item}:{n}"), target: bg_target(a, n), value: Value::dword(0) }, def.id, &item, app_label(a), true));
                    }
                }
            }
            _ => {}
        }
    }
    out.push(planned(
        Op { key: "default:power".into(), target: Target::PowerPlan, value: Value::power(SCHEME_BALANCED) },
        "power-balanced",
        "default:power",
        "Power plan aktif".into(),
        false,
    ));
    out
}

/// Perubahan untuk membatalkan satu entri catatan: nilai lama, urutan terbalik.
pub fn revert_ops(entry: &LogEntry) -> Vec<PlannedOp> {
    entry
        .perubahan
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, c)| c.status == ChangeStatus::Berhasil)
        .filter_map(|(i, c)| {
            let old = c.nilai_lama.clone()?;
            let key = format!("revert:{}:{i}", entry.id);
            let item = if c.item_key.is_empty() { key.clone() } else { format!("revert:{}", c.item_key) };
            Some(planned(Op { key: key.clone(), target: c.target.clone(), value: old }, &c.tweak_id, &item, c.label.clone(), false))
        })
        .collect()
}

/// Susun rencana untuk satu mode.
pub fn build(mode: Mode, inv: &Inventory, log: &PresetLog, opts: &PlanOptions, now_ms: u64) -> Plan {
    let machine = &inv.machine.machine_id;
    let (path, ops, reverts) = match mode {
        Mode::Kentang | Mode::Seimbang => {
            let ops: Vec<PlannedOp> = tweaks::tweaks_for(mode).into_iter().flat_map(|d| ops_for_tweak(d, inv, now_ms)).collect();
            (PlanPath::Apply, ops, Vec::new())
        }
        Mode::Normal => {
            let pending: Vec<&LogEntry> = log.for_machine(machine).into_iter().filter(|e| e.revertible()).collect();
            if pending.is_empty() {
                (PlanPath::Defaults, default_ops(inv), Vec::new())
            } else {
                // Terbaru dulu, supaya nilai yang berlapis kembali ke asal.
                let ops = pending.iter().rev().flat_map(|e| revert_ops(e)).collect();
                (PlanPath::RevertLog, ops, pending.iter().rev().map(|e| e.id.clone()).collect())
            }
        }
    };

    let mut kept = Vec::new();
    let mut user_skipped = Vec::new();
    for p in ops {
        let toggled_off = path != PlanPath::RevertLog && opts.disabled_tweaks.iter().any(|t| t == &p.tweak_id);
        if toggled_off {
            user_skipped.push((p, "Dimatikan lewat pilihan tweak".to_string()));
        } else if p.optional && opts.skip_items.iter().any(|k| k == &p.item_key) {
            user_skipped.push((p, "Tidak dicentang pengguna".to_string()));
        } else {
            kept.push(p);
        }
    }
    Plan { mode, path, ops: kept, user_skipped, reverts }
}

/// "Sudah sesuai" menurut makna, bukan hanya nilai persis: service yang sudah
/// Manual atau Dimatikan tidak perlu diubah ke Manual (Dimatikan → Manual malah
/// membuatnya lebih aktif). Program startup yang sudah tidak aktif juga sudah sesuai.
pub fn already_ok(op: &Op, before: &Value) -> bool {
    if before.same_as(&op.value) {
        return true;
    }
    match (&op.target, &op.value, before) {
        (Target::Service { .. }, Value::Service { start: want }, Value::Service { start: now }) if want == "demand" => {
            now == "demand" || now == "disabled"
        }
        (Target::Service { .. }, _, Value::Missing) => true,
        (Target::Registry { path, .. }, Value::Binary { .. }, Value::Binary { .. }) if path.contains("StartupApproved") => {
            op.value.first_byte().map(|b| b % 2) == before.first_byte().map(|b| b % 2)
        }
        _ => false,
    }
}

/// Untuk tes eksekutor di Windows: arahkan semua target registry ke bawah
/// `HKCU\Software\RepairDocTest\…` dan buang service/power. JANGAN dipakai di luar tes.
pub fn redirect_for_test(ops: &[PlannedOp], root: &str) -> Vec<PlannedOp> {
    ops.iter()
        .filter_map(|p| match &p.op.target {
            Target::Registry { hive, path, name } => {
                let mut q = p.clone();
                q.op.target = Target::reg(Hive::Hkcu, &format!("{root}\\{}\\{path}", hive.short()), name);
                Some(q)
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::tweaks::*;

    fn inv() -> Inventory {
        Inventory {
            machine: MachineInfo { machine_id: "PC-1".into(), computer_name: "BUDI-PC".into() },
            startup: vec![
                StartupItem { source: "HKCU".into(), name: "Spotify".into(), command: r"C:\Users\b\AppData\Roaming\Spotify\Spotify.exe /minimized".into(), approved_hex: None },
                StartupItem { source: "HKCU".into(), name: "OneDrive".into(), command: r"C:\Program Files\Microsoft OneDrive\OneDrive.exe /background".into(), approved_hex: None },
                StartupItem { source: "HKLM".into(), name: "SecurityHealth".into(), command: r"%windir%\system32\SecurityHealthSystray.exe".into(), approved_hex: None },
                StartupItem { source: "HKLM".into(), name: "RtkAudUService".into(), command: r"C:\Windows\System32\RtkAudUService64.exe".into(), approved_hex: None },
                StartupItem { source: "HKLM32".into(), name: "uTorrent".into(), command: r"C:\Program Files (x86)\uTorrent\uTorrent.exe".into(), approved_hex: Some("020000000000000000000000".into()) },
                StartupItem { source: "HKCU".into(), name: "Discord".into(), command: "Discord.exe".into(), approved_hex: Some("0300000011223344aabbccdd".into()) },
                StartupItem { source: "HKCU".into(), name: "SmadavProtect".into(), command: r"C:\Program Files\SMADAV\SM?RTP.exe".into(), approved_hex: None },
            ],
            apps: vec![
                AppItem { name: "SpotifyAB.SpotifyMusic".into(), family_name: "SpotifyAB.SpotifyMusic_zpdnekdrzrea0".into(), display_name: Some("Spotify".into()), signature_kind: "Store".into(), ..Default::default() },
                AppItem { name: "Microsoft.WindowsAlarms".into(), family_name: "Microsoft.WindowsAlarms_8wekyb3d8bbwe".into(), signature_kind: "Store".into(), ..Default::default() },
                AppItem { name: "Microsoft.Windows.Search".into(), family_name: "x".into(), signature_kind: "System".into(), non_removable: true, ..Default::default() },
                AppItem { name: "RealtekSemiconductorCorp.RealtekAudioControl".into(), family_name: "y".into(), signature_kind: "Store".into(), ..Default::default() },
                AppItem { name: "5319275A.WhatsAppDesktop".into(), family_name: "5319275A.WhatsAppDesktop_cv1g1gvanyjgm".into(), signature_kind: "Store".into(), disabled: Some(1), disabled_by_user: Some(1), ..Default::default() },
            ],
            global_bg_disabled: None,
            ..Default::default()
        }
    }

    fn keys(ops: &[PlannedOp]) -> Vec<String> {
        ops.iter().map(|p| p.op.key.clone()).collect()
    }

    #[test]
    fn startup_filter_protects_and_skips_disabled() {
        let i = inv();
        let names: Vec<&str> = startup_candidates(&i).iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Spotify", "uTorrent"], "Microsoft/keamanan/audio/antivirus dan yang sudah mati tidak boleh disentuh");
    }

    #[test]
    fn startup_uses_startup_approved_not_run() {
        let ops = ops_for_tweak(tweak("startup-off").unwrap(), &inv(), 0);
        for p in &ops {
            let Target::Registry { path, .. } = &p.op.target else { panic!() };
            assert!(path.contains("StartupApproved"), "entri Run tidak boleh disentuh: {path}");
            assert!(p.optional);
        }
        let ut = ops.iter().find(|p| p.label == "uTorrent").unwrap();
        assert_eq!(ut.op.target, Target::reg(Hive::Hklm, KEY_STARTUP_APPROVED_RUN32, "uTorrent"));
    }

    #[test]
    fn startup_disabled_bytes_look_like_task_manager() {
        let hex = startup_disabled_hex(1_764_000_000_000);
        assert_eq!(hex.len(), 24);
        assert!(hex.starts_with("03000000"));
        let v = Value::Binary { hex };
        assert_eq!(v.first_byte(), Some(3));
    }

    #[test]
    fn background_apps_only_third_party() {
        let ops = ops_for_tweak(tweak("background-apps-off").unwrap(), &inv(), 0);
        let items: Vec<&str> = ops.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(items, vec!["Spotify", "Spotify"], "dua nilai (Disabled & DisabledByUser) untuk Spotify saja");
        assert!(ops.iter().all(|p| p.item_key == "bg:SpotifyAB.SpotifyMusic_zpdnekdrzrea0"));
    }

    #[test]
    fn services_only_whitelist() {
        let ops = ops_for_tweak(tweak("services-manual").unwrap(), &inv(), 0);
        assert_eq!(ops.len(), MANUAL_SERVICES.len());
        for p in &ops {
            assert_eq!(p.op.value, Value::service("demand"));
            assert!(validate_target(&p.op.target).is_ok());
        }
    }

    #[test]
    fn kentang_and_seimbang_plans() {
        let log = PresetLog::new();
        let k = build(Mode::Kentang, &inv(), &log, &PlanOptions::default(), 0);
        assert_eq!(k.path, PlanPath::Apply);
        assert!(keys(&k.ops).contains(&"visual-best-performance:VisualFXSetting".to_string()));
        assert!(keys(&k.ops).contains(&"service:MapsBroker".to_string()));
        let s = build(Mode::Seimbang, &inv(), &log, &PlanOptions::default(), 0);
        assert!(!keys(&s.ops).iter().any(|k| k.starts_with("visual-") || k.starts_with("service:") || k.starts_with("startup:")));
        let power = s.ops.iter().find(|p| p.op.key == "power").unwrap();
        assert_eq!(power.op.value, Value::power(SCHEME_BALANCED));
    }

    #[test]
    fn toggles_and_unchecked_items_are_skipped() {
        let opts = PlanOptions { disabled_tweaks: vec!["services-manual".into()], skip_items: vec!["startup:HKCU:Spotify".into()] };
        let k = build(Mode::Kentang, &inv(), &PresetLog::new(), &opts, 0);
        assert!(!keys(&k.ops).iter().any(|k| k.starts_with("service:") || k == "startup:HKCU:Spotify"));
        assert_eq!(k.user_skipped.len(), MANUAL_SERVICES.len() + 1);
    }

    #[test]
    fn normal_without_log_uses_windows_defaults() {
        let n = build(Mode::Normal, &inv(), &PresetLog::new(), &PlanOptions::default(), 0);
        assert_eq!(n.path, PlanPath::Defaults);
        let find = |k: &str| n.ops.iter().find(|p| p.op.key == k).map(|p| p.op.value.clone());
        assert_eq!(find("default:visual-best-performance:VisualFXSetting"), Some(Value::dword(0)));
        assert_eq!(find("default:transparency-off:EnableTransparency"), Some(Value::dword(1)));
        assert_eq!(find("default:animations-off:MinAnimate"), Some(Value::string("1")));
        assert_eq!(find("default:game-dvr-off:GameDVR_Enabled"), Some(Value::dword(1)));
        assert_eq!(find("default:game-dvr-off:AllowGameDVR"), Some(Value::absent()));
        assert_eq!(find("default:power"), Some(Value::power(SCHEME_BALANCED)));
        // WhatsApp sedang dilarang jalan di latar → dinyalakan lagi.
        assert!(n.ops.iter().any(|p| p.item_key == "default:bg:5319275A.WhatsAppDesktop_cv1g1gvanyjgm"));
        // Startup & service tidak punya nilai bawaan yang pasti → tidak disentuh.
        assert!(!n.ops.iter().any(|p| matches!(p.op.target, Target::Service { .. }) || p.op.key.starts_with("startup")));
    }

    #[test]
    fn already_ok_semantics() {
        let svc = Op { key: "s".into(), target: Target::Service { name: "Fax".into() }, value: Value::service("demand") };
        assert!(already_ok(&svc, &Value::service("disabled")), "Dimatikan tidak boleh dinaikkan jadi Manual");
        assert!(already_ok(&svc, &Value::Missing));
        assert!(!already_ok(&svc, &Value::service("delayed-auto")));
        let st = Op { key: "x".into(), target: Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "A"), value: Value::Binary { hex: startup_disabled_hex(5) } };
        assert!(already_ok(&st, &Value::Binary { hex: "030000001122334455667788".into() }));
        assert!(!already_ok(&st, &Value::Binary { hex: "020000000000000000000000".into() }));
    }

    #[test]
    fn redirect_keeps_registry_only_under_test_root() {
        let k = build(Mode::Kentang, &inv(), &PresetLog::new(), &PlanOptions::default(), 0);
        let r = redirect_for_test(&k.ops, KEY_TEST_ROOT);
        assert!(!r.is_empty());
        for p in &r {
            let Target::Registry { hive, path, .. } = &p.op.target else { panic!("hanya registry") };
            assert_eq!(*hive, Hive::Hkcu);
            assert!(path.starts_with(r"Software\RepairDocTest\"), "{path}");
            assert!(validate_target(&p.op.target).is_ok());
        }
    }
}
