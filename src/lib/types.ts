// Cermin dari struktur data Rust (src-tauri/core/src/model.rs).

export type Status = 'sehat' | 'perhatian' | 'bermasalah'
export type Severity = 'ringan' | 'sedang' | 'berat'
export type Category = 'cpu' | 'memory' | 'storage' | 'gpu' | 'battery' | 'system' | 'programs' | 'cleanup' | 'security' | 'network'
export type Group = 'hardware' | 'windows' | 'cleanup' | 'security' | 'network'

export interface Finding {
  id: string
  category: Category
  status: Status
  severity: Severity | null
  title: string
  summary: string
  why: string | null
  steps: string[]
}

export interface TechItem {
  label: string
  value: string
}

export interface TechSection {
  title: string
  items: TechItem[]
}

export interface Verdict {
  status: Status
  headline: string
  detail: string
  topPriority: string | null
  problemCount: number
  attentionCount: number
  healthyCount: number
}

export interface ScanReport {
  verdict: Verdict
  findings: Finding[]
  tech: TechSection[]
  isAdmin: boolean
  needsAdmin: string[]
  finishedAtMs: number
  durationMs: number
}

export type ScanEvent =
  | { kind: 'categoryStarted'; category: Category }
  | { kind: 'categoryFinished'; category: Category; status: Status }
  | { kind: 'categoryFailed'; category: Category }
  | { kind: 'categorySkipped'; category: Category; reason: string }

export interface AppInfo {
  isAdmin: boolean
  version: string
  isWindows: boolean
}

export type CategoryState = 'menunggu' | 'berjalan' | 'selesai' | 'gagal' | 'dilewati'

// ---------------------------------------------------------------- Mode Performa (tahap 3)
// Cermin dari src-tauri/core/src/preset/model.rs dan engine.rs.

export type PresetMode = 'kentang' | 'seimbang' | 'normal'
export type PlanPath = 'apply' | 'revertLog' | 'defaults'
export type Effect = 'langsung' | 'setelahSignOut' | 'setelahRestart'
export type ChangeStatus = 'berhasil' | 'gagal' | 'dilewati'

export interface ModeInfo {
  mode: PresetMode
  title: string
  level: string
  tagline: string
  forWhom: string
  tweakIds: string[]
  bullets: string[]
}

export interface TweakInfo {
  id: string
  title: string
  description: string
  modes: PresetMode[]
  effect: Effect
}

export interface ChangeRecord {
  tweakId: string
  itemKey: string
  label: string
  targetTeks: string
  status: ChangeStatus
  keterangan: string | null
}

export interface LogEntry {
  id: string
  namaPc: string
  waktu: number
  mode: PresetMode
  jenis: 'terapkan' | 'kembalikan'
  restorePoint: string | null
  perubahan: ChangeRecord[]
  dikembalikan: boolean
  waktuDikembalikan: number | null
}

export interface PresetStatus {
  isWindows: boolean
  isAdmin: boolean
  machineName: string | null
  activeMode: PresetMode
  activeSince: number | null
  lastEntry: LogEntry | null
  revertTarget: LogEntry | null
  logPath: string
  modes: ModeInfo[]
  tweaks: TweakInfo[]
  error: string | null
}

export interface PreviewChange {
  key: string
  itemKey: string
  label: string
  targetText: string
  current: string
  new: string
  willChange: boolean
  optional: boolean
  note: string | null
}

export interface PreviewGroup {
  tweakId: string
  title: string
  description: string
  skippedByUser: boolean
  changes: PreviewChange[]
}

export interface Preview {
  mode: PresetMode
  path: PlanPath
  groups: PreviewGroup[]
  changeCount: number
  sameCount: number
  notes: string[]
  restorePointName: string
}

export type PresetErrorKind = 'notAdmin' | 'notWindows' | 'restorePoint' | 'nothingToRevert' | 'other'

export interface PresetError {
  kind: PresetErrorKind
  message: string
  help: string[]
}

export interface Outcome {
  ok: boolean
  mode: PresetMode | null
  error: PresetError | null
  entry: LogEntry | null
  changed: number
  skipped: number
  failed: number
  failures: { label: string; reason: string }[]
  restorePoint: string | null
  needsRestart: boolean
}

export type PresetEvent = { step: 'reading' } | { step: 'restorePoint' } | { step: 'applying'; total: number } | { step: 'saving' }

export interface PresetRequest {
  disabledTweaks: string[]
  skipItems: string[]
}
