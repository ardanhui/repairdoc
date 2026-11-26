//! Backend sungguhan: menjalankan `scripts/preset*.ps1` dan `restore_point.ps1`
//! lewat `platform::run_powershell_script`. Di sistem selain Windows semua
//! perintah mengembalikan galat "hanya Windows".

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
    let mut inv = Inventory { machine, startup, apps, global_bg_disabled: json::u64_of(v.get("globalBgDisabled")).map(|d| d as u32) };
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
        run_ops(ops, dry_run)
    }

    fn restore_point(&self, prefix: &str) -> Result<String, String> {
        let out = platform::run_powershell_script(RESTORE_POINT, "", &[("REPAIRDOC_RP_PREFIX", prefix)], RESTORE_TIMEOUT)?;
        parse_restore_point(&out)
    }
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
}
