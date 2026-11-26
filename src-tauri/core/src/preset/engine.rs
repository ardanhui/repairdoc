//! Alur Mode Performa: status → pratinjau → restore point → terapkan → catat,
//! dan "Kembalikan Semua". Tidak tahu-menahu soal PowerShell: semua akses ke
//! sistem lewat `Backend`, sehingga alur ini bisa dites dengan backend tiruan.

use std::path::Path;

use serde::Serialize;

use super::log;
use super::model::*;
use super::plan::{self, PlanOptions, PlannedOp};
use super::tweaks::{self, value_text, Effect};

/// Akses ke sistem. Implementasi sungguhan: `exec::PowerShellBackend`.
pub trait Backend {
    fn is_windows(&self) -> bool;
    fn is_admin(&self) -> bool;
    fn machine(&self) -> Result<MachineInfo, String>;
    fn inventory(&self) -> Result<Inventory, String>;
    /// Jalankan perubahan. `dry_run = true` hanya membaca dan melaporkan rencana.
    fn run(&self, ops: &[Op], dry_run: bool) -> Result<Vec<OpResult>, String>;
    /// Buat System Restore Point bernama "`prefix` <tanggal jam>". Ok(nama lengkap).
    fn restore_point(&self, prefix: &str) -> Result<String, String>;
}

// ---------------------------------------------------------------- status

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModeInfo {
    pub mode: Mode,
    pub title: String,
    /// "Low", "Medium", "High" — seperti pengaturan grafis game.
    pub level: String,
    pub tagline: String,
    pub for_whom: String,
    pub tweak_ids: Vec<String>,
    pub bullets: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TweakInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub modes: Vec<Mode>,
    pub effect: Effect,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PresetStatus {
    pub is_windows: bool,
    pub is_admin: bool,
    pub machine_name: Option<String>,
    pub active_mode: Mode,
    pub active_since: Option<u64>,
    pub last_entry: Option<LogEntry>,
    pub revert_target: Option<LogEntry>,
    pub log_path: String,
    pub modes: Vec<ModeInfo>,
    pub tweaks: Vec<TweakInfo>,
    pub error: Option<String>,
}

pub fn mode_infos() -> Vec<ModeInfo> {
    Mode::ALL
        .iter()
        .map(|&m| {
            let defs = tweaks::tweaks_for(m);
            let (level, tagline, for_whom) = match m {
                Mode::Kentang => ("Low", "Performa maksimal, tampilan dikorbankan", "PC sangat lemot, RAM kecil, atau masih pakai HDD"),
                Mode::Seimbang => ("Medium", "Sebagian efek dimatikan, tampilan tetap enak", "PC menengah yang ingin terasa lebih ringan"),
                Mode::Normal => ("High", "Kembali ke bawaan Windows", "Membatalkan semua perubahan RepairDoc"),
            };
            let bullets = match m {
                Mode::Normal => vec![
                    "Membatalkan semua perubahan RepairDoc yang tercatat, ke nilai aslinya".to_string(),
                    "Kalau belum ada catatan: efek visual, transparansi, animasi, Game Bar, aplikasi latar, dan power plan kembali ke bawaan Windows".to_string(),
                ],
                _ => defs.iter().map(|d| d.title.to_string()).collect(),
            };
            ModeInfo {
                mode: m,
                title: m.title().to_string(),
                level: level.into(),
                tagline: tagline.into(),
                for_whom: for_whom.into(),
                tweak_ids: defs.iter().map(|d| d.id.to_string()).collect(),
                bullets,
            }
        })
        .collect()
}

pub fn tweak_infos() -> Vec<TweakInfo> {
    tweaks::TWEAKS
        .iter()
        .map(|t| TweakInfo { id: t.id.into(), title: t.title.into(), description: t.description.into(), modes: t.modes.to_vec(), effect: t.effect })
        .collect()
}

pub fn status(backend: &dyn Backend, log_path: &Path) -> PresetStatus {
    let mut st = PresetStatus {
        is_windows: backend.is_windows(),
        is_admin: backend.is_admin(),
        machine_name: None,
        active_mode: Mode::Normal,
        active_since: None,
        last_entry: None,
        revert_target: None,
        log_path: log_path.display().to_string(),
        modes: mode_infos(),
        tweaks: tweak_infos(),
        error: None,
    };
    if !st.is_windows {
        return st;
    }
    match backend.machine() {
        Ok(m) => {
            let log = log::load(log_path);
            let (mode, since) = log.active_mode(&m.machine_id);
            st.active_mode = mode;
            st.active_since = since;
            st.last_entry = log.for_machine(&m.machine_id).last().map(|e| (*e).clone());
            st.revert_target = log.revert_target(&m.machine_id).cloned();
            st.machine_name = Some(m.computer_name);
        }
        Err(e) => st.error = Some(format!("Identitas PC tidak terbaca: {e}")),
    }
    st
}

// ---------------------------------------------------------------- pemeriksaan awal

fn precheck(backend: &dyn Backend) -> Result<(), PresetError> {
    let err = |kind, message: &str, help: &[&str]| PresetError { kind, message: message.into(), help: help.iter().map(|h| h.to_string()).collect() };
    if !backend.is_windows() {
        return Err(err(ErrorKind::NotWindows, "Mode Performa hanya bisa dipakai di Windows.", &[]));
    }
    if !backend.is_admin() {
        return Err(err(
            ErrorKind::NotAdmin,
            "Mode Performa butuh izin administrator.",
            &["Klik \"Minta izin administrator\". RepairDoc akan dibuka ulang dengan izin itu."],
        ));
    }
    Ok(())
}

fn failed(mode: Option<Mode>, e: PresetError) -> Outcome {
    let help: Vec<&str> = e.help.iter().map(|s| s.as_str()).collect();
    Outcome::error(mode, e.kind, e.message.clone(), &help)
}

pub const RESTORE_HELP: &[&str] = &[
    "Ketik \"Create a restore point\" di menu Start, lalu buka.",
    "Pilih drive Windows (biasanya C:, tertulis \"System\"), klik Configure.",
    "Pilih \"Turn on system protection\", geser Max Usage ke sekitar 5%, lalu OK.",
    "Kembali ke RepairDoc dan tekan Terapkan lagi.",
];

fn restore_prefix(mode: Mode) -> String {
    format!("RepairDoc {}", mode.title())
}

// ---------------------------------------------------------------- pratinjau

pub fn preview(backend: &dyn Backend, log_path: &Path, mode: Mode, opts: &PlanOptions, now_ms: u64) -> Result<Preview, PresetError> {
    precheck(backend)?;
    let inv = backend.inventory().map_err(other)?;
    let log = log::load(log_path);
    // Pratinjau menampilkan semua item; centang per item diterapkan saat Terapkan.
    let view_opts = PlanOptions { disabled_tweaks: opts.disabled_tweaks.clone(), skip_items: Vec::new() };
    let plan = plan::build(mode, &inv, &log, &view_opts, now_ms);
    let ops: Vec<Op> = plan.ops.iter().map(|p| p.op.clone()).collect();
    let results = backend.run(&ops, true).map_err(other)?;

    let mut groups: Vec<PreviewGroup> = Vec::new();
    let mut changing_items: Vec<&str> = Vec::new();
    let mut all_items: Vec<&str> = Vec::new();
    for p in &plan.ops {
        let r = results.iter().find(|r| r.key == p.op.key);
        let (will_change, current, note) = describe_dry(p, r);
        if will_change && !changing_items.contains(&p.item_key.as_str()) {
            changing_items.push(&p.item_key);
        }
        if !all_items.contains(&p.item_key.as_str()) {
            all_items.push(&p.item_key);
        }
        group_for(&mut groups, &p.tweak_id, false).changes.push(PreviewChange {
            key: p.op.key.clone(),
            item_key: p.item_key.clone(),
            label: p.label.clone(),
            target_text: p.op.target.text(),
            current,
            new: value_text(&p.op.target, &p.op.value),
            will_change,
            optional: p.optional,
            note,
        });
    }
    for (p, why) in &plan.user_skipped {
        group_for(&mut groups, &p.tweak_id, true).changes.push(PreviewChange {
            key: p.op.key.clone(),
            item_key: p.item_key.clone(),
            label: p.label.clone(),
            target_text: p.op.target.text(),
            current: "—".into(),
            new: value_text(&p.op.target, &p.op.value),
            will_change: false,
            optional: p.optional,
            note: Some(why.clone()),
        });
    }

    let change_count = changing_items.len();
    let same_count = all_items.len() - change_count;
    let mut notes = Vec::new();
    match plan.path {
        PlanPath::RevertLog => notes.push(format!(
            "Mode Normal akan membatalkan {} penerapan RepairDoc sebelumnya di PC ini, ke nilai aslinya.",
            plan.reverts.len()
        )),
        PlanPath::Defaults => notes.push("Belum ada perubahan RepairDoc yang tercatat di PC ini, jadi Mode Normal memasang nilai bawaan Windows.".into()),
        PlanPath::Apply => {}
    }
    let slow = plan.ops.iter().any(|p| tweaks::tweak(&p.tweak_id).is_some_and(|t| t.effect != Effect::Langsung));
    if slow && change_count > 0 {
        notes.push("Sebagian perubahan baru terasa setelah keluar-masuk akun (sign out) atau restart.".into());
    }
    if change_count == 0 {
        notes.push("Semua pengaturan sudah sesuai dengan mode ini. Tidak ada yang perlu diubah.".into());
    }

    Ok(Preview { mode, path: plan.path, groups, change_count, same_count, notes, restore_point_name: format!("{} (tanggal & jam saat diterapkan)", restore_prefix(mode)) })
}

fn other(e: String) -> PresetError {
    PresetError { kind: ErrorKind::Other, message: e, help: vec![] }
}

fn group_for<'a>(groups: &'a mut Vec<PreviewGroup>, tweak_id: &str, skipped: bool) -> &'a mut PreviewGroup {
    if let Some(i) = groups.iter().position(|g| g.tweak_id == tweak_id) {
        groups[i].skipped_by_user |= skipped;
        return &mut groups[i];
    }
    let (title, description) = tweaks::tweak(tweak_id).map(|t| (t.title, t.description)).unwrap_or(("Perubahan", ""));
    groups.push(PreviewGroup { tweak_id: tweak_id.into(), title: title.into(), description: description.into(), skipped_by_user: skipped, changes: Vec::new() });
    groups.last_mut().unwrap()
}

/// Dari hasil simulasi: (akan berubah?, nilai sekarang, catatan).
fn describe_dry(p: &PlannedOp, r: Option<&OpResult>) -> (bool, String, Option<String>) {
    let Some(r) = r else { return (false, "?".into(), Some("Tidak ada jawaban dari pemeriksaan".into())) };
    let current = r.before.as_ref().map(|b| value_text(&p.op.target, b)).unwrap_or_else(|| "?".into());
    match r.status {
        OpStatus::Planned if r.before.as_ref().is_some_and(|b| plan::already_ok(&p.op, b)) => (false, current, Some("Sudah sesuai".into())),
        OpStatus::Planned => (true, current, None),
        OpStatus::Same | OpStatus::Changed => (false, current, Some("Sudah sesuai".into())),
        OpStatus::NotPresent => (false, current, Some(r.error.clone().unwrap_or_else(|| "Tidak ada di PC ini".into()))),
        OpStatus::Refused => (false, current, Some(format!("Ditolak pengaman: {}", r.error.clone().unwrap_or_default()))),
        OpStatus::Failed => (false, current, Some(format!("Tidak bisa dibaca: {}", r.error.clone().unwrap_or_default()))),
    }
}

// ---------------------------------------------------------------- terapkan

fn record(p: &PlannedOp, old: Option<Value>, new: Value, status: ChangeStatus, note: Option<String>) -> ChangeRecord {
    ChangeRecord {
        tweak_id: p.tweak_id.clone(),
        item_key: p.item_key.clone(),
        label: p.label.clone(),
        target: p.op.target.clone(),
        target_teks: p.op.target.text(),
        nilai_lama: old,
        nilai_baru: new,
        status,
        keterangan: note,
    }
}

/// Terapkan satu mode. Urutan wajib: periksa → simulasi → restore point → terapkan → catat.
pub fn apply(backend: &dyn Backend, log_path: &Path, mode: Mode, opts: &PlanOptions, now_ms: u64, on_event: &mut dyn FnMut(PresetEvent)) -> Outcome {
    if let Err(e) = precheck(backend) {
        return failed(Some(mode), e);
    }
    on_event(PresetEvent::Reading);
    let inv = match backend.inventory() {
        Ok(i) => i,
        Err(e) => return Outcome::error(Some(mode), ErrorKind::Other, format!("Kondisi PC tidak terbaca: {e}"), &[]),
    };
    let mut log = log::load(log_path);
    let plan = plan::build(mode, &inv, &log, opts, now_ms);
    let ops: Vec<Op> = plan.ops.iter().map(|p| p.op.clone()).collect();
    let dry = match backend.run(&ops, true) {
        Ok(r) => r,
        Err(e) => return Outcome::error(Some(mode), ErrorKind::Other, format!("Pengaturan sekarang tidak terbaca: {e}"), &[]),
    };

    // Pisahkan yang benar-benar perlu diubah dari yang sudah sesuai / tidak ada.
    let mut records: Vec<Option<ChangeRecord>> = vec![None; plan.ops.len()];
    let mut to_apply: Vec<usize> = Vec::new();
    for (i, p) in plan.ops.iter().enumerate() {
        let r = dry.iter().find(|r| r.key == p.op.key);
        let before = r.and_then(|r| r.before.clone());
        match r.map(|r| r.status) {
            Some(OpStatus::Planned) if before.as_ref().is_some_and(|b| plan::already_ok(&p.op, b)) => {
                records[i] = Some(record(p, before, p.op.value.clone(), ChangeStatus::Dilewati, Some("Sudah sesuai".into())))
            }
            Some(OpStatus::Planned) => to_apply.push(i),
            Some(OpStatus::Same) | Some(OpStatus::Changed) => {
                records[i] = Some(record(p, before, p.op.value.clone(), ChangeStatus::Dilewati, Some("Sudah sesuai".into())))
            }
            Some(OpStatus::NotPresent) => {
                let note = r.and_then(|r| r.error.clone()).unwrap_or_else(|| "Tidak ada di PC ini".into());
                records[i] = Some(record(p, before, p.op.value.clone(), ChangeStatus::Dilewati, Some(note)))
            }
            Some(OpStatus::Refused) | Some(OpStatus::Failed) => {
                let note = r.and_then(|r| r.error.clone()).unwrap_or_else(|| "Gagal dibaca".into());
                records[i] = Some(record(p, before, p.op.value.clone(), ChangeStatus::Gagal, Some(note)))
            }
            None => records[i] = Some(record(p, None, p.op.value.clone(), ChangeStatus::Gagal, Some("Tidak ada jawaban dari pemeriksaan".into()))),
        }
    }

    let mut restore_point = None;
    if !to_apply.is_empty() {
        // Jaring pengaman WAJIB. Gagal = berhenti, tidak ada yang diubah.
        on_event(PresetEvent::RestorePoint);
        match backend.restore_point(&restore_prefix(mode)) {
            Ok(name) => restore_point = Some(name),
            Err(e) => {
                return Outcome::error(
                    Some(mode),
                    ErrorKind::RestorePoint,
                    format!("Restore point tidak bisa dibuat, jadi tidak ada pengaturan yang diubah. Pesan Windows: {e}"),
                    RESTORE_HELP,
                )
            }
        }

        on_event(PresetEvent::Applying { total: to_apply.len() });
        let batch: Vec<Op> = to_apply.iter().map(|&i| plan.ops[i].op.clone()).collect();
        let results = match backend.run(&batch, false) {
            Ok(r) => r,
            // Eksekutor berhenti di tengah jalan: baca ulang untuk tahu mana yang sudah berubah.
            Err(e) => match backend.run(&batch, true) {
                Ok(check) => check
                    .into_iter()
                    .map(|mut r| {
                        if r.status == OpStatus::Same {
                            r.status = OpStatus::Changed;
                            r.after = r.before.take();
                        } else {
                            r.status = OpStatus::Failed;
                            r.error = Some(format!("Eksekutor berhenti: {e}"));
                        }
                        r
                    })
                    .collect(),
                Err(_) => Vec::new(),
            },
        };
        for &i in &to_apply {
            let p = &plan.ops[i];
            let dry_before = dry.iter().find(|r| r.key == p.op.key).and_then(|r| r.before.clone());
            let rec = match results.iter().find(|r| r.key == p.op.key) {
                Some(r) => match r.status {
                    OpStatus::Changed => record(p, dry_before, r.after.clone().unwrap_or_else(|| p.op.value.clone()), ChangeStatus::Berhasil, None),
                    OpStatus::Same => record(p, dry_before, p.op.value.clone(), ChangeStatus::Dilewati, Some("Sudah sesuai".into())),
                    OpStatus::NotPresent => record(p, dry_before, p.op.value.clone(), ChangeStatus::Dilewati, r.error.clone().or(Some("Tidak ada di PC ini".into()))),
                    _ => record(p, dry_before, p.op.value.clone(), ChangeStatus::Gagal, r.error.clone().or(Some("Gagal".into()))),
                },
                None => record(p, dry_before, p.op.value.clone(), ChangeStatus::Gagal, Some("Tidak ada jawaban dari eksekutor".into())),
            };
            records[i] = Some(rec);
        }
    }

    let mut perubahan: Vec<ChangeRecord> = records.into_iter().flatten().collect();
    for (p, why) in &plan.user_skipped {
        perubahan.push(record(p, None, p.op.value.clone(), ChangeStatus::Dilewati, Some(why.clone())));
    }

    on_event(PresetEvent::Saving);
    let jenis = if plan.path == PlanPath::RevertLog { EntryKind::Kembalikan } else { EntryKind::Terapkan };
    let mut id = format!("{now_ms}-{}", mode.title().replace(' ', "-").to_lowercase());
    while log.entri.iter().any(|e| e.id == id) {
        id.push('x');
    }
    let entry = LogEntry {
        id,
        mesin: inv.machine.machine_id.clone(),
        nama_pc: inv.machine.computer_name.clone(),
        waktu: now_ms,
        mode,
        jenis,
        restore_point: restore_point.clone(),
        perubahan,
        dikembalikan: false,
        waktu_dikembalikan: None,
        hasil_pengembalian: Vec::new(),
        membatalkan: plan.reverts.clone(),
    };
    if jenis == EntryKind::Kembalikan {
        for rid in &plan.reverts {
            let prefix = format!("revert:{rid}:");
            let failed = plan.ops.iter().zip(entry.perubahan.iter()).any(|(p, c)| p.op.key.starts_with(&prefix) && c.status == ChangeStatus::Gagal);
            if !failed {
                if let Some(e) = log.entri.iter_mut().find(|e| &e.id == rid) {
                    e.dikembalikan = true;
                    e.waktu_dikembalikan = Some(now_ms);
                }
            }
        }
    }
    log.entri.push(entry.clone());
    let saved = log::save(log_path, &log);
    finish(Some(mode), entry, restore_point, saved)
}

fn finish(mode: Option<Mode>, entry: LogEntry, restore_point: Option<String>, saved: Result<(), String>) -> Outcome {
    let changes = if entry.hasil_pengembalian.is_empty() { &entry.perubahan } else { &entry.hasil_pengembalian };
    let items = item_statuses(changes);
    let changed = items.iter().filter(|(_, s)| *s == ChangeStatus::Berhasil).count();
    let skipped = items.iter().filter(|(_, s)| *s == ChangeStatus::Dilewati).count();
    let mut failures: Vec<Failure> = Vec::new();
    for c in changes.iter().filter(|c| c.status == ChangeStatus::Gagal) {
        if !failures.iter().any(|f| f.label == c.label) {
            failures.push(Failure { label: c.label.clone(), reason: c.keterangan.clone().unwrap_or_else(|| "Gagal".into()) });
        }
    }
    let needs_restart = changes
        .iter()
        .filter(|c| c.status == ChangeStatus::Berhasil)
        .any(|c| tweaks::tweak(&c.tweak_id).is_some_and(|t| t.effect != Effect::Langsung));
    let error = saved.err().map(|e| PresetError {
        kind: ErrorKind::Other,
        message: format!("Perubahan sudah dijalankan, tapi catatannya tidak bisa disimpan ({e}). Tombol Kembalikan Semua tidak bisa dipakai untuk perubahan ini — gunakan restore point."),
        help: vec![],
    });
    Outcome {
        ok: error.is_none(),
        mode,
        error,
        changed,
        skipped,
        failed: failures.len(),
        failures,
        restore_point,
        needs_restart,
        entry: Some(entry),
    }
}

// ---------------------------------------------------------------- kembalikan semua

/// Batalkan penerapan terakhir (yang belum dibatalkan) di PC ini: setiap perubahan
/// dikembalikan ke nilai lamanya, urutan terbalik. Restore point tetap jadi jaring
/// pengaman kedua; langkah ini sendiri tidak membuat restore point baru.
pub fn revert_last(backend: &dyn Backend, log_path: &Path, now_ms: u64, on_event: &mut dyn FnMut(PresetEvent)) -> Outcome {
    if let Err(e) = precheck(backend) {
        return failed(None, e);
    }
    on_event(PresetEvent::Reading);
    let machine = match backend.machine() {
        Ok(m) => m,
        Err(e) => return Outcome::error(None, ErrorKind::Other, format!("Identitas PC tidak terbaca: {e}"), &[]),
    };
    let mut log = log::load(log_path);
    let Some(target) = log.revert_target(&machine.machine_id).cloned() else {
        return Outcome::error(None, ErrorKind::NothingToRevert, "Tidak ada perubahan RepairDoc di PC ini yang perlu dikembalikan.", &[]);
    };
    let ops = plan::revert_ops(&target);
    on_event(PresetEvent::Applying { total: ops.len() });
    let batch: Vec<Op> = ops.iter().map(|p| p.op.clone()).collect();
    let (results, run_error) = match backend.run(&batch, false) {
        Ok(r) => (r, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let records: Vec<ChangeRecord> = ops
        .iter()
        .map(|p| match results.iter().find(|r| r.key == p.op.key) {
            Some(r) => match r.status {
                OpStatus::Changed => record(p, r.before.clone(), p.op.value.clone(), ChangeStatus::Berhasil, None),
                OpStatus::Same => record(p, r.before.clone(), p.op.value.clone(), ChangeStatus::Dilewati, Some("Sudah kembali seperti semula".into())),
                OpStatus::NotPresent => record(p, r.before.clone(), p.op.value.clone(), ChangeStatus::Dilewati, r.error.clone().or(Some("Tidak ada di PC ini".into()))),
                _ => record(p, r.before.clone(), p.op.value.clone(), ChangeStatus::Gagal, r.error.clone().or(Some("Gagal".into()))),
            },
            None => record(p, None, p.op.value.clone(), ChangeStatus::Gagal, Some(run_error.clone().unwrap_or_else(|| "Tidak ada jawaban dari eksekutor".into()))),
        })
        .collect();

    on_event(PresetEvent::Saving);
    let all_ok = records.iter().all(|r| r.status != ChangeStatus::Gagal);
    let entry = {
        let e = log.entri.iter_mut().find(|e| e.id == target.id).expect("entri target ada di catatan");
        e.hasil_pengembalian = records;
        if all_ok {
            e.dikembalikan = true;
            e.waktu_dikembalikan = Some(now_ms);
        }
        e.clone()
    };
    let saved = log::save(log_path, &log);
    finish(Some(entry.mode), entry, None, saved)
}
