//! Penilaian program: yang ikut jalan saat menyala, dan yang sedang membebani PC.

use crate::format::{percent, size};
use crate::model::*;
use crate::probe::live::ProcSample;
use crate::probe::programs::{display_name, StartupRaw};

/// Jumlah program startup aktif.
pub const STARTUP_MANY: usize = 8;
pub const STARTUP_SOME: usize = 4;
/// Satu program memakai prosesor ≥ ini (% dari seluruh tenaga) → perhatian.
pub const HEAVY_CPU_PCT: f64 = 30.0;
/// Satu program memakai RAM ≥ ini dari total → perhatian.
pub const HEAVY_MEM_FRACTION: f64 = 0.25;
pub const HEAVY_MEM_MIN: u64 = 1024 * 1024 * 1024;

/// Nama proses → nama yang dikenal pengguna, dan apakah ini tugas Windows yang
/// biasanya selesai sendiri (bukan program yang perlu ditutup).
pub fn friendly(name: &str) -> (String, bool) {
    let n = name.to_ascii_lowercase();
    let known: &[(&str, &str, bool)] = &[
        ("msmpeng.exe", "Windows Defender (sedang memindai)", true),
        ("mssense.exe", "Windows Defender", true),
        ("tiworker.exe", "Pemasang update Windows", true),
        ("trustedinstaller.exe", "Pemasang update Windows", true),
        ("wuauclt.exe", "Windows Update", true),
        ("usoclient.exe", "Windows Update", true),
        ("searchindexer.exe", "Pengindeks pencarian Windows", true),
        ("searchprotocolhost.exe", "Pengindeks pencarian Windows", true),
        ("compattelrunner.exe", "Telemetri Windows (CompatTelRunner)", true),
        ("wmiprvse.exe", "Layanan WMI Windows", true),
        ("svchost.exe", "Layanan Windows (svchost)", true),
        ("dwm.exe", "Efek tampilan Windows (Desktop Window Manager)", false),
        ("explorer.exe", "Windows Explorer", false),
        ("chrome.exe", "Google Chrome", false),
        ("msedge.exe", "Microsoft Edge", false),
        ("firefox.exe", "Mozilla Firefox", false),
        ("brave.exe", "Brave", false),
        ("opera.exe", "Opera", false),
        ("onedrive.exe", "OneDrive", false),
        ("teams.exe", "Microsoft Teams", false),
        ("ms-teams.exe", "Microsoft Teams", false),
        ("spotify.exe", "Spotify", false),
        ("discord.exe", "Discord", false),
        ("steam.exe", "Steam", false),
        ("steamwebhelper.exe", "Steam", false),
        ("winword.exe", "Microsoft Word", false),
        ("excel.exe", "Microsoft Excel", false),
        ("powerpnt.exe", "Microsoft PowerPoint", false),
        ("outlook.exe", "Microsoft Outlook", false),
        ("acrobat.exe", "Adobe Acrobat", false),
        ("acrord32.exe", "Adobe Reader", false),
        ("photoshop.exe", "Adobe Photoshop", false),
        ("zoom.exe", "Zoom", false),
        ("whatsapp.exe", "WhatsApp", false),
        ("telegram.exe", "Telegram", false),
        ("vlc.exe", "VLC", false),
        ("code.exe", "Visual Studio Code", false),
        ("smadav.exe", "Smadav", false),
        ("avastsvc.exe", "Avast", false),
        ("avgsvc.exe", "AVG", false),
        ("mcshield.exe", "McAfee", false),
        ("ccleaner64.exe", "CCleaner", false),
    ];
    for (proc_name, label, transient) in known {
        if n == *proc_name {
            return (label.to_string(), *transient);
        }
    }
    (name.trim_end_matches(".exe").to_string(), false)
}

pub fn analyze(startup: &StartupRaw, procs: &[ProcSample], total_mem: u64) -> CategoryResult {
    let cat = Category::Programs;
    let mut findings = Vec::new();

    // 1) Program startup
    let enabled = startup.enabled();
    let n = enabled.len();
    let names: Vec<String> = enabled.iter().take(5).map(|e| display_name(e)).collect();
    let more = if n > 5 { format!(", dan {} lagi", n - 5) } else { String::new() };
    let list = if names.is_empty() { String::new() } else { format!(" Di antaranya: {}{more}.", names.join(", ")) };
    let steps: &[&str] = &[
        "Tekan Ctrl + Shift + Esc untuk membuka Task Manager, buka tab Startup (atau \"Startup apps\").",
        "Klik kanan program yang tidak perlu jalan sendiri → Disable. Programnya tetap bisa dibuka manual kapan saja.",
        "Biarkan antivirus, driver audio, dan touchpad tetap aktif kalau tidak yakin.",
    ];
    if n >= STARTUP_MANY {
        findings.push(Finding::problem(
            "programs.startup",
            cat,
            Status::Bermasalah,
            Severity::Sedang,
            format!("{n} program ikut jalan saat PC menyala"),
            format!("Terlalu banyak untuk PC ini.{list}"),
            "Setiap program ini dimuat saat Windows menyala dan terus memakai memori di belakang layar walau tidak dipakai. Ini penyebab umum PC lambat menyala dan lemot sesudahnya.",
            steps,
        ));
    } else if n >= STARTUP_SOME {
        findings.push(Finding::problem(
            "programs.startup",
            cat,
            Status::Perhatian,
            Severity::Ringan,
            format!("{n} program ikut jalan saat PC menyala"),
            format!("Masih wajar, tapi bisa dikurangi.{list}"),
            "Program yang ikut jalan saat menyala memakai memori terus-menerus walau tidak dipakai.",
            &steps[..2],
        ));
    } else if !startup.entries.is_empty() || !crate::platform::IS_WINDOWS {
        findings.push(Finding::healthy(
            "programs.startup",
            cat,
            if n == 0 { "Tidak ada program tambahan saat PC menyala".to_string() } else { format!("Hanya {n} program yang ikut jalan saat PC menyala") },
            if n == 0 { "Bagus — Windows menyala tanpa beban tambahan.".to_string() } else { format!("Jumlah ini wajar.{list}") },
        ));
    }

    // 2) Program yang sedang membebani
    let heavy_cpu = procs.iter().find(|p| p.cpu_pct >= HEAVY_CPU_PCT);
    let mem_limit = ((total_mem as f64 * HEAVY_MEM_FRACTION) as u64).max(HEAVY_MEM_MIN);
    let heavy_mem = procs.iter().max_by_key(|p| p.memory).filter(|p| p.memory >= mem_limit);

    if let Some(p) = heavy_cpu {
        let (label, transient) = friendly(&p.name);
        if transient {
            findings.push(Finding::problem(
                "programs.heavy_cpu",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("{label} sedang membuat prosesor sibuk"),
                format!("Saat diperiksa, {label} memakai {} tenaga prosesor.", percent(p.cpu_pct)),
                "Ini tugas Windows yang biasanya selesai sendiri dalam beberapa menit sampai satu jam, terutama setelah PC lama tidak dinyalakan.",
                &["Biarkan PC menyala dan tersambung listrik sampai selesai, lalu scan ulang.", "Kalau setiap hari begini, minta teknisi memeriksa."],
            ));
        } else {
            findings.push(Finding::problem(
                "programs.heavy_cpu",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                format!("{label} membuat prosesor sibuk"),
                format!("Saat diperiksa, {label} memakai {} tenaga prosesor{}.", percent(p.cpu_pct), if p.instances > 1 { format!(" ({} proses)", p.instances) } else { String::new() }),
                "Satu program yang terus sibuk membuat semua program lain antre.",
                &[
                    "Kalau program itu tidak sedang dipakai, tutup lewat Task Manager (Ctrl + Shift + Esc → klik programnya → End task).",
                    "Kalau program itu memang sedang bekerja (mis. render video, update game), tunggu sampai selesai.",
                ],
            ));
        }
    }

    if let Some(p) = heavy_mem {
        let (label, _) = friendly(&p.name);
        let browser = matches!(p.name.as_str(), "chrome.exe" | "msedge.exe" | "firefox.exe" | "brave.exe" | "opera.exe");
        let steps: &[&str] = if browser {
            &["Tutup tab yang tidak dipakai — tiap tab memakan memori walau tidak dibuka.", "Matikan ekstensi browser yang tidak perlu.", "Restart browsernya sesekali."]
        } else {
            &["Tutup program itu kalau sedang tidak dipakai.", "Kalau selalu seperti ini, pertimbangkan menambah RAM."]
        };
        findings.push(Finding::problem(
            "programs.heavy_mem",
            cat,
            Status::Perhatian,
            Severity::Ringan,
            format!("{label} memakai banyak memori"),
            format!("Saat diperiksa, {label} memakai {}{}.", size(p.memory), if p.instances > 1 { format!(" ({} proses)", p.instances) } else { String::new() }),
            "Kalau satu program memakan sebagian besar RAM, program lain terpaksa pindah ke disk yang jauh lebih lambat.",
            steps,
        ));
    }

    if heavy_cpu.is_none() && heavy_mem.is_none() && !procs.is_empty() {
        let top: Vec<String> = procs.iter().take(3).map(|p| friendly(&p.name).0).collect();
        findings.push(Finding::healthy(
            "programs.heavy",
            cat,
            "Tidak ada program yang terlalu membebani saat ini",
            format!("Yang paling aktif saat diperiksa: {}.", top.join(", ")),
        ));
    }

    CategoryResult::new(cat, findings, tech(startup, procs))
}

fn tech(startup: &StartupRaw, procs: &[ProcSample]) -> TechSection {
    let mut t = TechSection::new("Program");
    let disabled = startup.entries.iter().filter(|e| !e.enabled).count();
    t.push("Startup", format!("{} aktif, {} dinonaktifkan", startup.enabled().len(), disabled));
    for e in startup.enabled() {
        let loc = if e.location.is_empty() { String::new() } else { format!(" · {}", e.location) };
        t.push(format!("Startup: {}", display_name(e)), format!("{}{loc}", if e.command.is_empty() { "-" } else { e.command.as_str() }));
    }
    let top_cpu: Vec<String> = procs.iter().take(5).map(|p| format!("{} {}", p.name, percent(p.cpu_pct))).collect();
    if !top_cpu.is_empty() {
        t.push("Proses teratas (CPU)", top_cpu.join(" · "));
    }
    let mut by_mem: Vec<&ProcSample> = procs.iter().collect();
    by_mem.sort_by_key(|p| std::cmp::Reverse(p.memory));
    let top_mem: Vec<String> = by_mem.iter().take(5).map(|p| format!("{} {}", p.name, size(p.memory))).collect();
    if !top_mem.is_empty() {
        t.push("Proses teratas (RAM)", top_mem.join(" · "));
    }
    for n in &startup.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::programs::StartupEntry;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn startup(n: usize) -> StartupRaw {
        StartupRaw {
            entries: (0..n).map(|i| StartupEntry { name: format!("App{i}"), enabled: true, ..Default::default() }).collect(),
            notes: vec![],
        }
    }

    fn proc_(name: &str, cpu: f64, mem: u64) -> ProcSample {
        ProcSample { name: name.into(), instances: 1, cpu_pct: cpu, memory: mem }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn startup_thresholds() {
        assert_eq!(find(&analyze(&startup(9), &[], 8 * GIB), "programs.startup").unwrap().status, Status::Bermasalah);
        assert_eq!(find(&analyze(&startup(5), &[], 8 * GIB), "programs.startup").unwrap().status, Status::Perhatian);
        assert_eq!(find(&analyze(&startup(2), &[], 8 * GIB), "programs.startup").unwrap().status, Status::Sehat);
    }

    #[test]
    fn heavy_cpu_distinguishes_transient_windows_tasks() {
        let r = analyze(&startup(0), &[proc_("msmpeng.exe", 55.0, 300_000_000)], 4 * GIB);
        let f = find(&r, "programs.heavy_cpu").unwrap();
        assert_eq!(f.severity, Some(Severity::Ringan));
        assert!(f.title.contains("Windows Defender"));
        let r = analyze(&startup(0), &[proc_("chrome.exe", 45.0, 300_000_000)], 4 * GIB);
        assert_eq!(find(&r, "programs.heavy_cpu").unwrap().severity, Some(Severity::Sedang));
    }

    #[test]
    fn heavy_memory_browser_advice() {
        let r = analyze(&startup(0), &[proc_("chrome.exe", 3.0, 2 * GIB)], 4 * GIB);
        let f = find(&r, "programs.heavy_mem").unwrap();
        assert!(f.summary.contains("2 GB"));
        assert!(f.steps[0].contains("tab"));
    }

    #[test]
    fn nothing_heavy_is_healthy() {
        let r = analyze(&startup(0), &[proc_("explorer.exe", 2.0, 100_000_000)], 4 * GIB);
        assert_eq!(find(&r, "programs.heavy").unwrap().status, Status::Sehat);
        assert!(find(&r, "programs.heavy_cpu").is_none());
    }
}
