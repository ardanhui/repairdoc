//! Backend sungguhan: menjalankan `scripts/preset*.ps1` dan `restore_point.ps1`
//! lewat `platform::run_powershell_script`. Di sistem selain Windows semua
//! perintah mengembalikan galat "hanya Windows".
//!
//! Tahap 4: shortcut di folder Startup dipindah oleh Rust sendiri (`run_file_ops`),
//! bukan PowerShell — aturannya (hanya `.lnk`, tidak mengikuti symlink, tidak menimpa)
//! bisa dites di mana saja. Tugas terjadwal ditangani `preset.ps1`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;

use super::engine::Backend;
use super::model::*;
use crate::platform;
use crate::probe::json;

const PRESET: &str = include_str!("../scripts/preset.ps1");
const INVENTORY: &str = include_str!("../scripts/preset_inventory.ps1");
const RESTORE_POINT: &str = include_str!("../scripts/restore_point.ps1");

/// Restore point di HDD lambat bisa makan beberapa menit.
const RESTORE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const RUN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const INVENTORY_TIMEOUT: Duration = Duration::from_secs(2 * 60);

pub struct PowerShellBackend;

/// Kirim daftar perubahan ke `preset.ps1` (lewat variabel lingkungan, bukan baris perintah).
pub fn run_ops(ops: &[Op], dry_run: bool) -> Result<Vec<OpResult>, String> {
    if ops.is_empty() {
        return Ok(Vec::new());
    }
    let payload = serde_json::to_string(ops).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());
    let args = if dry_run { "-DryRun" } else { "" };
    let out = platform::run_powershell_script(PRESET, args, &[("REPAIRDOC_OPS", &b64)], RUN_TIMEOUT)?;
    parse_results(&out)
}

pub fn parse_results(out: &str) -> Result<Vec<OpResult>, String> {
    let v = json::parse(out)?;
    json::list_of(Some(&v))
        .into_iter()
        .map(|r| serde_json::from_value::<OpResult>(r.clone()).map_err(|e| format!("jawaban eksekutor tidak terbaca: {e}")))
        .collect()
}

pub fn parse_inventory(out: &str) -> Result<Inventory, String> {
    let v = json::parse(out)?;
    let m = v.get("machine");
    let machine = MachineInfo {
        machine_id: json::str_of(m.and_then(|m| m.get("machineId"))),
        computer_name: json::str_of(m.and_then(|m| m.get("computerName"))),
    };
    let startup = json::list_of(v.get("startup"))
        .into_iter()
        .map(|s| StartupItem {
            source: json::str_of(s.get("source")),
            name: json::str_of(s.get("name")),
            command: json::str_of(s.get("command")),
            approved_hex: Some(json::str_of(s.get("approvedHex"))).filter(|h| !h.is_empty()),
        })
        .filter(|s| !s.name.is_empty())
        .collect();
    let apps = json::list_of(v.get("apps"))
        .into_iter()
        .map(|a| AppItem {
            name: json::str_of(a.get("name")),
            family_name: json::str_of(a.get("familyName")),
            display_name: Some(json::str_of(a.get("displayName"))).filter(|d| !d.is_empty()),
            signature_kind: json::str_of(a.get("signatureKind")),
            non_removable: a.get("nonRemovable").and_then(|b| b.as_bool()).unwrap_or(true),
            disabled: json::u64_of(a.get("disabled")).map(|d| d as u32),
            disabled_by_user: json::u64_of(a.get("disabledByUser")).map(|d| d as u32),
        })
        .filter(|a| !a.family_name.is_empty())
        .collect();
    let startup_files = json::list_of(v.get("startupFiles"))
        .into_iter()
        .map(|f| StartupFile {
            scope: json::str_of(f.get("scope")),
            name: json::str_of(f.get("name")),
            target: json::str_of(f.get("target")),
            approved_hex: Some(json::str_of(f.get("approvedHex"))).filter(|h| !h.is_empty()),
        })
        .filter(|f| !f.name.is_empty())
        .collect();
    let logon_tasks = json::list_of(v.get("logonTasks"))
        .into_iter()
        .map(|t| {
            let acts = json::list_of(t.get("actions"));
            LogonTask {
                path: json::str_of(t.get("path")),
                name: json::str_of(t.get("name")),
                state: json::str_of(t.get("state")),
                author: json::str_of(t.get("author")),
                actions: acts.iter().map(|a| json::str_of(a.get("execute"))).collect(),
                arguments: acts.iter().map(|a| json::str_of(a.get("arguments"))).collect(),
            }
        })
        .filter(|t| !t.name.is_empty() && !t.path.is_empty())
        .collect();
    let mut inv = Inventory {
        machine,
        startup,
        apps,
        global_bg_disabled: json::u64_of(v.get("globalBgDisabled")).map(|d| d as u32),
        startup_files,
        logon_tasks,
    };
    if inv.machine.machine_id.is_empty() {
        // Cadangan: nama komputer, supaya catatan tetap terpisah per PC.
        inv.machine.machine_id = format!("nama:{}", inv.machine.computer_name);
    }
    Ok(inv)
}

impl Backend for PowerShellBackend {
    fn is_windows(&self) -> bool {
        platform::IS_WINDOWS
    }

    fn is_admin(&self) -> bool {
        platform::is_elevated()
    }

    fn machine(&self) -> Result<MachineInfo, String> {
        let out = platform::run_powershell_script(INVENTORY, "-MachineOnly", &[], INVENTORY_TIMEOUT)?;
        Ok(parse_inventory(&out)?.machine)
    }

    fn inventory(&self) -> Result<Inventory, String> {
        let out = platform::run_powershell_script(INVENTORY, "", &[], INVENTORY_TIMEOUT)?;
        parse_inventory(&out)
    }

    fn run(&self, ops: &[Op], dry_run: bool) -> Result<Vec<OpResult>, String> {
        // Shortcut startup → Rust; sisanya → preset.ps1. Skrip dulu: kalau skrip gagal
        // total, shortcut belum disentuh dan engine bisa membaca ulang keadaannya.
        let (files, others): (Vec<Op>, Vec<Op>) = ops.iter().cloned().partition(|o| matches!(o.target, Target::StartupFile { .. }));
        let mut results = run_ops(&others, dry_run)?;
        if !files.is_empty() {
            let data = super::log::default_path().parent().map(Path::to_path_buf);
            let roots = data.and_then(StartupRoots::system);
            results.extend(run_file_ops(roots.as_ref(), &files, dry_run));
        }
        // Urutan jawaban mengikuti urutan permintaan.
        let mut ordered = Vec::with_capacity(ops.len());
        for o in ops {
            if let Some(i) = results.iter().position(|r| r.key == o.key) {
                ordered.push(results.swap_remove(i));
            }
        }
        Ok(ordered)
    }

    fn restore_point(&self, prefix: &str) -> Result<String, String> {
        let out = platform::run_powershell_script(RESTORE_POINT, "", &[("REPAIRDOC_RP_PREFIX", prefix)], RESTORE_TIMEOUT)?;
        parse_restore_point(&out)
    }
}

// ---------------------------------------------------------------- shortcut folder Startup (tahap 4)

/// Folder Startup dan folder data RepairDoc, untuk memindah shortcut.
#[derive(Clone, Debug)]
pub struct StartupRoots {
    /// `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`
    pub user: PathBuf,
    /// `%ProgramData%\Microsoft\Windows\Start Menu\Programs\StartUp`
    pub common: PathBuf,
    /// `RepairDoc-data` (cadangan disimpan di `startup-cadangan\…` di dalamnya).
    pub data: PathBuf,
}

impl StartupRoots {
    /// Folder sungguhan — jalurnya sama dengan yang dibaca `preset_inventory.ps1`.
    pub fn system(data: PathBuf) -> Option<Self> {
        let appdata = PathBuf::from(std::env::var_os("APPDATA")?);
        let progdata = PathBuf::from(std::env::var_os("ProgramData")?);
        Some(StartupRoots {
            user: appdata.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("Startup"),
            common: progdata.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("StartUp"),
            data,
        })
    }

    fn folder(&self, scope: &str) -> &Path {
        if scope == "common" {
            &self.common
        } else {
            &self.user
        }
    }

    /// Jalur cadangan dari bentuk relatif di catatan.
    pub fn backup_path(&self, rel: &str) -> PathBuf {
        rel.split(['/', '\\']).filter(|c| !c.is_empty()).fold(self.data.clone(), |p, c| p.join(c))
    }
}

/// File biasa (bukan folder, bukan symlink/junction — yang begitu tidak pernah disentuh).
fn is_plain_file(p: &Path) -> bool {
    std::fs::symlink_metadata(p).map(|m| m.file_type().is_file()).unwrap_or(false)
}

/// Letak shortcut sekarang: di folder Startup, di cadangan, atau tidak ada.
pub fn read_shortcut(roots: &StartupRoots, scope: &str, name: &str, backup: &str) -> Value {
    if is_plain_file(&roots.folder(scope).join(name)) {
        Value::Shortcut { location: FileLocation::Startup }
    } else if is_plain_file(&roots.backup_path(backup)) {
        Value::Shortcut { location: FileLocation::Backup }
    } else {
        Value::Missing
    }
}

/// Pindahkan satu file tanpa pernah menimpa. Di drive yang sama cukup `rename`;
/// beda drive (C: → flashdisk): salin, cek ukurannya, baru hapus yang asal.
fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(format!("Sudah ada file dengan nama sama di {}; tidak ditimpa.", to.display()));
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Folder {} tidak bisa dibuat: {e}", parent.display()))?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let len = std::fs::metadata(from).map_err(|e| format!("Shortcut tidak terbaca: {e}"))?.len();
    let copied = std::fs::copy(from, to).map_err(|e| format!("Shortcut tidak bisa disalin: {e}"))?;
    if copied != len {
        let _ = std::fs::remove_file(to);
        return Err("Salinan shortcut tidak lengkap; yang asal dibiarkan.".into());
    }
    if let Err(e) = std::fs::remove_file(from) {
        let _ = std::fs::remove_file(to);
        return Err(format!("Shortcut asal tidak bisa dipindah: {e}"));
    }
    Ok(())
}

/// Jalankan (atau simulasikan) perubahan shortcut startup. Status sama dengan `preset.ps1`.
pub fn run_file_ops(roots: Option<&StartupRoots>, ops: &[Op], dry_run: bool) -> Vec<OpResult> {
    ops.iter()
        .map(|op| {
            let mut r = OpResult { key: op.key.clone(), before: None, after: None, status: OpStatus::Failed, error: None };
            let (Target::StartupFile { scope, name, backup }, Value::Shortcut { location }) = (&op.target, &op.value) else {
                r.error = Some("Bukan perubahan shortcut startup.".into());
                return r;
            };
            if let Err(e) = super::tweaks::validate_target(&op.target) {
                r.status = OpStatus::Refused;
                r.error = Some(e);
                return r;
            }
            let Some(roots) = roots else {
                r.error = Some("Folder Startup tidak ditemukan.".into());
                return r;
            };
            let before = read_shortcut(roots, scope, name, backup);
            r.before = Some(before.clone());
            if before == Value::Missing {
                r.status = OpStatus::NotPresent;
                r.error = Some("Shortcut ini sudah tidak ada (di folder Startup maupun di cadangan).".into());
                return r;
            }
            if before.same_as(&op.value) {
                r.status = OpStatus::Same;
                return r;
            }
            if dry_run {
                r.status = OpStatus::Planned;
                return r;
            }
            let original = roots.folder(scope).join(name);
            let saved = roots.backup_path(backup);
            let moved = match location {
                FileLocation::Backup => move_file(&original, &saved),
                FileLocation::Startup => move_file(&saved, &original),
            };
            match moved {
                Ok(()) => {
                    let after = read_shortcut(roots, scope, name, backup);
                    if after.same_as(&op.value) {
                        r.status = OpStatus::Changed;
                    } else {
                        r.error = Some("Letak shortcut setelah dipindah tidak sesuai.".into());
                    }
                    r.after = Some(after);
                }
                Err(e) => r.error = Some(e),
            }
            r
        })
        .collect()
}

pub fn parse_restore_point(out: &str) -> Result<String, String> {
    let v = json::parse(out)?;
    if v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false) {
        Ok(json::str_of(v.get("name")))
    } else {
        let e = json::str_of(v.get("error"));
        Err(if e.is_empty() { "alasan tidak diketahui".into() } else { e })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_script_results() {
        let out = r#"[{"key":"a","before":{"type":"absent","keyExisted":false},"after":{"type":"dword","value":0},"status":"changed","error":null},
                      {"key":"b","before":{"type":"service","start":"delayed-auto"},"after":null,"status":"planned","error":null},
                      {"key":"c","before":{"type":"missing"},"after":null,"status":"notpresent","error":"Service ini tidak ada di PC ini."}]"#;
        let r = parse_results(out).unwrap();
        assert_eq!(r[0].before, Some(Value::Absent { key_existed: false }));
        assert_eq!(r[0].status, OpStatus::Changed);
        assert_eq!(r[1].before, Some(Value::service("delayed-auto")));
        assert_eq!(r[2].status, OpStatus::NotPresent);
        // PowerShell 5.1 bisa mengirim satu objek, bukan array.
        let one = parse_results(r#"{"key":"x","before":{"type":"binary","hex":"0300"},"status":"same"}"#).unwrap();
        assert_eq!(one.len(), 1);
    }

    #[test]
    fn parses_inventory() {
        let inv = parse_inventory(
            r#"{"machine":{"machineId":"abc","computerName":"BUDI"},
                "startup":{"source":"HKCU","name":"Spotify","command":"x.exe","approvedHex":null},
                "apps":[{"name":"SpotifyAB.SpotifyMusic","familyName":"SpotifyAB.SpotifyMusic_zpd","displayName":"Spotify","signatureKind":"Store","nonRemovable":false,"disabled":null,"disabledByUser":null}],
                "globalBgDisabled":null}"#,
        )
        .unwrap();
        assert_eq!(inv.machine.machine_id, "abc");
        assert_eq!(inv.startup.len(), 1);
        assert_eq!(inv.startup[0].approved_hex, None);
        assert_eq!(inv.apps[0].display_name.as_deref(), Some("Spotify"));
    }

    #[test]
    fn machine_id_falls_back_to_computer_name() {
        let inv = parse_inventory(r#"{"machine":{"machineId":null,"computerName":"KANTOR"},"startup":[],"apps":[]}"#).unwrap();
        assert_eq!(inv.machine.machine_id, "nama:KANTOR");
    }

    #[test]
    fn restore_point_answers() {
        assert_eq!(parse_restore_point(r#"{"ok":true,"name":"RepairDoc Mode Kentang 2026-11-26 13:50"}"#).unwrap(), "RepairDoc Mode Kentang 2026-11-26 13:50");
        assert!(parse_restore_point(r#"{"ok":false,"error":"The service cannot be started"}"#).unwrap_err().contains("service"));
    }

    #[test]
    fn ops_serialize_for_the_script() {
        let op = Op { key: "k".into(), target: Target::reg(Hive::Hklm, r"SOFTWARE\x", "V"), value: Value::Absent { key_existed: false } };
        let j = serde_json::to_string(&op).unwrap();
        assert_eq!(j, r#"{"key":"k","target":{"kind":"registry","hive":"HKLM","path":"SOFTWARE\\x","name":"V"},"value":{"type":"absent","keyExisted":false}}"#);
        let p = Op { key: "p".into(), target: Target::PowerPlan, value: Value::power("ABC") };
        assert_eq!(serde_json::to_string(&p).unwrap(), r#"{"key":"p","target":{"kind":"powerPlan"},"value":{"type":"power","guid":"abc"}}"#);
    }

    #[test]
    fn parses_startup_folders_and_logon_tasks() {
        // PowerShell 5.1 mengirim satu objek (bukan array) bila isinya hanya satu.
        let inv = parse_inventory(
            r#"{"machine":{"machineId":"abc","computerName":"BUDI"},"startup":[],"apps":[],
                "startupFiles":{"scope":"user","name":"Zoom.lnk","target":"C:\\Users\\b\\AppData\\Roaming\\Zoom\\bin\\Zoom.exe --autostart","approvedHex":null},
                "logonTasks":[{"path":"\\","name":"GoogleUpdateTaskMachineCore","state":"Ready","author":"","actions":{"execute":"C:\\Program Files (x86)\\Google\\Update\\GoogleUpdate.exe","arguments":"/c"}}]}"#,
        )
        .unwrap();
        assert_eq!(inv.startup_files.len(), 1);
        assert_eq!(inv.startup_files[0].scope, "user");
        assert_eq!(inv.logon_tasks.len(), 1);
        assert_eq!(inv.logon_tasks[0].path, "\\");
        assert_eq!(inv.logon_tasks[0].actions, vec![r"C:\Program Files (x86)\Google\Update\GoogleUpdate.exe".to_string()]);
        assert_eq!(inv.logon_tasks[0].arguments, vec!["/c".to_string()]);
        // Inventaris lama (tahap 3) tanpa kolom baru tetap terbaca.
        let old = parse_inventory(r#"{"machine":{"machineId":"x","computerName":"Y"},"startup":[],"apps":[]}"#).unwrap();
        assert!(old.startup_files.is_empty() && old.logon_tasks.is_empty());
    }

    // ------------------------------------------------ pemindah shortcut (jalan di mana saja)

    /// Folder Startup & data UJI di folder sementara, unik per tes, terhapus otomatis.
    struct Roots {
        base: PathBuf,
        roots: StartupRoots,
    }

    impl Drop for Roots {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    fn roots(test: &str) -> Roots {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("repairdoc-startup-test-{test}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let roots = StartupRoots { user: base.join("user-startup"), common: base.join("common-startup"), data: base.join("RepairDoc-data") };
        std::fs::create_dir_all(&roots.user).unwrap();
        std::fs::create_dir_all(&roots.common).unwrap();
        Roots { base, roots }
    }

    fn file_op(key: &str, scope: &str, name: &str, backup: &str, location: FileLocation) -> Op {
        Op { key: key.into(), target: Target::StartupFile { scope: scope.into(), name: name.into(), backup: backup.into() }, value: Value::Shortcut { location } }
    }

    const BAK: &str = "startup-cadangan/BUDI/2026-11-26_075000/pengguna/Zoom.lnk";

    #[test]
    fn shortcut_moves_to_backup_and_back_exactly() {
        let r = roots("bolak-balik");
        let original = r.roots.user.join("Zoom.lnk");
        let bytes: Vec<u8> = (0..=255u8).cycle().take(1500).collect();
        std::fs::write(&original, &bytes).unwrap();

        let off = file_op("z", "user", "Zoom.lnk", BAK, FileLocation::Backup);
        let res = run_file_ops(Some(&r.roots), std::slice::from_ref(&off), false);
        assert_eq!(res[0].status, OpStatus::Changed, "{res:?}");
        assert_eq!(res[0].before, Some(Value::Shortcut { location: FileLocation::Startup }));
        assert!(!original.exists(), "shortcut harus keluar dari folder Startup");
        let saved = r.roots.backup_path(BAK);
        assert_eq!(std::fs::read(&saved).unwrap(), bytes, "isi cadangan harus persis sama");

        // Kedua kali: sudah sesuai, tidak ada yang dipindah.
        assert_eq!(run_file_ops(Some(&r.roots), std::slice::from_ref(&off), false)[0].status, OpStatus::Same);

        let back = file_op("z", "user", "Zoom.lnk", BAK, FileLocation::Startup);
        assert_eq!(run_file_ops(Some(&r.roots), &[back], false)[0].status, OpStatus::Changed);
        assert_eq!(std::fs::read(&original).unwrap(), bytes, "kembali persis seperti semula");
        assert!(!saved.exists());
    }

    #[test]
    fn shortcut_dry_run_changes_nothing() {
        let r = roots("simulasi");
        std::fs::write(r.roots.common.join("ATK Hotkey.lnk"), b"lnk").unwrap();
        let op = file_op("a", "common", "ATK Hotkey.lnk", "startup-cadangan/BUDI/x/semua-pengguna/ATK Hotkey.lnk", FileLocation::Backup);
        let res = run_file_ops(Some(&r.roots), &[op], true);
        assert_eq!(res[0].status, OpStatus::Planned);
        assert!(r.roots.common.join("ATK Hotkey.lnk").exists());
        assert!(!r.roots.data.exists(), "simulasi tidak boleh membuat folder cadangan");
    }

    #[test]
    fn shortcut_is_never_overwritten() {
        let r = roots("tidak-timpa");
        std::fs::write(r.roots.user.join("Zoom.lnk"), b"baru").unwrap();
        let saved = r.roots.backup_path(BAK);
        std::fs::create_dir_all(saved.parent().unwrap()).unwrap();
        std::fs::write(&saved, b"lama").unwrap();
        let res = run_file_ops(Some(&r.roots), &[file_op("z", "user", "Zoom.lnk", BAK, FileLocation::Backup)], false);
        assert_eq!(res[0].status, OpStatus::Failed);
        assert!(res[0].error.as_deref().unwrap_or("").contains("tidak ditimpa"));
        assert_eq!(std::fs::read(r.roots.user.join("Zoom.lnk")).unwrap(), b"baru");
        assert_eq!(std::fs::read(&saved).unwrap(), b"lama");
    }

    #[test]
    fn missing_shortcut_is_not_present() {
        let r = roots("hilang");
        let res = run_file_ops(Some(&r.roots), &[file_op("z", "user", "Zoom.lnk", BAK, FileLocation::Backup)], false);
        assert_eq!(res[0].status, OpStatus::NotPresent);
    }

    #[test]
    fn unsafe_shortcut_targets_are_refused() {
        let r = roots("pengaman");
        std::fs::write(r.roots.user.join("app.exe"), b"x").unwrap();
        for op in [
            file_op("1", "user", "app.exe", BAK, FileLocation::Backup),
            file_op("2", "user", "..\\..\\Zoom.lnk", BAK, FileLocation::Backup),
            file_op("3", "user", "Zoom.lnk", "startup-cadangan/../../Windows/Zoom.lnk", FileLocation::Backup),
            file_op("4", "user", "Zoom.lnk", "C:/Windows/Zoom.lnk", FileLocation::Backup),
            file_op("5", "sistem", "Zoom.lnk", BAK, FileLocation::Backup),
        ] {
            let res = run_file_ops(Some(&r.roots), std::slice::from_ref(&op), false);
            assert_eq!(res[0].status, OpStatus::Refused, "{op:?}");
        }
        assert!(r.roots.user.join("app.exe").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_shortcut_is_not_followed() {
        let r = roots("symlink");
        let outside = r.base.join("di-luar.lnk");
        std::fs::write(&outside, b"jangan disentuh").unwrap();
        std::os::unix::fs::symlink(&outside, r.roots.user.join("Zoom.lnk")).unwrap();
        let res = run_file_ops(Some(&r.roots), &[file_op("z", "user", "Zoom.lnk", BAK, FileLocation::Backup)], false);
        assert_eq!(res[0].status, OpStatus::NotPresent, "symlink bukan shortcut biasa");
        assert!(outside.exists());
        assert!(std::fs::symlink_metadata(r.roots.user.join("Zoom.lnk")).is_ok(), "symlink-nya pun tidak dipindah");
    }

    #[cfg(not(windows))]
    #[test]
    fn real_backend_refuses_outside_windows() {
        let b = PowerShellBackend;
        assert!(!b.is_windows());
        assert!(b.inventory().is_err());
        assert!(run_ops(&[Op { key: "k".into(), target: Target::PowerPlan, value: Value::power("x") }], true).is_err());
    }
}

/// Tes yang hanya jalan di Windows. Eksekutor diuji di kunci UJI
/// `HKCU\Software\RepairDocTest` saja — TIDAK PERNAH kunci sistem asli.
///     cd src-tauri && cargo test -p repairdoc-core
#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use crate::preset::plan::{self, PlanOptions};
    use crate::preset::tweaks::{self, KEY_TEST_ROOT};

    /// Kunci uji dihapus sebelum dan sesudah tes.
    struct TestKey(String);

    impl TestKey {
        fn new(name: &str) -> Self {
            let k = TestKey(format!("{KEY_TEST_ROOT}\\{name}-{}", std::process::id()));
            k.wipe();
            k
        }
        fn wipe(&self) {
            let _ = platform::run_powershell(
                &format!("Remove-Item -LiteralPath 'Registry::HKEY_CURRENT_USER\\{}' -Recurse -Force -ErrorAction SilentlyContinue", self.0),
                Duration::from_secs(60),
            );
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            self.wipe();
        }
    }

    fn op(key: &str, path: &str, name: &str, value: Value) -> Op {
        Op { key: key.into(), target: Target::reg(Hive::Hkcu, path, name), value }
    }

    fn read(ops: &[Op]) -> Vec<Option<Value>> {
        run_ops(ops, true).unwrap().into_iter().map(|r| r.before).collect()
    }

    #[test]
    fn apply_revert_idempotent_and_dry_run_on_test_key() {
        let k = TestKey::new("dasar");
        let sub = format!("{}\\Sub", k.0);
        let ops = vec![
            op("dw", &k.0, "Angka", Value::dword(2)),
            op("sz", &k.0, "Teks", Value::string("0")),
            op("bin", &sub, "Biner", Value::binary(&[3, 0, 0, 0, 0xaa])),
        ];
        let initial = read(&ops);
        assert!(initial.iter().all(|v| matches!(v, Some(Value::Absent { .. }))), "{initial:?}");

        // Simulasi tidak menulis apa pun.
        let dry = run_ops(&ops, true).unwrap();
        assert!(dry.iter().all(|r| r.status == OpStatus::Planned), "{dry:?}");
        assert_eq!(read(&ops), initial, "-DryRun tidak boleh mengubah apa pun");

        // Terapkan.
        let applied = run_ops(&ops, false).unwrap();
        assert!(applied.iter().all(|r| r.status == OpStatus::Changed), "{applied:?}");
        assert_eq!(read(&ops), ops.iter().map(|o| Some(o.value.clone())).collect::<Vec<_>>());

        // Terapkan lagi: tidak ada yang berubah.
        let again = run_ops(&ops, false).unwrap();
        assert!(again.iter().all(|r| r.status == OpStatus::Same), "terapkan dua kali harus nol perubahan: {again:?}");

        // Kembalikan ke nilai lama (urutan terbalik), lalu buktikan sama dengan awal.
        let revert: Vec<Op> = applied.iter().rev().map(|r| Op { value: r.before.clone().unwrap(), ..ops.iter().find(|o| o.key == r.key).unwrap().clone() }).collect();
        let back = run_ops(&revert, false).unwrap();
        assert!(back.iter().all(|r| r.status == OpStatus::Changed), "{back:?}");
        assert_eq!(read(&ops), initial);
        // Kunci Sub yang dulu tidak ada ikut terhapus.
        let probe = platform::run_powershell(&format!("Test-Path 'Registry::HKEY_CURRENT_USER\\{sub}'"), Duration::from_secs(30)).unwrap();
        assert_eq!(probe.trim(), "False");
    }

    #[test]
    fn existing_values_come_back_exactly() {
        let k = TestKey::new("asli");
        let first = vec![op("a", &k.0, "MinAnimate", Value::string("1")), op("b", &k.0, "EnableTransparency", Value::dword(1))];
        run_ops(&first, false).unwrap();
        let change = vec![op("a", &k.0, "MinAnimate", Value::string("0")), op("b", &k.0, "EnableTransparency", Value::dword(0))];
        let applied = run_ops(&change, false).unwrap();
        assert_eq!(applied[0].before, Some(Value::string("1")), "nilai asli pengguna harus tercatat");
        let revert: Vec<Op> = applied.iter().zip(&change).map(|(r, o)| Op { value: r.before.clone().unwrap(), ..o.clone() }).collect();
        run_ops(&revert, false).unwrap();
        assert_eq!(read(&first), vec![Some(Value::string("1")), Some(Value::dword(1))]);
    }

    /// Definisi tweak sungguhan (bagian registry Mode Kentang) diarahkan ke kunci uji.
    #[test]
    fn kentang_registry_tweaks_on_test_key() {
        let k = TestKey::new("kentang");
        let inv = Inventory::default();
        let p = plan::build(Mode::Kentang, &inv, &PresetLog::new(), &PlanOptions::default(), 0);
        let redirected = plan::redirect_for_test(&p.ops, &k.0);
        assert!(redirected.len() >= 10);
        let ops: Vec<Op> = redirected.iter().map(|p| p.op.clone()).collect();
        let initial = read(&ops);
        let applied = run_ops(&ops, false).unwrap();
        assert!(applied.iter().all(|r| r.status == OpStatus::Changed), "{applied:?}");
        assert!(run_ops(&ops, false).unwrap().iter().all(|r| r.status == OpStatus::Same));
        let revert: Vec<Op> = applied.iter().rev().map(|r| Op { value: r.before.clone().unwrap(), ..ops.iter().find(|o| o.key == r.key).unwrap().clone() }).collect();
        run_ops(&revert, false).unwrap();
        assert_eq!(read(&ops), initial);
    }

    #[test]
    fn guards_refuse_even_in_dry_run() {
        let refused = run_ops(
            &[
                Op { key: "svc".into(), target: Target::Service { name: "wuauserv".into() }, value: Value::service("disabled") },
                op("reg", r"Software\Microsoft\Windows\CurrentVersion\Run", "Apa", Value::dword(1)),
            ],
            true,
        )
        .unwrap();
        assert!(refused.iter().all(|r| r.status == OpStatus::Refused), "{refused:?}");
        assert!(tweaks::validate_target(&Target::Service { name: "wuauserv".into() }).is_err());
    }

    #[test]
    fn reads_services_and_power_without_changing_them() {
        let ops = vec![
            Op { key: "s".into(), target: Target::Service { name: "Fax".into() }, value: Value::service("demand") },
            Op { key: "n".into(), target: Target::Service { name: "TidakAdaServiceIni123".into() }, value: Value::service("demand") },
            Op { key: "p".into(), target: Target::PowerPlan, value: Value::power(tweaks::SCHEME_BALANCED) },
        ];
        let r = run_ops(&ops, true).unwrap();
        assert!(matches!(r[0].before, Some(Value::Service { .. }) | Some(Value::Missing)), "{r:?}");
        assert_eq!(r[1].status, OpStatus::NotPresent);
        assert!(matches!(r[2].before, Some(Value::Power { .. })), "{r:?}");
    }

    #[test]
    fn inventory_script_outputs_json() {
        let inv = PowerShellBackend.inventory().unwrap();
        assert!(!inv.machine.machine_id.is_empty());
        let m = PowerShellBackend.machine().unwrap();
        assert_eq!(m.machine_id, inv.machine.machine_id);
    }

    #[test]
    fn restore_point_dry_run_creates_nothing() {
        let out = platform::run_powershell_script(RESTORE_POINT, "-DryRun", &[("REPAIRDOC_RP_PREFIX", "RepairDoc Tes")], Duration::from_secs(60)).unwrap();
        let v = json::parse(&out).unwrap();
        assert!(json::str_of(v.get("name")).starts_with("RepairDoc Tes "));
        assert_eq!(v.get("dryRun").and_then(|b| b.as_bool()), Some(true));
    }

    // ------------------------------------------------ tugas terjadwal (tahap 4)

    /// Tugas UJI di folder `\RepairDocTest\` (pemicu: saat masuk Windows, aksi: `cmd /c exit 0`).
    /// Dihapus lagi saat tes selesai. TIDAK PERNAH memakai tugas milik sistem.
    struct TestTask(String);

    impl TestTask {
        /// `None` bila tugas tidak bisa dibuat (mis. tidak ada izin) — tesnya lalu dilewati.
        fn new(tag: &str) -> Option<Self> {
            let name = format!("Tes-{tag}-{}", std::process::id());
            let script = r#"
try {
  $a = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument '/c exit 0'
  $t = New-ScheduledTaskTrigger -AtLogOn -User ($env:USERDOMAIN + '\' + $env:USERNAME)
  Register-ScheduledTask -TaskPath '\RepairDocTest\' -TaskName 'NAMA' -Action $a -Trigger $t -Force -ErrorAction Stop | Out-Null
  'ok'
} catch { 'gagal: ' + $_.Exception.Message }
"#
            .replace("NAMA", &name);
            match platform::run_powershell_script(&script, "", &[], Duration::from_secs(60)) {
                Ok(out) if out.trim() == "ok" => Some(TestTask(name)),
                other => {
                    eprintln!("Tugas uji tidak bisa dibuat ({other:?}) — tes ini dilewati (butuh izin admin?).");
                    None
                }
            }
        }

        fn target(&self) -> Target {
            Target::ScheduledTask { path: tweaks::TASK_TEST_PATH.into(), name: self.0.clone() }
        }
    }

    impl Drop for TestTask {
        fn drop(&mut self) {
            let script = r#"
Unregister-ScheduledTask -TaskPath '\RepairDocTest\' -TaskName 'NAMA' -Confirm:$false -ErrorAction SilentlyContinue
try {
  $s = New-Object -ComObject Schedule.Service; $s.Connect()
  if ($s.GetFolder('\RepairDocTest').GetTasks(1).Count -eq 0) { $s.GetFolder('\').DeleteFolder('RepairDocTest', 0) }
} catch {}
"#
            .replace("NAMA", &self.0);
            let _ = platform::run_powershell_script(&script, "", &[], Duration::from_secs(60));
        }
    }

    #[test]
    fn scheduled_task_disable_then_enable_returns_exactly() {
        let Some(t) = TestTask::new("aktif") else { return };
        let off = Op { key: "t".into(), target: t.target(), value: Value::Task { enabled: false } };
        let on = Op { key: "t".into(), target: t.target(), value: Value::Task { enabled: true } };

        let dry = run_ops(std::slice::from_ref(&off), true).unwrap();
        assert_eq!(dry[0].status, OpStatus::Planned, "{dry:?}");
        assert_eq!(dry[0].before, Some(Value::Task { enabled: true }));
        assert_eq!(read(std::slice::from_ref(&off)), vec![Some(Value::Task { enabled: true })], "simulasi tidak boleh mengubah apa pun");

        let r = run_ops(std::slice::from_ref(&off), false).unwrap();
        assert_eq!(r[0].status, OpStatus::Changed, "{r:?}");
        assert_eq!(run_ops(std::slice::from_ref(&off), false).unwrap()[0].status, OpStatus::Same, "kedua kali: sudah sesuai");

        let back = run_ops(std::slice::from_ref(&on), false).unwrap();
        assert_eq!(back[0].status, OpStatus::Changed, "{back:?}");
        assert_eq!(read(&[on]), vec![Some(Value::Task { enabled: true })], "kembali persis seperti semula");
    }

    #[test]
    fn windows_tasks_are_refused_even_in_dry_run() {
        // Hanya simulasi: pengaman harus menolak sebelum membaca apa pun.
        let op = Op {
            key: "w".into(),
            target: Target::ScheduledTask { path: r"\Microsoft\Windows\Defrag\".into(), name: "ScheduledDefrag".into() },
            value: Value::Task { enabled: false },
        };
        let r = run_ops(&[op], true).unwrap();
        assert_eq!(r[0].status, OpStatus::Refused, "{r:?}");
    }

    #[test]
    fn inventory_lists_logon_tasks_and_keeps_system_paths_out() {
        let Some(t) = TestTask::new("inventaris") else { return };
        let inv = PowerShellBackend.inventory().unwrap();
        let found = inv.logon_tasks.iter().find(|x| x.path == tweaks::TASK_TEST_PATH && x.name == t.0).expect("tugas uji harus terbaca");
        assert!(found.actions[0].to_ascii_lowercase().ends_with("cmd.exe"));
        // cmd.exe ada di folder Windows → bukan pihak ketiga → bukan kandidat.
        assert!(!plan::task_candidates(&inv).iter().any(|x| x.name == t.0));
        for f in &inv.startup_files {
            assert!(f.scope == "user" || f.scope == "common");
            assert!(f.name.to_ascii_lowercase().ends_with(".lnk"));
        }
    }
}
