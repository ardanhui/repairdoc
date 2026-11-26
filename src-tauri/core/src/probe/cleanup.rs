//! Menghitung file sampah: folder sementara, cache browser, Recycle Bin, sisa Windows lama.
//! Hanya menghitung ukuran — TIDAK menghapus apa pun.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Batas waktu seluruh penghitungan, supaya PC dengan HDD lambat tidak menunggu lama.
pub const TOTAL_BUDGET: Duration = Duration::from_secs(12);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JunkKind {
    /// %TEMP% milik pengguna dan C:\Windows\Temp.
    Temp,
    /// Cache browser (Chrome, Edge, Firefox, Brave, Opera) dan cache internet Windows.
    BrowserCache,
    /// Recycle Bin di semua drive.
    RecycleBin,
    /// Unduhan update yang sudah dipakai (SoftwareDistribution\Download).
    UpdateCache,
    /// Sisa Windows lama setelah upgrade (C:\Windows.old).
    WindowsOld,
}

impl JunkKind {
    pub fn label(self) -> &'static str {
        match self {
            JunkKind::Temp => "File sementara",
            JunkKind::BrowserCache => "Cache browser & internet",
            JunkKind::RecycleBin => "Recycle Bin",
            JunkKind::UpdateCache => "Sisa unduhan Windows Update",
            JunkKind::WindowsOld => "Sisa Windows lama (Windows.old)",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FolderSize {
    pub bytes: u64,
    pub files: u64,
    /// Ada bagian yang tidak bisa dibaca (biasanya butuh admin).
    pub denied: u64,
    /// Penghitungan dihentikan karena kehabisan waktu — angkanya batas bawah.
    pub truncated: bool,
}

impl FolderSize {
    fn add(&mut self, other: &FolderSize) {
        self.bytes += other.bytes;
        self.files += other.files;
        self.denied += other.denied;
        self.truncated |= other.truncated;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JunkEntry {
    pub kind: JunkKind,
    pub path: PathBuf,
    pub size: FolderSize,
}

#[derive(Clone, Debug, Default)]
pub struct CleanupRaw {
    pub entries: Vec<JunkEntry>,
    /// Ukuran hiberfil.sys bila terbaca (bukan sampah, tapi sering besar di PC kecil).
    pub hiberfil: Option<u64>,
    pub pagefile: Option<u64>,
    pub notes: Vec<String>,
}

impl CleanupRaw {
    pub fn total(&self, kind: JunkKind) -> FolderSize {
        let mut t = FolderSize::default();
        for e in self.entries.iter().filter(|e| e.kind == kind) {
            t.add(&e.size);
        }
        t
    }

    /// Semua sampah yang aman dibersihkan (tanpa Windows.old, yang perlu keputusan pengguna).
    pub fn safe_total(&self) -> FolderSize {
        let mut t = FolderSize::default();
        for e in self.entries.iter().filter(|e| e.kind != JunkKind::WindowsOld) {
            t.add(&e.size);
        }
        t
    }

    pub fn any_denied(&self) -> bool {
        self.entries.iter().any(|e| e.size.denied > 0)
    }
}

/// Hitung ukuran folder (rekursif) dengan batas waktu. Symlink/junction tidak diikuti.
pub fn folder_size(root: &Path, deadline: Instant) -> FolderSize {
    let mut out = FolderSize::default();
    let mut stack = vec![root.to_path_buf()];
    let mut since_check = 0u32;
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                out.denied += 1;
                continue;
            }
        };
        for entry in rd {
            since_check += 1;
            if since_check.is_multiple_of(64) && Instant::now() >= deadline {
                out.truncated = true;
                return out;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    out.denied += 1;
                    continue;
                }
            };
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => {
                    out.denied += 1;
                    continue;
                }
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                out.files += 1;
                out.bytes += meta.len();
            }
        }
    }
    out
}

/// Daftar lokasi sampah di PC ini (Windows). Folder yang tidak ada dilewati.
#[cfg(windows)]
fn locations() -> Vec<(JunkKind, PathBuf)> {
    use JunkKind::*;
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut v: Vec<(JunkKind, PathBuf)> = Vec::new();
    let sysroot = env("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let sysdrive = env("SystemDrive").unwrap_or_else(|| PathBuf::from("C:"));

    if let Some(t) = env("TEMP").or_else(|| env("TMP")) {
        v.push((Temp, t));
    }
    v.push((Temp, sysroot.join("Temp")));
    v.push((UpdateCache, sysroot.join("SoftwareDistribution").join("Download")));
    v.push((WindowsOld, PathBuf::from(format!("{}\\Windows.old", sysdrive.display()))));

    if let Some(local) = env("LOCALAPPDATA") {
        v.push((BrowserCache, local.join(r"Microsoft\Windows\INetCache")));
        // Browser berbasis Chromium: <User Data>\<Profil>\Cache dan Code Cache.
        for rel in [r"Google\Chrome\User Data", r"Microsoft\Edge\User Data", r"BraveSoftware\Brave-Browser\User Data", r"Opera Software\Opera Stable", r"Vivaldi\User Data"] {
            let ud = local.join(rel);
            if let Ok(rd) = std::fs::read_dir(&ud) {
                for p in rd.flatten() {
                    let dir = p.path();
                    for c in ["Cache", "Code Cache", "GPUCache"] {
                        let cd = dir.join(c);
                        if cd.is_dir() {
                            v.push((BrowserCache, cd));
                        }
                    }
                }
            }
        }
        // Firefox: Profiles\<profil>\cache2
        if let Ok(rd) = std::fs::read_dir(local.join(r"Mozilla\Firefox\Profiles")) {
            for p in rd.flatten() {
                let cd = p.path().join("cache2");
                if cd.is_dir() {
                    v.push((BrowserCache, cd));
                }
            }
        }
    }

    // Recycle Bin per drive tetap (C:\$Recycle.Bin, D:\$Recycle.Bin, …).
    if let Ok(disks) = sysinfo::Disks::new_with_refreshed_list() {
        for d in disks.iter().filter(|d| !d.is_removable()) {
            let rb = d.mount_point().join("$Recycle.Bin");
            if rb.is_dir() {
                v.push((RecycleBin, rb));
            }
        }
    }
    v.retain(|(_, p)| p.is_dir());
    v
}

#[cfg(not(windows))]
fn locations() -> Vec<(JunkKind, PathBuf)> {
    vec![(JunkKind::Temp, std::env::temp_dir())].into_iter().filter(|(_, p)| p.is_dir()).collect()
}

pub fn probe() -> CleanupRaw {
    let deadline = Instant::now() + TOTAL_BUDGET;
    measure(locations(), deadline)
}

pub fn measure(locations: Vec<(JunkKind, PathBuf)>, deadline: Instant) -> CleanupRaw {
    let mut raw = CleanupRaw::default();
    for (kind, path) in locations {
        let size = folder_size(&path, deadline);
        raw.entries.push(JunkEntry { kind, path, size });
    }
    if cfg!(windows) {
        let sysdrive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        raw.hiberfil = std::fs::metadata(format!("{sysdrive}\\hiberfil.sys")).ok().map(|m| m.len());
        raw.pagefile = std::fs::metadata(format!("{sysdrive}\\pagefile.sys")).ok().map(|m| m.len());
    }
    if raw.entries.iter().any(|e| e.size.truncated) {
        raw.notes.push("Penghitungan dihentikan setelah 12 detik; angka sampah adalah batas bawah.".into());
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};

    /// Folder uji sementara. Namanya unik per tes (nama tes + PID + nomor urut),
    /// karena tes Rust berjalan paralel: folder bersama bisa dihapus satu tes
    /// sementara tes lain masih mengukurnya. Folder dihapus otomatis saat selesai,
    /// termasuk bila tesnya gagal.
    struct Fixture {
        dir: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn fixture(test: &str) -> Fixture {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("repairdoc-cleanup-test-{test}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a").join("b")).unwrap();
        std::fs::write(dir.join("a").join("x.tmp"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.join("a").join("b").join("y.tmp"), vec![0u8; 2500]).unwrap();
        std::fs::write(dir.join("z.tmp"), vec![0u8; 500]).unwrap();
        Fixture { dir }
    }

    #[test]
    fn sums_recursively() {
        let f = fixture("sums_recursively");
        let s = folder_size(&f.dir, Instant::now() + Duration::from_secs(5));
        assert_eq!((s.bytes, s.files, s.denied, s.truncated), (4000, 3, 0, false));
    }

    #[test]
    fn missing_folder_counts_as_denied() {
        let s = folder_size(Path::new("/tidak/ada/folder/ini"), Instant::now() + Duration::from_secs(1));
        assert_eq!(s.denied, 1);
        assert_eq!(s.bytes, 0);
    }

    #[test]
    fn deadline_truncates() {
        let f = fixture("deadline_truncates");
        for i in 0..300 {
            std::fs::write(f.dir.join(format!("f{i}")), b"x").unwrap();
        }
        let s = folder_size(&f.dir, Instant::now() - Duration::from_secs(1));
        assert!(s.truncated);
    }

    #[test]
    fn totals_by_kind() {
        let f = fixture("totals_by_kind");
        let raw = measure(vec![(JunkKind::Temp, f.dir.clone()), (JunkKind::WindowsOld, f.dir.join("a"))], Instant::now() + Duration::from_secs(5));
        assert_eq!(raw.total(JunkKind::Temp).bytes, 4000);
        assert_eq!(raw.total(JunkKind::WindowsOld).bytes, 3500);
        assert_eq!(raw.safe_total().bytes, 4000);
    }

    #[test]
    fn fixtures_are_unique() {
        let a = fixture("sama");
        let b = fixture("sama");
        assert_ne!(a.dir, b.dir);
        let kept = a.dir.clone();
        drop(a);
        assert!(!kept.exists(), "folder uji harus terhapus otomatis");
        assert!(b.dir.join("z.tmp").exists(), "menghapus satu fixture tidak boleh mengganggu yang lain");
    }
}
