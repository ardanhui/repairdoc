//! Mode Performa (tahap 3): preset ala pengaturan grafis game.
//!
//! Dipisah seperti `probe`/`analyze`:
//! - `tweaks` : definisi tweak sebagai data + daftar aman (logika murni)
//! - `plan`   : menyusun daftar perubahan dari definisi + kondisi PC (logika murni)
//! - `engine` : alur status → pratinjau → restore point → terapkan → catat (lewat `Backend`)
//! - `log`    : catatan perubahan `preset-log.json`
//! - `exec`   : backend sungguhan (PowerShell: registry, `sc.exe`, `powercfg`, restore point)
//! - `fake`   : backend tiruan di memori untuk tes dan data contoh
//!
//! Prinsip: tidak ada perubahan tanpa pengaman. Scan (modul `probe`) tetap hanya membaca;
//! semua penulisan ada di sini.
//!
//! Tahap 4 memperluas tweak `startup-off` ke empat sumber: kunci Run (HKCU, HKLM,
//! HKLM 32-bit) lewat StartupApproved, shortcut di folder Startup pengguna & semua
//! pengguna (dipindah ke `RepairDoc-data\startup-cadangan`), dan tugas terjadwal pihak
//! ketiga dengan pemicu logon (dinonaktifkan). Daftar lindung dipersempit ke
//! Microsoft/Windows/keamanan/antivirus (`tweaks::PROTECTED_WORDS`).

pub mod engine;
pub mod exec;
pub mod fake;
pub mod log;
pub mod model;
pub mod plan;
pub mod tweaks;

pub use engine::{apply, preview, revert_last, status, Backend, ModeInfo, PresetStatus, TweakInfo};
pub use model::*;
pub use plan::PlanOptions;

/// Waktu sekarang dalam milidetik sejak 1970.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::fake::FakeBackend;
    use super::tweaks::*;
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ------------------------------------------------------------ definisi

    #[test]
    fn every_tweak_is_complete() {
        let mut ids = std::collections::HashSet::new();
        for t in TWEAKS {
            assert!(ids.insert(t.id), "id {} dobel", t.id);
            assert!(!t.id.is_empty() && !t.title.is_empty() && !t.description.is_empty(), "{} belum lengkap", t.id);
            assert!(!t.read_how.is_empty(), "{}: cara baca nilai lama kosong", t.id);
            assert!(!t.revert_how.is_empty(), "{}: cara kembali kosong", t.id);
            assert!(!t.modes.is_empty() && !t.modes.contains(&Mode::Normal), "{}: mode tidak valid", t.id);
            if let TweakKind::Registry(settings) = t.kind {
                assert!(!settings.is_empty());
                for s in settings {
                    assert!(!s.label.is_empty());
                    assert!(validate_target(&s.target()).is_ok(), "{} menulis di luar daftar izin", s.name);
                }
            }
        }
        for want in [
            "visual-best-performance",
            "transparency-off",
            "animations-off",
            "game-dvr-off",
            "background-apps-off",
            "startup-off",
            "services-manual",
            "power-high-performance",
            "power-balanced",
        ] {
            assert!(tweak(want).is_some(), "tweak {want} dari spesifikasi tidak ada");
        }
    }

    #[test]
    fn mode_lists_match_spec() {
        let ids = |m| tweaks_for(m).iter().map(|t| t.id).collect::<Vec<_>>();
        assert_eq!(
            ids(Mode::Kentang),
            vec![
                "visual-best-performance",
                "transparency-off",
                "animations-off",
                "game-dvr-off",
                "background-apps-off",
                "startup-off",
                "services-manual",
                "power-high-performance"
            ]
        );
        assert_eq!(ids(Mode::Seimbang), vec!["transparency-off", "animations-off", "game-dvr-off", "power-balanced"]);
        assert!(ids(Mode::Normal).is_empty(), "Normal tidak punya tweak sendiri — ia mengembalikan");
    }

    /// Seimbang adalah subset Kentang. Satu-satunya pengecualian adalah power plan:
    /// keduanya mengatur setelan yang sama (power plan aktif), hanya nilainya berbeda.
    #[test]
    fn seimbang_is_subset_of_kentang() {
        let kentang = tweaks_for(Mode::Kentang);
        for t in tweaks_for(Mode::Seimbang) {
            let in_kentang = kentang.iter().any(|k| k.id == t.id);
            let same_setting = matches!(t.kind, TweakKind::Power(_)) && kentang.iter().any(|k| matches!(k.kind, TweakKind::Power(_)));
            assert!(in_kentang || same_setting, "{} ada di Seimbang tapi tidak di Kentang", t.id);
        }
        // Tanpa service & startup & efek visual penuh.
        for t in tweaks_for(Mode::Seimbang) {
            assert!(!matches!(t.kind, TweakKind::Services(_) | TweakKind::Startup | TweakKind::BackgroundApps));
            assert_ne!(t.id, "visual-best-performance");
        }
    }

    #[test]
    fn protected_services_are_never_touched() {
        for t in TWEAKS {
            if let TweakKind::Services(names) = t.kind {
                for n in names {
                    assert!(!is_protected_service(n), "{n} ada di daftar JANGAN SENTUH");
                }
            }
            if let TweakKind::Registry(settings) = t.kind {
                for s in settings {
                    for p in PROTECTED_SERVICES {
                        let needle = format!("services\\{}", p.to_ascii_lowercase());
                        assert!(!s.path.to_ascii_lowercase().contains(&needle), "{} menyentuh service {p}", s.path);
                    }
                }
            }
        }
        for must in ["wuauserv", "WinDefend", "MpsSvc", "WlanSvc", "Audiosrv", "Spooler", "WSearch", "SysMain", "BITS", "Dhcp", "Dnscache", "EventLog", "PlugPlay", "Power", "RpcSs"] {
            assert!(is_protected_service(must), "{must} harus terlindung");
            assert!(validate_target(&Target::Service { name: must.into() }).is_err());
        }
        let fake = FakeBackend::potato_pc();
        let r = fake.run(&[Op { key: "x".into(), target: Target::Service { name: "wuauserv".into() }, value: Value::service("disabled") }], false).unwrap();
        assert_eq!(r[0].status, OpStatus::Refused);
    }

    #[test]
    fn registry_outside_allowlist_is_refused() {
        assert!(validate_target(&Target::reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Services\wuauserv", "Start")).is_err());
        assert!(validate_target(&Target::reg(Hive::Hkcu, r"Software\RepairDocTest\a\b", "x")).is_ok());
        assert!(validate_target(&Target::reg(Hive::Hklm, KEY_TEST_ROOT, "x")).is_err(), "kunci uji hanya di HKCU");
    }

    #[test]
    fn script_lists_match_rust_lists() {
        let ps = include_str!("../scripts/preset.ps1");
        for (_, p) in REGISTRY_ALLOWED {
            assert!(ps.contains(p), "daftar izin di preset.ps1 tidak memuat {p}");
        }
        for s in PROTECTED_SERVICES {
            assert!(ps.contains(&format!("'{s}'")), "daftar terlindung di preset.ps1 tidak memuat {s}");
        }
    }

    #[test]
    fn scripts_fit_on_the_command_line() {
        for (name, script) in [
            ("preset.ps1", include_str!("../scripts/preset.ps1")),
            ("preset_inventory.ps1", include_str!("../scripts/preset_inventory.ps1")),
            ("restore_point.ps1", include_str!("../scripts/restore_point.ps1")),
        ] {
            let len = crate::platform::encode_command(&crate::platform::script_call(script, "-DryRun")).len();
            assert!(len < crate::platform::MAX_ENCODED_COMMAND, "{name} terlalu panjang: {len}");
        }
    }

    #[test]
    fn value_texts_are_plain() {
        let t = Target::reg(Hive::Hkcu, KEY_PERSONALIZE, "EnableTransparency");
        assert_eq!(value_text(&t, &Value::dword(1)), "Nyala");
        assert_eq!(value_text(&t, &Value::dword(0)), "Mati");
        let m = Target::reg(Hive::Hkcu, KEY_WINDOW_METRICS, "MinAnimate");
        assert_eq!(value_text(&m, &Value::string("0")), "Mati");
        assert_eq!(value_text(&Target::Service { name: "Fax".into() }, &Value::service("delayed-auto")), "Otomatis (tertunda)");
        assert_eq!(value_text(&Target::PowerPlan, &Value::power(SCHEME_MIN)), "High performance");
        let st = Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "Spotify");
        assert_eq!(value_text(&st, &Value::absent()), "Nyala saat PC menyala");
        assert_eq!(value_text(&st, &Value::Binary { hex: plan::startup_disabled_hex(1) }), "Tidak ikut menyala");
        let mask = Target::reg(Hive::Hkcu, KEY_DESKTOP, "UserPreferencesMask");
        let custom = Value::Binary { hex: "9e1e078012000000".into() };
        assert_eq!(value_text(&mask, &custom), "Sebagian efek nyala");
        assert!(!value_text(&mask, &custom).contains("9e1e"));
    }

    // ------------------------------------------------------------ alur (backend tiruan)

    struct TempLog(PathBuf);

    impl Drop for TempLog {
        fn drop(&mut self) {
            if let Some(d) = self.0.parent() {
                let _ = std::fs::remove_dir_all(d);
            }
        }
    }

    fn temp_log(test: &str) -> TempLog {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!("repairdoc-preset-{test}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        TempLog(d.join(log::LOG_FILE))
    }

    const T0: u64 = 1_764_139_800_000;

    fn run_apply(b: &FakeBackend, l: &TempLog, mode: Mode, opts: &PlanOptions, t: u64) -> Outcome {
        apply(b, &l.0, mode, opts, t, &mut |_| {})
    }

    fn run_revert(b: &FakeBackend, l: &TempLog, t: u64) -> Outcome {
        revert_last(b, &l.0, t, &mut |_| {})
    }

    fn val(b: &FakeBackend, t: Target) -> Value {
        b.read(&t)
    }

    #[test]
    fn kentang_applies_and_revert_restores_exactly() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("kentang");
        let before = b.snapshot();
        let mut steps = Vec::new();
        let o = apply(&b, &l.0, Mode::Kentang, &PlanOptions::default(), T0, &mut |e| steps.push(e));
        assert!(o.ok, "{:?}", o.error);
        assert!(o.changed > 10, "harus banyak yang berubah: {}", o.changed);
        assert_eq!(o.failed, 0);
        assert_eq!(b.restore_points.borrow().len(), 1, "restore point dibuat sebelum mengubah");
        assert_eq!(steps.first(), Some(&PresetEvent::Reading));
        assert!(steps.contains(&PresetEvent::RestorePoint));

        assert_eq!(val(&b, Target::reg(Hive::Hkcu, KEY_VISUAL_EFFECTS, "VisualFXSetting")), Value::dword(2));
        assert_eq!(val(&b, Target::reg(Hive::Hkcu, KEY_WINDOW_METRICS, "MinAnimate")), Value::string("0"));
        assert_eq!(val(&b, Target::reg(Hive::Hklm, KEY_GAMEDVR_POLICY, "AllowGameDVR")), Value::dword(0));
        assert_eq!(val(&b, Target::Service { name: "MapsBroker".into() }), Value::service("demand"));
        assert_eq!(val(&b, Target::Service { name: "RemoteRegistry".into() }), Value::service("disabled"), "Dimatikan tidak boleh dinaikkan jadi Manual");
        assert_eq!(val(&b, Target::PowerPlan), Value::power(SCHEME_MIN));
        assert!(val(&b, Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "Spotify")).first_byte() == Some(3));
        assert_eq!(val(&b, Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "OneDrive")), Value::absent(), "Microsoft tidak boleh disentuh");
        assert_eq!(val(&b, Target::reg(Hive::Hklm, KEY_STARTUP_APPROVED_RUN32, "SMADAV")), Value::absent(), "antivirus tidak boleh disentuh");
        let wa = Target::reg(Hive::Hkcu, &format!("{KEY_BACKGROUND_APPS}\\5319275A.WhatsAppDesktop_cv1g1gvanyjgm"), "Disabled");
        assert_eq!(val(&b, wa), Value::dword(1));
        let alarms = Target::reg(Hive::Hkcu, &format!("{KEY_BACKGROUND_APPS}\\Microsoft.WindowsAlarms_8wekyb3d8bbwe"), "Disabled");
        assert_eq!(val(&b, alarms), Value::absent(), "aplikasi Microsoft tidak boleh dibatasi");

        let entry = o.entry.unwrap();
        assert!(entry.perubahan.iter().any(|c| c.label == "Spotify" && c.tweak_id == "startup-off" && c.status == ChangeStatus::Berhasil));
        assert_eq!(status(&b, &l.0).active_mode, Mode::Kentang);

        let r = run_revert(&b, &l, T0 + 1);
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.failed, 0);
        assert_eq!(b.snapshot(), before, "semua nilai harus kembali persis seperti semula");
        let st = status(&b, &l.0);
        assert_eq!(st.active_mode, Mode::Normal);
        assert!(st.revert_target.is_none());
        assert!(st.last_entry.unwrap().dikembalikan);
    }

    #[test]
    fn applying_twice_changes_nothing_the_second_time() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("idempotent");
        assert!(run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0).ok);
        let after_first = b.snapshot();
        let o = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0 + 60_000);
        assert!(o.ok);
        assert_eq!(o.changed, 0, "terapkan dua kali → perubahan kedua nol");
        assert_eq!(b.writes.borrow().len(), 1, "tidak ada penulisan pada penerapan kedua");
        assert_eq!(b.restore_points.borrow().len(), 1, "tidak perlu restore point kalau tidak ada yang diubah");
        assert_eq!(b.snapshot(), after_first);
        let entry = o.entry.unwrap();
        assert!(entry.perubahan.iter().all(|c| c.status == ChangeStatus::Dilewati));
        assert!(entry.perubahan.iter().any(|c| c.keterangan.as_deref() == Some("Sudah sesuai")));
    }

    #[test]
    fn preview_changes_nothing() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("preview");
        let before = b.snapshot();
        let p = preview(&b, &l.0, Mode::Kentang, &PlanOptions::default(), T0).unwrap();
        assert_eq!(b.snapshot(), before, "simulasi tidak boleh mengubah apa pun");
        assert!(b.writes.borrow().is_empty());
        assert!(b.restore_points.borrow().is_empty());
        assert!(p.change_count > 10);
        let startup = p.groups.iter().find(|g| g.tweak_id == "startup-off").unwrap();
        let names: Vec<&str> = startup.changes.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(names, vec!["Spotify", "Steam", "uTorrent"], "Discord sudah mati; Microsoft, antivirus, audio dilindungi");
        let maps = p.groups.iter().flat_map(|g| &g.changes).find(|c| c.key == "service:MapsBroker").unwrap();
        assert_eq!((maps.current.as_str(), maps.new.as_str(), maps.will_change), ("Otomatis (tertunda)", "Manual", true));
        let remote = p.groups.iter().flat_map(|g| &g.changes).find(|c| c.key == "service:RemoteRegistry").unwrap();
        assert!(!remote.will_change);
        let wmp = p.groups.iter().flat_map(|g| &g.changes).find(|c| c.key == "service:WMPNetworkSvc").unwrap();
        assert_eq!(wmp.note.as_deref(), Some("Tidak ada di PC ini"));
    }

    #[test]
    fn normal_reverts_kentang_and_seimbang_to_the_original() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("normal-revert");
        let before = b.snapshot();
        assert!(run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0).ok);
        let s = run_apply(&b, &l, Mode::Seimbang, &PlanOptions::default(), T0 + 1000);
        assert!(s.ok);
        assert_eq!(b.read(&Target::PowerPlan), Value::power(SCHEME_BALANCED));
        assert_eq!(status(&b, &l.0).active_mode, Mode::Seimbang);

        let p = preview(&b, &l.0, Mode::Normal, &PlanOptions::default(), T0 + 2000).unwrap();
        assert_eq!(p.path, PlanPath::RevertLog);
        let n = run_apply(&b, &l, Mode::Normal, &PlanOptions::default(), T0 + 3000);
        assert!(n.ok, "{:?}", n.error);
        assert_eq!(b.snapshot(), before, "Normal harus mengembalikan semua yang diubah Kentang + Seimbang");
        let log = log::load(&l.0);
        assert_eq!(log.entri.len(), 3);
        assert!(log.entri[0].dikembalikan && log.entri[1].dikembalikan);
        assert_eq!(log.entri[2].jenis, EntryKind::Kembalikan);
        assert_eq!(log.entri[2].membatalkan.len(), 2);
        let st = status(&b, &l.0);
        assert_eq!(st.active_mode, Mode::Normal);
        assert!(st.revert_target.is_none(), "Normal yang membatalkan tidak dibatalkan lagi lewat Kembalikan Semua");
    }

    #[test]
    fn revert_all_undoes_one_application_at_a_time() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("revert-steps");
        let before = b.snapshot();
        run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0);
        let after_kentang = b.snapshot();
        run_apply(&b, &l, Mode::Seimbang, &PlanOptions::default(), T0 + 1000);
        assert!(run_revert(&b, &l, T0 + 2000).ok);
        assert_eq!(b.snapshot(), after_kentang, "Kembalikan Semua membatalkan penerapan terakhir (Seimbang)");
        assert!(run_revert(&b, &l, T0 + 3000).ok);
        assert_eq!(b.snapshot(), before);
        let o = run_revert(&b, &l, T0 + 4000);
        assert_eq!(o.error.unwrap().kind, ErrorKind::NothingToRevert);
    }

    #[test]
    fn normal_without_log_sets_windows_defaults_and_can_be_undone() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("defaults");
        let before = b.snapshot();
        let o = run_apply(&b, &l, Mode::Normal, &PlanOptions::default(), T0);
        assert!(o.ok);
        assert_eq!(o.entry.as_ref().unwrap().jenis, EntryKind::Terapkan);
        assert_eq!(b.read(&Target::PowerPlan), Value::power(SCHEME_BALANCED), "Power saver → Balanced");
        assert_eq!(b.read(&Target::reg(Hive::Hkcu, KEY_GAMEDVR_USER, "AppCaptureEnabled")), Value::dword(1));
        assert_eq!(b.read(&Target::reg(Hive::Hkcu, KEY_VISUAL_EFFECTS, "VisualFXSetting")), Value::dword(0));
        assert!(run_revert(&b, &l, T0 + 1).ok);
        assert_eq!(b.snapshot(), before);
    }

    #[test]
    fn restore_point_failure_stops_everything() {
        let mut b = FakeBackend::potato_pc();
        b.restore_ok = false;
        let l = temp_log("no-restore");
        let before = b.snapshot();
        let o = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0);
        assert!(!o.ok);
        let err = o.error.unwrap();
        assert_eq!(err.kind, ErrorKind::RestorePoint);
        assert!(err.help.iter().any(|h| h.contains("system protection")), "harus ada cara menyalakan System Restore");
        assert_eq!(b.snapshot(), before, "tanpa restore point, tidak boleh ada yang diubah");
        assert!(b.writes.borrow().is_empty());
        assert!(log::load(&l.0).entri.is_empty());
    }

    #[test]
    fn not_admin_changes_nothing() {
        let mut b = FakeBackend::potato_pc();
        b.admin = false;
        let l = temp_log("not-admin");
        let o = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0);
        assert_eq!(o.error.unwrap().kind, ErrorKind::NotAdmin);
        assert_eq!(preview(&b, &l.0, Mode::Kentang, &PlanOptions::default(), T0).unwrap_err().kind, ErrorKind::NotAdmin);
        assert_eq!(run_revert(&b, &l, T0).error.unwrap().kind, ErrorKind::NotAdmin);
        assert!(b.writes.borrow().is_empty());
        assert!(!status(&b, &l.0).is_admin);
    }

    #[test]
    fn toggled_off_tweaks_and_unchecked_items_are_logged_as_skipped() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("toggles");
        let opts = PlanOptions { disabled_tweaks: vec!["power-high-performance".into()], skip_items: vec!["startup:HKCU:Spotify".into()] };
        let o = run_apply(&b, &l, Mode::Kentang, &opts, T0);
        assert!(o.ok);
        assert_eq!(b.read(&Target::PowerPlan), Value::power(SCHEME_MAX), "tweak yang dimatikan tidak diterapkan");
        assert_eq!(b.read(&Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "Spotify")), Value::absent());
        let e = o.entry.unwrap();
        let power = e.perubahan.iter().find(|c| c.tweak_id == "power-high-performance").unwrap();
        assert_eq!(power.status, ChangeStatus::Dilewati);
        assert_eq!(power.keterangan.as_deref(), Some("Dimatikan lewat pilihan tweak"));
        let spotify = e.perubahan.iter().find(|c| c.label == "Spotify" && c.tweak_id == "startup-off").unwrap();
        assert_eq!(spotify.status, ChangeStatus::Dilewati);
    }

    #[test]
    fn a_failing_tweak_does_not_stop_the_others() {
        let b = FakeBackend::potato_pc();
        b.fail_keys.borrow_mut().push("transparency-off:EnableTransparency".into());
        let l = temp_log("one-fails");
        let before = b.snapshot();
        let o = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0);
        assert_eq!(o.failed, 1);
        assert_eq!(o.failures[0].label, "Efek transparan taskbar & menu Start");
        assert!(o.changed > 10, "tweak lain tetap jalan");
        let e = o.entry.unwrap();
        assert_eq!(e.count(ChangeStatus::Gagal), 1);
        assert!(run_revert(&b, &l, T0 + 1).ok);
        assert_eq!(b.snapshot(), before);
    }

    #[test]
    fn another_pcs_changes_are_never_reverted() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("other-pc");
        assert!(run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0).ok);
        // Flashdisk yang sama dicolok ke PC lain.
        let mut other = FakeBackend::potato_pc();
        other.base.machine = MachineInfo { machine_id: "PC-LAIN".into(), computer_name: "KANTOR-01".into() };
        let before_other = other.snapshot();
        assert_eq!(run_revert(&other, &l, T0 + 1).error.unwrap().kind, ErrorKind::NothingToRevert);
        assert_eq!(status(&other, &l.0).active_mode, Mode::Normal);
        let n = preview(&other, &l.0, Mode::Normal, &PlanOptions::default(), T0 + 2).unwrap();
        assert_eq!(n.path, PlanPath::Defaults, "catatan PC lain tidak dipakai");
        assert_eq!(other.snapshot(), before_other);
    }

    #[test]
    fn counts_are_per_item_not_per_registry_value() {
        let b = FakeBackend::potato_pc();
        let l = temp_log("per-item");
        let p = preview(&b, &l.0, Mode::Kentang, &PlanOptions::default(), T0).unwrap();
        let bg = p.groups.iter().find(|g| g.tweak_id == "background-apps-off").unwrap();
        let mut items: Vec<&str> = bg.changes.iter().map(|c| c.item_key.as_str()).collect();
        items.dedup();
        assert_eq!(items.len(), 3, "Spotify, WhatsApp, Candy Crush — masing-masing dua nilai registry");
        let o = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0);
        assert_eq!(o.changed, p.change_count, "jumlah di pratinjau = jumlah yang berhasil");
        let r = run_revert(&b, &l, T0 + 1);
        assert_eq!(r.changed, o.changed, "jumlah yang dikembalikan = jumlah yang diubah");
    }

    #[test]
    fn mode_infos_for_cards() {
        let infos = engine::mode_infos();
        assert_eq!(infos.iter().map(|m| m.level.as_str()).collect::<Vec<_>>(), vec!["Low", "Medium", "High"]);
        assert_eq!(infos[0].tweak_ids.len(), 8);
        assert_eq!(infos[1].tweak_ids.len(), 4);
    }

    // ------------------------------------------------------------ tahap 4: startup lebih ganas

    use super::plan::{startup_candidates, startup_file_candidates, task_candidates, SOURCE_FOLDER, SOURCE_REGISTRY, SOURCE_TASK};

    #[test]
    fn protected_list_is_exactly_the_owners_list() {
        let owner = [
            "microsoft", "windows", "security", "defender", "antivirus", "anti-virus", "virus", "smadav", "avast", "avg", "kaspersky", "eset", "mcafee",
            "norton", "bitdefender", "avira", "sophos", "malwarebytes", "trend micro", "panda", "comodo", "360 total",
        ];
        assert_eq!(PROTECTED_WORDS, owner);
        let hardware = [
            "realtek", "rtkaud", "rtkngui", "ravcpl", "waves", "maxxaudio", "dolby", "conexant", "synaptics", "syntp", "elan", "etdctrl", "alps",
            "touchpad", "igfx", "hkcmd", "persistence", "intel", "nvidia", "nvcpl", "amd", "radeon", "bluetooth", "btray", "hotkey", "atkkey", "fnkey",
            "onekey", "dptf",
        ];
        assert_eq!(HARDWARE_HELPER_WORDS, hardware);
        assert!(!PROTECTED_WORDS.iter().any(|w| HARDWARE_HELPER_WORDS.contains(w)));
    }

    /// Inventaris berisi program "penting" di keempat sumber.
    fn important_everywhere() -> Inventory {
        let names = ["Windows Defender notification", "SecurityHealth", "Microsoft Teams", "Avast Antivirus", "Kaspersky", "ESET GUI", "McAfee", "Norton", "Bitdefender Agent", "Malwarebytes Tray", "Trend Micro", "360 Total Security", "SMADAV", "AVG UI", "Panda Dome", "Comodo", "Sophos", "Avira"];
        let mut inv = Inventory { machine: MachineInfo { machine_id: "M".into(), computer_name: "PC".into() }, ..Default::default() };
        for (i, n) in names.iter().enumerate() {
            let exe = format!(r"C:\Program Files\Vendor{i}\app.exe");
            for src in ["HKCU", "HKLM", "HKLM32"] {
                inv.startup.push(StartupItem { source: src.into(), name: n.to_string(), command: exe.clone(), approved_hex: None });
            }
            inv.startup_files.push(StartupFile { scope: "user".into(), name: format!("{n}.lnk"), target: exe.clone(), approved_hex: None });
            inv.logon_tasks.push(LogonTask { path: "\\".into(), name: n.to_string(), state: "Ready".into(), author: String::new(), actions: vec![exe.clone()], arguments: vec![String::new()] });
        }
        // Nama biasa, tapi program/penulisnya milik Microsoft atau ada di folder Windows.
        inv.startup.push(StartupItem { source: "HKCU".into(), name: "OneDrive".into(), command: r"C:\Users\b\AppData\Local\Microsoft\OneDrive\OneDrive.exe".into(), approved_hex: None });
        inv.startup.push(StartupItem { source: "HKLM".into(), name: "RtkAudUService".into(), command: r"C:\Windows\System32\RtkAudUService64.exe".into(), approved_hex: None });
        inv.startup_files.push(StartupFile { scope: "common".into(), name: "Send to OneNote.lnk".into(), target: r"C:\Program Files\Microsoft Office\ONENOTEM.EXE".into(), approved_hex: None });
        inv.logon_tasks.push(LogonTask { path: "\\".into(), name: "Helper".into(), state: "Ready".into(), author: "Microsoft Corporation".into(), actions: vec![r"C:\Program Files\Helper\h.exe".into()], arguments: vec![String::new()] });
        inv.logon_tasks.push(LogonTask { path: "\\Microsoft\\Windows\\Shell\\".into(), name: "Lain".into(), state: "Ready".into(), author: String::new(), actions: vec![r"C:\Program Files\Lain\l.exe".into()], arguments: vec![String::new()] });
        inv
    }

    #[test]
    fn important_apps_are_never_startup_candidates() {
        let inv = important_everywhere();
        assert!(startup_candidates(&inv).is_empty(), "{:?}", startup_candidates(&inv));
        assert!(startup_file_candidates(&inv).is_empty(), "{:?}", startup_file_candidates(&inv));
        assert!(task_candidates(&inv).is_empty(), "{:?}", task_candidates(&inv));
        assert!(plan::ops_for_tweak(tweak("startup-off").unwrap(), &inv, T0).is_empty());
    }

    #[test]
    fn hardware_vendor_apps_become_candidates_with_effect_notes() {
        let b = FakeBackend::potato_pc_lengkap();
        let inv = b.inventory().unwrap();
        let ops = plan::ops_for_tweak(tweak("startup-off").unwrap(), &inv, T0);
        let find = |label: &str| ops.iter().find(|p| p.label == label).unwrap_or_else(|| panic!("{label} harus jadi kandidat"));
        // GeForce Experience (NvBackend), Realtek (RtkNGUI64), Synaptics, tombol Fn ASUS.
        assert!(find("NvBackend").effect.as_deref().unwrap().contains("NVIDIA"));
        assert!(find("RtHDVCpl").effect.as_deref().unwrap().contains("Suara tetap keluar"));
        assert!(find("SynTPEnh").effect.as_deref().unwrap().contains("touchpad"));
        assert!(find("ATK Hotkey").effect.as_deref().unwrap().contains("Tombol Fn"));
        // Program biasa tidak diberi catatan karangan; peluncur Discord bukan "pembaru".
        assert_eq!(find("Spotify").effect, None);
        assert_eq!(startup_effect_note("Discord", r"C:\Users\Budi\AppData\Local\Discord\Update.exe --processStart Discord.exe"), None);
        assert!(find("GoogleUpdateTaskMachineCore{6A2B}").effect.as_deref().unwrap().contains("Pembaru otomatis"));
        // Pembantu hardware yang file-nya di folder Windows tetap terlindung (jalurnya mengandung "windows").
        assert!(!ops.iter().any(|p| p.label == "RtkAudUService"));
        // Kata pendek tidak salah tangkap: IntelliJ bukan Intel, "command" bukan AMD.
        assert_eq!(startup_effect_note("IntelliJ IDEA Toolbox", r"C:\Users\b\AppData\Local\JetBrains\Toolbox\bin\jetbrains-toolbox.exe"), None);
        assert_eq!(startup_effect_note("Commander", r"C:\Program Files\Commander\commander.exe --command"), None);
        assert!(startup_effect_note("Intel(R) Graphics", r"C:\Program Files\Intel\x.exe").is_some());
    }

    #[test]
    fn every_startup_item_says_its_source_and_command() {
        let b = FakeBackend::potato_pc_lengkap();
        let ops = plan::ops_for_tweak(tweak("startup-off").unwrap(), &b.inventory().unwrap(), T0);
        let by = |src: &str| ops.iter().filter(|p| p.source.as_deref() == Some(src)).map(|p| p.label.as_str()).collect::<Vec<_>>();
        assert_eq!(by(SOURCE_REGISTRY), vec!["Spotify", "Steam", "uTorrent", "RtHDVCpl", "NvBackend", "SynTPEnh"], "Discord sudah mati, OneDrive/SecurityHealth/RtkAudUService/SMADAV/Avast terlindung");
        assert_eq!(by(SOURCE_FOLDER), vec!["Zoom", "ATK Hotkey"], "OneNote (Microsoft) dan Telegram (sudah mati) tidak ikut");
        assert_eq!(by(SOURCE_TASK), vec!["GoogleUpdateTaskMachineCore{6A2B}", "Opera scheduled assistant Autoupdate 1700000000"]);
        assert!(ops.iter().all(|p| p.optional && p.detail.is_some() && p.tweak_id == "startup-off"));
    }

    #[test]
    fn runonce_winlogon_and_services_are_never_startup_sources() {
        let mut inv = Inventory { machine: MachineInfo { machine_id: "M".into(), computer_name: "PC".into() }, ..Default::default() };
        for src in ["RunOnce", "HKCU-RunOnce", "Winlogon", "Userinit", "Shell", "Service", ""] {
            inv.startup.push(StartupItem { source: src.into(), name: format!("Prog{src}"), command: r"C:\Program Files\Prog\p.exe".into(), approved_hex: None });
        }
        assert!(startup_candidates(&inv).is_empty());
        // Skrip inventaris memang tidak membaca sumber itu (komentar diabaikan).
        let code = crate::platform::compact_script(include_str!("../scripts/preset_inventory.ps1")).to_ascii_lowercase();
        for banned in ["runonce", "winlogon", "userinit", "get-service", "win32_service"] {
            assert!(!code.contains(banned), "preset_inventory.ps1 tidak boleh membaca {banned}");
        }
    }

    #[test]
    fn startup_folder_and_task_targets_are_guarded() {
        let ok = Target::StartupFile { scope: "user".into(), name: "Zoom.lnk".into(), backup: plan::backup_rel("LAPTOP BUDI", "user", "Zoom.lnk", T0) };
        assert!(validate_target(&ok).is_ok(), "{:?}", validate_target(&ok));
        assert_eq!(plan::backup_rel("LAPTOP BUDI", "common", "ATK Hotkey.lnk", T0), "startup-cadangan/LAPTOP_BUDI/2025-11-26_065000/semua-pengguna/ATK Hotkey.lnk");
        for bad in [
            Target::StartupFile { scope: "user".into(), name: "app.exe".into(), backup: "startup-cadangan/x/app.exe".into() },
            Target::StartupFile { scope: "user".into(), name: "Zoom.lnk".into(), backup: "lain/Zoom.lnk".into() },
            Target::StartupFile { scope: "user".into(), name: "Zoom.lnk".into(), backup: "startup-cadangan/../Zoom.lnk".into() },
            Target::ScheduledTask { path: "\\Microsoft\\Windows\\Defrag\\".into(), name: "ScheduledDefrag".into() },
            Target::ScheduledTask { path: "\\microsoft\\".into(), name: "x".into() },
            Target::ScheduledTask { path: "tanpa-garis".into(), name: "x".into() },
        ] {
            assert!(validate_target(&bad).is_err(), "{bad:?}");
        }
        // Daftar larangan yang sama tertulis di preset.ps1.
        let script = include_str!("../scripts/preset.ps1");
        assert!(script.contains(&format!("$TaskForbidden = @('{TASK_PATH_FORBIDDEN}')")));
    }

    #[test]
    fn kentang_handles_all_startup_sources_and_revert_restores_exactly() {
        let b = FakeBackend::potato_pc_lengkap();
        let l = temp_log("empat-sumber");
        let before = b.snapshot();
        let o = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0);
        assert!(o.ok, "{:?}", o.error);
        assert_eq!(o.failed, 0, "{:?}", o.failures);

        let zoom = Target::StartupFile { scope: "user".into(), name: "Zoom.lnk".into(), backup: plan::backup_rel("LAPTOP-BUDI", "user", "Zoom.lnk", T0) };
        assert_eq!(b.read(&zoom), Value::Shortcut { location: FileLocation::Backup }, "shortcut dipindah ke cadangan, bukan dihapus");
        let google = Target::ScheduledTask { path: "\\".into(), name: "GoogleUpdateTaskMachineCore{6A2B}".into() };
        assert_eq!(b.read(&google), Value::Task { enabled: false }, "tugas dinonaktifkan, bukan dihapus");
        assert_eq!(b.read(&Target::ScheduledTask { path: "\\".into(), name: "MicrosoftEdgeUpdateTaskMachineCore".into() }), Value::Task { enabled: true });
        assert_eq!(b.read(&Target::ScheduledTask { path: "\\".into(), name: "Avast Emergency Update".into() }), Value::Task { enabled: true });
        let one_note = Target::StartupFile { scope: "user".into(), name: "Send to OneNote.lnk".into(), backup: String::new() };
        assert_eq!(b.read(&one_note), Value::Shortcut { location: FileLocation::Startup });
        assert!(b.read(&Target::reg(Hive::Hklm, KEY_STARTUP_APPROVED_RUN, "RtHDVCpl")).first_byte() == Some(3));

        let entry = o.entry.unwrap();
        for label in ["Zoom", "ATK Hotkey", "GoogleUpdateTaskMachineCore{6A2B}"] {
            let c = entry.perubahan.iter().find(|c| c.label == label).unwrap();
            assert_eq!(c.status, ChangeStatus::Berhasil);
            assert!(c.nilai_lama.is_some(), "nilai lama {label} harus tercatat");
        }

        let r = run_revert(&b, &l, T0 + 1);
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.failed, 0);
        assert_eq!(b.snapshot(), before, "ketiga jenis sumber harus kembali persis seperti semula");
    }

    #[test]
    fn new_sources_are_idempotent_and_preview_changes_nothing() {
        let b = FakeBackend::potato_pc_lengkap();
        let l = temp_log("dua-kali-t4");
        let before = b.snapshot();
        let p = preview(&b, &l.0, Mode::Kentang, &PlanOptions::default(), T0).unwrap();
        assert_eq!(b.snapshot(), before, "pratinjau tidak mengubah apa pun");
        let g = p.groups.iter().find(|g| g.tweak_id == "startup-off").unwrap();
        for src in [SOURCE_REGISTRY, SOURCE_FOLDER, SOURCE_TASK] {
            assert!(g.changes.iter().any(|c| c.source.as_deref() == Some(src) && c.will_change && c.detail.is_some()), "{src}");
        }
        assert!(g.changes.iter().any(|c| c.label == "NvBackend" && c.effect.is_some()));

        assert!(run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0).ok);
        let o2 = run_apply(&b, &l, Mode::Kentang, &PlanOptions::default(), T0 + 5);
        assert_eq!(o2.changed, 0, "terapkan dua kali → perubahan kedua nol");
        assert_eq!(b.writes.borrow().len(), 1);
    }

    #[test]
    fn unchecked_folder_and_task_items_are_skipped_and_logged() {
        let b = FakeBackend::potato_pc_lengkap();
        let l = temp_log("centang-t4");
        let opts = PlanOptions { disabled_tweaks: vec![], skip_items: vec!["startup:folder:user:Zoom.lnk".into(), "startup:task:\\GoogleUpdateTaskMachineCore{6A2B}".into()] };
        let o = run_apply(&b, &l, Mode::Kentang, &opts, T0);
        assert!(o.ok);
        let zoom = Target::StartupFile { scope: "user".into(), name: "Zoom.lnk".into(), backup: plan::backup_rel("LAPTOP-BUDI", "user", "Zoom.lnk", T0) };
        assert_eq!(b.read(&zoom), Value::Shortcut { location: FileLocation::Startup });
        assert_eq!(b.read(&Target::ScheduledTask { path: "\\".into(), name: "GoogleUpdateTaskMachineCore{6A2B}".into() }), Value::Task { enabled: true });
        let e = o.entry.unwrap();
        for label in ["Zoom", "GoogleUpdateTaskMachineCore{6A2B}"] {
            let c = e.perubahan.iter().find(|c| c.label == label).unwrap();
            assert_eq!(c.status, ChangeStatus::Dilewati);
            assert_eq!(c.keterangan.as_deref(), Some("Tidak dicentang pengguna"));
        }
    }

    #[test]
    fn seimbang_and_normal_defaults_never_touch_startup() {
        let b = FakeBackend::potato_pc_lengkap();
        let l = temp_log("seimbang-t4");
        let s = preview(&b, &l.0, Mode::Seimbang, &PlanOptions::default(), T0).unwrap();
        assert!(!s.groups.iter().any(|g| g.tweak_id == "startup-off" || g.tweak_id == "services-manual"));
        let n = preview(&b, &l.0, Mode::Normal, &PlanOptions::default(), T0).unwrap();
        assert!(!n.groups.iter().any(|g| g.tweak_id == "startup-off"));
    }

    #[test]
    fn background_apps_keep_hardware_protection_from_tahap_3() {
        let b = FakeBackend::potato_pc_lengkap();
        let inv = b.inventory().unwrap();
        let realtek = inv.apps.iter().find(|a| a.name.starts_with("RealtekSemiconductorCorp")).unwrap();
        assert!(!plan::app_is_candidate(realtek), "pelonggaran tahap 4 hanya untuk startup");
    }
}
