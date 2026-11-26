//! Aturan pembersihan sebagai logika murni — tidak menyentuh sistem, bisa dites di mana saja.
//!
//! Prinsip: hanya lokasi dari `probe::cleanup::locations()` yang dibersihkan, hanya
//! ISINYA (folder lokasinya sendiri dibiarkan), dan lokasi yang bentuknya tidak dikenali
//! ditolak. Tidak ada penghapusan berdasarkan tebakan jalur.

use std::time::{Duration, SystemTime};

use super::model::*;
use crate::probe::cleanup::{CleanupRaw, FolderSize};

/// File sementara yang dibuat/diubah dalam waktu ini dilewati: mungkin masih dipakai
/// installer atau update yang menunggu restart.
pub const TEMP_MIN_AGE: Duration = Duration::from_secs(24 * 60 * 60);

pub const NOTE_TEMP_NEW: &str = "File yang dibuat atau diubah 24 jam terakhir dilewati, karena mungkin masih dipakai program atau installer.";
pub const NOTE_IN_USE: &str = "File yang sedang dipakai program dilewati — itu normal, bukan galat.";
pub const NOTE_UPDATE: &str = "File yang sedang dipakai Windows Update dilewati.";
pub const NOTE_RECYCLE: &str = "Isi Recycle Bin terhapus permanen. Periksa dulu kalau ada file yang masih ingin dikembalikan.";
pub const WINDOWS_OLD_WARNING: &str = "Windows.old dibutuhkan untuk kembali ke Windows lama dalam 10 hari setelah upgrade; menghapusnya tidak bisa dibatalkan.";

/// Recycle Bin dan Windows.old TIDAK tercentang saat layar dibuka.
pub fn selected_by_default(j: Jenis) -> bool {
    matches!(j, Jenis::Temp | Jenis::BrowserCache | Jenis::UpdateCache)
}

/// Jenis yang benar-benar akan dihapus, dalam urutan penghapusan.
/// Windows.old HANYA lewat centang konfirmasinya sendiri (`windows_old`), walaupun
/// ada di `kinds`.
pub fn chosen(req: &CleanRequest) -> Vec<Jenis> {
    let mut out: Vec<Jenis> = Jenis::NORMAL.iter().copied().filter(|j| req.kinds.contains(j)).collect();
    if req.windows_old {
        out.push(Jenis::WindowsOld);
    }
    out
}

/// Potong jalur menjadi bagian-bagian (\ dan / sama saja), tanpa bagian kosong.
fn parts(path: &str) -> Vec<String> {
    path.split(['\\', '/']).filter(|c| !c.is_empty()).map(|c| c.to_string()).collect()
}

/// Lokasi ini boleh dibersihkan untuk jenis ini? Pengaman terakhir kalau variabel
/// lingkungan (mis. %TEMP%) menunjuk ke tempat yang aneh seperti `C:\` atau folder pengguna.
pub fn root_is_sane(jenis: Jenis, path: &str) -> Result<(), String> {
    let p = parts(path);
    let lower: Vec<String> = p.iter().map(|c| c.to_ascii_lowercase()).collect();
    if lower.iter().any(|c| c == "." || c == "..") {
        return Err(format!("{path}: jalur mengandung '.' atau '..'"));
    }
    if p.len() < 2 {
        return Err(format!("{path}: terlalu dekat ke akar drive"));
    }
    let last = lower.last().map(String::as_str).unwrap_or("");
    let ok = match jenis {
        // ...\Temp, atau ...\Temp\<angka> (folder sementara per sesi Remote Desktop).
        Jenis::Temp => {
            let is_temp = |c: &str| c == "temp" || c == "tmp";
            is_temp(last) || (lower.len() >= 3 && is_temp(&lower[lower.len() - 2]) && last.chars().all(|c| c.is_ascii_digit()))
        }
        Jenis::BrowserCache => ["cache", "code cache", "gpucache", "cache2", "inetcache"].contains(&last),
        Jenis::UpdateCache => lower.len() >= 2 && lower[lower.len() - 2] == "softwaredistribution" && last == "download",
        Jenis::RecycleBin => last == "$recycle.bin",
        Jenis::WindowsOld => p.len() == 2 && p[0].ends_with(':') && last == "windows.old",
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: bukan lokasi {} yang dikenal", jenis.label().to_lowercase()))
    }
}

/// Browser pemilik sebuah folder cache: (nama, nama proses).
pub fn browser_for(path: &str) -> Option<(&'static str, &'static str)> {
    let p = format!("\\{}\\", parts(path).join("\\").to_ascii_lowercase());
    const LIST: &[(&str, &str, &str)] = &[
        ("\\google\\chrome\\user data\\", "Chrome", "chrome.exe"),
        ("\\microsoft\\edge\\user data\\", "Edge", "msedge.exe"),
        ("\\bravesoftware\\brave-browser\\", "Brave", "brave.exe"),
        ("\\opera software\\", "Opera", "opera.exe"),
        ("\\vivaldi\\user data\\", "Vivaldi", "vivaldi.exe"),
        ("\\mozilla\\firefox\\profiles\\", "Firefox", "firefox.exe"),
    ];
    LIST.iter().find(|(m, _, _)| p.contains(m)).map(|(_, n, e)| (*n, *e))
}

/// Nama browser yang sedang terbuka (dari daftar proses, huruf kecil).
pub fn running_browser_names(processes: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (exe, name) in [("chrome.exe", "Chrome"), ("msedge.exe", "Edge"), ("brave.exe", "Brave"), ("opera.exe", "Opera"), ("vivaldi.exe", "Vivaldi"), ("firefox.exe", "Firefox")] {
        if processes.iter().any(|p| p.eq_ignore_ascii_case(exe)) && !out.iter().any(|o| o == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// Apa yang dilakukan pada satu isi folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryRule {
    /// Hapus (file), atau masuki lalu hapus isinya dan foldernya (folder).
    Delete,
    /// Masuki folder, tapi foldernya sendiri dibiarkan.
    Descend,
    /// Biarkan, tidak dihitung.
    Keep,
}

/// Aturan untuk isi folder pada kedalaman `depth` (0 = langsung di dalam lokasi).
/// Recycle Bin: di dalamnya ada folder per pengguna (SID) yang dibiarkan; yang dihapus
/// hanya isinya yang bernama `$R…` (file yang dibuang) dan `$I…` (keterangannya).
/// File lain seperti `desktop.ini` dibiarkan.
pub fn rule_for(jenis: Jenis, depth: usize, name: &str, is_dir: bool) -> EntryRule {
    match jenis {
        Jenis::RecycleBin => match depth {
            0 if is_dir => EntryRule::Descend,
            0 => EntryRule::Keep,
            1 => {
                let n = name.to_ascii_uppercase();
                if n.starts_with("$R") || n.starts_with("$I") {
                    EntryRule::Delete
                } else {
                    EntryRule::Keep
                }
            }
            _ => EntryRule::Delete,
        },
        _ => EntryRule::Delete,
    }
}

/// File sementara yang masih baru (atau waktunya tak terbaca) dilewati.
pub fn too_new(jenis: Jenis, modified: Option<SystemTime>, now: SystemTime) -> bool {
    if jenis != Jenis::Temp {
        return false;
    }
    match modified {
        Some(m) => now.duration_since(m).map(|age| age < TEMP_MIN_AGE).unwrap_or(true),
        None => true,
    }
}

fn add(t: &mut FolderSize, s: &FolderSize) {
    t.bytes += s.bytes;
    t.files += s.files;
    t.denied += s.denied;
    t.truncated |= s.truncated;
}

/// Susun pratinjau dari hasil pengukuran (`probe::cleanup::measure`, fungsi yang sama
/// dengan scan). `windows_old_created` = waktu folder Windows.old dibuat, bila terbaca.
pub fn preview_from(raw: &CleanupRaw, running: &[String], windows_old_created: Option<u64>) -> CleanPreview {
    let mut kinds = Vec::new();
    for j in Jenis::NORMAL {
        let entries: Vec<_> = raw.entries.iter().filter(|e| Jenis::from_kind(e.kind) == j).collect();
        let mut total = FolderSize::default();
        for e in &entries {
            add(&mut total, &e.size);
        }
        let mut notes = Vec::new();
        match j {
            Jenis::Temp => notes.extend([NOTE_TEMP_NEW.to_string(), NOTE_IN_USE.to_string()]),
            Jenis::BrowserCache => {
                let open: Vec<&str> = entries
                    .iter()
                    .filter_map(|e| browser_for(&e.path.to_string_lossy()).map(|(n, _)| n))
                    .filter(|n| running.iter().any(|r| r == n))
                    .fold(Vec::new(), |mut v, n| {
                        if !v.contains(&n) {
                            v.push(n);
                        }
                        v
                    });
                if !open.is_empty() {
                    notes.push(format!("{} sedang terbuka, jadi cache-nya dilewati. Tutup dulu browsernya kalau ingin ikut dibersihkan.", open.join(", ")));
                }
                notes.push(NOTE_IN_USE.to_string());
            }
            Jenis::UpdateCache => notes.push(NOTE_UPDATE.to_string()),
            Jenis::RecycleBin => notes.push(NOTE_RECYCLE.to_string()),
            Jenis::WindowsOld => {}
        }
        kinds.push(KindPreview {
            jenis: j,
            label: j.label().to_string(),
            description: j.description().to_string(),
            bytes: total.bytes,
            files: total.files,
            approximate: total.truncated || total.denied > 0,
            locations: entries.iter().map(|e| e.path.to_string_lossy().to_string()).collect(),
            selected_by_default: selected_by_default(j),
            notes,
        });
    }

    let windows_old = raw.entries.iter().find(|e| Jenis::from_kind(e.kind) == Jenis::WindowsOld).map(|e| WindowsOldPreview {
        bytes: e.size.bytes,
        files: e.size.files,
        approximate: e.size.truncated || e.size.denied > 0,
        location: e.path.to_string_lossy().to_string(),
        created_ms: windows_old_created,
        warning: WINDOWS_OLD_WARNING.to_string(),
    });

    let mut notes = vec!["File yang dihapus tidak bisa dikembalikan. Catatan pembersihan hanya untuk riwayat, bukan tombol undo.".to_string()];
    if raw.hiberfil.is_some() || raw.pagefile.is_some() {
        notes.push("hiberfil.sys dan pagefile.sys adalah file sistem Windows, bukan sampah — hanya ditampilkan, tidak pernah dihapus.".into());
    }
    notes.extend(raw.notes.iter().cloned());

    CleanPreview { kinds, windows_old, hiberfil: raw.hiberfil, pagefile: raw.pagefile, running_browsers: running.to_vec(), notes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::cleanup::{JunkEntry, JunkKind};
    use std::path::PathBuf;

    #[test]
    fn windows_old_is_never_in_the_normal_list() {
        assert!(!Jenis::NORMAL.contains(&Jenis::WindowsOld));
        assert!(!selected_by_default(Jenis::WindowsOld));
        assert!(!selected_by_default(Jenis::RecycleBin), "Recycle Bin harus dicentang sendiri");
        // Walau ikut dikirim di daftar biasa, Windows.old tetap tidak dihapus tanpa centang terpisah.
        let sneaky = CleanRequest { kinds: vec![Jenis::Temp, Jenis::WindowsOld], windows_old: false };
        assert_eq!(chosen(&sneaky), vec![Jenis::Temp]);
        let explicit = CleanRequest { kinds: vec![], windows_old: true };
        assert_eq!(chosen(&explicit), vec![Jenis::WindowsOld]);
        let all = CleanRequest { kinds: vec![Jenis::RecycleBin, Jenis::UpdateCache, Jenis::Temp, Jenis::BrowserCache], windows_old: false };
        assert_eq!(chosen(&all), Jenis::NORMAL.to_vec(), "urutan tetap");
    }

    #[test]
    fn safe_total_still_excludes_windows_old() {
        let raw = CleanupRaw {
            entries: vec![
                JunkEntry { kind: JunkKind::Temp, path: PathBuf::from("C:/T/Temp"), size: FolderSize { bytes: 100, files: 1, ..Default::default() } },
                JunkEntry { kind: JunkKind::WindowsOld, path: PathBuf::from("C:/Windows.old"), size: FolderSize { bytes: 9_000, files: 9, ..Default::default() } },
            ],
            ..Default::default()
        };
        assert_eq!(raw.safe_total().bytes, 100);
        let p = preview_from(&raw, &[], Some(5));
        assert!(!p.kinds.iter().any(|k| k.jenis == Jenis::WindowsOld), "Windows.old tampil terpisah");
        let w = p.windows_old.unwrap();
        assert_eq!((w.bytes, w.created_ms), (9_000, Some(5)));
        assert!(w.warning.contains("10 hari") && w.warning.contains("tidak bisa dibatalkan"));
    }

    #[test]
    fn roots_must_look_like_known_junk_folders() {
        for (j, p) in [
            (Jenis::Temp, r"C:\Users\Budi\AppData\Local\Temp"),
            (Jenis::Temp, r"C:\Users\BUDI~1\AppData\Local\Temp\2"),
            (Jenis::Temp, r"C:\Windows\Temp"),
            (Jenis::BrowserCache, r"C:\Users\Budi\AppData\Local\Google\Chrome\User Data\Default\Cache"),
            (Jenis::BrowserCache, r"C:\Users\Budi\AppData\Local\Microsoft\Windows\INetCache"),
            (Jenis::BrowserCache, r"C:\Users\Budi\AppData\Local\Mozilla\Firefox\Profiles\ab.default\cache2"),
            (Jenis::UpdateCache, r"C:\Windows\SoftwareDistribution\Download"),
            (Jenis::RecycleBin, r"D:\$Recycle.Bin"),
            (Jenis::WindowsOld, r"C:\Windows.old"),
        ] {
            assert!(root_is_sane(j, p).is_ok(), "{p}");
        }
        for (j, p) in [
            (Jenis::Temp, r"C:\"),
            (Jenis::Temp, r"C:\Users\Budi"),
            (Jenis::Temp, r"C:\Windows"),
            (Jenis::Temp, r"C:\Users\Budi\AppData\Local\Temp\..\.."),
            (Jenis::Temp, r"C:\Temp\Dokumen"),
            (Jenis::BrowserCache, r"C:\Users\Budi\AppData\Local\Google\Chrome\User Data"),
            (Jenis::UpdateCache, r"C:\Windows\SoftwareDistribution"),
            (Jenis::RecycleBin, r"C:\Users"),
            (Jenis::WindowsOld, r"C:\Users\Budi\Windows.old"),
            (Jenis::WindowsOld, r"C:\Windows"),
        ] {
            assert!(root_is_sane(j, p).is_err(), "{p} harus ditolak");
        }
    }

    #[test]
    fn recycle_bin_keeps_user_folders_and_desktop_ini() {
        assert_eq!(rule_for(Jenis::RecycleBin, 0, "S-1-5-21-1", true), EntryRule::Descend);
        assert_eq!(rule_for(Jenis::RecycleBin, 0, "desktop.ini", false), EntryRule::Keep);
        assert_eq!(rule_for(Jenis::RecycleBin, 1, "$RAB12CD.docx", false), EntryRule::Delete);
        assert_eq!(rule_for(Jenis::RecycleBin, 1, "$IAB12CD.docx", false), EntryRule::Delete);
        assert_eq!(rule_for(Jenis::RecycleBin, 1, "$R0QX1Z1", true), EntryRule::Delete);
        assert_eq!(rule_for(Jenis::RecycleBin, 1, "desktop.ini", false), EntryRule::Keep);
        assert_eq!(rule_for(Jenis::RecycleBin, 2, "isi.txt", false), EntryRule::Delete);
        assert_eq!(rule_for(Jenis::Temp, 0, "x.tmp", false), EntryRule::Delete);
    }

    #[test]
    fn new_temp_files_are_skipped() {
        let now = SystemTime::now();
        let hour = Duration::from_secs(3600);
        assert!(too_new(Jenis::Temp, Some(now - hour), now));
        assert!(!too_new(Jenis::Temp, Some(now - 25 * hour), now));
        assert!(too_new(Jenis::Temp, None, now), "waktu tak terbaca → anggap baru");
        assert!(too_new(Jenis::Temp, Some(now + hour), now), "jam ngaco → anggap baru");
        assert!(!too_new(Jenis::BrowserCache, Some(now), now), "aturan umur hanya untuk file sementara");
    }

    #[test]
    fn browsers_are_recognised_from_cache_paths() {
        assert_eq!(browser_for(r"C:\Users\b\AppData\Local\Google\Chrome\User Data\Profile 1\Code Cache"), Some(("Chrome", "chrome.exe")));
        assert_eq!(browser_for(r"C:\Users\b\AppData\Local\Microsoft\Edge\User Data\Default\Cache"), Some(("Edge", "msedge.exe")));
        assert_eq!(browser_for(r"C:\Users\b\AppData\Local\Mozilla\Firefox\Profiles\x.default\cache2"), Some(("Firefox", "firefox.exe")));
        assert_eq!(browser_for(r"C:\Users\b\AppData\Local\Microsoft\Windows\INetCache"), None);
        let running = running_browser_names(&["explorer.exe".into(), "CHROME.EXE".into(), "chrome.exe".into(), "msedge.exe".into()]);
        assert_eq!(running, vec!["Chrome", "Edge"]);
    }

    #[test]
    fn preview_lists_every_kind_with_locations_and_notes() {
        let raw = CleanupRaw {
            entries: vec![
                JunkEntry { kind: JunkKind::Temp, path: PathBuf::from(r"C:\Users\b\AppData\Local\Temp"), size: FolderSize { bytes: 10, files: 2, ..Default::default() } },
                JunkEntry { kind: JunkKind::Temp, path: PathBuf::from(r"C:\Windows\Temp"), size: FolderSize { bytes: 5, files: 1, denied: 1, ..Default::default() } },
                JunkEntry {
                    kind: JunkKind::BrowserCache,
                    path: PathBuf::from(r"C:\Users\b\AppData\Local\Google\Chrome\User Data\Default\Cache"),
                    size: FolderSize { bytes: 7, files: 3, ..Default::default() },
                },
            ],
            hiberfil: Some(3_000),
            ..Default::default()
        };
        let p = preview_from(&raw, &["Chrome".to_string()], None);
        assert_eq!(p.kinds.iter().map(|k| k.jenis).collect::<Vec<_>>(), Jenis::NORMAL.to_vec());
        let t = &p.kinds[0];
        assert_eq!((t.bytes, t.files, t.approximate, t.locations.len()), (15, 3, true, 2));
        let b = &p.kinds[1];
        assert!(b.notes[0].starts_with("Chrome sedang terbuka"));
        assert!(!p.kinds[3].selected_by_default, "Recycle Bin tidak tercentang");
        assert!(p.notes[0].contains("tidak bisa dikembalikan"));
        assert!(p.notes.iter().any(|n| n.contains("hiberfil.sys")));
        assert!(p.windows_old.is_none());
    }
}
