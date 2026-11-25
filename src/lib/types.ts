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
