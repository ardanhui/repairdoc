//! Penghapus file sampah. Dipisah dari `probe::cleanup` (yang hanya mengukur), tapi
//! memakai aturan jalan yang sama: symlink/junction TIDAK PERNAH diikuti.
//!
//! Penghapusan file dilakukan oleh Rust sendiri (bukan PowerShell) supaya aturannya
//! sama persis dengan pengukuran dan bisa dites di mana saja. Hanya Windows.old yang
//! lewat skrip (`scripts/cleanup_windows_old.ps1`), karena memakai Disk Cleanup bawaan
//! Windows yang mengurus izin file sistem di dalamnya.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::engine::CleanBackend;
use super::model::*;
use super::plan::{self, EntryRule};
use crate::platform;
use crate::preset::{Backend, MachineInfo};
use crate::probe::json;

const WINDOWS_OLD_SCRIPT: &str = include_str!("../scripts/cleanup_windows_old.ps1");
/// Disk Cleanup untuk Windows.old di HDD lambat bisa lama.
const WINDOWS_OLD_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// Hasil membersihkan satu lokasi.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RootResult {
    pub deleted_files: u64,
    /// Ukuran file yang BENAR-BENAR terhapus (atau, saat simulasi, yang akan terhapus).
    pub freed_bytes: u64,
    /// Sedang dipakai program lain (terkunci).
    pub skipped_in_use: u64,
    /// Tidak ada izin.
    pub skipped_denied: u64,
    /// File sementara yang masih baru (< 24 jam).
    pub skipped_new: u64,
    /// Sebab lain.
    pub skipped_other: u64,
    /// Folder yang tidak bisa dibaca.
    pub unreadable_dirs: u64,
}

impl RootResult {
    pub fn skipped(&self) -> u64 {
        self.skipped_in_use + self.skipped_denied + self.skipped_new + self.skipped_other
    }
}

/// Galat "sedang dipakai" (Windows: ERROR_SHARING_VIOLATION / ERROR_LOCK_VIOLATION).
fn in_use(e: &std::io::Error) -> bool {
    cfg!(windows) && matches!(e.raw_os_error(), Some(32) | Some(33))
}

/// Hapus satu file. Ok(true) = terhapus, Ok(false) = sudah tidak ada.
fn remove_file(p: &Path) -> std::io::Result<bool> {
    match std::fs::remove_file(p) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // File sementara kadang bertanda read-only: buka tandanya, coba sekali lagi.
            match std::fs::symlink_metadata(p) {
                Ok(m) if m.permissions().readonly() && m.file_type().is_file() => {
                    let mut perm = m.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perm.set_readonly(false);
                    std::fs::set_permissions(p, perm)?;
                    std::fs::remove_file(p).map(|_| true)
                }
                _ => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

/// Bersihkan ISI satu lokasi (folder lokasinya sendiri dibiarkan).
/// `dry_run` = hanya menghitung apa yang akan terhapus.
pub fn clean_root(jenis: Jenis, root: &Path, now: SystemTime, dry_run: bool) -> RootResult {
    let mut out = RootResult::default();
    // (folder, kedalaman: 0 = isi langsung lokasi)
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    // Folder yang dikosongkan, untuk dihapus setelah isinya (dari yang terdalam).
    let mut emptied: Vec<PathBuf> = Vec::new();

    while let Some((dir, depth)) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                out.unreadable_dirs += 1;
                continue;
            }
        };
        for entry in rd {
            let Ok(entry) = entry else {
                out.skipped_other += 1;
                continue;
            };
            // file_type() dari DirEntry TIDAK mengikuti symlink; di Windows junction juga
            // terbaca sebagai symlink. Yang begitu dibiarkan sama sekali.
            let Ok(ft) = entry.file_type() else {
                out.skipped_other += 1;
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let rule = plan::rule_for(jenis, depth, &name, ft.is_dir());
            match rule {
                EntryRule::Keep => {}
                EntryRule::Descend => {
                    if ft.is_dir() {
                        stack.push((path, depth + 1));
                    }
                }
                EntryRule::Delete if ft.is_dir() => {
                    emptied.push(path.clone());
                    stack.push((path, depth + 1));
                }
                EntryRule::Delete => {
                    let meta = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => {
                            out.skipped_other += 1;
                            continue;
                        }
                    };
                    if plan::too_new(jenis, meta.modified().ok(), now) {
                        out.skipped_new += 1;
                        continue;
                    }
                    if dry_run {
                        out.deleted_files += 1;
                        out.freed_bytes += meta.len();
                        continue;
                    }
                    match remove_file(&path) {
                        Ok(true) => {
                            out.deleted_files += 1;
                            out.freed_bytes += meta.len();
                        }
                        Ok(false) => {}
                        Err(e) if in_use(&e) => out.skipped_in_use += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => out.skipped_denied += 1,
                        Err(_) => out.skipped_other += 1,
                    }
                }
            }
        }
    }

    if !dry_run {
        // Folder yang sudah kosong ikut dihapus, dari yang terdalam. Yang masih berisi
        // (ada file terkunci / masih baru / symlink) otomatis gagal dan dibiarkan.
        emptied.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for d in emptied {
            if d != root {
                let _ = std::fs::remove_dir(&d);
            }
        }
    }
    out
}

/// Root yang berupa symlink/junction tidak dibersihkan (isinya ada di tempat lain).
pub fn root_is_link(root: &Path) -> bool {
    std::fs::symlink_metadata(root).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

/// Waktu folder dibuat (ms sejak 1970), bila terbaca.
pub fn created_ms(path: &Path) -> Option<u64> {
    let t = std::fs::metadata(path).ok()?.created().ok()?;
    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

// ---------------------------------------------------------------- Windows.old

/// Jawaban `cleanup_windows_old.ps1`.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowsOldAnswer {
    pub ok: bool,
    pub still_exists: bool,
    pub error: Option<String>,
}

pub fn parse_windows_old(out: &str) -> Result<WindowsOldAnswer, String> {
    let v = json::parse(out)?;
    let e = json::str_of(v.get("error"));
    Ok(WindowsOldAnswer {
        ok: v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false),
        still_exists: v.get("stillExists").and_then(|b| b.as_bool()).unwrap_or(true),
        error: Some(e).filter(|e| !e.is_empty()),
    })
}

pub fn run_windows_old(path: &Path, dry_run: bool) -> Result<WindowsOldAnswer, String> {
    let target = path.to_string_lossy().to_string();
    plan::root_is_sane(Jenis::WindowsOld, &target)?;
    let args = if dry_run { "-DryRun" } else { "" };
    let out = platform::run_powershell_script(WINDOWS_OLD_SCRIPT, args, &[("REPAIRDOC_WINOLD", &target)], WINDOWS_OLD_TIMEOUT)?;
    parse_windows_old(&out)
}

// ---------------------------------------------------------------- backend sungguhan

/// Backend sungguhan: lokasi dari `probe::cleanup::locations()`, daftar proses dari
/// sysinfo, identitas PC dari inventaris Mode Performa.
pub struct SystemCleanBackend;

impl CleanBackend for SystemCleanBackend {
    fn is_windows(&self) -> bool {
        platform::IS_WINDOWS
    }

    fn is_admin(&self) -> bool {
        platform::is_elevated()
    }

    fn machine(&self) -> Result<MachineInfo, String> {
        crate::preset::exec::PowerShellBackend.machine()
    }

    fn locations(&self) -> Vec<(Jenis, PathBuf)> {
        crate::probe::cleanup::locations().into_iter().map(|(k, p)| (Jenis::from_kind(k), p)).collect()
    }

    fn processes(&self) -> Vec<String> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
        let Ok(mut sys) = System::new_with_specifics(RefreshKind::nothing()) else { return Vec::new() };
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        sys.processes().values().map(|p| p.name().to_string_lossy().trim().to_ascii_lowercase()).collect()
    }

    fn windows_old(&self, path: &Path, dry_run: bool) -> Result<WindowsOldAnswer, String> {
        run_windows_old(path, dry_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Folder UJI sementara, nama unik per tes (pola fixture `probe::cleanup`), terhapus otomatis.
    pub(crate) struct Fixture {
        pub dir: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    pub(crate) fn fixture(test: &str) -> Fixture {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("repairdoc-cleaner-test-{test}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Fixture { dir }
    }

    fn write(p: &Path, len: usize) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, vec![7u8; len]).unwrap();
    }

    /// Jam "sekarang" dua hari di depan, supaya file uji tidak dianggap baru.
    fn later() -> SystemTime {
        SystemTime::now() + Duration::from_secs(2 * 24 * 3600)
    }

    #[test]
    fn deletes_contents_but_keeps_the_location_folder() {
        let f = fixture("isi");
        let root = f.dir.join("Temp");
        write(&root.join("a.tmp"), 1000);
        write(&root.join("sub").join("deep").join("b.tmp"), 2500);
        let r = clean_root(Jenis::Temp, &root, later(), false);
        assert_eq!((r.deleted_files, r.freed_bytes, r.skipped()), (2, 3500, 0));
        assert!(root.is_dir(), "folder lokasinya sendiri tidak dihapus");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0, "subfolder yang kosong ikut dihapus");
    }

    #[test]
    fn dry_run_changes_nothing() {
        let f = fixture("simulasi");
        let root = f.dir.join("Temp");
        write(&root.join("a.tmp"), 300);
        write(&root.join("x").join("b.tmp"), 200);
        let r = clean_root(Jenis::Temp, &root, later(), true);
        assert_eq!((r.deleted_files, r.freed_bytes), (2, 500));
        assert!(root.join("a.tmp").exists() && root.join("x").join("b.tmp").exists());
    }

    #[test]
    fn new_temp_files_are_kept() {
        let f = fixture("baru");
        let root = f.dir.join("Temp");
        write(&root.join("installer-sedang-jalan.tmp"), 50);
        let r = clean_root(Jenis::Temp, &root, SystemTime::now(), false);
        assert_eq!((r.deleted_files, r.skipped_new), (0, 1));
        assert!(root.join("installer-sedang-jalan.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_never_followed() {
        let f = fixture("symlink");
        let root = f.dir.join("Temp");
        let outside = f.dir.join("data-penting");
        write(&outside.join("skripsi.docx"), 100);
        write(&root.join("a.tmp"), 10);
        std::os::unix::fs::symlink(&outside, root.join("tautan-folder")).unwrap();
        std::os::unix::fs::symlink(outside.join("skripsi.docx"), root.join("tautan-file.docx")).unwrap();
        let r = clean_root(Jenis::Temp, &root, later(), false);
        assert_eq!(r.deleted_files, 1, "hanya a.tmp");
        assert!(outside.join("skripsi.docx").exists(), "isi di balik symlink tidak boleh tersentuh");
        assert!(std::fs::symlink_metadata(root.join("tautan-folder")).is_ok());
        // Root yang berupa symlink dikenali (engine melewatinya).
        let link_root = f.dir.join("TempLink");
        std::os::unix::fs::symlink(&outside, &link_root).unwrap();
        assert!(root_is_link(&link_root));
        assert!(!root_is_link(&root));
    }

    #[cfg(windows)]
    #[test]
    fn locked_files_are_skipped_not_errors() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = fixture("terkunci");
        let root = f.dir.join("Temp");
        write(&root.join("dipakai.tmp"), 64);
        write(&root.join("bebas.tmp"), 32);
        // Dibuka tanpa izin berbagi (share_mode 0) = seperti program yang sedang memakainya.
        let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(root.join("dipakai.tmp")).unwrap();
        let r = clean_root(Jenis::Temp, &root, later(), false);
        drop(lock);
        assert_eq!((r.deleted_files, r.freed_bytes, r.skipped_in_use), (1, 32, 1), "{r:?}");
        assert!(root.join("dipakai.tmp").exists());
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_never_followed() {
        let f = fixture("junction");
        let root = f.dir.join("Temp");
        let outside = f.dir.join("data-penting");
        write(&outside.join("skripsi.docx"), 100);
        write(&root.join("a.tmp"), 10);
        let link = root.join("tautan");
        let out = std::process::Command::new("cmd").args(["/c", "mklink", "/J"]).arg(&link).arg(&outside).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        let r = clean_root(Jenis::Temp, &root, later(), false);
        assert_eq!(r.deleted_files, 1);
        assert!(outside.join("skripsi.docx").exists(), "isi di balik junction tidak boleh tersentuh");
    }

    #[test]
    fn recycle_bin_removes_only_dollar_r_and_i_items() {
        let f = fixture("recycle");
        let root = f.dir.join("$Recycle.Bin");
        let sid = root.join("S-1-5-21-111-222-333-1001");
        write(&sid.join("desktop.ini"), 10);
        write(&sid.join("$IAB12CD.docx"), 20);
        write(&sid.join("$RAB12CD.docx"), 300);
        write(&sid.join("$R0QX1Z1").join("foto.jpg"), 400);
        write(&root.join("desktop.ini"), 5);
        let r = clean_root(Jenis::RecycleBin, &root, later(), false);
        assert_eq!((r.deleted_files, r.freed_bytes), (3, 720));
        assert!(sid.is_dir(), "folder per pengguna dibiarkan");
        assert!(sid.join("desktop.ini").exists() && root.join("desktop.ini").exists());
        assert!(!sid.join("$R0QX1Z1").exists(), "folder yang dibuang ikut terhapus");
    }

    #[test]
    fn windows_old_script_is_safe_and_fits() {
        let s = WINDOWS_OLD_SCRIPT;
        assert!(s.contains("param([switch]$DryRun"), "skrip yang mengubah sistem wajib punya -DryRun");
        assert!(s.contains("if ($DryRun)"));
        // Nilai StateFlags sementara selalu dihapus lagi, berhasil ataupun gagal.
        let fin = s.find("} finally {").expect("blok finally");
        assert!(s[fin..].contains("Remove-ItemProperty -Path $key -Name $flag"));
        // Hanya <drive>:\Windows.old; tidak pernah menghapus/mengubah izin sendiri.
        assert!(s.contains(r"'^[A-Za-z]:\\Windows\.old$'"));
        let code = crate::platform::compact_script(s).to_ascii_lowercase();
        for banned in ["remove-item ", "del ", "takeown", "icacls", "rd /s", "rmdir"] {
            assert!(!code.contains(banned), "{banned}");
        }
        let len = crate::platform::encode_command(&crate::platform::script_call(s, "-DryRun")).len();
        assert!(len < crate::platform::MAX_ENCODED_COMMAND);
    }

    #[test]
    fn windows_old_answers() {
        let a = parse_windows_old(r#"{"ok":true,"dryRun":false,"exists":true,"stillExists":false,"error":null}"#).unwrap();
        assert!(a.ok && !a.still_exists && a.error.is_none());
        let b = parse_windows_old(r#"{"ok":false,"stillExists":true,"error":"Disk Cleanup tidak ada"}"#).unwrap();
        assert_eq!(b.error.as_deref(), Some("Disk Cleanup tidak ada"));
        assert!(run_windows_old(Path::new(r"C:\Users\Budi"), true).is_err(), "jalur selain X:\\Windows.old ditolak sebelum skrip jalan");
    }
}

/// Tes yang hanya jalan di Windows: skrip Windows.old HANYA dalam mode `-DryRun`.
#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn windows_old_script_dry_run_changes_nothing() {
        let sysdrive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let p = PathBuf::from(format!("{sysdrive}\\Windows.old"));
        let existed = p.exists();
        let a = run_windows_old(&p, true).unwrap();
        assert_eq!(p.exists(), existed, "simulasi tidak boleh menghapus apa pun");
        if !existed {
            assert!(!a.ok);
        }
        // Nilai StateFlags sementara tidak boleh tertinggal.
        let check = platform::run_powershell(
            "(Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VolumeCaches\\Previous Installations' -Name StateFlags4207 -ErrorAction SilentlyContinue) -ne $null",
            Duration::from_secs(60),
        )
        .unwrap_or_default();
        assert!(!check.to_ascii_lowercase().contains("true"));
    }
}
