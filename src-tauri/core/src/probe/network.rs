//! Data jaringan: koneksi, Wi-Fi, DNS, cek internet, dan tes unduh sederhana.

use std::time::Duration;

use super::json;
use super::net;
use crate::platform;

/// Alamat yang dipakai Windows sendiri untuk mengecek "ada internet atau tidak".
pub const CONNECT_TESTS: &[(&str, &str)] =
    &[("http://www.msftconnecttest.com/connecttest.txt", "Microsoft Connect Test"), ("http://www.msftncsi.com/ncsi.txt", "Microsoft NCSI")];

/// File publik untuk tes unduh. Dicoba berurutan; yang pertama berhasil dipakai.
/// Unduhan dihentikan setelah `DOWNLOAD_MAX_BYTES` atau `DOWNLOAD_BUDGET`.
pub const DOWNLOAD_URLS: &[&str] = &[
    "http://speedtest.tele2.net/10MB.zip",
    "http://ipv4.download.thinkbroadband.com/10MB.zip",
    "http://speed.hetzner.de/100MB.bin",
];
pub const DOWNLOAD_MAX_BYTES: usize = 10 * 1024 * 1024;
pub const DOWNLOAD_BUDGET: Duration = Duration::from_secs(8);

/// Nama yang di-resolve untuk mengukur DNS.
pub const DNS_PROBES: &[&str] = &["www.google.com", "www.microsoft.com", "www.detik.com"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileRaw {
    pub name: String,
    pub alias: String,
    /// "Internet", "LocalNetwork", "NoTraffic", "Disconnected", …
    pub ipv4: String,
    pub category: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdapterRaw {
    pub name: String,
    pub description: String,
    pub link_speed: String,
    pub wifi: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DnsResult {
    pub host: String,
    pub millis: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadResult {
    pub url: String,
    pub bytes: u64,
    pub millis: u64,
}

impl DownloadResult {
    pub fn mbps(&self) -> f64 {
        if self.millis == 0 {
            return 0.0;
        }
        self.bytes as f64 * 8.0 / (self.millis as f64 / 1000.0) / 1_000_000.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct NetworkRaw {
    pub profiles: Vec<ProfileRaw>,
    pub adapters: Vec<AdapterRaw>,
    pub wifi_signal_pct: Option<u32>,
    pub wifi_ssid: Option<String>,
    pub dns_servers: Vec<String>,
    /// Hasil cek internet gaya Windows: `Some(true)` = tersambung, `Some(false)` = tidak,
    /// `None` = tidak sempat dicek.
    pub internet: Option<bool>,
    pub dns: Vec<DnsResult>,
    pub download: Option<DownloadResult>,
    pub notes: Vec<String>,
}

impl NetworkRaw {
    pub fn has_link(&self) -> bool {
        !self.adapters.is_empty() || self.profiles.iter().any(|p| !p.ipv4.eq_ignore_ascii_case("Disconnected"))
    }

    pub fn on_wifi(&self) -> bool {
        self.adapters.iter().any(|a| a.wifi) || self.wifi_ssid.is_some()
    }

    pub fn dns_ok(&self) -> usize {
        self.dns.iter().filter(|d| d.millis.is_some()).count()
    }

    pub fn dns_slowest_ms(&self) -> Option<u64> {
        self.dns.iter().filter_map(|d| d.millis).max()
    }
}

pub fn probe() -> NetworkRaw {
    let mut raw = NetworkRaw::default();
    if platform::IS_WINDOWS {
        match platform::run_powershell(include_str!("../scripts/network.ps1"), Duration::from_secs(40)) {
            Ok(out) => apply_wmi(&mut raw, &out),
            Err(e) => raw.notes.push(format!("Data adapter jaringan tidak terbaca: {e}")),
        }
    }
    live_tests(&mut raw);
    raw
}

/// Bagian yang benar-benar mencoba ke internet. Dipisah supaya bisa dilewati di tes.
pub fn live_tests(raw: &mut NetworkRaw) {
    for host in DNS_PROBES {
        let millis = net::resolve(host, 80, Duration::from_secs(4)).ok().map(|(_, t)| t.as_millis() as u64);
        raw.dns.push(DnsResult { host: host.to_string(), millis });
    }

    let mut internet = false;
    for (url, expect) in CONNECT_TESTS {
        if let Ok(s) = net::http_get(url, 1024, Duration::from_secs(6)) {
            if s.status == 200 && String::from_utf8_lossy(&s.body).trim() == *expect {
                internet = true;
                break;
            }
        }
    }
    raw.internet = Some(internet);
    if !internet {
        return;
    }

    for url in DOWNLOAD_URLS {
        match net::http_get(url, DOWNLOAD_MAX_BYTES, DOWNLOAD_BUDGET) {
            Ok(s) if s.status == 200 && !s.body.is_empty() => {
                raw.download = Some(DownloadResult { url: url.to_string(), bytes: s.body.len() as u64, millis: s.elapsed.as_millis().max(1) as u64 });
                break;
            }
            Ok(s) => raw.notes.push(format!("{url}: jawaban {}", s.status)),
            Err(e) => raw.notes.push(format!("{url}: {e}")),
        }
    }
    if raw.download.is_none() {
        raw.notes.push("Tes kecepatan unduh tidak bisa dijalankan (semua alamat tes gagal).".into());
    }
}

pub fn apply_wmi(raw: &mut NetworkRaw, out: &str) {
    let v = match json::parse(out) {
        Ok(v) => v,
        Err(e) => {
            raw.notes.push(e);
            return;
        }
    };
    raw.profiles = json::list_of(v.get("profiles"))
        .into_iter()
        .map(|p| ProfileRaw {
            name: json::str_of(p.get("name")),
            alias: json::str_of(p.get("alias")),
            ipv4: json::str_of(p.get("ipv4")),
            category: json::str_of(p.get("category")),
        })
        .collect();
    raw.adapters = json::list_of(v.get("adapters"))
        .into_iter()
        .map(|a| AdapterRaw {
            name: json::str_of(a.get("name")),
            description: json::str_of(a.get("description")),
            link_speed: json::str_of(a.get("linkSpeed")),
            wifi: a.get("wifi").and_then(|b| b.as_bool()).unwrap_or(false),
        })
        .collect();
    raw.wifi_signal_pct = json::u64_of(v.get("wifiSignalPct")).map(|p| p.min(100) as u32);
    raw.wifi_ssid = Some(json::str_of(v.get("wifiSsid"))).filter(|s| !s.is_empty());
    raw.dns_servers = json::list_of(v.get("dnsServers")).into_iter().map(|s| json::str_of(Some(s))).filter(|s| !s.is_empty()).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wifi_output() {
        let mut raw = NetworkRaw::default();
        apply_wmi(
            &mut raw,
            r#"{"profiles":{"name":"Rumah","alias":"Wi-Fi","ipv4":"Internet","category":"Private"},
                "adapters":[{"name":"Wi-Fi","description":"Qualcomm Atheros QCA9377","linkSpeed":"72.2 Mbps","wifi":true}],
                "wifiSignalPct":38,"wifiSsid":"Rumah","dnsServers":["192.168.1.1","8.8.8.8"]}"#,
        );
        assert!(raw.has_link() && raw.on_wifi());
        assert_eq!(raw.wifi_signal_pct, Some(38));
        assert_eq!(raw.dns_servers.len(), 2);
    }

    #[test]
    fn mbps_math() {
        let d = DownloadResult { url: String::new(), bytes: 5_000_000, millis: 4000 };
        assert!((d.mbps() - 10.0).abs() < 0.01);
    }
}
