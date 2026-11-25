//! Alur scan lengkap: jalankan tiap kategori berurutan sambil melapor progres.
//! Scan HANYA membaca — tidak ada pengaturan PC yang diubah.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::model::*;
use crate::probe::live::ProcSample;
use crate::{analyze, dates, platform, probe};

/// Lama pengukuran pemakaian prosesor & proses.
pub const CPU_SAMPLE_WINDOW: Duration = Duration::from_secs(2);

/// Hasil satu kategori: ada temuan, dilewati (tidak berlaku di PC ini), atau gagal.
enum Outcome {
    Done(CategoryResult),
    Skipped(String),
}

/// Data yang dipakai bersama antar kategori.
#[derive(Default)]
struct Shared {
    procs: Vec<ProcSample>,
    uptime_secs: Option<u64>,
    total_mem: u64,
    sys_free: Option<u64>,
}

fn scan_category(cat: Category, shared: &mut Shared) -> Outcome {
    match cat {
        // Diukur paling awal, sebelum PowerShell apa pun dijalankan,
        // supaya pemeriksaan RepairDoc sendiri tidak ikut terhitung.
        Category::Cpu => {
            let live = probe::live::probe(CPU_SAMPLE_WINDOW);
            shared.procs = live.procs;
            shared.uptime_secs = live.uptime_secs;
            Outcome::Done(analyze::cpu::analyze(&live.cpu))
        }
        Category::Memory => {
            let raw = probe::memory::probe();
            shared.total_mem = raw.total;
            Outcome::Done(analyze::memory::analyze(&raw))
        }
        Category::Storage => {
            let raw = probe::disk::probe();
            shared.sys_free = raw.volumes.iter().find(|v| v.letter.is_some() && v.letter == raw.system_letter).map(|v| v.available);
            Outcome::Done(analyze::disk::analyze(&raw))
        }
        Category::Gpu => Outcome::Done(analyze::gpu::analyze(&probe::gpu::probe())),
        Category::Battery => match analyze::battery::analyze(&probe::battery::probe()) {
            Ok(r) => Outcome::Done(r),
            Err(reason) => Outcome::Skipped(reason),
        },
        Category::System => Outcome::Done(analyze::system::analyze(&probe::system::probe(shared.uptime_secs), dates::today())),
        Category::Programs => Outcome::Done(analyze::programs::analyze(&probe::programs::probe(), &shared.procs, shared.total_mem)),
        Category::Cleanup => Outcome::Done(analyze::cleanup::analyze(&probe::cleanup::probe(), shared.sys_free)),
        Category::Security => Outcome::Done(analyze::security::analyze(&probe::security::probe())),
        Category::Network => Outcome::Done(analyze::network::analyze(&probe::network::probe())),
    }
}

/// Jalankan scan. `on_event` dipanggil saat kategori mulai/selesai (untuk progress bar).
pub fn run(mut on_event: impl FnMut(ScanEvent)) -> ScanReport {
    let started = Instant::now();
    let is_admin = platform::is_elevated();
    let mut findings = Vec::new();
    let mut tech = Vec::new();
    let mut needs_admin: Vec<String> = Vec::new();
    let mut shared = Shared::default();

    for cat in Category::ALL {
        on_event(ScanEvent::CategoryStarted { category: cat });
        match catch_unwind(AssertUnwindSafe(|| scan_category(cat, &mut shared))) {
            Ok(Outcome::Done(result)) => {
                on_event(ScanEvent::CategoryFinished { category: cat, status: result.worst() });
                findings.extend(result.findings);
                tech.push(result.tech);
                for n in result.needs_admin {
                    if !needs_admin.contains(&n) {
                        needs_admin.push(n);
                    }
                }
            }
            Ok(Outcome::Skipped(reason)) => {
                on_event(ScanEvent::CategorySkipped { category: cat, reason: reason.clone() });
                let mut t = TechSection::new(cat.label());
                t.push("Dilewati", reason);
                tech.push(t);
            }
            Err(_) => {
                on_event(ScanEvent::CategoryFailed { category: cat });
                let mut t = TechSection::new(cat.label());
                t.push("Catatan", "Pemeriksaan kategori ini gagal dijalankan (terjadi galat internal).");
                tech.push(t);
            }
        }
    }

    let mut info = TechSection::new("Tentang pemeriksaan ini");
    info.push("Versi RepairDoc", env!("CARGO_PKG_VERSION"));
    if let Ok(os) = sysinfo::System::long_os_version() {
        info.push("Sistem operasi", os);
    }
    info.push("Izin administrator", if is_admin { "Ya" } else { "Tidak" });
    if let Some(l) = platform::exe_drive_letter() {
        info.push("Dijalankan dari", format!("Drive {l}: (dilewati saat diagnosa)"));
    }
    tech.insert(0, info);

    if is_admin {
        needs_admin.clear();
    }
    finish(findings, tech, is_admin, needs_admin, started.elapsed())
}

/// Susun laporan akhir dari temuan yang sudah terkumpul.
pub fn finish(mut findings: Vec<Finding>, tech: Vec<TechSection>, is_admin: bool, needs_admin: Vec<String>, elapsed: Duration) -> ScanReport {
    analyze::sort_findings(&mut findings);
    ScanReport {
        verdict: analyze::verdict(&findings),
        findings,
        tech,
        is_admin,
        needs_admin,
        finished_at_ms: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
        duration_ms: elapsed.as_millis() as u64,
    }
}
