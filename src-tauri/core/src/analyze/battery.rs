//! Penilaian baterai laptop. Mengembalikan `Err(alasan)` bila kategori ini
//! tidak berlaku (PC desktop) sehingga tampilan bisa menandainya "dilewati".

use crate::model::*;
use crate::probe::battery::BatteryRaw;

pub const HEALTH_GOOD: f64 = 80.0;
pub const HEALTH_WEAK: f64 = 50.0;

pub fn analyze(raw: &BatteryRaw) -> Result<CategoryResult, String> {
    let cat = Category::Battery;
    if !raw.has_battery {
        return match raw.is_laptop() {
            Some(true) => Ok(CategoryResult::new(
                cat,
                vec![Finding::problem(
                    "battery.missing",
                    cat,
                    Status::Perhatian,
                    Severity::Sedang,
                    "Baterai tidak terdeteksi",
                    "PC ini laptop, tapi Windows tidak melihat baterainya.",
                    "Baterai mungkin dilepas, longgar, atau sudah mati total. Tanpa baterai, laptop langsung mati saat kabel tercabut dan listrik padam.",
                    &["Pastikan baterai terpasang dengan benar.", "Kalau terpasang tapi tetap tidak terbaca, baterai kemungkinan perlu diganti (tanya teknisi)."],
                )],
                tech(raw),
            )),
            Some(false) => Err("PC desktop, tidak memakai baterai".into()),
            None => Err("Tidak ada baterai terdeteksi".into()),
        };
    }

    let mut findings = Vec::new();
    let charge = raw.charge_pct.map(|p| format!(" Saat ini terisi {p}%.")).unwrap_or_default();
    match raw.health_pct() {
        Some(h) => {
            let hp = h.round() as i64;
            if h >= HEALTH_GOOD {
                findings.push(Finding::healthy(
                    "battery.health",
                    cat,
                    "Baterai masih bagus",
                    format!("Kapasitasnya sekitar {hp}% dari saat baru.{charge}"),
                ));
            } else if h >= HEALTH_WEAK {
                findings.push(Finding::problem(
                    "battery.health",
                    cat,
                    Status::Perhatian,
                    Severity::Ringan,
                    "Baterai mulai menurun",
                    format!("Kapasitasnya tinggal sekitar {hp}% dari saat baru, jadi tidak setahan dulu.{charge}"),
                    "Ini wajar untuk baterai yang sudah lama dipakai. Belum perlu diganti kalau masih cukup untuk kebutuhan sehari-hari.",
                    &[
                        "Hindari membiarkan baterai sering habis sampai 0%.",
                        "Kalau laptop lebih sering tercolok listrik, ini tidak terlalu berpengaruh.",
                    ],
                ));
            } else {
                findings.push(Finding::problem(
                    "battery.health",
                    cat,
                    Status::Bermasalah,
                    Severity::Sedang,
                    "Baterai sudah lemah",
                    format!("Kapasitasnya tinggal sekitar {hp}% dari saat baru.{charge}"),
                    "Baterai selemah ini cepat habis dan bisa membuat laptop mati mendadak. Baterai yang menggembung juga berbahaya — kalau casing terlihat menonjol, segera lepas dan ganti.",
                    &[
                        "Rencanakan ganti baterai (tanya teknisi jenis yang cocok).",
                        "Sementara itu, pakai laptop dengan charger tercolok.",
                        "Simpan pekerjaan sesering mungkin.",
                    ],
                ));
            }
        }
        None => findings.push(Finding::healthy(
            "battery.health",
            cat,
            "Baterai terdeteksi",
            format!("Kesehatan baterai tidak bisa dibaca di laptop ini.{charge}"),
        )),
    }

    Ok(CategoryResult::new(cat, findings, tech(raw)))
}

fn tech(raw: &BatteryRaw) -> TechSection {
    let mut t = TechSection::new("Baterai");
    let chassis = raw.chassis.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ");
    let kind = match raw.is_laptop() {
        Some(true) => "laptop",
        Some(false) => "desktop",
        None => "tidak diketahui",
    };
    t.push("Jenis casing (SMBIOS)", if chassis.is_empty() { kind.to_string() } else { format!("{kind} (kode {chassis})") });
    t.push("Baterai", if raw.has_battery { "terdeteksi" } else { "tidak ada" });
    if let Some(p) = raw.charge_pct {
        t.push("Terisi", format!("{p}%"));
    }
    if let Some(ac) = raw.on_ac {
        t.push("Sumber daya", if ac { "listrik (charger)" } else { "baterai" });
    }
    if let (Some(f), Some(d)) = (raw.full_capacity, raw.design_capacity) {
        t.push("Kapasitas penuh / awal", format!("{f} / {d} mWh"));
    }
    if let Some(c) = raw.cycles {
        t.push("Siklus pengisian", c.to_string());
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laptop(full: u64, design: u64) -> BatteryRaw {
        BatteryRaw { chassis: vec![10], has_battery: true, charge_pct: Some(70), full_capacity: Some(full), design_capacity: Some(design), ..Default::default() }
    }

    #[test]
    fn health_thresholds() {
        assert_eq!(analyze(&laptop(90, 100)).unwrap().findings[0].status, Status::Sehat);
        assert_eq!(analyze(&laptop(65, 100)).unwrap().findings[0].status, Status::Perhatian);
        assert_eq!(analyze(&laptop(30, 100)).unwrap().findings[0].status, Status::Bermasalah);
    }

    #[test]
    fn desktop_is_skipped() {
        let raw = BatteryRaw { chassis: vec![3], ..Default::default() };
        assert!(analyze(&raw).is_err());
    }

    #[test]
    fn laptop_without_battery_is_flagged() {
        let raw = BatteryRaw { chassis: vec![9], ..Default::default() };
        assert_eq!(analyze(&raw).unwrap().findings[0].status, Status::Perhatian);
    }

    #[test]
    fn unknown_health_makes_no_claim() {
        let raw = BatteryRaw { chassis: vec![10], has_battery: true, charge_pct: Some(50), ..Default::default() };
        let f = &analyze(&raw).unwrap().findings[0];
        assert!(f.summary.contains("tidak bisa dibaca"));
    }
}
