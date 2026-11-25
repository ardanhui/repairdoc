//! Penilaian: mengubah data mentah menjadi temuan berbahasa awam.
//! Semua fungsi di sini murni (tanpa akses sistem) sehingga mudah dites.

pub mod battery;
pub mod cleanup;
pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod programs;
pub mod security;
pub mod system;

use crate::model::*;

const BULAN: [&str; 12] = [
    "Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November",
    "Desember",
];

/// "2026-03-14" → "14 Maret 2026". Kalau formatnya lain, dikembalikan apa adanya.
pub fn tanggal(ymd: &str) -> String {
    let parts: Vec<&str> = ymd.split('-').collect();
    if let [y, m, d] = parts[..] {
        if let (Ok(m), Ok(d)) = (m.parse::<usize>(), d.parse::<u32>()) {
            if (1..=12).contains(&m) {
                return format!("{d} {} {y}", BULAN[m - 1]);
            }
        }
    }
    ymd.to_string()
}

/// Urutkan temuan: bermasalah → perhatian → sehat; di dalamnya yang paling berat dulu.
pub fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| b.status.cmp(&a.status).then(b.severity.cmp(&a.severity)));
}

pub fn verdict(findings: &[Finding]) -> Verdict {
    let problem_count = findings.iter().filter(|f| f.status == Status::Bermasalah).count();
    let attention_count = findings.iter().filter(|f| f.status == Status::Perhatian).count();
    let healthy_count = findings.iter().filter(|f| f.status == Status::Sehat).count();

    let status = if problem_count > 0 {
        Status::Bermasalah
    } else if attention_count > 0 {
        Status::Perhatian
    } else {
        Status::Sehat
    };

    let headline = match status {
        Status::Bermasalah => "PC ini punya masalah yang perlu dibereskan",
        Status::Perhatian => "PC ini perlu sedikit perhatian",
        Status::Sehat => "PC ini sehat",
    }
    .to_string();

    let detail = match (problem_count, attention_count) {
        (0, 0) => "Semua yang diperiksa dalam kondisi baik.".to_string(),
        (p, 0) => format!("Ditemukan {p} masalah."),
        (0, a) => format!("Ada {a} hal yang perlu diperhatikan."),
        (p, a) => format!("Ditemukan {p} masalah dan {a} hal yang perlu diperhatikan."),
    };

    let mut sorted: Vec<&Finding> = findings.iter().filter(|f| f.status != Status::Sehat).collect();
    sorted.sort_by(|a, b| b.status.cmp(&a.status).then(b.severity.cmp(&a.severity)));
    let top_priority = sorted.first().map(|f| format!("Yang paling penting: {}.", f.title));

    Verdict { status, headline, detail, top_priority, problem_count, attention_count, healthy_count }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(status: Status, severity: Option<Severity>, title: &str) -> Finding {
        Finding {
            id: title.into(),
            category: Category::Cpu,
            status,
            severity,
            title: title.into(),
            summary: String::new(),
            why: None,
            steps: vec![],
        }
    }

    #[test]
    fn tanggal_indonesia() {
        assert_eq!(tanggal("2026-03-14"), "14 Maret 2026");
        assert_eq!(tanggal("2026-12-01"), "1 Desember 2026");
        assert_eq!(tanggal("kemarin"), "kemarin");
    }

    #[test]
    fn verdict_picks_worst_and_top_priority() {
        let list = vec![
            f(Status::Sehat, None, "A"),
            f(Status::Perhatian, Some(Severity::Ringan), "B"),
            f(Status::Bermasalah, Some(Severity::Sedang), "C"),
            f(Status::Bermasalah, Some(Severity::Berat), "D"),
        ];
        let v = verdict(&list);
        assert_eq!(v.status, Status::Bermasalah);
        assert_eq!(v.detail, "Ditemukan 2 masalah dan 1 hal yang perlu diperhatikan.");
        assert_eq!(v.top_priority.as_deref(), Some("Yang paling penting: D."));
    }

    #[test]
    fn verdict_all_healthy() {
        let v = verdict(&[f(Status::Sehat, None, "A")]);
        assert_eq!(v.status, Status::Sehat);
        assert_eq!(v.headline, "PC ini sehat");
        assert!(v.top_priority.is_none());
    }

    #[test]
    fn sorting_puts_worst_first() {
        let mut list = vec![
            f(Status::Sehat, None, "A"),
            f(Status::Bermasalah, Some(Severity::Sedang), "C"),
            f(Status::Perhatian, Some(Severity::Ringan), "B"),
            f(Status::Bermasalah, Some(Severity::Berat), "D"),
        ];
        sort_findings(&mut list);
        let order: Vec<&str> = list.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(order, ["D", "C", "B", "A"]);
    }
}
