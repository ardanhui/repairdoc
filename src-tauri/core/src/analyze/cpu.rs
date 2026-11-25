//! Penilaian prosesor.

use crate::format::{decimal, percent};
use crate::model::*;
use crate::probe::cpu::{CpuRaw, TempSource};

/// Rata-rata pemakaian di atas ini = prosesor kewalahan.
pub const USAGE_HIGH: f64 = 90.0;
/// Rata-rata pemakaian di atas ini = prosesor cukup sibuk.
pub const USAGE_BUSY: f64 = 70.0;
/// Suhu zona termal (°C).
pub const TEMP_HOT: f64 = 90.0;
pub const TEMP_WARM: f64 = 80.0;
/// Di bawah ini, pembacaan sensor ACPI biasanya palsu/statis — tidak ditampilkan ke pengguna.
pub const TEMP_DOUBTFUL_BELOW: f64 = 30.0;
/// Prosesor dengan jumlah utas sebanyak ini atau kurang tergolong lemah untuk Windows 10/11.
pub const WEAK_THREADS: usize = 2;

const STEP_WAIT_AFTER_BOOT: &str =
    "Kalau PC baru saja dinyalakan, tunggu 5–10 menit lalu scan ulang. Windows memang sibuk sebentar setelah menyala.";
const STEP_CLOSE_APPS: &str = "Tutup program yang sedang tidak dipakai, terutama browser dengan banyak tab.";
const STEP_TASK_MANAGER: &str =
    "Kalau tetap tinggi, tekan Ctrl + Shift + Esc untuk membuka Task Manager, lalu lihat program paling atas di kolom CPU.";
const STEP_RESTART: &str = "Restart PC kalau sudah berhari-hari tidak dimatikan.";

pub fn analyze(raw: &CpuRaw) -> CategoryResult {
    let mut findings = Vec::new();
    let cat = Category::Cpu;

    if let Some(u) = raw.usage_avg {
        let pct = percent(u);
        if u >= USAGE_HIGH {
            findings.push(Finding::problem(
                "cpu.usage",
                cat,
                Status::Bermasalah,
                Severity::Berat,
                "Prosesor sedang kewalahan",
                format!("Saat diperiksa, prosesor terpakai rata-rata {pct}. Hampir tidak ada sisa tenaga untuk pekerjaan lain."),
                "Prosesor adalah \"otak\" komputer. Kalau sudah penuh, setiap klik harus antre, sehingga PC terasa macet atau \"hang\".",
                &[STEP_WAIT_AFTER_BOOT, STEP_CLOSE_APPS, STEP_TASK_MANAGER, STEP_RESTART],
            ));
        } else if u >= USAGE_BUSY {
            findings.push(Finding::problem(
                "cpu.usage",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                "Prosesor cukup sibuk",
                format!("Saat diperiksa, prosesor terpakai rata-rata {pct}. Masih jalan, tapi sisa tenaganya tinggal sedikit."),
                "Kalau prosesor sering sibuk, membuka program baru akan terasa lambat.",
                &[STEP_WAIT_AFTER_BOOT, STEP_CLOSE_APPS, STEP_TASK_MANAGER],
            ));
        } else {
            findings.push(Finding::healthy(
                "cpu.usage",
                cat,
                "Prosesor bekerja normal",
                format!("Prosesor tidak sedang kewalahan (terpakai sekitar {pct})."),
            ));
        }
    }

    if raw.logical_cores > 0 && raw.logical_cores <= WEAK_THREADS {
        findings.push(Finding::problem(
            "cpu.weak",
            cat,
            Status::Perhatian,
            Severity::Ringan,
            "Prosesor tergolong lemah",
            format!(
                "Prosesor PC ini hanya bisa mengerjakan {} tugas sekaligus. Untuk Windows 10/11, itu tergolong kecil.",
                raw.logical_cores
            ),
            "Ini bukan kerusakan, tapi batas kemampuan hardware. Pengaturan yang lebih ringan bisa membantu, tapi PC tidak akan secepat komputer baru.",
            &[
                "Jalankan satu atau dua program saja dalam satu waktu.",
                "Pilih aplikasi yang ringan, misalnya browser dengan sedikit tab.",
                "Kalau sering dipakai untuk pekerjaan berat, pertimbangkan ganti PC (tanya teknisi).",
            ],
        ));
    }

    if let Some(t) = raw.temp.as_ref().filter(|t| t.celsius >= TEMP_DOUBTFUL_BELOW) {
        let c = t.celsius.round() as i64;
        let cooling_steps: &[&str] = &[
            "Bersihkan debu di kipas dan lubang angin (bisa minta tolong teknisi).",
            "Untuk laptop: jangan dipakai di atas kasur, bantal, atau sarung yang menutup lubang angin.",
            "Pastikan kipas berputar dan tidak berbunyi kasar.",
            "Kalau PC sudah bertahun-tahun, pasta pendingin prosesor mungkin perlu diganti — minta teknisi.",
        ];
        if t.celsius >= TEMP_HOT {
            findings.push(Finding::problem(
                "cpu.temp",
                cat,
                Status::Bermasalah,
                Severity::Berat,
                "PC terlalu panas",
                format!("Sensor suhu di dalam PC membaca sekitar {c}°C. Ini sudah terlalu panas."),
                "Kalau terlalu panas, prosesor sengaja memperlambat diri supaya tidak rusak, jadi PC terasa lemot. Panas berlebih juga memperpendek umur komponen.",
                cooling_steps,
            ));
        } else if t.celsius >= TEMP_WARM {
            findings.push(Finding::problem(
                "cpu.temp",
                cat,
                Status::Perhatian,
                Severity::Sedang,
                "PC mulai panas",
                format!("Sensor suhu di dalam PC membaca sekitar {c}°C. Masih jalan, tapi mendekati batas."),
                "Kalau suhu terus naik, prosesor akan memperlambat diri supaya tidak rusak.",
                &cooling_steps[..3],
            ));
        } else {
            findings.push(Finding::healthy(
                "cpu.temp",
                cat,
                "Suhu PC normal",
                format!("Sensor suhu di dalam PC membaca sekitar {c}°C — masih aman."),
            ));
        }
    }

    CategoryResult::new(cat, findings, tech(raw))
}

fn tech(raw: &CpuRaw) -> TechSection {
    let mut t = TechSection::new("Prosesor");
    t.push("Model", if raw.name.is_empty() { "Tidak terbaca".to_string() } else { raw.name.clone() });
    let cores = match raw.physical_cores {
        Some(p) => format!("{p} inti fisik, {} utas", raw.logical_cores),
        None => format!("{} utas", raw.logical_cores),
    };
    t.push("Inti / utas", cores);
    match (raw.usage_avg, raw.usage_peak) {
        (Some(a), Some(p)) => t.push("Pemakaian (±2 detik)", format!("rata-rata {}, puncak {}", percent(a), percent(p))),
        _ => t.push("Pemakaian", "Tidak terbaca"),
    }
    match &raw.temp {
        Some(tr) => {
            let src = match tr.source {
                TempSource::ThermalZone => "thermal zone (perf counter)",
                TempSource::Acpi => "MSAcpi_ThermalZoneTemperature",
            };
            let doubt = if tr.celsius < TEMP_DOUBTFUL_BELOW { " — nilai meragukan, tidak ditampilkan ke pengguna" } else { "" };
            t.push("Suhu zona termal", format!("{}°C via {src}{doubt}", decimal(tr.celsius, 1)));
        }
        None => t.push("Suhu", "Tidak tersedia di PC ini"),
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::cpu::TempReading;

    fn raw(usage: f64, threads: usize, temp: Option<f64>) -> CpuRaw {
        CpuRaw {
            name: "Tes CPU".into(),
            physical_cores: Some(threads.div_ceil(2)),
            logical_cores: threads,
            usage_avg: Some(usage),
            usage_peak: Some(usage),
            temp: temp.map(|c| TempReading { celsius: c, source: TempSource::ThermalZone }),
            notes: vec![],
        }
    }

    fn status_of(r: &CategoryResult, id: &str) -> Option<Status> {
        r.findings.iter().find(|f| f.id == id).map(|f| f.status)
    }

    #[test]
    fn usage_thresholds() {
        assert_eq!(status_of(&analyze(&raw(95.0, 4, None)), "cpu.usage"), Some(Status::Bermasalah));
        assert_eq!(status_of(&analyze(&raw(75.0, 4, None)), "cpu.usage"), Some(Status::Perhatian));
        assert_eq!(status_of(&analyze(&raw(12.0, 4, None)), "cpu.usage"), Some(Status::Sehat));
    }

    #[test]
    fn weak_cpu_flagged() {
        assert_eq!(status_of(&analyze(&raw(10.0, 2, None)), "cpu.weak"), Some(Status::Perhatian));
        assert_eq!(status_of(&analyze(&raw(10.0, 4, None)), "cpu.weak"), None);
    }

    #[test]
    fn temperature_rules() {
        assert_eq!(status_of(&analyze(&raw(10.0, 4, Some(93.0))), "cpu.temp"), Some(Status::Bermasalah));
        assert_eq!(status_of(&analyze(&raw(10.0, 4, Some(84.0))), "cpu.temp"), Some(Status::Perhatian));
        assert_eq!(status_of(&analyze(&raw(10.0, 4, Some(55.0))), "cpu.temp"), Some(Status::Sehat));
        // Nilai ACPI palsu (mis. 27,8°C statis) tidak ditampilkan ke pengguna.
        assert_eq!(status_of(&analyze(&raw(10.0, 4, Some(27.8))), "cpu.temp"), None);
    }

    #[test]
    fn problems_always_have_steps() {
        let r = analyze(&raw(99.0, 2, Some(95.0)));
        for f in r.findings.iter().filter(|f| f.status != Status::Sehat) {
            assert!(f.why.is_some() && !f.steps.is_empty(), "{} tidak punya solusi", f.id);
        }
    }
}
