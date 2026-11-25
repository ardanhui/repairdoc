//! Penilaian kebersihan: file sampah yang bisa dibersihkan.

use crate::format::size;
use crate::model::*;
use crate::probe::cleanup::{CleanupRaw, JunkKind};

pub const JUNK_SOME: u64 = 500 * 1024 * 1024;
pub const JUNK_MUCH: u64 = 2 * 1024 * 1024 * 1024;
/// Bila drive C: sisa kurang dari ini, sampah yang banyak dianggap "bermasalah".
pub const SYS_FREE_TIGHT: u64 = 5 * 1024 * 1024 * 1024;
pub const WINDOWS_OLD_MIN: u64 = 1024 * 1024 * 1024;

const KINDS: [JunkKind; 4] = [JunkKind::Temp, JunkKind::BrowserCache, JunkKind::RecycleBin, JunkKind::UpdateCache];

pub fn analyze(raw: &CleanupRaw, sys_free: Option<u64>) -> CategoryResult {
    let cat = Category::Cleanup;
    let mut findings = Vec::new();

    let total = raw.safe_total();
    let approx = if total.truncated || total.denied > 0 { "setidaknya " } else { "sekitar " };
    let mut parts: Vec<String> = Vec::new();
    for k in KINDS {
        let t = raw.total(k);
        if t.bytes >= 50 * 1024 * 1024 {
            parts.push(format!("{} {}", k.label().to_lowercase(), size(t.bytes)));
        }
    }
    let breakdown = if parts.is_empty() { String::new() } else { format!(" Rinciannya: {}.", parts.join(", ")) };
    let steps: &[&str] = &[
        "Jalankan Disk Cleanup: ketik \"Disk Cleanup\" di menu Start, pilih drive C:, centang semua, lalu OK.",
        "Kosongkan Recycle Bin (klik kanan ikonnya di desktop → Empty Recycle Bin).",
        "Bersihkan cache browser: di Chrome/Edge tekan Ctrl + Shift + Delete, pilih \"Cached images and files\", lalu Clear.",
    ];
    if !raw.entries.is_empty() {
        if total.bytes >= JUNK_MUCH {
            let tight = sys_free.is_some_and(|f| f < SYS_FREE_TIGHT);
            findings.push(Finding::problem(
                "cleanup.junk",
                cat,
                if tight { Status::Bermasalah } else { Status::Perhatian },
                Severity::Sedang,
                format!("Ada {approx}{} file sampah yang bisa dibersihkan", size(total.bytes)),
                format!("File sementara, cache, dan isi Recycle Bin menumpuk.{breakdown}"),
                if tight {
                    "Drive C: sudah hampir penuh, dan sampah sebanyak ini ikut menyita ruang yang dibutuhkan Windows. Membersihkannya aman — isinya hanya file sementara."
                } else {
                    "Sampah ini tidak merusak, tapi menyita ruang dan membuat pencadangan serta pemindaian antivirus lebih lama. Membersihkannya aman — isinya hanya file sementara."
                },
                steps,
            ));
        } else if total.bytes >= JUNK_SOME {
            findings.push(Finding::problem(
                "cleanup.junk",
                cat,
                Status::Perhatian,
                Severity::Ringan,
                format!("Ada {approx}{} file sampah", size(total.bytes)),
                format!("Masih wajar, tapi bisa dibersihkan kapan saja.{breakdown}"),
                "File sementara dan cache menumpuk seiring pemakaian. Membersihkannya aman.",
                &steps[..2],
            ));
        } else {
            findings.push(Finding::healthy("cleanup.junk", cat, "File sampah hanya sedikit", format!("Hanya {approx}{}. Tidak perlu dibersihkan sekarang.", size(total.bytes))));
        }
    }

    let old = raw.total(JunkKind::WindowsOld);
    if old.bytes >= WINDOWS_OLD_MIN {
        findings.push(Finding::problem(
            "cleanup.windows_old",
            cat,
            Status::Perhatian,
            Severity::Sedang,
            format!("Ada sisa Windows lama sebesar {}{}", if old.truncated { "lebih dari " } else { "" }, size(old.bytes)),
            "Folder Windows.old berisi salinan Windows sebelum upgrade.",
            "Salinan ini hanya berguna kalau mau kembali ke Windows sebelumnya, dan Windows sendiri menghapusnya setelah 10 hari. Kalau PC berjalan normal, folder ini cuma memakan tempat.",
            &[
                "Buka Settings → System → Storage → Temporary files, centang \"Previous Windows installation(s)\", lalu Remove files.",
                "Jangan dihapus lewat File Explorer biasa — pakai cara di atas supaya bersih.",
            ],
        ));
    }

    let mut result = CategoryResult::new(cat, findings, tech(raw));
    if raw.any_denied() && crate::platform::IS_WINDOWS {
        result.needs_admin.push("sampah milik pengguna lain dan folder sistem".into());
    }
    result
}

fn tech(raw: &CleanupRaw) -> TechSection {
    let mut t = TechSection::new("Kebersihan");
    for e in &raw.entries {
        let mut flags = Vec::new();
        if e.size.truncated {
            flags.push("dihentikan (batas waktu)");
        }
        if e.size.denied > 0 {
            flags.push("sebagian tidak terbaca");
        }
        let flags = if flags.is_empty() { String::new() } else { format!(" · {}", flags.join(", ")) };
        t.push(format!("{}: {}", e.kind.label(), e.path.display()), format!("{} · {} file{flags}", size(e.size.bytes), e.size.files));
    }
    if let Some(h) = raw.hiberfil {
        t.push("hiberfil.sys (file hibernasi, bukan sampah)", size(h));
    }
    if let Some(p) = raw.pagefile {
        t.push("pagefile.sys (memori virtual, bukan sampah)", size(p));
    }
    for n in &raw.notes {
        t.push("Catatan", n.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::cleanup::{FolderSize, JunkEntry};
    use std::path::PathBuf;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn raw(entries: &[(JunkKind, u64)]) -> CleanupRaw {
        CleanupRaw {
            entries: entries
                .iter()
                .map(|(k, b)| JunkEntry { kind: *k, path: PathBuf::from("x"), size: FolderSize { bytes: *b, files: 1, denied: 0, truncated: false } })
                .collect(),
            ..Default::default()
        }
    }

    fn find<'a>(r: &'a CategoryResult, id: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.id == id)
    }

    #[test]
    fn junk_thresholds() {
        let small = raw(&[(JunkKind::Temp, 100 * 1024 * 1024)]);
        assert_eq!(find(&analyze(&small, None), "cleanup.junk").unwrap().status, Status::Sehat);
        let some = raw(&[(JunkKind::Temp, 800 * 1024 * 1024)]);
        assert_eq!(find(&analyze(&some, None), "cleanup.junk").unwrap().status, Status::Perhatian);
        let much = raw(&[(JunkKind::Temp, 2 * GIB), (JunkKind::RecycleBin, GIB)]);
        let res = analyze(&much, Some(50 * GIB));
        let f = find(&res, "cleanup.junk").unwrap();
        assert_eq!(f.status, Status::Perhatian);
        assert!(f.title.contains("3 GB"));
        assert!(f.summary.contains("recycle bin 1 GB"));
        assert_eq!(find(&analyze(&much, Some(2 * GIB)), "cleanup.junk").unwrap().status, Status::Bermasalah, "C: hampir penuh → bermasalah");
    }

    #[test]
    fn windows_old_is_separate() {
        let r = analyze(&raw(&[(JunkKind::WindowsOld, 20 * GIB), (JunkKind::Temp, 10 * 1024 * 1024)]), None);
        assert_eq!(find(&r, "cleanup.windows_old").unwrap().status, Status::Perhatian);
        assert_eq!(find(&r, "cleanup.junk").unwrap().status, Status::Sehat, "Windows.old tidak dihitung sebagai sampah biasa");
    }
}
