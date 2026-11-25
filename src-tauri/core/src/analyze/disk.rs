//! Penilaian penyimpanan: ruang kosong per drive dan kondisi disk fisik.

use crate::format::{bytes_to_gib, percent, size};
use crate::model::*;
use crate::probe::disk::{BusKind, DiskHealth, DiskRaw, MediaKind, PhysDiskRaw, VolumeRaw};

/// Batas ruang kosong drive Windows (C:). Dipakai angka GB, bukan persen,
/// karena yang dibutuhkan Windows (file sementara, update) kurang lebih tetap
/// berapa pun ukuran disknya: 25 GB kosong di disk 1 TB tetap aman.
pub const SYS_CRITICAL_GB: f64 = 2.0;
pub const SYS_LOW_GB: f64 = 5.0;
pub const SYS_WARN_GB: f64 = 10.0;
/// Di bawah persen ini DAN di bawah `SYS_WARN_PCT_MAX_GB` → mulai penuh.
pub const SYS_WARN_PCT: f64 = 10.0;
pub const SYS_WARN_PCT_MAX_GB: f64 = 25.0;
/// Drive lain (D:, E:, …) hanya diberi catatan bila hampir penuh.
pub const OTHER_WARN_PCT: f64 = 5.0;
/// Umur pakai SSD (%).
pub const WEAR_CRITICAL: u64 = 90;
pub const WEAR_WARN: u64 = 70;
/// Suhu disk (°C).
pub const HDD_HOT_C: f64 = 55.0;
pub const SSD_HOT_C: f64 = 70.0;

const STEP_BACKUP: &str = "Segera salin data penting (foto, dokumen) ke flashdisk, hardisk eksternal, atau Google Drive.";

pub fn analyze(raw: &DiskRaw) -> CategoryResult {
    let cat = Category::Storage;
    let mut findings = Vec::new();

    // 1) Ruang kosong per drive.
    for v in raw.volumes.iter().filter(|v| counts(v, raw)) {
        findings.push(volume_finding(v, raw.system_letter));
    }

    // 2) Kondisi disk fisik (selain USB / flashdisk).
    let internal: Vec<&PhysDiskRaw> = raw
        .physical
        .iter()
        .filter(|d| d.bus != BusKind::Usb)
        .filter(|d| !(raw.exe_letter.is_some_and(|e| d.letters.contains(&e))))
        .collect();
    for (i, d) in internal.iter().enumerate() {
        let label = disk_label(d, raw.system_letter, i, internal.len());
        findings.extend(physical_findings(d, &label, i));
    }

    // 3) Windows di HDD atau SSD?
    if let Some(f) = system_media_finding(raw, &internal) {
        findings.push(f);
    }

    let mut result = CategoryResult::new(cat, findings, tech(raw));
    if crate::platform::IS_WINDOWS && !raw.reliability_readable {
        result.needs_admin.push("umur SSD, suhu disk, dan riwayat error disk".into());
    }
    result
}

/// Drive yang ikut dinilai: bukan flashdisk, bukan drive tempat RepairDoc, dan berukuran wajar.
fn counts(v: &VolumeRaw, raw: &DiskRaw) -> bool {
    if v.removable || v.total < 1024 * 1024 * 1024 {
        return false;
    }
    if raw.exe_letter.is_some() && v.letter == raw.exe_letter {
        return false;
    }
    // Di Windows hanya drive berhuruf yang relevan bagi pengguna.
    !crate::platform::IS_WINDOWS || v.letter.is_some()
}

fn drive_name(v: &VolumeRaw) -> String {
    match v.letter {
        Some(l) => format!("Drive {l}:"),
        None => format!("Drive {}", v.mount),
    }
}

fn volume_finding(v: &VolumeRaw, system_letter: Option<char>) -> Finding {
    let cat = Category::Storage;
    let name = drive_name(v);
    let id = format!("storage.volume.{}", v.letter.map(String::from).unwrap_or_else(|| v.mount.clone()));
    let free_gb = bytes_to_gib(v.available);
    let pct = if v.total > 0 { v.available as f64 / v.total as f64 * 100.0 } else { 100.0 };
    let is_system = v.letter.is_some() && v.letter == system_letter || (!crate::platform::IS_WINDOWS && v.mount == "/");
    let space = format!("{} dari {} ({})", size(v.available), size(v.total), percent(pct));

    if is_system {
        let critical = free_gb < SYS_CRITICAL_GB;
        let low = free_gb < SYS_LOW_GB;
        let warn = free_gb < SYS_WARN_GB || (pct < SYS_WARN_PCT && free_gb < SYS_WARN_PCT_MAX_GB);
        let why = format!(
            "{name} adalah tempat Windows terpasang. Windows butuh ruang kosong untuk file sementara dan update. Kalau hampir penuh, PC jadi lambat dan update bisa gagal."
        );
        let steps: &[&str] = &[
            "Kosongkan Recycle Bin.",
            "Pindahkan foto, video, dan unduhan besar ke drive lain, flashdisk, atau hardisk eksternal.",
            "Hapus program yang tidak pernah dipakai: buka Settings → Apps, pilih programnya, lalu Uninstall.",
            "Jalankan Disk Cleanup: ketik \"Disk Cleanup\" di menu Start, pilih drive C:, centang semua, lalu OK.",
        ];
        if critical || low {
            return Finding::problem(
                id,
                cat,
                Status::Bermasalah,
                if critical { Severity::Berat } else { Severity::Sedang },
                format!("{name} hampir penuh"),
                format!("Sisa ruang hanya {space}."),
                why,
                steps,
            );
        }
        if warn {
            return Finding::problem(
                id,
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("{name} mulai penuh"),
                format!("Sisa ruang {space}."),
                "Masih aman untuk sekarang, tapi kalau terus terisi, PC akan melambat dan update Windows bisa gagal.",
                &steps[..2],
            );
        }
    } else if pct < OTHER_WARN_PCT {
        return Finding::problem(
            id,
            cat,
            Status::Perhatian,
            Severity::Ringan,
            format!("{name} hampir penuh"),
            format!("Sisa ruang hanya {space}."),
            "Ini tidak memperlambat Windows, tapi sebentar lagi kamu tidak bisa menyimpan file baru di drive ini.",
            &[
                "Pindahkan file besar yang jarang dipakai ke hardisk eksternal.",
                "Hapus unduhan lama dan file yang dobel.",
            ],
        );
    }

    Finding::healthy(id, cat, format!("{name} masih lega"), format!("Sisa ruang {space}."))
}

/// Nama disk yang mudah dipahami: "Disk utama (C:)", "Disk tambahan (D:)".
fn disk_label(d: &PhysDiskRaw, system_letter: Option<char>, index: usize, count: usize) -> String {
    let letters = d.letters.iter().map(|l| format!("{l}:")).collect::<Vec<_>>().join(", ");
    let is_system = system_letter.is_some_and(|s| d.letters.contains(&s));
    match (is_system, letters.is_empty()) {
        (true, _) => format!("Disk utama ({letters})"),
        (false, false) => format!("Disk tambahan ({letters})"),
        (false, true) if count == 1 => "Disk utama".to_string(),
        (false, true) => format!("Disk ke-{}", index + 1),
    }
}

fn physical_findings(d: &PhysDiskRaw, label: &str, index: usize) -> Vec<Finding> {
    let cat = Category::Storage;
    let base = format!("storage.disk{index}");
    let mut out = Vec::new();

    match d.health {
        DiskHealth::Unhealthy => out.push(Finding::problem(
            format!("{base}.health"),
            cat,
            Status::Bermasalah,
            Severity::Berat,
            format!("{label} menunjukkan tanda-tanda rusak"),
            "Windows melaporkan disk ini tidak sehat. Disk bisa berhenti bekerja sewaktu-waktu.",
            "Kalau disk rusak total, Windows tidak bisa menyala dan data di dalamnya bisa hilang.",
            &[
                STEP_BACKUP,
                "Bawa PC ke teknisi untuk mengganti disk.",
                "Sampai diganti, hindari mematikan PC secara paksa (tekan lama tombol power).",
            ],
        )),
        DiskHealth::Warning => out.push(Finding::problem(
            format!("{base}.health"),
            cat,
            Status::Perhatian,
            Severity::Sedang,
            format!("{label} mulai bermasalah"),
            "Windows memberi peringatan tentang kondisi disk ini.",
            "Peringatan seperti ini sering muncul sebelum disk rusak. Lebih baik bersiap dari sekarang.",
            &[STEP_BACKUP, "Minta teknisi memeriksa disk ini lebih lanjut."],
        )),
        _ => {}
    }

    if let Some(w) = d.wear.filter(|_| d.media == MediaKind::Ssd) {
        if w >= WEAR_WARN {
            let critical = w >= WEAR_CRITICAL;
            out.push(Finding::problem(
                format!("{base}.wear"),
                cat,
                if critical { Status::Bermasalah } else { Status::Perhatian },
                if critical { Severity::Sedang } else { Severity::Ringan },
                if critical { format!("{label}: umur SSD hampir habis") } else { format!("{label}: umur SSD sudah banyak terpakai") },
                format!("SSD ini sudah terpakai sekitar {w}% dari perkiraan umur pakainya."),
                "SSD punya batas jumlah tulis data. Mendekati batas itu, SSD bisa tiba-tiba rusak.",
                &[STEP_BACKUP, "Rencanakan penggantian SSD dalam waktu dekat (tanya teknisi)."],
            ));
        }
    }

    if let Some(t) = d.temp_c {
        let limit = if d.media == MediaKind::Hdd { HDD_HOT_C } else { SSD_HOT_C };
        if t >= limit {
            out.push(Finding::problem(
                format!("{base}.temp"),
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("{label} terlalu hangat"),
                format!("Suhu disk sekitar {}°C.", t.round() as i64),
                "Disk yang sering kepanasan lebih cepat rusak.",
                &[
                    "Pastikan kipas bersih dan aliran udara di dalam PC lancar.",
                    "Untuk laptop: jangan tutup lubang anginnya.",
                ],
            ));
        }
    }

    if d.health != DiskHealth::Unhealthy {
        if let Some(n) = d.read_errors_uncorrected.filter(|n| *n > 0) {
            out.push(Finding::problem(
                format!("{base}.errors"),
                cat,
                Status::Perhatian,
                Severity::Sedang,
                format!("{label} pernah gagal membaca data"),
                format!("Tercatat {n} kali disk ini gagal membaca data."),
                "Ini bisa jadi tanda awal kerusakan pada disk.",
                &[STEP_BACKUP, "Minta teknisi memeriksa disk ini lebih lanjut."],
            ));
        }
    }

    if out.is_empty() && d.health == DiskHealth::Healthy {
        out.push(Finding::healthy(
            format!("{base}.health"),
            cat,
            format!("{label} dalam kondisi baik"),
            "Windows tidak menemukan tanda kerusakan pada disk ini.",
        ));
    }
    out
}

/// Apakah Windows berjalan dari HDD (lambat) atau SSD?
fn system_media_finding(raw: &DiskRaw, internal: &[&PhysDiskRaw]) -> Option<Finding> {
    let sys = raw.system_letter?;
    let from_physical = internal.iter().find(|d| d.letters.contains(&sys)).map(|d| d.media).filter(|m| *m != MediaKind::Unknown);
    let from_volume = raw.volumes.iter().find(|v| v.letter == Some(sys)).map(|v| v.kind).filter(|m| *m != MediaKind::Unknown);
    match from_physical.or(from_volume)? {
        MediaKind::Hdd => Some(Finding::problem(
            "storage.system_hdd",
            Category::Storage,
            Status::Perhatian,
            Severity::Sedang,
            "Windows masih di hardisk jenis lama (HDD)",
            "Windows di PC ini berjalan dari hardisk piringan (HDD), bukan SSD (penyimpanan modern tanpa piringan berputar).",
            "HDD jauh lebih lambat dari SSD. Ini penyebab paling umum PC lama terasa lemot saat menyala dan membuka program — pengaturan apa pun tidak bisa menutup selisihnya.",
            &[
                "Ganti ke SSD. Untuk PC lama, ini biasanya peningkatan yang paling terasa: menyala dan membuka program jadi jauh lebih cepat.",
                "Minta teknisi memindahkan (clone) Windows dan data ke SSD baru, supaya tidak perlu install ulang.",
                "Sambil menunggu, jaga Drive C: tetap lega dan hindari menjalankan banyak program sekaligus.",
            ],
        )),
        MediaKind::Ssd => Some(Finding::healthy(
            "storage.system_hdd",
            Category::Storage,
            "Windows sudah berjalan di SSD",
            "Bagus — SSD jauh lebih cepat daripada hardisk lama (HDD).",
        )),
        MediaKind::Unknown => None,
    }
}

fn media_name(m: MediaKind) -> &'static str {
    match m {
        MediaKind::Hdd => "HDD",
        MediaKind::Ssd => "SSD",
        MediaKind::Unknown => "jenis tidak diketahui",
    }
}

fn tech(raw: &DiskRaw) -> TechSection {
    let mut t = TechSection::new("Penyimpanan");
    for v in &raw.volumes {
        let mut tags = vec![media_name(v.kind).to_string()];
        if !v.file_system.is_empty() {
            tags.push(v.file_system.clone());
        }
        if v.removable {
            tags.push("removable".into());
        }
        if raw.exe_letter.is_some() && v.letter == raw.exe_letter {
            tags.push("tempat RepairDoc, dilewati".into());
        }
        let label = if v.label.is_empty() { String::new() } else { format!(" \"{}\"", v.label) };
        t.push(
            format!("{}{label}", drive_name(v)),
            format!("{} kosong dari {} · {}", size(v.available), size(v.total), tags.join(" · ")),
        );
    }
    for d in &raw.physical {
        let bus = match &d.bus {
            BusKind::Usb => "USB".to_string(),
            BusKind::Sata => "SATA".to_string(),
            BusKind::Nvme => "NVMe".to_string(),
            BusKind::Other(s) if s.is_empty() => "bus ?".to_string(),
            BusKind::Other(s) => format!("bus {s}"),
        };
        let health = match d.health {
            DiskHealth::Healthy => "Healthy",
            DiskHealth::Warning => "Warning",
            DiskHealth::Unhealthy => "Unhealthy",
            DiskHealth::Unknown => "status tidak diketahui",
        };
        let mut parts = vec![media_name(d.media).to_string(), bus, health.to_string(), size(d.size)];
        if let Some(w) = d.wear {
            parts.push(format!("wear {w}%"));
        }
        if let Some(c) = d.temp_c {
            parts.push(format!("{}°C", c.round() as i64));
        }
        if let Some(e) = d.read_errors_uncorrected {
            parts.push(format!("{e} read error"));
        }
        if let Some(h) = d.power_on_hours {
            parts.push(format!("{h} jam menyala"));
        }
        let letters = if d.letters.is_empty() { String::new() } else { format!(" [{}]", d.letters.iter().map(|l| format!("{l}:")).collect::<Vec<_>>().join(" ")) };
        let name = if d.name.is_empty() { "Disk".to_string() } else { d.name.clone() };
        t.push(format!("{name}{letters}"), parts.join(" · "));
    }
    if let Some(src) = &raw.physical_source {
        t.push("Sumber data disk", if src == "storage" { "MSFT_PhysicalDisk (lengkap)" } else { "Win32_DiskDrive (terbatas)" });
    }
    if crate::platform::IS_WINDOWS && !raw.reliability_readable {
        t.push("Catatan", "Umur SSD, suhu disk, dan riwayat error hanya bisa dibaca dengan izin Administrator.");
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn vol(letter: char, total_gb: u64, free_gb: f64, kind: MediaKind) -> VolumeRaw {
        VolumeRaw {
            mount: format!("{letter}:\\"),
            letter: Some(letter),
            label: String::new(),
            file_system: "NTFS".into(),
            total: total_gb * GIB,
            available: (free_gb * GIB as f64) as u64,
            removable: false,
            kind,
        }
    }

    fn disk(letters: &[char], media: MediaKind, health: DiskHealth) -> PhysDiskRaw {
        PhysDiskRaw {
            name: "Tes Disk".into(),
            media,
            bus: BusKind::Sata,
            health,
            size: 500 * GIB,
            letters: letters.to_vec(),
            wear: None,
            temp_c: None,
            read_errors_uncorrected: None,
            power_on_hours: None,
        }
    }

    fn raw(volumes: Vec<VolumeRaw>, physical: Vec<PhysDiskRaw>) -> DiskRaw {
        DiskRaw { volumes, physical, system_letter: Some('C'), exe_letter: Some('E'), ..Default::default() }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn system_drive_space_thresholds() {
        let check = |free: f64| find(&analyze(&raw(vec![vol('C', 118, free, MediaKind::Ssd)], vec![])), "storage.volume.C").unwrap().clone();
        let f = check(1.5);
        assert_eq!((f.status, f.severity), (Status::Bermasalah, Some(Severity::Berat)));
        assert_eq!(f.title, "Drive C: hampir penuh");
        assert!(f.summary.contains("1,5 GB dari 118 GB"));
        assert_eq!(check(4.0).severity, Some(Severity::Sedang));
        assert_eq!(check(9.0).status, Status::Perhatian);
        assert_eq!(check(11.0).status, Status::Perhatian, "11 GB dari 118 GB = 9%");
        assert_eq!(check(40.0).status, Status::Sehat);
        // Disk besar: 25 GB kosong dari 1 TB tetap aman.
        let big = find(&analyze(&raw(vec![vol('C', 1000, 26.0, MediaKind::Ssd)], vec![])), "storage.volume.C").unwrap().status;
        assert_eq!(big, Status::Sehat);
    }

    #[test]
    fn data_drive_is_lenient() {
        let r = analyze(&raw(vec![vol('D', 500, 40.0, MediaKind::Hdd)], vec![]));
        assert_eq!(find(&r, "storage.volume.D").unwrap().status, Status::Sehat);
        let r = analyze(&raw(vec![vol('D', 500, 10.0, MediaKind::Hdd)], vec![]));
        assert_eq!(find(&r, "storage.volume.D").unwrap().status, Status::Perhatian);
    }

    #[test]
    fn flashdisk_is_skipped() {
        let mut usb = vol('E', 32, 0.5, MediaKind::Unknown);
        usb.removable = true;
        let mut usb_phys = disk(&['E'], MediaKind::Unknown, DiskHealth::Unhealthy);
        usb_phys.bus = BusKind::Usb;
        let r = analyze(&raw(vec![usb], vec![usb_phys]));
        assert!(r.findings.is_empty(), "flashdisk tidak boleh dinilai: {:?}", r.findings);
    }

    #[test]
    fn exe_drive_skipped_even_if_fixed() {
        // Hardisk eksternal USB yang terbaca sebagai "fixed" tetap dilewati bila RepairDoc dijalankan dari situ.
        let r = analyze(&raw(vec![vol('E', 1000, 1.0, MediaKind::Hdd)], vec![]));
        assert!(find(&r, "storage.volume.E").is_none());
    }

    #[test]
    fn hdd_system_drive_is_flagged() {
        let r = analyze(&raw(vec![vol('C', 465, 200.0, MediaKind::Hdd)], vec![disk(&['C'], MediaKind::Hdd, DiskHealth::Healthy)]));
        assert_eq!(find(&r, "storage.system_hdd").unwrap().status, Status::Perhatian);
        assert_eq!(find(&r, "storage.disk0.health").unwrap().title, "Disk utama (C:) dalam kondisi baik");
    }

    #[test]
    fn ssd_system_is_praised() {
        let r = analyze(&raw(vec![vol('C', 238, 100.0, MediaKind::Ssd)], vec![]));
        assert_eq!(find(&r, "storage.system_hdd").unwrap().status, Status::Sehat);
    }

    #[test]
    fn unhealthy_disk_is_serious() {
        let r = analyze(&raw(vec![], vec![disk(&['C'], MediaKind::Hdd, DiskHealth::Unhealthy)]));
        let f = find(&r, "storage.disk0.health").unwrap();
        assert_eq!((f.status, f.severity), (Status::Bermasalah, Some(Severity::Berat)));
        assert!(f.steps[0].contains("salin data"));
    }

    #[test]
    fn ssd_wear_and_heat() {
        let mut d = disk(&['C'], MediaKind::Ssd, DiskHealth::Healthy);
        d.wear = Some(93);
        d.temp_c = Some(74.0);
        let r = analyze(&raw(vec![], vec![d]));
        assert_eq!(find(&r, "storage.disk0.wear").unwrap().status, Status::Bermasalah);
        assert_eq!(find(&r, "storage.disk0.temp").unwrap().status, Status::Perhatian);
        assert!(find(&r, "storage.disk0.health").is_none(), "jangan bilang 'kondisi baik' kalau ada masalah");
    }

    #[test]
    fn unknown_health_makes_no_claim() {
        let r = analyze(&raw(vec![], vec![disk(&['C'], MediaKind::Unknown, DiskHealth::Unknown)]));
        assert!(r.findings.is_empty());
    }
}
