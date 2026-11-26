//! Backend tiruan: "PC" di dalam memori. Dipakai tes alur Mode Performa dan
//! untuk membuat data contoh mode pratinjau (`examples/mock_preset.rs`).
//! Aturannya meniru `scripts/preset.ps1`: daftar izin, service terlindung,
//! "sudah sesuai" dilewati, simulasi tidak menulis apa pun.

use std::cell::RefCell;
use std::collections::HashMap;

use super::engine::Backend;
use super::model::*;
use super::tweaks::{self, *};

pub struct FakeBackend {
    pub windows: bool,
    pub admin: bool,
    /// Daftar program startup & aplikasi. Status StartupApproved/Disabled-nya
    /// selalu dibaca dari `values`, seperti di PC sungguhan.
    pub base: Inventory,
    pub values: RefCell<HashMap<Target, Value>>,
    pub power_plans: Vec<String>,
    pub restore_ok: bool,
    pub restore_points: RefCell<Vec<String>>,
    /// Kunci op yang selalu gagal saat ditulis (untuk tes).
    pub fail_keys: RefCell<Vec<String>>,
    /// Jumlah op yang ditulis sungguhan (bukan simulasi), per panggilan.
    pub writes: RefCell<Vec<usize>>,
}

impl FakeBackend {
    /// Laptop kentang khas: efek Windows bawaan, banyak program startup, power saver.
    pub fn potato_pc() -> Self {
        let base = Inventory {
            machine: MachineInfo { machine_id: "0b3c5e2a-1111-4c1d-9e0f-REPAIRDOC".into(), computer_name: "LAPTOP-BUDI".into() },
            startup: vec![
                startup("HKCU", "Spotify", r"C:\Users\Budi\AppData\Roaming\Spotify\Spotify.exe /minimized"),
                startup("HKCU", "Discord", r"C:\Users\Budi\AppData\Local\Discord\Update.exe --processStart Discord.exe"),
                startup("HKCU", "Steam", r#""C:\Program Files (x86)\Steam\steam.exe" -silent"#),
                startup("HKCU", "OneDrive", r#""C:\Program Files\Microsoft OneDrive\OneDrive.exe" /background"#),
                startup("HKLM", "SecurityHealth", r"%windir%\system32\SecurityHealthSystray.exe"),
                startup("HKLM", "RtkAudUService", r#""C:\Windows\System32\DriverStore\FileRepository\realtekservice.inf\RtkAudUService64.exe" -background"#),
                startup("HKLM32", "uTorrent", r#""C:\Program Files (x86)\uTorrent\uTorrent.exe" /MINIMIZED"#),
                startup("HKLM32", "SMADAV", r#""C:\Program Files (x86)\SMADAV\SM?RTP.exe""#),
            ],
            apps: vec![
                app("SpotifyAB.SpotifyMusic", "SpotifyAB.SpotifyMusic_zpdnekdrzrea0", Some("Spotify"), "Store"),
                app("5319275A.WhatsAppDesktop", "5319275A.WhatsAppDesktop_cv1g1gvanyjgm", Some("WhatsApp"), "Store"),
                app("king.com.CandyCrushSaga", "king.com.CandyCrushSaga_kgqvnymyfvs32", Some("Candy Crush Saga"), "Store"),
                app("Microsoft.WindowsAlarms", "Microsoft.WindowsAlarms_8wekyb3d8bbwe", Some("Jam"), "Store"),
                app("RealtekSemiconductorCorp.RealtekAudioControl", "RealtekSemiconductorCorp.RealtekAudioControl_dt26b99r8h8gj", None, "Store"),
                app("Microsoft.Windows.ShellExperienceHost", "Microsoft.Windows.ShellExperienceHost_cw5n1h2txyewy", None, "System"),
            ],
            global_bg_disabled: None,
            startup_files: Vec::new(),
            logon_tasks: Vec::new(),
        };
        let mut v = HashMap::new();
        let r = |p: &str, n: &str| Target::reg(Hive::Hkcu, p, n);
        v.insert(r(KEY_VISUAL_EFFECTS, "VisualFXSetting"), Value::dword(0));
        v.insert(r(KEY_DESKTOP, "UserPreferencesMask"), Value::binary(&[0x9e, 0x1e, 0x07, 0x80, 0x12, 0x00, 0x00, 0x00]));
        v.insert(r(KEY_DESKTOP, "DragFullWindows"), Value::string("1"));
        v.insert(r(KEY_EXPLORER_ADVANCED, "ListviewAlphaSelect"), Value::dword(1));
        v.insert(r(KEY_EXPLORER_ADVANCED, "ListviewShadow"), Value::dword(1));
        v.insert(r(KEY_PERSONALIZE, "EnableTransparency"), Value::dword(1));
        v.insert(r(KEY_WINDOW_METRICS, "MinAnimate"), Value::string("1"));
        v.insert(r(KEY_GAME_CONFIG, "GameDVR_Enabled"), Value::dword(1));
        v.insert(Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "Discord"), Value::Binary { hex: "030000006a1b2c3d4e5f6a01".into() });
        v.insert(Target::reg(Hive::Hkcu, KEY_STARTUP_APPROVED_RUN, "Steam"), Value::Binary { hex: "020000000000000000000000".into() });
        for (name, start) in [
            ("XblGameSave", "demand"),
            ("XboxGipSvc", "demand"),
            ("XboxNetApiSvc", "demand"),
            ("XblAuthManager", "demand"),
            ("Fax", "demand"),
            ("RemoteRegistry", "disabled"),
            ("MapsBroker", "delayed-auto"),
        ] {
            v.insert(Target::Service { name: name.into() }, Value::service(start));
        }
        // WMPNetworkSvc tidak ada (Windows Media Player lama tidak terpasang).
        v.insert(Target::PowerPlan, Value::power(SCHEME_MAX));
        FakeBackend {
            windows: true,
            admin: true,
            base,
            values: RefCell::new(v),
            power_plans: vec![SCHEME_BALANCED.into(), SCHEME_MIN.into(), SCHEME_MAX.into()],
            restore_ok: true,
            restore_points: RefCell::new(Vec::new()),
            fail_keys: RefCell::new(Vec::new()),
            writes: RefCell::new(Vec::new()),
        }
    }

    /// Laptop kentang yang sama, ditambah sumber startup tahap 4: shortcut di folder
    /// Startup, tugas terjadwal saat logon, dan aplikasi tambahan vendor hardware.
    pub fn potato_pc_lengkap() -> Self {
        let mut b = Self::potato_pc();
        b.base.startup.extend([
            startup("HKLM", "RtHDVCpl", r#""C:\Program Files\Realtek\Audio\HDA\RtkNGUI64.exe" -s"#),
            startup("HKLM32", "NvBackend", r#""C:\Program Files (x86)\NVIDIA Corporation\Update Core\NvBackend.exe""#),
            startup("HKLM", "SynTPEnh", r"%ProgramFiles%\Synaptics\SynTP\SynTPEnh.exe"),
            startup("HKCU", "Avast Antivirus", r#""C:\Program Files\Avast Software\Avast\AvastUI.exe" /nogui"#),
        ]);
        b.base.startup_files = vec![
            sfile("user", "Zoom.lnk", r"C:\Users\Budi\AppData\Roaming\Zoom\bin\Zoom.exe --autostart"),
            sfile("user", "Send to OneNote.lnk", r"C:\Program Files\Microsoft Office\root\Office16\ONENOTEM.EXE"),
            sfile("user", "Telegram.lnk", r"C:\Users\Budi\AppData\Roaming\Telegram Desktop\Telegram.exe -autostart"),
            sfile("common", "ATK Hotkey.lnk", r"C:\Program Files (x86)\ASUS\ATK Package\ATK Hotkey\HControl.exe"),
        ];
        // Telegram sudah dimatikan lewat Task Manager → bukan kandidat.
        b.base.startup_files[2].approved_hex = Some("030000001122334455667788".into());
        b.base.logon_tasks = vec![
            task(r"\", "GoogleUpdateTaskMachineCore{6A2B}", "Ready", "", r"C:\Program Files (x86)\Google\Update\GoogleUpdate.exe", "/c"),
            task(r"\", "Opera scheduled assistant Autoupdate 1700000000", "Ready", "", r"C:\Users\Budi\AppData\Local\Programs\Opera\launcher.exe", "--scheduledautoupdate"),
            task(r"\", "MicrosoftEdgeUpdateTaskMachineCore", "Ready", "", r"C:\Program Files (x86)\Microsoft\EdgeUpdate\MicrosoftEdgeUpdate.exe", "/c"),
            task(r"\Microsoft\Windows\Shell\", "FamilySafetyMonitor", "Ready", "Microsoft Corporation", r"C:\Windows\System32\WpcTsk.exe", ""),
            task(r"\", "Avast Emergency Update", "Ready", "Avast Software", r"C:\Program Files\Avast Software\Avast\AvEmUpdate.exe", ""),
            task(r"\", "CCleanerSkipUAC - Budi", "Disabled", "", r"C:\Program Files\CCleaner\CCleaner.exe", "$(Arg0)"),
            task(r"\", "BantuanCetak", "Ready", "", r"C:\Windows\System32\cmd.exe", "/c start printer.exe"),
        ];
        {
            let mut v = b.values.borrow_mut();
            for f in &b.base.startup_files {
                v.insert(file_key(&f.scope, &f.name), Value::Shortcut { location: FileLocation::Startup });
            }
            for t in &b.base.logon_tasks {
                v.insert(Target::ScheduledTask { path: t.path.clone(), name: t.name.clone() }, Value::Task { enabled: t.state != "Disabled" });
            }
        }
        b
    }

    /// Nilai sebuah target saat ini (registry yang belum ada = "tidak ada").
    pub fn read(&self, t: &Target) -> Value {
        if let Target::StartupFile { scope, name, backup } = t {
            let values = self.values.borrow();
            return if values.contains_key(&file_key(scope, name)) {
                Value::Shortcut { location: FileLocation::Startup }
            } else if values.contains_key(&backup_key(backup)) {
                Value::Shortcut { location: FileLocation::Backup }
            } else {
                Value::Missing
            };
        }
        if let Some(v) = self.values.borrow().get(t) {
            return v.clone();
        }
        match t {
            Target::Registry { .. } => Value::absent(),
            Target::Service { .. } | Target::ScheduledTask { .. } | Target::StartupFile { .. } => Value::Missing,
            Target::PowerPlan => Value::power(SCHEME_BALANCED),
        }
    }

    /// Tulis satu nilai seperti eksekutor sungguhan (shortcut = pindah file).
    fn write(&self, target: &Target, value: &Value) {
        let mut values = self.values.borrow_mut();
        match (target, value) {
            (Target::StartupFile { scope, name, backup }, Value::Shortcut { location }) => {
                let (from, to) = match location {
                    FileLocation::Backup => (file_key(scope, name), backup_key(backup)),
                    FileLocation::Startup => (backup_key(backup), file_key(scope, name)),
                };
                values.remove(&from);
                values.insert(to, Value::Shortcut { location: *location });
            }
            (_, Value::Absent { .. }) => {
                values.remove(target);
            }
            (t, v) => {
                values.insert(t.clone(), v.clone());
            }
        }
    }

    /// Salinan semua nilai, untuk membandingkan "sebelum" dan "sesudah".
    pub fn snapshot(&self) -> HashMap<Target, Value> {
        self.values.borrow().clone()
    }
}

fn startup(source: &str, name: &str, command: &str) -> StartupItem {
    StartupItem { source: source.into(), name: name.into(), command: command.into(), approved_hex: None }
}

fn sfile(scope: &str, name: &str, target: &str) -> StartupFile {
    StartupFile { scope: scope.into(), name: name.into(), target: target.into(), approved_hex: None }
}

fn task(path: &str, name: &str, state: &str, author: &str, exe: &str, args: &str) -> LogonTask {
    LogonTask { path: path.into(), name: name.into(), state: state.into(), author: author.into(), actions: vec![exe.into()], arguments: vec![args.into()] }
}

/// Kunci "file di folder Startup" di peta nilai tiruan.
fn file_key(scope: &str, name: &str) -> Target {
    Target::StartupFile { scope: scope.into(), name: name.into(), backup: String::new() }
}

/// Kunci "file di folder cadangan" di peta nilai tiruan.
fn backup_key(backup: &str) -> Target {
    Target::StartupFile { scope: "cadangan".into(), name: backup.into(), backup: String::new() }
}

fn app(name: &str, family: &str, display: Option<&str>, kind: &str) -> AppItem {
    AppItem {
        name: name.into(),
        family_name: family.into(),
        display_name: display.map(|d| d.to_string()),
        signature_kind: kind.into(),
        non_removable: kind == "System",
        disabled: None,
        disabled_by_user: None,
    }
}

impl Backend for FakeBackend {
    fn is_windows(&self) -> bool {
        self.windows
    }

    fn is_admin(&self) -> bool {
        self.admin
    }

    fn machine(&self) -> Result<MachineInfo, String> {
        Ok(self.base.machine.clone())
    }

    fn inventory(&self) -> Result<Inventory, String> {
        let mut inv = self.base.clone();
        for s in inv.startup.iter_mut() {
            if let Some(t) = super::plan::approved_target(s) {
                s.approved_hex = match self.read(&t) {
                    Value::Binary { hex } => Some(hex),
                    _ => None,
                };
            }
        }
        for a in inv.apps.iter_mut() {
            let path = format!("{KEY_BACKGROUND_APPS}\\{}", a.family_name);
            let dw = |n: &str| match self.read(&Target::reg(Hive::Hkcu, &path, n)) {
                Value::Dword { value } => Some(value),
                _ => None,
            };
            a.disabled = dw("Disabled");
            a.disabled_by_user = dw("DisabledByUser");
        }
        inv.global_bg_disabled = match self.read(&Target::reg(Hive::Hkcu, KEY_BACKGROUND_APPS, "GlobalUserDisabled")) {
            Value::Dword { value } => Some(value),
            _ => None,
        };
        // Shortcut yang sudah dipindah ke cadangan tidak terlihat lagi di folder Startup.
        let values = self.values.borrow();
        inv.startup_files.retain(|f| values.contains_key(&file_key(&f.scope, &f.name)));
        inv.logon_tasks.retain_mut(|t| match values.get(&Target::ScheduledTask { path: t.path.clone(), name: t.name.clone() }) {
            Some(Value::Task { enabled }) => {
                if !enabled {
                    t.state = "Disabled".into();
                } else if t.state == "Disabled" {
                    t.state = "Ready".into();
                }
                true
            }
            _ => false,
        });
        Ok(inv)
    }

    fn run(&self, ops: &[Op], dry_run: bool) -> Result<Vec<OpResult>, String> {
        let mut out = Vec::new();
        let mut written = 0;
        for op in ops {
            let mut r = OpResult { key: op.key.clone(), before: None, after: None, status: OpStatus::Failed, error: None };
            if let Err(e) = tweaks::validate_target(&op.target) {
                r.status = OpStatus::Refused;
                r.error = Some(e);
                out.push(r);
                continue;
            }
            let before = self.read(&op.target);
            r.before = Some(before.clone());
            let missing_plan = matches!((&op.target, &op.value), (Target::PowerPlan, Value::Power { guid }) if !self.power_plans.iter().any(|p| p.eq_ignore_ascii_case(guid)));
            if before == Value::Missing || missing_plan {
                r.status = OpStatus::NotPresent;
                r.error = Some("Tidak ada di PC ini".into());
            } else if before.same_as(&op.value) {
                r.status = OpStatus::Same;
            } else if dry_run {
                r.status = OpStatus::Planned;
            } else if self.fail_keys.borrow().contains(&op.key) {
                r.error = Some("Gagal ditulis (simulasi tes)".into());
            } else {
                self.write(&op.target, &op.value);
                written += 1;
                r.after = Some(op.value.clone());
                r.status = OpStatus::Changed;
            }
            out.push(r);
        }
        if !dry_run {
            self.writes.borrow_mut().push(written);
        }
        Ok(out)
    }

    fn restore_point(&self, prefix: &str) -> Result<String, String> {
        if !self.restore_ok {
            return Err("System Restore dimatikan di drive ini.".into());
        }
        let name = format!("{prefix} 2026-11-26 13:50");
        self.restore_points.borrow_mut().push(name.clone());
        Ok(name)
    }
}
