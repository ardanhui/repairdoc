// Jembatan ke backend Rust. Kalau tampilan dibuka di browser biasa
// (`npm run dev` tanpa Tauri), dipakai data contoh supaya desain bisa dikerjakan
// tanpa harus build Rust.
import type { AppInfo, Category, ScanEvent, ScanReport } from './types'
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
