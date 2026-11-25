//! Penilaian Windows: versi, update, driver, waktu boot, power plan, restart.

use super::tanggal;
use crate::dates::{ago_text, days_ago};
use crate::model::*;
use crate::probe::system::{PowerPlan, SystemRaw};

/// Windows 11: update terakhir lebih lama dari ini (hari) → perhatian / bermasalah.
pub const UPDATE_STALE_DAYS: i64 = 45;
pub const UPDATE_VERY_STALE_DAYS: i64 = 120;
/// Belum restart lebih lama dari ini (hari) → perhatian.
pub const UPTIME_LONG_DAYS: u64 = 7;
/// Waktu boot (jalur utama, detik).
pub const BOOT_SLOW_S: u64 = 90;
pub const BOOT_VERY_SLOW_S: u64 = 150;
/// Perangkat bermasalah sebanyak ini atau lebih → bermasalah (bukan sekadar perhatian).
pub const PNP_MANY: usize = 3;

/// Arti kode error Device Manager yang sering muncul.
pub fn pnp_code_text(code: u32) -> &'static str {
    match code {
        1 => "belum dikonfigurasi",
        3 => "driver rusak atau kehabisan memori",
        10 => "perangkat tidak bisa dijalankan",
        12 => "kekurangan sumber daya",
        18 => "driver perlu dipasang ulang",
        19 => "data registry rusak",
        24 => "perangkat tidak ada atau tidak bekerja",
        28 => "driver belum terpasang",
        31 | 37 | 39 | 52 => "driver gagal dimuat",
        43 => "perangkat melaporkan masalah",
        _ => "bermasalah",
    }
}

pub fn analyze(raw: &SystemRaw, today: i64) -> CategoryResult {
    let cat = Category::System;
    let mut findings = Vec::new();
    let major = raw.os.as_ref().and_then(|o| o.major());

    // 1) Versi Windows
    if let Some(os) = &raw.os {
        let ver = os.display_version.as_deref().map(|v| format!(" versi {v}")).unwrap_or_default();
        match major {
            Some(11) => findings.push(Finding::healthy("system.version", cat, "Windows 11 — masih didukung", format!("{}{ver}.", os.caption))),
            Some(10) => findings.push(Finding::problem(
                "system.version",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                "Windows 10 sudah tidak mendapat pembaruan keamanan",
                format!("{}{ver}. Microsoft menghentikan update keamanan rutin Windows 10 pada 14 Oktober 2025.", os.caption),
                "PC tetap bisa dipakai, tapi celah keamanan yang ditemukan setelah itu tidak akan ditambal. Risiko paling besar ada di browsing dan file unduhan.",
                &[
                    "Pastikan antivirus aktif dan rutin diperbarui — ini jadi pertahanan utama.",
                    "Ekstra hati-hati dengan lampiran email, unduhan, dan program bajakan.",
                    "Cek apakah PC ini bisa di-upgrade ke Windows 11 (gratis) lewat aplikasi \"PC Health Check\" dari Microsoft. Banyak PC lama tidak memenuhi syarat — itu wajar.",
                ],
            )),
            Some(7) | Some(8) => findings.push(Finding::problem(
                "system.version",
                cat,
                Status::Bermasalah,
                Severity::Berat,
                "Windows ini sudah sangat lama",
                format!("{}. Windows versi ini sudah bertahun-tahun tidak mendapat update apa pun.", os.caption),
                "Banyak program dan browser baru tidak bisa dipasang lagi, dan celah keamanannya tidak pernah ditambal.",
                &[
                    "Simpan data penting ke flashdisk atau hardisk eksternal.",
                    "Minta teknisi memasang Windows 10 atau 11 (tergantung kemampuan PC).",
                ],
            )),
            _ => {}
        }
    }

    // 2) Windows Update
    let latest = raw.update.latest();
    let age = latest.as_deref().and_then(|d| days_ago(d, today));
    let last_text = match (&latest, age) {
        (Some(d), Some(a)) => format!("Update terakhir terpasang {} ({}).", tanggal(d), ago_text(a)),
        _ => "Tanggal update terakhir tidak terbaca.".to_string(),
    };
    if raw.update.service_disabled() {
        findings.push(Finding::problem(
            "system.update",
            cat,
            Status::Bermasalah,
            Severity::Sedang,
            "Windows Update dimatikan",
            format!("Layanan Windows Update di PC ini sengaja dimatikan. {last_text}"),
            "Tanpa Windows Update, tambalan keamanan dan pembaruan antivirus bawaan (Windows Defender) ikut berhenti.",
            &[
                "Ketik \"services\" di menu Start, buka Services, cari \"Windows Update\", klik dua kali, ubah Startup type ke Manual, lalu Start.",
                "Kalau dimatikan karena update membuat PC lambat, lebih baik atur jam aktif (Active hours) di Settings → Windows Update daripada mematikannya.",
            ],
        ));
    } else if major == Some(11) {
        match age {
            Some(a) if a > UPDATE_VERY_STALE_DAYS => findings.push(Finding::problem(
                "system.update",
                cat,
                Status::Bermasalah,
                Severity::Sedang,
                "Update Windows tertinggal jauh",
                last_text.clone(),
                "Update yang tertinggal lama berarti banyak celah keamanan belum ditambal, dan saat akhirnya jalan, update besar bisa membuat PC lambat berjam-jam.",
                &[
                    "Sambungkan internet, buka Settings → Windows Update → Check for updates, lalu biarkan sampai selesai (bisa lama).",
                    "Kalau selalu gagal, catat kode errornya dan minta bantuan teknisi.",
                ],
            )),
            Some(a) if a > UPDATE_STALE_DAYS => findings.push(Finding::problem(
                "system.update",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                "Update Windows agak tertinggal",
                last_text.clone(),
                "Microsoft merilis tambalan keamanan tiap bulan. Lebih dari sebulan tanpa update berarti ada yang terlewat.",
                &["Buka Settings → Windows Update → Check for updates."],
            )),
            Some(_) => findings.push(Finding::healthy("system.update", cat, "Update Windows terpasang", last_text.clone())),
            None => {}
        }
    } else if major == Some(10) && age.is_some() {
        findings.push(Finding::healthy(
            "system.update",
            cat,
            "Windows Update tidak dimatikan",
            format!("{last_text} Untuk Windows 10, update rutin memang sudah berhenti dari Microsoft."),
        ));
    }

    // 3) Menunggu restart
    if raw.update.reboot_pending {
        findings.push(Finding::problem(
            "system.reboot",
            cat,
            Status::Perhatian,
            Severity::Ringan,
            "Windows menunggu restart untuk menyelesaikan update",
            "Ada update yang sudah diunduh tapi baru selesai dipasang setelah PC di-restart.",
            "Sampai di-restart, update itu belum berlaku, dan PC kadang terasa lebih lambat karena pemasangan tertunda.",
            &["Simpan pekerjaan, lalu klik Start → Power → Restart (bukan Shut down)."],
        ));
    }

    // 4) Sudah lama tidak restart
    if let Some(up) = raw.uptime_secs {
        let days = up / 86_400;
        if days >= UPTIME_LONG_DAYS {
            let fast = if raw.fast_startup == Some(true) {
                " Di PC ini \"Shut down\" tidak benar-benar mematikan Windows (fitur Fast Startup), jadi yang dihitung hanya Restart."
            } else {
                ""
            };
            findings.push(Finding::problem(
                "system.uptime",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("PC belum di-restart selama {days} hari"),
                format!("Windows sudah berjalan terus {days} hari tanpa restart yang sungguhan.{fast}"),
                "Makin lama tanpa restart, memori makin penuh sisa-sisa program dan update tertunda menumpuk. PC jadi makin lambat.",
                &["Klik Start → Power → Restart. Biasakan restart seminggu sekali."],
            ));
        } else {
            findings.push(Finding::healthy("system.uptime", cat, "PC rutin di-restart", format!("Terakhir restart {} hari lalu.", days)));
        }
    }

    // 5) Driver bermasalah
    if !raw.pnp_errors.is_empty() {
        let n = raw.pnp_errors.len();
        let names: Vec<String> = raw.pnp_errors.iter().take(3).map(|d| format!("{} ({})", d.name, pnp_code_text(d.code))).collect();
        let more = if n > 3 { format!(", dan {} lagi", n - 3) } else { String::new() };
        findings.push(Finding::problem(
            "system.drivers",
            cat,
            if n >= PNP_MANY { Status::Bermasalah } else { Status::Perhatian },
            Severity::Sedang,
            if n == 1 { "Ada perangkat yang bermasalah di Device Manager".to_string() } else { format!("{n} perangkat bermasalah di Device Manager") },
            format!("{}{more}.", names.join("; ")),
            "Perangkat dengan tanda seru di Device Manager tidak bekerja dengan benar — bisa jadi Wi-Fi, suara, kartu grafis, atau port USB yang tidak jalan.",
            &[
                "Sambungkan internet, buka Settings → Windows Update → Check for updates, lalu lihat \"Optional updates\" untuk driver.",
                "Unduh driver dari situs resmi merek PC/laptop sesuai tipenya.",
                "Kalau perangkatnya memang tidak dipakai (mis. modem lama), boleh diabaikan.",
            ],
        ));
    } else if raw.os.is_some() {
        findings.push(Finding::healthy("system.drivers", cat, "Semua perangkat dikenali Windows", "Tidak ada tanda seru di Device Manager."));
    }

    // 6) Power plan
    match raw.power_plan {
        Some(PowerPlan::PowerSaver) => findings.push(Finding::problem(
            "system.power",
            cat,
            Status::Perhatian,
            Severity::Ringan,
            "Mode hemat daya aktif",
            "Windows sedang memakai power plan \"Power saver\".",
            "Dalam mode ini Windows sengaja menahan kecepatan prosesor untuk menghemat baterai. Di PC yang sudah lambat, ini terasa sekali.",
            &[
                "Ketik \"power plan\" di menu Start → Choose a power plan → pilih Balanced (atau High performance bila PC selalu tercolok listrik).",
                "Untuk laptop yang sering lepas charger, Balanced adalah pilihan tengah yang baik.",
            ],
        )),
        Some(PowerPlan::Balanced) => findings.push(Finding::healthy("system.power", cat, "Power plan seimbang (bawaan Windows)", "Windows mengatur kecepatan prosesor sesuai kebutuhan.")),
        Some(PowerPlan::HighPerformance) | Some(PowerPlan::Ultimate) => {
            findings.push(Finding::healthy("system.power", cat, "Power plan performa tinggi", "Prosesor dibiarkan bekerja penuh."))
        }
        _ => {}
    }

    // 7) Waktu boot
    if let Some(b) = &raw.boot {
        if let Some(ms) = b.main_path_ms.or(b.boot_ms) {
            let s = ms / 1000;
            let when = format!("Diukur saat PC menyala pada {}.", tanggal(&b.date));
            if s >= BOOT_VERY_SLOW_S {
                findings.push(Finding::problem(
                    "system.boot",
                    cat,
                    Status::Perhatian,
                    Severity::Sedang,
                    format!("Windows butuh {s} detik untuk menyala"),
                    format!("Itu sangat lama. {when}"),
                    "Boot selama ini biasanya karena hardisk lama (HDD) ditambah banyak program yang ikut jalan saat menyala.",
                    &["Kurangi program startup (lihat bagian Program).", "Kalau Windows masih di HDD, ganti ke SSD adalah perbaikan yang paling terasa."],
                ));
            } else if s >= BOOT_SLOW_S {
                findings.push(Finding::problem(
                    "system.boot",
                    cat,
                    Status::Perhatian,
                    Severity::Ringan,
                    format!("Windows butuh {s} detik untuk menyala"),
                    format!("Agak lama. {when}"),
                    "Sebagian besar waktu ini biasanya habis untuk program yang ikut jalan saat menyala.",
                    &["Kurangi program startup (lihat bagian Program)."],
                ));
            } else {
                findings.push(Finding::healthy("system.boot", cat, format!("Windows menyala dalam {s} detik"), when));
            }
        }
    }

    let mut result = CategoryResult::new(cat, findings, tech(raw));
    if raw.boot_unreadable && crate::platform::IS_WINDOWS {
        result.needs_admin.push("catatan waktu boot Windows".into());
    }
    result
}

fn tech(raw: &SystemRaw) -> TechSection {
    let mut t = TechSection::new("Windows");
    match &raw.os {
        Some(os) => {
            let build = match os.ubr {
                Some(u) => format!("{}.{u}", os.build),
                None => os.build.to_string(),
            };
            let ver = os.display_version.as_deref().map(|v| format!("{v} · ")).unwrap_or_default();
            t.push("Versi", format!("{} · {ver}build {build} · {}", os.caption, os.arch));
            if let Some(d) = &os.install_date {
                t.push("Windows dipasang", tanggal(d));
            }
            if let Some(b) = &os.last_boot {
                t.push("Menyala sejak", b.clone());
            }
        }
        None => t.push("Versi", "Tidak terbaca"),
    }
    if let Some(m) = &raw.pc_model {
        t.push("Model PC", m.clone());
    }
    let u = &raw.update;
    t.push(
        "Update terakhir (registry / hotfix / event log)",
        format!(
            "{} / {} / {}",
            u.last_success.as_deref().unwrap_or("-"),
            u.last_hotfix.as_deref().unwrap_or("-"),
            u.last_event.as_deref().unwrap_or("-")
        ),
    );
    t.push(
        "Layanan Windows Update",
        match u.wu_start {
            Some(2) => "Automatic",
            Some(3) => "Manual",
            Some(4) => "Disabled",
            _ => "tidak terbaca",
        },
    );
    t.push("Menunggu restart", if u.reboot_pending { "ya" } else { "tidak" });
    if let Some(g) = &raw.power_guid {
        t.push("Power plan (GUID)", g.clone());
    }
    if let Some(f) = raw.fast_startup {
        t.push("Fast Startup", if f { "aktif" } else { "mati" });
    }
    if let Some(p) = raw.page_file_auto {
        t.push("Page file", if p { "diatur otomatis oleh Windows" } else { "diatur manual" });
    }
    if let Some(up) = raw.uptime_secs {
        t.push("Uptime", format!("{} jam", up / 3600));
    }
    for d in &raw.pnp_errors {
        t.push(format!("Device Manager: {}", d.name), format!("kode {} ({}){}", d.code, pnp_code_text(d.code), if d.class.is_empty() { String::new() } else { format!(" · {}", d.class) }));
    }
    match &raw.boot {
        Some(b) => t.push(
            "Boot terakhir (event 100)",
            format!(
                "{} · total {} · jalur utama {} · setelah login {}",
                b.date,
                b.boot_ms.map(|m| format!("{} dtk", m / 1000)).unwrap_or_else(|| "?".into()),
                b.main_path_ms.map(|m| format!("{} dtk", m / 1000)).unwrap_or_else(|| "?".into()),
                b.post_boot_ms.map(|m| format!("{} dtk", m / 1000)).unwrap_or_else(|| "?".into())
            ),
        ),
        None if raw.boot_unreadable => t.push("Boot terakhir", "Log tidak terbaca (biasanya butuh izin Administrator)"),
        None => {}
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::days_from_civil;
    use crate::probe::system::{BootRaw, OsRaw, PnpErrorRaw, UpdateRaw};

    fn today() -> i64 {
        days_from_civil(2026, 11, 25)
    }

    fn win(major: u32) -> SystemRaw {
        let (caption, build) = match major {
            11 => ("Microsoft Windows 11 Home", 26100),
            10 => ("Microsoft Windows 10 Pro", 19045),
            _ => ("Microsoft Windows 7 Ultimate", 7601),
        };
        SystemRaw { os: Some(OsRaw { caption: caption.into(), build, ..Default::default() }), ..Default::default() }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn version_rules() {
        assert_eq!(find(&analyze(&win(11), today()), "system.version").unwrap().status, Status::Sehat);
        assert_eq!(find(&analyze(&win(10), today()), "system.version").unwrap().status, Status::Perhatian);
        assert_eq!(find(&analyze(&win(7), today()), "system.version").unwrap().status, Status::Bermasalah);
    }

    #[test]
    fn update_age_on_windows_11() {
        let mut r = win(11);
        r.update = UpdateRaw { last_hotfix: Some("2026-06-01".into()), ..Default::default() };
        assert_eq!(find(&analyze(&r, today()), "system.update").unwrap().status, Status::Bermasalah);
        r.update.last_hotfix = Some("2026-10-01".into());
        assert_eq!(find(&analyze(&r, today()), "system.update").unwrap().status, Status::Perhatian);
        r.update.last_hotfix = Some("2026-11-12".into());
        assert_eq!(find(&analyze(&r, today()), "system.update").unwrap().status, Status::Sehat);
    }

    #[test]
    fn disabled_update_service_is_a_problem_on_any_windows() {
        let mut r = win(10);
        r.update.wu_start = Some(4);
        assert_eq!(find(&analyze(&r, today()), "system.update").unwrap().status, Status::Bermasalah);
    }

    #[test]
    fn uptime_and_reboot() {
        let mut r = win(11);
        r.uptime_secs = Some(9 * 86_400);
        r.update.reboot_pending = true;
        r.fast_startup = Some(true);
        let res = analyze(&r, today());
        let up = find(&res, "system.uptime").unwrap();
        assert_eq!(up.status, Status::Perhatian);
        assert!(up.summary.contains("Fast Startup"));
        assert_eq!(find(&res, "system.reboot").unwrap().status, Status::Perhatian);
    }

    #[test]
    fn drivers_power_and_boot() {
        let mut r = win(11);
        r.pnp_errors = (0..3).map(|i| PnpErrorRaw { name: format!("Dev{i}"), code: 28, class: String::new() }).collect();
        r.power_plan = Some(PowerPlan::PowerSaver);
        r.boot = Some(BootRaw { date: "2026-11-20".into(), main_path_ms: Some(160_000), ..Default::default() });
        let res = analyze(&r, today());
        assert_eq!(find(&res, "system.drivers").unwrap().status, Status::Bermasalah);
        assert_eq!(find(&res, "system.power").unwrap().status, Status::Perhatian);
        let b = find(&res, "system.boot").unwrap();
        assert_eq!((b.status, b.severity), (Status::Perhatian, Some(Severity::Sedang)));
        assert!(b.title.contains("160 detik"));
    }

    #[test]
    fn boot_log_unreadable_needs_admin() {
        let mut r = win(11);
        r.boot_unreadable = true;
        let res = analyze(&r, today());
        assert_eq!(res.needs_admin.len(), if crate::platform::IS_WINDOWS { 1 } else { 0 });
    }
}
