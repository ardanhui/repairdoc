//! Penilaian kartu grafis.

use crate::format::size;
use crate::model::*;
use crate::probe::gpu::{AdapterRaw, GpuRaw};

pub const TEMP_HOT: f64 = 90.0;
pub const TEMP_WARM: f64 = 80.0;

/// Nama adapter yang berarti Windows memakai driver darurat bawaan.
const FALLBACK_DRIVER_NAMES: &[&str] = &["microsoft basic display", "microsoft basic render", "standard vga"];

pub fn is_fallback_driver(a: &AdapterRaw) -> bool {
    let n = a.name.to_ascii_lowercase();
    FALLBACK_DRIVER_NAMES.iter().any(|f| n.contains(f))
}

pub fn analyze(raw: &GpuRaw) -> CategoryResult {
    let cat = Category::Gpu;
    let mut findings = Vec::new();

    for (i, a) in raw.adapters.iter().enumerate() {
        let id = format!("gpu.adapter{i}");
        let label = if raw.adapters.len() > 1 { format!("Kartu grafis ke-{}", i + 1) } else { "Kartu grafis".to_string() };
        if is_fallback_driver(a) {
            findings.push(Finding::problem(
                id,
                cat,
                Status::Bermasalah,
                Severity::Sedang,
                "Driver kartu grafis belum terpasang",
                "Windows sedang memakai driver tampilan darurat bawaan, bukan driver kartu grafis yang sebenarnya.",
                "Tanpa driver yang benar, semua tampilan dikerjakan prosesor: layar terasa patah-patah, video berat, dan resolusi sering tidak pas.",
                &[
                    "Sambungkan internet, lalu buka Settings → Windows Update → Check for updates. Lihat juga \"Optional updates\" → \"Driver updates\".",
                    "Kalau tidak muncul, unduh driver dari situs resmi merek laptop/PC (atau merek kartu grafisnya: Intel, AMD, NVIDIA).",
                    "Kalau ragu, minta teknisi memasangkan drivernya.",
                ],
            ));
        } else if let Some(code) = a.error_code.filter(|c| *c != 0) {
            findings.push(Finding::problem(
                id,
                cat,
                Status::Bermasalah,
                Severity::Sedang,
                format!("{label} bermasalah di Device Manager"),
                format!("{} dilaporkan bermasalah oleh Windows (kode {code}).", a.name),
                "Kartu grafis yang bermasalah membuat layar berkedip, hitam, atau tampilan jadi lambat.",
                &[
                    "Restart PC dulu — kadang cukup.",
                    "Pasang ulang driver kartu grafis dari situs resmi mereknya.",
                    "Kalau masih bermasalah, minta teknisi memeriksa kartunya.",
                ],
            ));
        } else {
            findings.push(Finding::healthy(id, cat, format!("{label} bekerja normal"), format!("{} dengan driver yang terpasang.", a.name)));
        }
    }

    if let Some(t) = raw.nvidia.as_ref().and_then(|n| n.temp_c) {
        let c = t.round() as i64;
        if t >= TEMP_HOT {
            findings.push(Finding::problem(
                "gpu.temp",
                cat,
                Status::Bermasalah,
                Severity::Berat,
                "Kartu grafis terlalu panas",
                format!("Suhu kartu grafis sekitar {c}°C."),
                "Kartu grafis yang kepanasan memperlambat diri dan bisa rusak permanen.",
                &[
                    "Bersihkan debu di kipas kartu grafis dan lubang angin (minta teknisi bila perlu).",
                    "Pastikan kipasnya berputar.",
                    "Jangan pakai laptop di atas kasur atau bantal.",
                ],
            ));
        } else if t >= TEMP_WARM {
            findings.push(Finding::problem(
                "gpu.temp",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                "Kartu grafis mulai panas",
                format!("Suhu kartu grafis sekitar {c}°C."),
                "Kalau terus naik, kartu grafis akan memperlambat diri supaya tidak rusak.",
                &["Bersihkan debu di kipas dan lubang angin.", "Pastikan aliran udara di sekitar PC tidak tertutup."],
            ));
        } else {
            findings.push(Finding::healthy("gpu.temp", cat, "Suhu kartu grafis normal", format!("Sekitar {c}°C — masih aman.")));
        }
    }

    CategoryResult::new(cat, findings, tech(raw))
}

fn tech(raw: &GpuRaw) -> TechSection {
    let mut t = TechSection::new("Kartu grafis");
    if raw.adapters.is_empty() {
        t.push("Adapter", "Tidak terbaca");
    }
    for a in &raw.adapters {
        let mut parts = Vec::new();
        if let Some(v) = a.vram {
            parts.push(format!("VRAM {} (menurut Windows, maks. terbaca 4 GB)", size(v)));
        }
        if !a.driver_version.is_empty() {
            parts.push(format!("driver {}", a.driver_version));
        }
        if let Some(d) = &a.driver_date {
            parts.push(format!("tanggal driver {d}"));
        }
        if let Some((w, h)) = a.resolution {
            parts.push(format!("{w}×{h}"));
        }
        if let Some(c) = a.error_code.filter(|c| *c != 0) {
            parts.push(format!("kode error {c}"));
        }
        t.push(a.name.clone(), parts.join(" · "));
    }
    match &raw.nvidia {
        Some(n) => {
            let temp = n.temp_c.map(|c| format!("{}°C", c.round() as i64)).unwrap_or_else(|| "suhu ?".into());
            let util = n.utilization_pct.map(|u| format!(", pemakaian {}%", u.round() as i64)).unwrap_or_default();
            t.push("nvidia-smi", format!("{temp}{util}"));
        }
        None => t.push("Suhu", "Tidak tersedia (hanya bisa dibaca untuk NVIDIA lewat nvidia-smi)"),
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::gpu::NvidiaRaw;

    fn adapter(name: &str, code: Option<u32>) -> AdapterRaw {
        AdapterRaw { name: name.into(), error_code: code, ..Default::default() }
    }

    #[test]
    fn basic_display_adapter_means_missing_driver() {
        let raw = GpuRaw { adapters: vec![adapter("Microsoft Basic Display Adapter", Some(0))], ..Default::default() };
        let r = analyze(&raw);
        assert_eq!(r.findings[0].status, Status::Bermasalah);
        assert_eq!(r.findings[0].title, "Driver kartu grafis belum terpasang");
    }

    #[test]
    fn error_code_and_healthy() {
        let raw = GpuRaw { adapters: vec![adapter("Intel HD Graphics", Some(43)), adapter("NVIDIA GeForce 920M", Some(0))], ..Default::default() };
        let r = analyze(&raw);
        assert_eq!(r.findings[0].status, Status::Bermasalah);
        assert_eq!(r.findings[1].status, Status::Sehat);
        assert!(r.findings[1].title.contains("ke-2"));
    }

    #[test]
    fn nvidia_temperature() {
        let hot = GpuRaw { nvidia: Some(NvidiaRaw { name: "x".into(), temp_c: Some(92.0), utilization_pct: None }), ..Default::default() };
        assert_eq!(analyze(&hot).findings[0].status, Status::Bermasalah);
        let ok = GpuRaw { nvidia: Some(NvidiaRaw { name: "x".into(), temp_c: Some(55.0), utilization_pct: None }), ..Default::default() };
        assert_eq!(analyze(&ok).findings[0].status, Status::Sehat);
    }
}
