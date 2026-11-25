//! Penilaian jaringan: tersambung atau tidak, Wi-Fi, DNS, kecepatan unduh.

use crate::format::decimal;
use crate::model::*;
use crate::probe::network::NetworkRaw;

pub const SPEED_VERY_SLOW_MBPS: f64 = 1.0;
pub const SPEED_SLOW_MBPS: f64 = 3.0;
pub const SPEED_GOOD_MBPS: f64 = 25.0;
pub const WIFI_WEAK_PCT: u32 = 40;
pub const DNS_SLOW_MS: u64 = 1500;

pub fn analyze(raw: &NetworkRaw) -> CategoryResult {
    let cat = Category::Network;
    let mut findings = Vec::new();

    // 1) Tersambung ke internet?
    match raw.internet {
        Some(true) => findings.push(Finding::healthy(
            "network.internet",
            cat,
            "Internet tersambung",
            match (&raw.wifi_ssid, raw.on_wifi()) {
                (Some(ssid), _) => format!("Lewat Wi-Fi \"{ssid}\"."),
                (None, true) => "Lewat Wi-Fi.".to_string(),
                (None, false) => "Lewat kabel LAN.".to_string(),
            },
        )),
        Some(false) if raw.has_link() => findings.push(Finding::problem(
            "network.internet",
            cat,
            Status::Perhatian,
            Severity::Sedang,
            "Tersambung ke jaringan, tapi tidak bisa ke internet",
            format!(
                "PC terhubung ke {}, tapi saat dicoba, internetnya tidak sampai.{}",
                if raw.on_wifi() { "Wi-Fi" } else { "jaringan kabel" },
                if raw.dns_ok() == 0 { " Nama situs juga tidak bisa dicari (DNS tidak menjawab)." } else { "" }
            ),
            "Biasanya masalahnya di modem/router atau di langganan internet, bukan di PC.",
            &[
                "Cabut kabel listrik modem/router, tunggu 30 detik, colok lagi, lalu tunggu 2 menit.",
                "Cek dengan HP di Wi-Fi yang sama. Kalau HP juga tidak bisa, hubungi penyedia internet (cek kuota/tagihan).",
                "Kalau HP bisa tapi PC tidak: klik kanan ikon jaringan di pojok kanan bawah → Troubleshoot problems. Kalau masih gagal: Settings → Network & internet → Advanced network settings → Network reset.",
            ],
        )),
        Some(false) => findings.push(Finding::problem(
            "network.internet",
            cat,
            Status::Perhatian,
            Severity::Sedang,
            "PC ini tidak tersambung ke jaringan",
            "Tidak ada Wi-Fi atau kabel LAN yang terhubung.",
            "Tanpa internet, Windows dan antivirus tidak bisa diperbarui.",
            &[
                "Klik ikon jaringan di pojok kanan bawah, pilih Wi-Fi rumah, masukkan sandinya.",
                "Untuk laptop: pastikan Wi-Fi tidak dimatikan lewat tombol/saklar, dan Airplane mode mati.",
                "Kalau pakai kabel LAN, pastikan kabelnya terpasang kencang di kedua ujung.",
            ],
        )),
        None => {}
    }

    // 2) Sinyal Wi-Fi
    if let Some(sig) = raw.wifi_signal_pct.filter(|_| raw.on_wifi()) {
        if sig < WIFI_WEAK_PCT {
            findings.push(Finding::problem(
                "network.wifi",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("Sinyal Wi-Fi lemah ({sig}%)"),
                "Sinyal selemah ini membuat internet lambat dan sering putus, walau paketnya kencang.",
                "Tembok, jarak, dan lantai yang berbeda melemahkan sinyal Wi-Fi.",
                &["Dekatkan PC ke router, atau pindahkan router ke tempat yang lebih terbuka.", "Kalau memungkinkan, pakai kabel LAN — lebih stabil dari Wi-Fi."],
            ));
        } else {
            findings.push(Finding::healthy("network.wifi", cat, format!("Sinyal Wi-Fi bagus ({sig}%)"), "Cukup kuat untuk pemakaian normal."));
        }
    }

    // 3) Kecepatan unduh
    if let Some(d) = &raw.download {
        let mbps = d.mbps();
        let text = format!("{} Mbps", decimal(mbps, 1));
        if mbps < SPEED_VERY_SLOW_MBPS {
            findings.push(Finding::problem(
                "network.speed",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                format!("Internet sangat lambat ({text})"),
                "Selambat ini, membuka situs biasa pun terasa menunggu, dan update Windows bisa berhari-hari.",
                "Kecepatan ini berasal dari jaringan/penyedia internet, bukan dari PC. Tapi dampaknya terasa seperti PC lemot.",
                &["Coba dekatkan ke router atau pakai kabel LAN.", "Restart modem/router.", "Cek paket internet dan apakah ada perangkat lain yang sedang mengunduh besar-besaran."],
            ));
        } else if mbps < SPEED_SLOW_MBPS {
            findings.push(Finding::problem(
                "network.speed",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("Internet lambat ({text})"),
                "Cukup untuk browsing dan chat, tapi video dan update besar akan lama.",
                "Kecepatan ini berasal dari jaringan/penyedia internet, bukan dari PC.",
                &["Coba dekatkan ke router atau pakai kabel LAN.", "Cek apakah perangkat lain sedang memakai internet besar-besaran."],
            ));
        } else {
            let quality = if mbps >= SPEED_GOOD_MBPS { "Lancar untuk video HD dan update." } else { "Cukup untuk video dan update." };
            findings.push(Finding::healthy("network.speed", cat, format!("Kecepatan unduh sekitar {text}"), quality.to_string()));
        }
    }

    // 4) DNS lambat
    if raw.internet == Some(true) {
        if let Some(ms) = raw.dns_slowest_ms().filter(|ms| *ms >= DNS_SLOW_MS) {
            findings.push(Finding::problem(
                "network.dns",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                "Pencarian alamat situs (DNS) lambat",
                format!("Mencari alamat satu situs butuh sampai {} detik — seharusnya kurang dari sedetik.", decimal(ms as f64 / 1000.0, 1)),
                "Setiap kali membuka situs, PC harus \"menanyakan alamatnya\" dulu ke server DNS. Kalau server itu lambat, semua situs terasa lama terbuka walau internetnya kencang.",
                &["Restart modem/router.", "Kalau masih lambat, ganti DNS ke 1.1.1.1 atau 8.8.8.8: Settings → Network & internet → Wi-Fi/Ethernet → DNS server assignment → Edit → Manual."],
            ));
        }
    }

    CategoryResult::new(cat, findings, tech(raw))
}

fn tech(raw: &NetworkRaw) -> TechSection {
    let mut t = TechSection::new("Jaringan");
    for p in &raw.profiles {
        t.push(format!("Profil: {}", p.name), format!("{} · IPv4 {} · {}", p.alias, p.ipv4, p.category));
    }
    for a in &raw.adapters {
        t.push(format!("Adapter: {}", a.name), format!("{} · {}{}", a.description, a.link_speed, if a.wifi { " · Wi-Fi" } else { "" }));
    }
    if let Some(s) = &raw.wifi_ssid {
        t.push("SSID", format!("{s}{}", raw.wifi_signal_pct.map(|p| format!(" · sinyal {p}%")).unwrap_or_default()));
    }
    if !raw.dns_servers.is_empty() {
        t.push("Server DNS", raw.dns_servers.join(", "));
    }
    let dns: Vec<String> = raw.dns.iter().map(|d| format!("{} {}", d.host, d.millis.map(|m| format!("{m} ms")).unwrap_or_else(|| "gagal".into()))).collect();
    if !dns.is_empty() {
        t.push("Tes DNS", dns.join(" · "));
    }
    t.push(
        "Cek internet (msftconnecttest)",
        match raw.internet {
            Some(true) => "tersambung",
            Some(false) => "tidak tersambung",
            None => "tidak dicek",
        },
    );
    if let Some(d) = &raw.download {
        t.push("Tes unduh", format!("{} · {} byte dalam {} ms · {} Mbps", d.url, d.bytes, d.millis, decimal(d.mbps(), 1)));
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::network::{AdapterRaw, DnsResult, DownloadResult};

    fn wifi(signal: u32) -> NetworkRaw {
        NetworkRaw {
            adapters: vec![AdapterRaw { name: "Wi-Fi".into(), wifi: true, ..Default::default() }],
            wifi_signal_pct: Some(signal),
            wifi_ssid: Some("Rumah".into()),
            internet: Some(true),
            dns: vec![DnsResult { host: "a".into(), millis: Some(40) }],
            ..Default::default()
        }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn connected_with_good_wifi() {
        let r = analyze(&wifi(80));
        assert_eq!(find(&r, "network.internet").unwrap().status, Status::Sehat);
        assert_eq!(find(&r, "network.wifi").unwrap().status, Status::Sehat);
        assert!(find(&r, "network.dns").is_none());
    }

    #[test]
    fn weak_wifi_and_slow_dns() {
        let mut raw = wifi(25);
        raw.dns.push(DnsResult { host: "b".into(), millis: Some(2200) });
        let r = analyze(&raw);
        assert_eq!(find(&r, "network.wifi").unwrap().status, Status::Perhatian);
        assert_eq!(find(&r, "network.dns").unwrap().status, Status::Perhatian);
    }

    #[test]
    fn link_without_internet() {
        let mut raw = wifi(80);
        raw.internet = Some(false);
        raw.dns = vec![DnsResult { host: "a".into(), millis: None }];
        let res = analyze(&raw);
        let f = find(&res, "network.internet").unwrap();
        assert_eq!(f.status, Status::Perhatian);
        assert!(f.summary.contains("DNS"));
    }

    #[test]
    fn no_link_at_all() {
        let raw = NetworkRaw { internet: Some(false), ..Default::default() };
        assert_eq!(find(&analyze(&raw), "network.internet").unwrap().title, "PC ini tidak tersambung ke jaringan");
    }

    #[test]
    fn speed_tiers() {
        let speed = |mbps: f64| {
            let mut raw = wifi(80);
            raw.download = Some(DownloadResult { url: String::new(), bytes: (mbps * 1_000_000.0 / 8.0) as u64, millis: 1000 });
            find(&analyze(&raw), "network.speed").unwrap().clone()
        };
        assert_eq!(speed(0.5).severity, Some(Severity::Sedang));
        assert_eq!(speed(2.0).severity, Some(Severity::Ringan));
        assert_eq!(speed(12.0).status, Status::Sehat);
        assert!(speed(40.0).summary.contains("HD"));
    }
}
