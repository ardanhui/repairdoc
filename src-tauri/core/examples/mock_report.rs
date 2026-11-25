//! Membuat laporan contoh (laptop kentang khas) dari logika diagnosa yang asli.
//! Dipakai tampilan saat dibuka di browser biasa (mode pratinjau), supaya
//! teksnya persis sama dengan yang dihasilkan aplikasi.
//!     cargo run -p repairdoc-core --example mock_report > ../src/lib/mock-report.json
use std::path::PathBuf;
use std::time::Duration;

use repairdoc_core::analyze;
use repairdoc_core::dates::days_from_civil;
use repairdoc_core::model::{CategoryResult, TechSection};
use repairdoc_core::probe::battery::BatteryRaw;
use repairdoc_core::probe::cleanup::{CleanupRaw, FolderSize, JunkEntry, JunkKind};
use repairdoc_core::probe::cpu::{CpuRaw, TempReading, TempSource};
use repairdoc_core::probe::disk::{BusKind, DiskHealth, DiskRaw, MediaKind, PhysDiskRaw, VolumeRaw};
use repairdoc_core::probe::gpu::{AdapterRaw, GpuRaw};
use repairdoc_core::probe::live::ProcSample;
use repairdoc_core::probe::memory::{MemModule, MemRaw};
use repairdoc_core::probe::network::{AdapterRaw as NetAdapterRaw, DnsResult, DownloadResult, NetworkRaw, ProfileRaw};
use repairdoc_core::probe::programs::{StartupEntry, StartupRaw};
use repairdoc_core::probe::security::{AvProductRaw, DefenderRaw, ProgramRaw, SecurityRaw};
use repairdoc_core::probe::system::{BootRaw, OsRaw, PnpErrorRaw, PowerPlan, SystemRaw, UpdateRaw};
use repairdoc_core::scan;

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;

fn main() {
    let today = days_from_civil(2026, 11, 25);

    let cpu = CpuRaw {
        name: "Intel(R) Celeron(R) CPU N3350 @ 1.10GHz".into(),
        physical_cores: Some(2),
        logical_cores: 2,
        usage_avg: Some(34.0),
        usage_peak: Some(58.0),
        temp: Some(TempReading { celsius: 58.0, source: TempSource::ThermalZone }),
        notes: vec![],
    };
    let procs = vec![
        ProcSample { name: "chrome.exe".into(), instances: 14, cpu_pct: 11.0, memory: 1_700 * MIB },
        ProcSample { name: "msmpeng.exe".into(), instances: 1, cpu_pct: 8.0, memory: 210 * MIB },
        ProcSample { name: "explorer.exe".into(), instances: 1, cpu_pct: 2.0, memory: 120 * MIB },
        ProcSample { name: "onedrive.exe".into(), instances: 1, cpu_pct: 1.0, memory: 95 * MIB },
    ];
    let mem = MemRaw {
        total: (3.9 * GIB as f64) as u64,
        available: (0.5 * GIB as f64) as u64,
        swap_total: 4 * GIB,
        swap_used: (2.6 * GIB as f64) as u64,
        installed: Some(4 * GIB),
        slots_total: Some(1),
        slots_used: Some(1),
        modules: vec![MemModule { capacity: 4 * GIB, speed_mhz: Some(1600), maker: "Samsung".into(), slot: "ChannelA-DIMM0".into() }],
        memtest: None,
        notes: vec![],
    };
    let vol = |letter: char, label: &str, total: u64, free: f64| VolumeRaw {
        mount: format!("{letter}:\\"),
        letter: Some(letter),
        label: label.into(),
        file_system: "NTFS".into(),
        total: total * GIB,
        available: (free * GIB as f64) as u64,
        removable: false,
        kind: MediaKind::Hdd,
    };
    let mut usb = vol('E', "REPAIRDOC", 29, 21.4);
    usb.removable = true;
    usb.kind = MediaKind::Unknown;
    let disk = DiskRaw {
        volumes: vec![vol('C', "", 118, 1.8), vol('D', "DATA", 347, 312.0), usb],
        physical: vec![
            PhysDiskRaw {
                name: "WDC WD5000LPCX-24VHAT0".into(),
                media: MediaKind::Hdd,
                bus: BusKind::Sata,
                health: DiskHealth::Healthy,
                size: 500_107_862_016,
                letters: vec!['C', 'D'],
                wear: None,
                temp_c: None,
                read_errors_uncorrected: None,
                power_on_hours: None,
            },
            PhysDiskRaw {
                name: "SanDisk Cruzer Blade".into(),
                media: MediaKind::Unknown,
                bus: BusKind::Usb,
                health: DiskHealth::Healthy,
                size: 31_000_000_000,
                letters: vec!['E'],
                wear: None,
                temp_c: None,
                read_errors_uncorrected: None,
                power_on_hours: None,
            },
        ],
        physical_source: Some("storage".into()),
        reliability_readable: false,
        system_letter: Some('C'),
        exe_letter: Some('E'),
        notes: vec![],
    };
    let gpu = GpuRaw {
        adapters: vec![AdapterRaw {
            name: "Intel(R) HD Graphics 500".into(),
            vram: Some(GIB),
            driver_version: "27.20.100.9466".into(),
            driver_date: Some("2021-03-04".into()),
            status: "OK".into(),
            error_code: Some(0),
            resolution: Some((1366, 768)),
        }],
        nvidia: None,
        notes: vec![],
    };
    let battery = BatteryRaw {
        chassis: vec![10],
        has_battery: true,
        charge_pct: Some(64),
        on_ac: Some(true),
        full_capacity: Some(16_900),
        design_capacity: Some(36_600),
        cycles: Some(612),
        notes: vec![],
    };
    let system = SystemRaw {
        os: Some(OsRaw {
            caption: "Microsoft Windows 10 Home Single Language".into(),
            version: "10.0.19045".into(),
            build: 19045,
            ubr: Some(5131),
            display_version: Some("22H2".into()),
            arch: "64-bit".into(),
            install_date: Some("2021-05-02".into()),
            last_boot: Some("2026-11-13 07:12".into()),
        }),
        update: UpdateRaw { last_success: Some("2025-06-11".into()), last_hotfix: Some("2025-06-11".into()), last_event: None, wu_start: Some(4), reboot_pending: false },
        power_plan: Some(PowerPlan::PowerSaver),
        power_guid: Some("a1841308-3541-4fab-bc81-f71556f20b4a".into()),
        fast_startup: Some(true),
        page_file_auto: Some(true),
        pc_model: Some("ASUSTeK COMPUTER INC. X441NA".into()),
        pnp_errors: vec![PnpErrorRaw { name: "Unknown device".into(), code: 28, class: "Unknown".into() }],
        boot: Some(BootRaw { date: "2026-11-13".into(), boot_ms: Some(176_000), main_path_ms: Some(121_000), post_boot_ms: Some(55_000) }),
        boot_unreadable: false,
        uptime_secs: Some(12 * 86_400 + 5 * 3600),
        notes: vec![],
    };
    let startup_names = ["OneDrive", "Spotify", "Discord", "Steam", "WhatsApp", "Skype", "CCleaner64", "Epic Games Launcher", "iTunesHelper", "Adobe Updater", "uTorrent"];
    let startup = StartupRaw {
        entries: startup_names
            .iter()
            .map(|n| StartupEntry { name: n.to_string(), command: format!("\"C:\\Program Files\\{n}\\{n}.exe\" /background"), location: "HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run".into(), enabled: true })
            .chain(std::iter::once(StartupEntry { name: "Microsoft Teams".into(), command: String::new(), location: "StoreApp".into(), enabled: false }))
            .collect(),
        notes: vec![],
    };
    let junk = |kind: JunkKind, path: &str, bytes: u64, files: u64| JunkEntry { kind, path: PathBuf::from(path), size: FolderSize { bytes, files, denied: 0, truncated: false } };
    let cleanup = CleanupRaw {
        entries: vec![
            junk(JunkKind::Temp, "C:\\Users\\Budi\\AppData\\Local\\Temp", 1_340 * MIB, 4_812),
            junk(JunkKind::Temp, "C:\\Windows\\Temp", 210 * MIB, 311),
            junk(JunkKind::BrowserCache, "C:\\Users\\Budi\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache", 890 * MIB, 9_104),
            junk(JunkKind::RecycleBin, "C:\\$Recycle.Bin", 760 * MIB, 58),
            junk(JunkKind::UpdateCache, "C:\\Windows\\SoftwareDistribution\\Download", 95 * MIB, 40),
            junk(JunkKind::WindowsOld, "C:\\Windows.old", 18 * GIB, 102_331),
        ],
        hiberfil: Some((1.6 * GIB as f64) as u64),
        pagefile: Some(4 * GIB),
        notes: vec![],
    };
    let security = SecurityRaw {
        defender: Some(DefenderRaw {
            service_enabled: true,
            antivirus_enabled: true,
            real_time: true,
            signature_age_days: Some(19),
            signature_date: Some("2026-11-06".into()),
            quick_scan_age_days: Some(2),
            full_scan_age_days: None,
        }),
        av_products: vec![AvProductRaw { name: "Windows Defender".into(), enabled: true, up_to_date: false }],
        firewall_domain: Some(true),
        firewall_private: Some(true),
        firewall_public: Some(true),
        uac_enabled: Some(true),
        programs: vec![
            ProgramRaw { name: "Google Chrome".into(), publisher: "Google LLC".into(), install_date: "20240102".into(), size_kb: Some(340_000) },
            ProgramRaw { name: "KMSpico 10.2.0".into(), publisher: String::new(), install_date: "20230811".into(), size_kb: Some(12_000) },
            ProgramRaw { name: "Microsoft Office Professional Plus 2016".into(), publisher: "Microsoft Corporation".into(), install_date: "20230811".into(), size_kb: None },
            ProgramRaw { name: "CCleaner".into(), publisher: "Piriform Software Ltd".into(), install_date: "20240220".into(), size_kb: Some(90_000) },
        ],
        notes: vec![],
    };
    let network = NetworkRaw {
        profiles: vec![ProfileRaw { name: "Rumah".into(), alias: "Wi-Fi".into(), ipv4: "Internet".into(), category: "Private".into() }],
        adapters: vec![NetAdapterRaw { name: "Wi-Fi".into(), description: "Qualcomm Atheros QCA9377 Wireless Network Adapter".into(), link_speed: "72.2 Mbps".into(), wifi: true }],
        wifi_signal_pct: Some(32),
        wifi_ssid: Some("Rumah".into()),
        dns_servers: vec!["192.168.1.1".into()],
        internet: Some(true),
        dns: vec![
            DnsResult { host: "www.google.com".into(), millis: Some(61) },
            DnsResult { host: "www.microsoft.com".into(), millis: Some(88) },
            DnsResult { host: "www.detik.com".into(), millis: Some(70) },
        ],
        download: Some(DownloadResult { url: "http://speedtest.tele2.net/10MB.zip".into(), bytes: 4_200_000, millis: 8_000 }),
        notes: vec![],
    };

    let results: Vec<CategoryResult> = vec![
        analyze::cpu::analyze(&cpu),
        analyze::memory::analyze(&mem),
        analyze::disk::analyze(&disk),
        analyze::gpu::analyze(&gpu),
        analyze::battery::analyze(&battery).expect("laptop contoh punya baterai"),
        analyze::system::analyze(&system, today),
        analyze::programs::analyze(&startup, &procs, mem.total),
        analyze::cleanup::analyze(&cleanup, Some((1.8 * GIB as f64) as u64)),
        analyze::security::analyze(&security),
        analyze::network::analyze(&network),
    ];

    let mut info = TechSection::new("Tentang pemeriksaan ini");
    info.push("Versi RepairDoc", env!("CARGO_PKG_VERSION"));
    info.push("Sistem operasi", "Windows 10 Home 22H2 (contoh)");
    info.push("Izin administrator", "Tidak");
    info.push("Dijalankan dari", "Drive E: (dilewati saat diagnosa)");
    let mut tech = vec![info];
    let mut findings = Vec::new();
    for r in results {
        findings.extend(r.findings);
        tech.push(r.tech);
    }
    let needs_admin = vec![
        "umur SSD, suhu disk, dan riwayat error disk".to_string(),
        "sampah milik pengguna lain dan folder sistem".to_string(),
    ];
    let report = scan::finish(findings, tech, false, needs_admin, Duration::from_millis(31_400));
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
