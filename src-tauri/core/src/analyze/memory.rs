//! Penilaian RAM.

use super::tanggal;
use crate::format::{installed_gb_guess, percent, size};
use crate::model::*;
use crate::probe::memory::MemRaw;

/// RAM terpasang sebanyak ini (GB) atau kurang = terlalu kecil untuk Windows 10/11.
pub const RAM_TOO_SMALL_GB: u64 = 3;
/// Sebanyak ini atau kurang = pas-pasan.
pub const RAM_TIGHT_GB: u64 = 5;
/// Pemakaian RAM (%).
pub const USAGE_FULL: f64 = 90.0;
pub const USAGE_HIGH: f64 = 80.0;

pub fn analyze(raw: &MemRaw) -> CategoryResult {
    let cat = Category::Memory;
    let mut findings = Vec::new();

    if raw.total > 0 {
        let gb = installed_gb_guess(raw.installed.unwrap_or(raw.total));
        let upgrade = upgrade_step(raw);
        if gb <= RAM_TOO_SMALL_GB {
            findings.push(Finding::problem(
                "memory.size",
                cat,
                Status::Bermasalah,
                Severity::Sedang,
                "RAM terlalu kecil",
                format!("PC ini hanya punya RAM {gb} GB. Windows 10/11 sendiri sudah memakai sebagian besarnya."),
                "RAM itu seperti meja kerja: makin kecil mejanya, makin sedikit program yang bisa dibuka bersamaan. Kalau penuh, Windows memakai disk sebagai gantinya, dan itu jauh lebih lambat.",
                &[upgrade.as_str(), "Sementara itu, buka satu atau dua program saja dalam satu waktu."],
            ));
        } else if gb <= RAM_TIGHT_GB {
            findings.push(Finding::problem(
                "memory.size",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                "RAM pas-pasan",
                format!("PC ini punya RAM {gb} GB. Cukup untuk pekerjaan ringan, tapi cepat penuh kalau membuka browser dengan banyak tab."),
                "RAM itu seperti meja kerja: kalau mejanya penuh, Windows terpaksa memakai disk yang jauh lebih lambat.",
                &[upgrade.as_str(), "Biasakan menutup tab dan program yang sudah tidak dipakai."],
            ));
        } else {
            findings.push(Finding::healthy(
                "memory.size",
                cat,
                "Ukuran RAM cukup",
                format!("PC ini punya RAM {gb} GB, cukup untuk pemakaian sehari-hari."),
            ));
        }

        if let Some(pct_val) = raw.used_pct() {
            let pct = percent(pct_val);
            let free = size(raw.available);
            let steps: &[&str] = &[
                "Tutup program dan tab browser yang tidak dipakai.",
                "Restart PC untuk mengosongkan memori.",
                "Matikan program yang ikut jalan saat PC menyala: tekan Ctrl + Shift + Esc, buka tab Startup, lalu pilih Disable pada program yang tidak perlu.",
            ];
            if pct_val >= USAGE_FULL {
                findings.push(Finding::problem(
                    "memory.usage",
                    cat,
                    Status::Bermasalah,
                    Severity::Sedang,
                    "Memori hampir habis",
                    format!("Saat diperiksa, RAM terpakai {pct} — sisanya tinggal {free}."),
                    "Kalau RAM penuh, Windows terpaksa memindahkan data ke disk yang jauh lebih lambat. Akibatnya PC tersendat saat pindah program.",
                    steps,
                ));
            } else if pct_val >= USAGE_HIGH {
                findings.push(Finding::problem(
                    "memory.usage",
                    cat,
                    Status::Perhatian,
                    Severity::Ringan,
                    "Memori cukup penuh",
                    format!("Saat diperiksa, RAM terpakai {pct} — sisanya {free}."),
                    "Kalau ditambah satu-dua program lagi, PC bisa mulai tersendat.",
                    &steps[..2],
                ));
            } else {
                findings.push(Finding::healthy(
                    "memory.usage",
                    cat,
                    "Memori masih lega",
                    format!("RAM terpakai {pct}, masih ada sisa {free}."),
                ));
            }
        }
    }

    if let Some(t) = &raw.memtest {
        let when = tanggal(&t.date);
        if t.errors_found {
            findings.push(Finding::problem(
                "memory.test",
                cat,
                Status::Bermasalah,
                Severity::Berat,
                "RAM terdeteksi rusak",
                format!("Tes memori Windows pada {when} menemukan kerusakan pada RAM."),
                "RAM yang rusak bisa membuat PC sering hang, layar biru (blue screen), atau file jadi rusak.",
                &[
                    "Simpan dulu data penting ke flashdisk atau penyimpanan lain.",
                    "Bawa ke teknisi untuk mengecek dan mengganti keping RAM yang rusak.",
                ],
            ));
        } else {
            findings.push(Finding::healthy(
                "memory.test",
                cat,
                "Tes memori terakhir: tidak ada kerusakan",
                format!("Tes memori Windows pada {when} tidak menemukan masalah."),
            ));
        }
    }

    CategoryResult::new(cat, findings, tech(raw))
}

/// Saran menambah RAM, disesuaikan dengan info slot bila ada.
fn upgrade_step(raw: &MemRaw) -> String {
    match (raw.slots_used, raw.slots_total) {
        (Some(used), Some(total)) if used < total => format!(
            "Tambah RAM. Menurut data PC, masih ada slot kosong ({used} dari {total} terisi), jadi biasanya cukup menambah satu keping. Minta teknisi memastikan jenis RAM yang cocok."
        ),
        (Some(_), Some(_)) => "Tambah RAM. Semua slot sudah terisi, jadi keping lama perlu diganti dengan yang lebih besar. Tanya teknisi jenis yang cocok.".to_string(),
        _ => "Tambah RAM — biasanya ini cara paling terasa untuk mempercepat PC lama. Minta teknisi memeriksa jenis RAM yang cocok (pada sebagian laptop, RAM tidak bisa ditambah).".to_string(),
    }
}

fn tech(raw: &MemRaw) -> TechSection {
    let mut t = TechSection::new("Memori (RAM)");
    if let Some(i) = raw.installed {
        t.push("Terpasang", size(i));
    }
    t.push("Bisa dipakai Windows", size(raw.total));
    t.push("Terpakai / tersedia", format!("{} / {}", size(raw.used()), size(raw.available)));
    if raw.swap_total > 0 {
        t.push("Page file (memori virtual)", format!("{} dari {} terpakai", size(raw.swap_used), size(raw.swap_total)));
    }
    if let (Some(u), Some(tot)) = (raw.slots_used, raw.slots_total) {
        t.push("Slot RAM", format!("{u} dari {tot} terisi (menurut data motherboard)"));
    }
    for m in &raw.modules {
        let speed = m.speed_mhz.map(|s| format!(" · {s} MHz")).unwrap_or_default();
        let maker = if m.maker.is_empty() { String::new() } else { format!(" · {}", m.maker) };
        let slot = if m.slot.is_empty() { "Keping RAM".to_string() } else { m.slot.clone() };
        t.push(slot, format!("{}{speed}{maker}", size(m.capacity)));
    }
    match &raw.memtest {
        Some(m) => t.push(
            "Windows Memory Diagnostic",
            format!("{} pada {}", if m.errors_found { "ADA ERROR (event 1202)" } else { "tidak ada error (event 1201)" }, m.date),
        ),
        None => t.push("Windows Memory Diagnostic", "Belum pernah dijalankan (atau log sudah terhapus)"),
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::memory::MemTest;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn raw(total_gib: f64, used_pct: f64) -> MemRaw {
        let total = (total_gib * GIB as f64) as u64;
        MemRaw { total, available: (total as f64 * (1.0 - used_pct / 100.0)) as u64, ..Default::default() }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn size_thresholds() {
        assert_eq!(find(&analyze(&raw(1.9, 50.0)), "memory.size").unwrap().status, Status::Bermasalah);
        assert_eq!(find(&analyze(&raw(3.86, 50.0)), "memory.size").unwrap().status, Status::Perhatian);
        assert_eq!(find(&analyze(&raw(7.88, 50.0)), "memory.size").unwrap().status, Status::Sehat);
    }

    #[test]
    fn installed_size_wins_over_usable() {
        // 4 GB terpasang, tapi 0,6 GB dipakai kartu grafis bawaan → Windows melihat 3,4 GB.
        let mut r = raw(3.4, 50.0);
        r.installed = Some(4 * GIB);
        assert!(find(&analyze(&r), "memory.size").unwrap().summary.contains("4 GB"));
    }

    #[test]
    fn usage_thresholds() {
        assert_eq!(find(&analyze(&raw(8.0, 94.0)), "memory.usage").unwrap().status, Status::Bermasalah);
        assert_eq!(find(&analyze(&raw(8.0, 85.0)), "memory.usage").unwrap().status, Status::Perhatian);
        assert_eq!(find(&analyze(&raw(8.0, 40.0)), "memory.usage").unwrap().status, Status::Sehat);
    }

    #[test]
    fn slot_advice() {
        let mut r = raw(3.9, 50.0);
        r.slots_used = Some(1);
        r.slots_total = Some(2);
        assert!(find(&analyze(&r), "memory.size").unwrap().steps[0].contains("slot kosong"));
    }

    #[test]
    fn memtest_errors_are_serious() {
        let mut r = raw(8.0, 40.0);
        r.memtest = Some(MemTest { errors_found: true, date: "2026-03-14".into() });
        let f = find(&analyze(&r), "memory.test").unwrap().clone();
        assert_eq!((f.status, f.severity), (Status::Bermasalah, Some(Severity::Berat)));
        assert!(f.summary.contains("14 Maret 2026"));
    }
}
