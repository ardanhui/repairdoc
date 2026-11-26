// Jembatan ke backend Rust. Kalau tampilan dibuka di browser biasa
// (`npm run dev` tanpa Tauri), dipakai data contoh supaya desain bisa dikerjakan
// tanpa harus build Rust.
import type { AppInfo, Category, ScanEvent, ScanReport } from './types'
import type { Outcome, PresetError, PresetEvent, PresetMode, PresetRequest, PresetStatus, Preview } from './types'
import type { CleanEntry, CleanEvent, CleanJenis, CleanOutcome, CleanPreview, CleanRequest, CleanStatus, KindRecord } from './types'
import mockReport from './mock-report.json'

export const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export async function getAppInfo(): Promise<AppInfo> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke<AppInfo>('app_info')
  }
  return { isAdmin: false, version: 'pratinjau', isWindows: false }
}

export async function startScan(onEvent: (event: ScanEvent) => void): Promise<ScanReport> {
  if (isTauri()) {
    const { invoke, Channel } = await import('@tauri-apps/api/core')
    const channel = new Channel<ScanEvent>()
    channel.onmessage = onEvent
    return invoke<ScanReport>('start_scan', { onEvent: channel })
  }
  return mockScan(onEvent)
}

export async function relaunchAsAdmin(): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('relaunch_as_admin')
    return
  }
  throw new Error('Hanya bisa di aplikasi Windows, bukan di mode pratinjau.')
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

async function mockScan(onEvent: (event: ScanEvent) => void): Promise<ScanReport> {
  const report = mockReport as ScanReport
  const plan: [Category, number][] = [
    ['cpu', 2100],
    ['memory', 700],
    ['storage', 1200],
    ['gpu', 600],
    ['battery', 500],
    ['system', 2400],
    ['programs', 800],
    ['cleanup', 2600],
    ['security', 1500],
    ['network', 3000],
  ]
  for (const [category, ms] of plan) {
    onEvent({ kind: 'categoryStarted', category })
    await sleep(ms)
    const worst = report.findings
      .filter((f) => f.category === category)
      .reduce<ScanReport['verdict']['status']>((w, f) => (rank[f.status] > rank[w] ? f.status : w), 'sehat')
    onEvent({ kind: 'categoryFinished', category, status: worst })
  }
  return { ...report, finishedAtMs: Date.now() }
}

const rank = { sehat: 0, perhatian: 1, bermasalah: 2 } as const

// ---------------------------------------------------------------- Mode Performa (tahap 3)


export async function presetStatus(): Promise<PresetStatus> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke<PresetStatus>('preset_status')
  }
  return mockPresetStatus()
}

/** Pratinjau (simulasi) — tidak mengubah apa pun. */
export async function presetPreview(mode: PresetMode, request: PresetRequest): Promise<{ preview: Preview | null; error: PresetError | null }> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('preset_preview', { mode, request })
  }
  await loadMock()
  await sleep(700)
  return { preview: mockPresetPreview(mode, request), error: null }
}

export async function presetApply(mode: PresetMode, request: PresetRequest, onEvent: (e: PresetEvent) => void): Promise<Outcome> {
  if (isTauri()) {
    const { invoke, Channel } = await import('@tauri-apps/api/core')
    const channel = new Channel<PresetEvent>()
    channel.onmessage = onEvent
    return invoke<Outcome>('preset_apply', { mode, request, onEvent: channel })
  }
  return mockPresetApply(mode, request, onEvent)
}

export async function presetRevertAll(onEvent: (e: PresetEvent) => void): Promise<Outcome> {
  if (isTauri()) {
    const { invoke, Channel } = await import('@tauri-apps/api/core')
    const channel = new Channel<PresetEvent>()
    channel.onmessage = onEvent
    return invoke<Outcome>('preset_revert_all', { onEvent: channel })
  }
  return mockPresetRevert(onEvent)
}

// ---- tiruan untuk mode pratinjau (data dari examples/mock_preset.rs)

type MockAfter = { outcome: Outcome; status: PresetStatus; previewNormal: Preview; revert: Outcome; statusAfterRevert: PresetStatus }
type PresetData = {
  status: PresetStatus
  preview: Record<PresetMode, Preview>
  after: Record<PresetMode, MockAfter>
  restoreFailed: Outcome
  cleanup: { status: CleanStatus; preview: CleanPreview; records: Record<CleanJenis, KindRecord>; entryTemplate: CleanEntry }
}
// Dimuat hanya di mode pratinjau (file terpisah), supaya tidak ikut membebani aplikasi Windows.
let presetData: PresetData
async function loadMock() {
  if (!presetData) {
    presetData = (await import('./mock-preset.json')).default as unknown as PresetData
    mockStatus = presetData.status
    mockCleanStatus = presetData.cleanup.status
  }
}
/** Mode yang "sedang diterapkan" di PC tiruan; null = belum ada perubahan. */
let mockApplied: PresetMode | null = null
let mockStatus: PresetStatus

async function mockPresetStatus(): Promise<PresetStatus> {
  await loadMock()
  return structuredClone(mockStatus)
}

function mockPresetPreview(mode: PresetMode, request: PresetRequest): Preview {
  const base = mode === 'normal' && mockApplied && mockApplied !== 'normal' ? presetData.after[mockApplied].previewNormal : presetData.preview[mode]
  const p = structuredClone(base)
  if (p.path !== 'revertLog') {
    for (const g of p.groups) {
      if (request.disabledTweaks.includes(g.tweakId)) {
        g.skippedByUser = true
        for (const c of g.changes) {
          c.willChange = false
          c.note = 'Dimatikan lewat pilihan tweak'
        }
      }
    }
  }
  const changing = new Set(p.groups.flatMap((g) => g.changes.filter((c) => c.willChange).map((c) => c.itemKey)))
  p.changeCount = changing.size
  return p
}

async function mockSteps(onEvent: (e: PresetEvent) => void, total: number, withRestore: boolean) {
  onEvent({ step: 'reading' })
  await sleep(700)
  if (withRestore) {
    onEvent({ step: 'restorePoint' })
    await sleep(2200)
  }
  onEvent({ step: 'applying', total })
  await sleep(1200)
  onEvent({ step: 'saving' })
  await sleep(400)
}

/** Data contoh dibuat dengan semua pilihan aktif; hitungan disesuaikan dengan centang/saklar yang dimatikan. */
function mockAdjust(outcome: Outcome, mode: PresetMode, request: PresetRequest): Outcome {
  const p = mockPresetPreview(mode, request)
  const skipped = new Set(request.skipItems)
  const changing = new Set(p.groups.flatMap((g) => g.changes.filter((c) => c.willChange && !skipped.has(c.itemKey)).map((c) => c.itemKey)))
  const removed = Math.max(0, outcome.changed - changing.size)
  return { ...outcome, changed: outcome.changed - removed, skipped: outcome.skipped + removed }
}

async function mockPresetApply(mode: PresetMode, request: PresetRequest, onEvent: (e: PresetEvent) => void): Promise<Outcome> {
  await loadMock()
  if (new URLSearchParams(location.search).get('restore') === 'gagal') {
    onEvent({ step: 'reading' })
    await sleep(600)
    onEvent({ step: 'restorePoint' })
    await sleep(1500)
    return structuredClone(presetData.restoreFailed)
  }
  if (mode === 'normal' && mockApplied && mockApplied !== 'normal') {
    const r = presetData.after[mockApplied]
    await mockSteps(onEvent, r.revert.changed, true)
    mockStatus = r.statusAfterRevert
    mockApplied = null
    return { ...structuredClone(r.revert), mode: 'normal', restorePoint: 'RepairDoc Mode Normal 2026-11-26 14:05' }
  }
  const r = presetData.after[mode]
  const outcome = mockAdjust(structuredClone(r.outcome), mode, request)
  await mockSteps(onEvent, outcome.changed, true)
  mockApplied = mode
  mockStatus = r.status
  return outcome
}

async function mockPresetRevert(onEvent: (e: PresetEvent) => void): Promise<Outcome> {
  await loadMock()
  if (!mockApplied) {
    return {
      ok: false,
      mode: null,
      error: { kind: 'nothingToRevert', message: 'Tidak ada perubahan RepairDoc di PC ini yang perlu dikembalikan.', help: [] },
      entry: null,
      changed: 0,
      skipped: 0,
      failed: 0,
      failures: [],
      restorePoint: null,
      needsRestart: false,
    }
  }
  const r = presetData.after[mockApplied]
  await mockSteps(onEvent, r.revert.changed, false)
  mockStatus = r.statusAfterRevert
  mockApplied = null
  return structuredClone(r.revert)
}

// ---------------------------------------------------------------- Bersihkan File Sampah (tahap 4)

export async function cleanupStatus(): Promise<CleanStatus> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke<CleanStatus>('cleanup_status')
  }
  await loadMock()
  return structuredClone(mockCleanStatus)
}

export async function cleanupPreview(): Promise<{ preview: CleanPreview | null; error: PresetError | null }> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('cleanup_preview')
  }
  await loadMock()
  await sleep(1400)
  const p = structuredClone(presetData.cleanup.preview)
  // Setelah dibersihkan, angka jenis yang sudah dibersihkan ikut turun.
  for (const k of p.kinds) {
    if (mockCleaned.has(k.jenis)) {
      k.bytes = Math.round(k.bytes * 0.06)
      k.files = Math.round(k.files * 0.18)
    }
  }
  if (mockCleaned.has('windowsOld')) p.windowsOld = null
  return { preview: p, error: null }
}

export async function cleanupApply(request: CleanRequest, onEvent: (e: CleanEvent) => void): Promise<CleanOutcome> {
  if (isTauri()) {
    const { invoke, Channel } = await import('@tauri-apps/api/core')
    const channel = new Channel<CleanEvent>()
    channel.onmessage = onEvent
    return invoke<CleanOutcome>('cleanup_apply', { request, onEvent: channel })
  }
  await loadMock()
  // Urutan & aturan sama dengan Rust: Windows.old hanya lewat centangnya sendiri.
  const order: CleanJenis[] = ['temp', 'browserCache', 'updateCache', 'recycleBin']
  const chosen = order.filter((j) => request.kinds.includes(j))
  if (request.windowsOld) chosen.push('windowsOld')
  onEvent({ step: 'preparing' })
  await sleep(500)
  const records: KindRecord[] = []
  for (const [i, j] of chosen.entries()) {
    const r = structuredClone(presetData.cleanup.records[j])
    onEvent({ step: 'cleaning', jenis: j, label: r.label, index: i, total: chosen.length })
    await sleep(j === 'windowsOld' ? 2600 : 1100)
    records.push(r)
    mockCleaned.add(j)
  }
  onEvent({ step: 'saving' })
  await sleep(300)
  const entry: CleanEntry = { ...structuredClone(presetData.cleanup.entryTemplate), jenis: records }
  mockCleanStatus = { ...mockCleanStatus, last: entry }
  const sum = (f: (r: KindRecord) => number) => records.reduce((a, r) => a + f(r), 0)
  return {
    ok: true,
    error: null,
    entry,
    freedBytes: sum((r) => r.byteDibebaskan),
    deletedFiles: sum((r) => r.fileTerhapus),
    skippedFiles: sum((r) => r.dilewati),
    failures: [],
  }
}

let mockCleanStatus: CleanStatus
const mockCleaned = new Set<CleanJenis>()
