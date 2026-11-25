//! Pengukuran "langsung": pemakaian prosesor dan proses-proses yang berjalan,
//! diukur dalam satu jendela waktu yang sama (±2 detik) di awal scan,
//! sebelum RepairDoc menjalankan PowerShell apa pun — supaya kerja RepairDoc
//! sendiri tidak ikut terhitung.

use std::collections::HashMap;
use std::time::Duration;

use sysinfo::{CpuRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

use super::cpu::CpuRaw;

/// Satu program (semua prosesnya digabung, mis. 12 proses chrome.exe = satu "Chrome").
#[derive(Clone, Debug, PartialEq)]
pub struct ProcSample {
    /// Nama file program, huruf kecil, mis. "chrome.exe".
    pub name: String,
    pub instances: u32,
    /// Bagian dari seluruh tenaga prosesor, 0–100 (sudah dibagi jumlah utas).
    pub cpu_pct: f64,
    pub memory: u64,
}

#[derive(Clone, Debug, Default)]
pub struct LiveRaw {
    pub cpu: CpuRaw,
    /// Diurutkan dari pemakaian prosesor tertinggi.
    pub procs: Vec<ProcSample>,
    pub uptime_secs: Option<u64>,
}

/// Proses yang bukan "program" dari sudut pandang pengguna.
const IGNORED: &[&str] = &["", "system idle process", "idle", "system", "registry", "memory compression", "secure system"];

pub fn probe(window: Duration) -> LiveRaw {
    let mut live = LiveRaw { uptime_secs: System::uptime().ok(), ..Default::default() };
    let proc_kind = ProcessRefreshKind::nothing().with_cpu().with_memory().with_exe(UpdateKind::Never);
    let kind = RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()).with_processes(proc_kind);

    let mut sys = match System::new_with_specifics(kind) {
        Ok(s) => s,
        Err(e) => {
            live.cpu.notes.push(format!("Data prosesor tidak terbaca: {e}"));
            return live;
        }
    };

    live.cpu.logical_cores = sys.cpus().len();
    live.cpu.name = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    live.cpu.physical_cores = System::physical_core_count().ok();

    const SAMPLES: u32 = 5;
    let step = (window / SAMPLES).max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    let mut samples = Vec::with_capacity(SAMPLES as usize);
    for _ in 0..SAMPLES {
        std::thread::sleep(step);
        sys.refresh_cpu_usage();
        let u = sys.global_cpu_usage() as f64;
        if u.is_finite() {
            samples.push(u.clamp(0.0, 100.0));
        }
    }
    if !samples.is_empty() {
        live.cpu.usage_avg = Some(samples.iter().sum::<f64>() / samples.len() as f64);
        live.cpu.usage_peak = samples.iter().cloned().fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))));
    }

    // Pemakaian proses dihitung dari selisih sejak refresh pertama (saat `new_with_specifics`).
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, proc_kind);
    let me = sysinfo::get_current_pid().ok();
    let threads = live.cpu.logical_cores.max(1) as f64;
    let mut grouped: HashMap<String, ProcSample> = HashMap::new();
    for (pid, p) in sys.processes() {
        if Some(*pid) == me {
            continue;
        }
        let name = p.name().to_string_lossy().trim().to_ascii_lowercase();
        if IGNORED.contains(&name.as_str()) {
            continue;
        }
        let cpu = (p.cpu_usage() as f64 / threads).clamp(0.0, 100.0);
        let e = grouped.entry(name.clone()).or_insert(ProcSample { name, instances: 0, cpu_pct: 0.0, memory: 0 });
        e.instances += 1;
        e.cpu_pct += cpu;
        e.memory += p.memory();
    }
    let mut procs: Vec<ProcSample> = grouped.into_values().collect();
    procs.sort_by(|a, b| b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(std::cmp::Ordering::Equal).then(b.memory.cmp(&a.memory)));
    live.procs = procs;

    super::cpu::probe_temp(&mut live.cpu);
    live
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_runs_and_sees_processes() {
        let live = probe(Duration::from_millis(500));
        assert!(live.cpu.logical_cores >= 1);
        if let Some(u) = live.cpu.usage_avg {
            assert!((0.0..=100.0).contains(&u));
        }
        assert!(!live.procs.is_empty());
        for p in &live.procs {
            assert!(p.cpu_pct <= 100.0 && p.instances >= 1, "{p:?}");
        }
        let names: Vec<&str> = live.procs.iter().map(|p| p.name.as_str()).collect();
        assert!(!names.contains(&""), "nama kosong harus dibuang");
    }
}
