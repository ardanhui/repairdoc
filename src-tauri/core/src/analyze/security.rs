//! Penilaian keamanan: antivirus, firewall, UAC, program yang sering tidak perlu.

use crate::model::*;
use crate::probe::security::{ProgramRaw, SecurityRaw};

/// Definisi antivirus lebih tua dari ini (hari) → perhatian / bermasalah.
pub const SIGNATURE_STALE_DAYS: u64 = 7;
pub const SIGNATURE_VERY_STALE_DAYS: u64 = 30;
/// Belum scan virus lebih lama dari ini (hari) → perhatian.
pub const SCAN_STALE_DAYS: u64 = 30;

/// Pola nama program yang sering tidak perlu atau berisiko, dengan alasannya.
/// Dicocokkan ke nama program (huruf kecil). Daftar ini sengaja pendek dan konservatif.
pub const SUSPICIOUS: &[(&str, &str)] = &[
    ("kmspico", "aktivator bajakan — sering disusupi virus dan mematikan antivirus"),
    ("kmsauto", "aktivator bajakan — sering disusupi virus dan mematikan antivirus"),
    ("autokms", "aktivator bajakan — sering disusupi virus dan mematikan antivirus"),
    ("re-loader", "aktivator bajakan — sering disusupi virus dan mematikan antivirus"),
    ("microsoft toolkit", "aktivator bajakan — sering disusupi virus dan mematikan antivirus"),
    ("windows loader", "aktivator bajakan — sering disusupi virus dan mematikan antivirus"),
    ("driver booster", "pemasang driver otomatis — sering memasang driver yang salah dan menambah program lain"),
    ("driver updater", "pemasang driver otomatis — sering memasang driver yang salah dan menambah program lain"),
    ("driverpack", "pemasang driver otomatis — sering memasang driver yang salah dan menambah program lain"),
    ("driver support", "pemasang driver otomatis — sering memasang driver yang salah dan menambah program lain"),
    ("pc accelerate", "program \"pempercepat\" yang menakut-nakuti dengan peringatan palsu"),
    ("pc optimizer pro", "program \"pempercepat\" yang menakut-nakuti dengan peringatan palsu"),
    ("onesafe", "program \"pembersih\" yang menakut-nakuti dengan peringatan palsu"),
    ("mycleanpc", "program \"pembersih\" yang menakut-nakuti dengan peringatan palsu"),
    ("speedupmypc", "program \"pempercepat\" yang menakut-nakuti dengan peringatan palsu"),
    ("slimcleaner", "program \"pembersih\" yang menakut-nakuti dengan peringatan palsu"),
    ("reimage", "program \"perbaikan\" yang menakut-nakuti dengan peringatan palsu"),
    ("segurazo", "antivirus palsu yang sulit dihapus"),
    ("bytefence", "antivirus yang biasanya ikut terpasang tanpa diminta"),
    ("search protect", "pengubah pencarian browser"),
    ("web companion", "program tambahan browser yang ikut terpasang tanpa diminta"),
    ("toolbar", "toolbar browser — memperlambat browser dan mengubah pencarian"),
    ("conduit", "pengubah pencarian browser"),
    ("babylon", "pengubah pencarian browser"),
    ("mindspark", "pengubah pencarian browser"),
    ("myway", "pengubah pencarian browser"),
    ("baidu", "program bundel yang sering ikut terpasang tanpa disadari"),
];

pub fn suspicious_reason(p: &ProgramRaw) -> Option<&'static str> {
    let n = p.name.to_ascii_lowercase();
    SUSPICIOUS.iter().find(|(pat, _)| n.contains(pat)).map(|(_, why)| *why)
}

fn is_defender(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("defender") || n.contains("microsoft") && n.contains("security")
}

pub fn analyze(raw: &SecurityRaw) -> CategoryResult {
    let cat = Category::Security;
    let mut findings = Vec::new();

    // 1) Antivirus
    let third_party: Vec<_> = raw.av_products.iter().filter(|a| !is_defender(&a.name)).collect();
    let third_party_on: Vec<_> = third_party.iter().filter(|a| a.enabled).collect();
    let defender_on = raw.defender.as_ref().is_some_and(|d| d.antivirus_enabled && d.real_time);
    let steps_defender: &[&str] = &[
        "Buka Windows Security (ketik \"Windows Security\" di menu Start) → Virus & threat protection → Manage settings → nyalakan Real-time protection.",
        "Kalau tombolnya abu-abu atau mati lagi sendiri, ada program lain yang mematikannya — minta teknisi memeriksa.",
    ];

    if !third_party_on.is_empty() {
        let names: Vec<&str> = third_party_on.iter().map(|a| a.name.as_str()).collect();
        if third_party_on.len() >= 2 || (defender_on && !third_party_on.is_empty()) {
            let mut all: Vec<&str> = names.clone();
            if defender_on {
                all.push("Windows Defender");
            }
            findings.push(Finding::problem(
                "security.antivirus",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                "Lebih dari satu antivirus aktif bersamaan",
                format!("Yang aktif: {}.", all.join(", ")),
                "Dua antivirus saling memeriksa satu sama lain dan berebut file — PC jadi lambat, dan kadang malah saling mematikan.",
                &["Pilih satu saja. Hapus yang lain lewat Settings → Apps.", "Kalau yang dihapus antivirus pihak ketiga, Windows Defender otomatis aktif kembali."],
            ));
        } else {
            findings.push(Finding::healthy("security.antivirus", cat, format!("Antivirus aktif: {}", names[0]), "Perlindungan virus berjalan."));
        }
        if let Some(a) = third_party_on.iter().find(|a| !a.up_to_date) {
            findings.push(Finding::problem(
                "security.av_outdated",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                format!("{} belum diperbarui", a.name),
                "Windows melaporkan definisi virusnya sudah lama.",
                "Antivirus yang tidak diperbarui tidak mengenali virus-virus baru.",
                &[format!("Buka {} dan jalankan Update.", a.name).as_str(), "Kalau masa langganannya habis, pertimbangkan kembali ke Windows Defender (gratis dan cukup untuk kebanyakan orang)."],
            ));
        }
    } else if let Some(d) = &raw.defender {
        if defender_on {
            findings.push(Finding::healthy("security.antivirus", cat, "Windows Defender aktif", "Perlindungan virus bawaan Windows berjalan."));
            match d.signature_age_days {
                Some(age) if age >= SIGNATURE_VERY_STALE_DAYS => findings.push(Finding::problem(
                    "security.signatures",
                    cat,
                    Status::Bermasalah,
                    Severity::Sedang,
                    format!("Antivirus belum diperbarui {age} hari"),
                    "Daftar virus yang dikenal Windows Defender sudah sangat lama.",
                    "Tanpa pembaruan, Defender tidak mengenali virus yang muncul belakangan — sama saja seperti tidak ada antivirus untuk ancaman baru.",
                    &["Sambungkan internet, buka Windows Security → Virus & threat protection → Protection updates → Check for updates.", "Pastikan Windows Update tidak dimatikan (lihat bagian Windows)."],
                )),
                Some(age) if age >= SIGNATURE_STALE_DAYS => findings.push(Finding::problem(
                    "security.signatures",
                    cat,
                    Status::Perhatian,
                    Severity::Ringan,
                    format!("Antivirus belum diperbarui {age} hari"),
                    "Biasanya Windows Defender memperbarui diri tiap hari.",
                    "PC yang jarang tersambung internet tidak bisa memperbarui daftar virusnya.",
                    &["Sambungkan internet beberapa menit, lalu buka Windows Security → Virus & threat protection → Check for updates."],
                )),
                _ => {}
            }
            let last_scan = match (d.quick_scan_age_days, d.full_scan_age_days) {
                (Some(q), Some(f)) => Some(q.min(f)),
                (q, f) => q.or(f),
            };
            match last_scan {
                Some(a) if a >= SCAN_STALE_DAYS => findings.push(Finding::problem(
                    "security.scan",
                    cat,
                    Status::Perhatian,
                    Severity::Ringan,
                    format!("Sudah {a} hari tidak scan virus"),
                    "Pemindaian penuh terakhir sudah lama.",
                    "Perlindungan langsung (real-time) menangkap sebagian besar ancaman, tapi scan berkala menemukan yang sempat lolos.",
                    &["Buka Windows Security → Virus & threat protection → Quick scan."],
                )),
                None => findings.push(Finding::problem(
                    "security.scan",
                    cat,
                    Status::Perhatian,
                    Severity::Ringan,
                    "Belum pernah scan virus",
                    "Windows Defender belum pernah menjalankan pemindaian di PC ini.",
                    "Scan berkala menemukan ancaman yang sempat lolos dari perlindungan langsung.",
                    &["Buka Windows Security → Virus & threat protection → Quick scan."],
                )),
                _ => {}
            }
        } else {
            let other = third_party.first().map(|a| format!(" Ada {} terpasang tapi tidak aktif.", a.name)).unwrap_or_default();
            findings.push(Finding::problem(
                "security.antivirus",
                cat,
                Status::Bermasalah,
                Severity::Berat,
                "PC ini tidak punya antivirus yang aktif",
                format!("Perlindungan langsung Windows Defender mati.{other}"),
                "Tanpa antivirus aktif, virus dari flashdisk, unduhan, atau email bisa masuk tanpa ada yang menghadang. Di Windows 10 yang sudah tidak diupdate, ini makin berbahaya.",
                steps_defender,
            ));
        }
    }

    // 2) Firewall
    if raw.firewall_off_somewhere() {
        let mut off = Vec::new();
        if raw.firewall_private == Some(false) {
            off.push("jaringan privat (rumah)");
        }
        if raw.firewall_public == Some(false) {
            off.push("jaringan publik");
        }
        findings.push(Finding::problem(
            "security.firewall",
            cat,
            Status::Perhatian,
            Severity::Sedang,
            "Firewall Windows mati",
            format!("Firewall tidak aktif untuk {}.", off.join(" dan ")),
            "Firewall mencegah program dan komputer lain di jaringan masuk ke PC ini tanpa izin. Tanpa firewall, PC lebih mudah diserang, terutama di Wi-Fi umum.",
            &["Buka Windows Security → Firewall & network protection, lalu nyalakan untuk semua jaringan.", "Kalau mati sendiri lagi, mungkin ada antivirus lain yang mengambil alih — periksa pengaturannya."],
        ));
    } else if raw.firewall_all_known() {
        findings.push(Finding::healthy("security.firewall", cat, "Firewall aktif", "Firewall Windows menyala untuk jaringan privat dan publik."));
    }

    // 3) UAC
    match raw.uac_enabled {
        Some(false) => findings.push(Finding::problem(
            "security.uac",
            cat,
            Status::Perhatian,
            Severity::Sedang,
            "Peringatan izin (UAC) dimatikan",
            "Windows tidak lagi bertanya \"Do you want to allow this app to make changes?\" saat program ingin mengubah sistem.",
            "Peringatan itu adalah rem terakhir sebelum virus atau program nakal mengubah Windows. Tanpa rem ini, semua berjalan diam-diam.",
            &["Ketik \"UAC\" di menu Start → Change User Account Control settings → geser ke posisi kedua dari atas → OK."],
        )),
        Some(true) => findings.push(Finding::healthy("security.uac", cat, "Peringatan izin (UAC) aktif", "Windows bertanya dulu sebelum program mengubah sistem.")),
        None => {}
    }

    // 4) Program yang sering tidak perlu / berisiko
    let flagged: Vec<(&ProgramRaw, &str)> = raw.programs.iter().filter_map(|p| suspicious_reason(p).map(|r| (p, r))).collect();
    if !flagged.is_empty() {
        let n = flagged.len();
        let has_activator = flagged.iter().any(|(_, r)| r.starts_with("aktivator"));
        let list: Vec<String> = flagged.iter().take(4).map(|(p, r)| format!("{} ({r})", p.name)).collect();
        let more = if n > 4 { format!(", dan {} lagi", n - 4) } else { String::new() };
        findings.push(Finding::problem(
            "security.suspicious",
            cat,
            if has_activator { Status::Bermasalah } else { Status::Perhatian },
            Severity::Sedang,
            if n == 1 { "Ada program terpasang yang sebaiknya dihapus".to_string() } else { format!("{n} program terpasang yang sebaiknya dihapus") },
            format!("{}{more}.", list.join("; ")),
            "Program jenis ini sering ikut terpasang tanpa disadari, memperlambat PC, memunculkan iklan atau peringatan palsu, dan sebagian membawa virus.",
            &[
                "Buka Settings → Apps → Installed apps, cari namanya, lalu Uninstall.",
                "Setelah itu jalankan scan virus penuh lewat Windows Security.",
                "Kalau tidak yakin program itu apa, tanyakan ke teknisi sebelum menghapus.",
            ],
        ));
    } else if !raw.programs.is_empty() {
        findings.push(Finding::healthy(
            "security.suspicious",
            cat,
            "Tidak ada program mencurigakan yang dikenali",
            format!("{} program terpasang diperiksa terhadap daftar program yang sering bermasalah.", raw.programs.len()),
        ));
    }

    CategoryResult::new(cat, findings, tech(raw))
}

fn tech(raw: &SecurityRaw) -> TechSection {
    let mut t = TechSection::new("Keamanan");
    match &raw.defender {
        Some(d) => {
            t.push(
                "Windows Defender",
                format!(
                    "layanan {} · antivirus {} · real-time {}",
                    if d.service_enabled { "aktif" } else { "mati" },
                    if d.antivirus_enabled { "aktif" } else { "mati" },
                    if d.real_time { "aktif" } else { "mati" }
                ),
            );
            t.push(
                "Definisi virus",
                match (&d.signature_date, d.signature_age_days) {
                    (Some(date), Some(age)) => format!("{date} ({age} hari lalu)"),
                    (_, Some(age)) => format!("{age} hari lalu"),
                    _ => "tidak terbaca".into(),
                },
            );
            let scan = |a: Option<u64>| a.map(|d| format!("{d} hari lalu")).unwrap_or_else(|| "belum pernah".into());
            t.push("Scan cepat / penuh terakhir", format!("{} / {}", scan(d.quick_scan_age_days), scan(d.full_scan_age_days)));
        }
        None => t.push("Windows Defender", "Status tidak terbaca (cmdlet Get-MpComputerStatus tidak tersedia)"),
    }
    for a in &raw.av_products {
        t.push(
            format!("Security Center: {}", a.name),
            format!("{}{}", if a.enabled { "aktif" } else { "tidak aktif" }, if a.up_to_date { "" } else { " · definisi lama" }),
        );
    }
    let fw = |v: Option<bool>| match v {
        Some(true) => "aktif",
        Some(false) => "MATI",
        None => "?",
    };
    t.push("Firewall (domain / privat / publik)", format!("{} / {} / {}", fw(raw.firewall_domain), fw(raw.firewall_private), fw(raw.firewall_public)));
    t.push("UAC", fw(raw.uac_enabled));
    t.push("Program terpasang", format!("{} entri", raw.programs.len()));
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::security::{AvProductRaw, DefenderRaw};

    fn defender(real_time: bool, sig_age: u64) -> SecurityRaw {
        SecurityRaw {
            defender: Some(DefenderRaw {
                service_enabled: true,
                antivirus_enabled: real_time,
                real_time,
                signature_age_days: Some(sig_age),
                quick_scan_age_days: Some(3),
                ..Default::default()
            }),
            firewall_private: Some(true),
            firewall_public: Some(true),
            uac_enabled: Some(true),
            ..Default::default()
        }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn defender_active_and_healthy() {
        let r = analyze(&defender(true, 1));
        assert_eq!(find(&r, "security.antivirus").unwrap().status, Status::Sehat);
        assert!(find(&r, "security.signatures").is_none());
        assert_eq!(find(&r, "security.firewall").unwrap().status, Status::Sehat);
    }

    #[test]
    fn no_active_antivirus_is_severe() {
        let f = analyze(&defender(false, 1));
        let f = find(&f, "security.antivirus").unwrap();
        assert_eq!((f.status, f.severity), (Status::Bermasalah, Some(Severity::Berat)));
    }

    #[test]
    fn stale_signatures() {
        assert_eq!(find(&analyze(&defender(true, 10)), "security.signatures").unwrap().status, Status::Perhatian);
        assert_eq!(find(&analyze(&defender(true, 60)), "security.signatures").unwrap().status, Status::Bermasalah);
    }

    #[test]
    fn two_antivirus_conflict() {
        let mut r = defender(true, 1);
        r.av_products = vec![AvProductRaw { name: "Avast Antivirus".into(), enabled: true, up_to_date: true }];
        let res = analyze(&r);
        let f = find(&res, "security.antivirus").unwrap();
        assert_eq!(f.status, Status::Perhatian);
        assert!(f.summary.contains("Avast") && f.summary.contains("Defender"));
    }

    #[test]
    fn third_party_only_is_fine() {
        let mut r = defender(false, 1);
        r.av_products = vec![AvProductRaw { name: "Smadav Antivirus".into(), enabled: true, up_to_date: true }];
        assert_eq!(find(&analyze(&r), "security.antivirus").unwrap().status, Status::Sehat);
    }

    #[test]
    fn firewall_and_uac_off() {
        let mut r = defender(true, 1);
        r.firewall_public = Some(false);
        r.uac_enabled = Some(false);
        let res = analyze(&r);
        assert_eq!(find(&res, "security.firewall").unwrap().status, Status::Perhatian);
        assert_eq!(find(&res, "security.uac").unwrap().status, Status::Perhatian);
    }

    #[test]
    fn suspicious_programs() {
        let mut r = defender(true, 1);
        r.programs = vec![
            ProgramRaw { name: "KMSpico 10.2".into(), ..Default::default() },
            ProgramRaw { name: "Google Chrome".into(), ..Default::default() },
            ProgramRaw { name: "Driver Booster 11".into(), ..Default::default() },
        ];
        let res = analyze(&r);
        let f = find(&res, "security.suspicious").unwrap();
        assert_eq!(f.status, Status::Bermasalah, "aktivator → bermasalah");
        assert!(f.title.starts_with("2 program"));
        assert!(f.summary.contains("KMSpico") && !f.summary.contains("Chrome"));
    }
}
