//! Alur "Bersihkan File Sampah": periksa izin → ukur (pratinjau, tidak menghapus) →
//! hapus per jenis (yang gagal dicatat, lanjut ke jenis berikutnya) → catat → ringkasan.
//!
//! TIDAK membuat restore point, dan itu disengaja: System Restore tidak menyimpan dan
//! tidak mengembalikan file sementara, cache, atau isi Recycle Bin. Restore point di sini
//! hanya memberi rasa aman palsu dan membuat pengguna menunggu lama. Pengamannya adalah:
//! pratinjau + centang per jenis + lokasi yang dibatasi (`plan::root_is_sane`) + file
//! terkunci/baru dilewati + symlink tidak diikuti + konfirmasi sekali lagi di tampilan.
//! Jangan "perbaiki" dengan menambah restore point.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use super::exec::{self, RootResult, WindowsOldAnswer};
use super::log;
use super::model::*;
use super::plan;
use crate::preset::{ErrorKind, Failure, MachineInfo, PresetError};
use crate::probe::cleanup::{self, folder_size, JunkKind};

/// Pratinjau boleh mengukur lebih lama dari scan (12 detik), supaya angkanya lengkap.
pub const PREVIEW_BUDGET: Duration = Duration::from_secs(30);
/// Pengukuran Windows.old sebelum & sesudah Disk Cleanup.
const WINDOWS_OLD_BUDGET: Duration = Duration::from_secs(120);

pub trait CleanBackend {
    fn is_windows(&self) -> bool;
    fn is_admin(&self) -> bool;
    fn machine(&self) -> Result<MachineInfo, String>;
    /// Lokasi sampah — SAMA dengan yang diukur scan (`probe::cleanup::locations`).
    fn locations(&self) -> Vec<(Jenis, PathBuf)>;
    /// Nama proses yang sedang jalan (huruf kecil), untuk mengenali browser yang terbuka.
    fn processes(&self) -> Vec<String>;
    /// Hapus Windows.old lewat Disk Cleanup (atau hanya periksa, bila `dry_run`).
    fn windows_old(&self, path: &Path, dry_run: bool) -> Result<WindowsOldAnswer, String>;
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub const ADMIN_HELP: &[&str] = &[
    "Tekan \"Minta izin administrator\", lalu pilih Yes di jendela yang muncul.",
    "Folder C:\\Windows\\Temp dan sisa unduhan Windows Update hanya bisa dibersihkan dengan izin itu.",
];

fn precheck(b: &dyn CleanBackend) -> Result<(), PresetError> {
    if !b.is_windows() {
        return Err(PresetError { kind: ErrorKind::NotWindows, message: "Bersihkan File Sampah hanya bisa dipakai di Windows.".into(), help: vec![] });
    }
    if !b.is_admin() {
        return Err(PresetError {
            kind: ErrorKind::NotAdmin,
            message: "Membersihkan file sampah butuh izin administrator.".into(),
            help: ADMIN_HELP.iter().map(|s| s.to_string()).collect(),
        });
    }
    Ok(())
}

fn machine_or_name(b: &dyn CleanBackend) -> MachineInfo {
    b.machine().unwrap_or_else(|_| {
        let name = std::env::var("COMPUTERNAME").unwrap_or_default();
        MachineInfo { machine_id: format!("nama:{name}"), computer_name: name }
    })
}

/// Status untuk layar: izin, pembersihan terakhir di PC ini. Hanya membaca.
pub fn status(b: &dyn CleanBackend, log_path: &Path) -> CleanStatus {
    let mut st = CleanStatus { is_windows: b.is_windows(), is_admin: false, last: None, log_path: log_path.to_string_lossy().to_string(), error: None };
    if !st.is_windows {
        st.error = Some("Bersihkan File Sampah hanya bisa dipakai di Windows.".into());
        return st;
    }
    st.is_admin = b.is_admin();
    let m = machine_or_name(b);
    st.last = log::load(log_path).last_for(&m.machine_id).cloned();
    st
}

/// Pratinjau: ukur ulang semua lokasi dengan fungsi yang sama dengan scan. Tidak menghapus apa pun.
/// (Hasil scan tidak menyimpan angka per jenis, dan `scan.rs` tidak boleh diubah; mengukur
/// ulang juga membuat angkanya sesuai kondisi sekarang, mis. setelah pembersihan sebelumnya.)
pub fn preview(b: &dyn CleanBackend) -> Result<CleanPreview, PresetError> {
    precheck(b)?;
    let locs: Vec<(JunkKind, PathBuf)> = b.locations().into_iter().map(|(j, p)| (j.kind(), p)).collect();
    let created = locs.iter().find(|(k, _)| *k == JunkKind::WindowsOld).and_then(|(_, p)| exec::created_ms(p));
    let raw = cleanup::measure(locs, Instant::now() + PREVIEW_BUDGET);
    let running = plan::running_browser_names(&b.processes());
    Ok(plan::preview_from(&raw, &running, created))
}

/// Angka dengan pemisah ribuan gaya Indonesia: 1615 → "1.615".
fn ribuan(n: u64) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Bersihkan semua lokasi satu jenis biasa (bukan Windows.old).
fn clean_kind(jenis: Jenis, roots: &[PathBuf], running: &[String], now: SystemTime) -> KindRecord {
    let mut total = RootResult::default();
    let mut notes: Vec<String> = Vec::new();
    let mut browsers_skipped: Vec<&str> = Vec::new();
    let mut refused = 0usize;
    for root in roots {
        let text = root.to_string_lossy().to_string();
        if let Err(e) = plan::root_is_sane(jenis, &text) {
            notes.push(format!("Dilewati, lokasi tidak dikenali: {e}"));
            refused += 1;
            continue;
        }
        if exec::root_is_link(root) {
            notes.push(format!("Dilewati, {text} berupa tautan (symlink/junction) ke tempat lain."));
            refused += 1;
            continue;
        }
        if let Some((name, _)) = plan::browser_for(&text).filter(|(n, _)| running.iter().any(|r| r == n)) {
            // Browser yang terbuka memakai cache-nya: lewati seluruh foldernya, hitung isinya.
            let s = folder_size(root, Instant::now() + Duration::from_secs(10));
            total.skipped_other += s.files;
            if !browsers_skipped.contains(&name) {
                browsers_skipped.push(name);
            }
            continue;
        }
        let r = exec::clean_root(jenis, root, now, false);
        total.deleted_files += r.deleted_files;
        total.freed_bytes += r.freed_bytes;
        total.skipped_in_use += r.skipped_in_use;
        total.skipped_denied += r.skipped_denied;
        total.skipped_new += r.skipped_new;
        total.skipped_other += r.skipped_other;
        total.unreadable_dirs += r.unreadable_dirs;
    }
    if !browsers_skipped.is_empty() {
        notes.push(format!("Cache {} dilewati karena browsernya sedang terbuka.", browsers_skipped.join(", ")));
    }
    if total.skipped_in_use > 0 {
        notes.push(format!("{} file dilewati karena sedang dipakai.", ribuan(total.skipped_in_use)));
    }
    if total.skipped_new > 0 {
        notes.push(format!("{} file dilewati karena masih baru (24 jam terakhir).", ribuan(total.skipped_new)));
    }
    if total.skipped_denied > 0 {
        notes.push(format!("{} file dilewati karena tidak ada izin.", ribuan(total.skipped_denied)));
    }
    if total.unreadable_dirs > 0 {
        notes.push(format!("{} folder tidak bisa dibuka.", ribuan(total.unreadable_dirs)));
    }
    let skipped = total.skipped();
    let status = if roots.is_empty() || (refused == roots.len() && total.deleted_files == 0) {
        if roots.is_empty() {
            notes.push("Lokasi ini tidak ada di PC ini.".into());
            KindStatus::Berhasil
        } else {
            KindStatus::Gagal
        }
    } else if total.deleted_files == 0 && (total.unreadable_dirs > 0 || total.skipped_denied > 0) && skipped == total.skipped_denied {
        KindStatus::Gagal
    } else if skipped > 0 || total.unreadable_dirs > 0 || refused > 0 {
        KindStatus::Sebagian
    } else {
        KindStatus::Berhasil
    };
    KindRecord {
        jenis,
        label: jenis.label().to_string(),
        lokasi: roots.iter().map(|p| p.to_string_lossy().to_string()).collect(),
        file_terhapus: total.deleted_files,
        byte_dibebaskan: total.freed_bytes,
        dilewati: skipped,
        status,
        catatan: notes,
    }
}

/// Windows.old lewat Disk Cleanup. Ukuran yang dibebaskan = sebelum − sesudah (diukur).
fn clean_windows_old(b: &dyn CleanBackend, root: Option<&PathBuf>) -> KindRecord {
    let mut rec = KindRecord {
        jenis: Jenis::WindowsOld,
        label: Jenis::WindowsOld.label().to_string(),
        lokasi: root.map(|p| vec![p.to_string_lossy().to_string()]).unwrap_or_default(),
        file_terhapus: 0,
        byte_dibebaskan: 0,
        dilewati: 0,
        status: KindStatus::Gagal,
        catatan: Vec::new(),
    };
    let Some(root) = root else {
        rec.status = KindStatus::Berhasil;
        rec.catatan.push("Windows.old tidak ada di PC ini.".into());
        return rec;
    };
    if let Err(e) = plan::root_is_sane(Jenis::WindowsOld, &root.to_string_lossy()) {
        rec.catatan.push(format!("Dilewati: {e}"));
        return rec;
    }
    let before = folder_size(root, Instant::now() + WINDOWS_OLD_BUDGET);
    let answer = b.windows_old(root, false);
    let after = if root.exists() { folder_size(root, Instant::now() + WINDOWS_OLD_BUDGET) } else { Default::default() };
    rec.byte_dibebaskan = before.bytes.saturating_sub(after.bytes);
    rec.file_terhapus = before.files.saturating_sub(after.files);
    rec.dilewati = after.files;
    if before.truncated || after.truncated {
        rec.catatan.push("Folder ini sangat besar; angka di atas perkiraan dari penghitungan yang dihentikan karena waktu.".into());
    }
    match answer {
        Ok(a) if a.ok && !root.exists() => rec.status = KindStatus::Berhasil,
        Ok(a) => {
            rec.status = if rec.file_terhapus > 0 { KindStatus::Sebagian } else { KindStatus::Gagal };
            rec.catatan.push(a.error.unwrap_or_else(|| "Sebagian Windows.old masih ada.".into()));
        }
        Err(e) => {
            rec.status = if rec.file_terhapus > 0 { KindStatus::Sebagian } else { KindStatus::Gagal };
            rec.catatan.push(format!("Disk Cleanup tidak bisa dijalankan: {e}"));
        }
    }
    rec
}

/// Bersihkan jenis yang dipilih, satu per satu.
pub fn apply(b: &dyn CleanBackend, log_path: &Path, req: &CleanRequest, now_ms: u64, on_event: &mut dyn FnMut(CleanEvent)) -> CleanOutcome {
    if let Err(e) = precheck(b) {
        return CleanOutcome::error(e);
    }
    let chosen = plan::chosen(req);
    if chosen.is_empty() {
        return CleanOutcome::error(PresetError { kind: ErrorKind::Other, message: "Belum ada jenis sampah yang dicentang.".into(), help: vec![] });
    }
    on_event(CleanEvent::Preparing);
    let machine = machine_or_name(b);
    let locs = b.locations();
    let running = plan::running_browser_names(&b.processes());
    let now = b.now();

    let mut records = Vec::new();
    for (i, jenis) in chosen.iter().enumerate() {
        on_event(CleanEvent::Cleaning { jenis: *jenis, label: jenis.label().to_string(), index: i, total: chosen.len() });
        let roots: Vec<PathBuf> = locs.iter().filter(|(j, _)| j == jenis).map(|(_, p)| p.clone()).collect();
        let rec = if *jenis == Jenis::WindowsOld { clean_windows_old(b, roots.first()) } else { clean_kind(*jenis, &roots, &running, now) };
        records.push(rec);
    }

    on_event(CleanEvent::Saving);
    let mut log = log::load(log_path);
    let mut id = format!("{now_ms}-bersihkan");
    while log.entri.iter().any(|e| e.id == id) {
        id.push('x');
    }
    let entry = CleanEntry { id, mesin: machine.machine_id, nama_pc: machine.computer_name, waktu: now_ms, jenis: records };
    log.entri.push(entry.clone());
    let saved = log::save(log_path, &log);

    let failures: Vec<Failure> = entry
        .jenis
        .iter()
        .filter(|k| k.status == KindStatus::Gagal)
        .map(|k| Failure { label: k.label.clone(), reason: k.catatan.first().cloned().unwrap_or_else(|| "Tidak ada yang bisa dihapus.".into()) })
        .collect();
    let mut out = CleanOutcome {
        ok: true,
        error: None,
        freed_bytes: entry.freed(),
        deleted_files: entry.deleted(),
        skipped_files: entry.skipped(),
        failures,
        entry: Some(entry),
    };
    if let Err(e) = saved {
        out.error = Some(PresetError { kind: ErrorKind::Other, message: format!("File sudah dibersihkan, tapi catatannya tidak bisa disimpan: {e}"), help: vec![] });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Backend uji: lokasi sampah di folder UJI sementara (nama unik per tes, terhapus
    /// otomatis) — TIDAK PERNAH folder sistem asli. Penghapusannya sungguhan.
    struct DirBackend {
        base: PathBuf,
        admin: bool,
        windows: bool,
        locs: Vec<(Jenis, PathBuf)>,
        processes: Vec<String>,
        windows_old_calls: RefCell<Vec<(PathBuf, bool)>>,
    }

    impl Drop for DirBackend {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    impl CleanBackend for DirBackend {
        fn is_windows(&self) -> bool {
            self.windows
        }
        fn is_admin(&self) -> bool {
            self.admin
        }
        fn machine(&self) -> Result<MachineInfo, String> {
            Ok(MachineInfo { machine_id: "PC-UJI".into(), computer_name: "LAPTOP-BUDI".into() })
        }
        fn locations(&self) -> Vec<(Jenis, PathBuf)> {
            self.locs.clone()
        }
        fn processes(&self) -> Vec<String> {
            self.processes.clone()
        }
        fn windows_old(&self, path: &Path, dry_run: bool) -> Result<WindowsOldAnswer, String> {
            self.windows_old_calls.borrow_mut().push((path.to_path_buf(), dry_run));
            Ok(WindowsOldAnswer { ok: false, still_exists: true, error: Some("tiruan".into()) })
        }
        fn now(&self) -> SystemTime {
            SystemTime::now() + Duration::from_secs(3 * 24 * 3600)
        }
    }

    fn write(p: &Path, len: usize) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, vec![1u8; len]).unwrap();
    }

    fn pc(test: &str) -> DirBackend {
        static N: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir().join(format!("repairdoc-cleaner-flow-{test}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&base);
        let temp = base.join("Users/Budi/AppData/Local/Temp");
        let wtemp = base.join("Windows/Temp");
        let chrome = base.join("Users/Budi/AppData/Local/Google/Chrome/User Data/Default/Cache");
        let edge = base.join("Users/Budi/AppData/Local/Microsoft/Edge/User Data/Default/Cache");
        let upd = base.join("Windows/SoftwareDistribution/Download");
        let rb = base.join("$Recycle.Bin");
        write(&temp.join("a.tmp"), 1000);
        write(&temp.join("setup").join("b.tmp"), 2000);
        write(&wtemp.join("c.log"), 500);
        write(&chrome.join("data_0"), 4000);
        write(&edge.join("data_0"), 3000);
        write(&upd.join("abc").join("update.cab"), 8000);
        write(&rb.join("S-1-5-21-1").join("$RAB.docx"), 600);
        write(&rb.join("S-1-5-21-1").join("$IAB.docx"), 40);
        write(&rb.join("S-1-5-21-1").join("desktop.ini"), 10);
        let locs = vec![(Jenis::Temp, temp), (Jenis::Temp, wtemp), (Jenis::UpdateCache, upd), (Jenis::BrowserCache, chrome), (Jenis::BrowserCache, edge), (Jenis::RecycleBin, rb)];
        DirBackend { base, admin: true, windows: true, locs, processes: vec!["explorer.exe".into()], windows_old_calls: RefCell::new(Vec::new()) }
    }

    fn log_path(b: &DirBackend) -> PathBuf {
        b.base.join("RepairDoc-data").join(log::LOG_FILE)
    }

    fn all_but_recycle() -> CleanRequest {
        CleanRequest { kinds: vec![Jenis::Temp, Jenis::BrowserCache, Jenis::UpdateCache], windows_old: false }
    }

    #[test]
    fn preview_measures_and_deletes_nothing() {
        let b = pc("pratinjau");
        let p = preview(&b).unwrap();
        let k = |j: Jenis| p.kinds.iter().find(|k| k.jenis == j).unwrap();
        assert_eq!((k(Jenis::Temp).bytes, k(Jenis::Temp).files, k(Jenis::Temp).locations.len()), (3500, 3, 2));
        assert_eq!(k(Jenis::BrowserCache).bytes, 7000);
        assert_eq!(k(Jenis::RecycleBin).bytes, 650, "pengukuran = fungsi scan (desktop.ini ikut terhitung)");
        assert!(!k(Jenis::RecycleBin).selected_by_default);
        assert!(b.locs.iter().all(|(_, p)| std::fs::read_dir(p).unwrap().count() > 0), "pratinjau tidak menghapus apa pun");
    }

    #[test]
    fn cleans_selected_kinds_reports_real_bytes_and_logs() {
        let b = pc("bersihkan");
        let mut steps = Vec::new();
        let o = apply(&b, &log_path(&b), &all_but_recycle(), 1_000, &mut |e| steps.push(e));
        assert!(o.ok && o.error.is_none(), "{:?}", o.error);
        assert_eq!(o.freed_bytes, 3500 + 7000 + 8000, "ukuran nyata yang terhapus");
        assert_eq!(o.deleted_files, 3 + 2 + 1);
        assert_eq!(o.skipped_files, 0);
        assert!(b.locs.iter().all(|(_, p)| p.is_dir()), "folder lokasinya sendiri tetap ada");
        let rb = &b.locs[5].1;
        assert!(rb.join("S-1-5-21-1").join("$RAB.docx").exists(), "Recycle Bin tidak dicentang → tidak disentuh");
        assert!(matches!(steps.first(), Some(CleanEvent::Preparing)));
        assert_eq!(steps.iter().filter(|e| matches!(e, CleanEvent::Cleaning { .. })).count(), 3);

        let log = log::load(&log_path(&b));
        let e = &log.entri[0];
        assert_eq!(e.mesin, "PC-UJI");
        let t = e.jenis.iter().find(|k| k.jenis == Jenis::Temp).unwrap();
        assert_eq!((t.file_terhapus, t.byte_dibebaskan, t.dilewati, t.status, t.lokasi.len()), (3, 3500, 0, KindStatus::Berhasil, 2));
        // Nama kolom sesuai spesifikasi.
        let json = std::fs::read_to_string(log_path(&b)).unwrap();
        for col in ["\"jenis\"", "\"lokasi\"", "\"fileTerhapus\"", "\"byteDibebaskan\"", "\"dilewati\"", "\"status\""] {
            assert!(json.contains(col), "{col}");
        }
        assert_eq!(status(&b, &log_path(&b)).last.unwrap().freed(), o.freed_bytes);
    }

    #[test]
    fn recycle_bin_only_when_ticked() {
        let b = pc("recycle");
        let o = apply(&b, &log_path(&b), &CleanRequest { kinds: vec![Jenis::RecycleBin], windows_old: false }, 1, &mut |_| {});
        assert_eq!(o.freed_bytes, 640);
        let sid = b.locs[5].1.join("S-1-5-21-1");
        assert!(sid.join("desktop.ini").exists());
        assert!(b.locs[0].1.join("a.tmp").exists(), "jenis lain tidak disentuh");
    }

    #[test]
    fn open_browser_cache_is_skipped_and_counted() {
        let mut b = pc("browser");
        b.processes = vec!["chrome.exe".into()];
        let o = apply(&b, &log_path(&b), &CleanRequest { kinds: vec![Jenis::BrowserCache], windows_old: false }, 1, &mut |_| {});
        let k = &o.entry.as_ref().unwrap().jenis[0];
        assert_eq!((k.byte_dibebaskan, k.dilewati, k.status), (3000, 1, KindStatus::Sebagian), "{k:?}");
        assert!(k.catatan.iter().any(|c| c.contains("Chrome") && c.contains("terbuka")));
        assert!(b.locs[3].1.join("data_0").exists(), "cache Chrome yang sedang dipakai tidak disentuh");
    }

    #[test]
    fn windows_old_needs_its_own_tick() {
        let mut b = pc("winold");
        let wo = b.base.join("C:").join("Windows.old");
        write(&wo.join("Windows").join("x.dll"), 100);
        b.locs.push((Jenis::WindowsOld, wo.clone()));
        // Ikut di daftar biasa saja → tidak disentuh sama sekali.
        let o = apply(&b, &log_path(&b), &CleanRequest { kinds: vec![Jenis::WindowsOld, Jenis::Temp], windows_old: false }, 1, &mut |_| {});
        assert!(!o.entry.unwrap().jenis.iter().any(|k| k.jenis == Jenis::WindowsOld));
        assert!(b.windows_old_calls.borrow().is_empty());
        assert!(wo.join("Windows").join("x.dll").exists());
    }

    #[test]
    fn unknown_locations_are_refused_and_failures_reported() {
        let mut b = pc("aneh");
        // %TEMP% yang salah atur menunjuk ke folder pengguna → harus ditolak, isinya utuh.
        let home = b.base.join("Users/Budi/Dokumen");
        write(&home.join("skripsi.docx"), 100);
        b.locs = vec![(Jenis::Temp, home.clone()), (Jenis::UpdateCache, b.base.join("Windows/SoftwareDistribution/Download"))];
        let o = apply(&b, &log_path(&b), &all_but_recycle(), 1, &mut |_| {});
        assert!(home.join("skripsi.docx").exists());
        let e = o.entry.unwrap();
        let t = e.jenis.iter().find(|k| k.jenis == Jenis::Temp).unwrap();
        assert_eq!(t.status, KindStatus::Gagal);
        assert_eq!(o.failures.len(), 1, "gagal dilaporkan, jenis lain tetap jalan");
        assert_eq!(e.jenis.iter().find(|k| k.jenis == Jenis::UpdateCache).unwrap().byte_dibebaskan, 8000);
    }

    #[test]
    fn not_admin_or_not_windows_deletes_nothing() {
        let mut b = pc("bukan-admin");
        b.admin = false;
        assert_eq!(preview(&b).unwrap_err().kind, ErrorKind::NotAdmin);
        let o = apply(&b, &log_path(&b), &all_but_recycle(), 1, &mut |_| {});
        assert_eq!(o.error.unwrap().kind, ErrorKind::NotAdmin);
        b.admin = true;
        b.windows = false;
        assert_eq!(apply(&b, &log_path(&b), &all_but_recycle(), 1, &mut |_| {}).error.unwrap().kind, ErrorKind::NotWindows);
        assert!(b.locs[0].1.join("a.tmp").exists());
        assert!(!log_path(&b).exists(), "tidak ada catatan kalau tidak ada yang dilakukan");
    }

    #[test]
    fn numbers_use_indonesian_thousands() {
        assert_eq!((ribuan(7), ribuan(1615), ribuan(104_522), ribuan(1_000_000)), ("7".into(), "1.615".into(), "104.522".into(), "1.000.000".into()));
    }

    #[test]
    fn nothing_ticked_is_an_error_not_a_silent_success() {
        let b = pc("kosong");
        let o = apply(&b, &log_path(&b), &CleanRequest::default(), 1, &mut |_| {});
        assert!(!o.ok);
        assert!(o.error.unwrap().message.contains("dicentang"));
    }
}
