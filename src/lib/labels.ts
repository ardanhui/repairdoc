import type { Category, Effect, Group, PresetMode, Severity, Status } from './types'

/** Kategori halus (urutan = urutan scan di backend). */
export const categoryOrder: Category[] = ['cpu', 'memory', 'storage', 'gpu', 'battery', 'system', 'programs', 'cleanup', 'security', 'network']

export const categoryInfo: Record<Category, { name: string; doing: string; group: Group }> = {
  cpu: { name: 'Prosesor', doing: 'Memeriksa prosesor', group: 'hardware' },
  memory: { name: 'Memori (RAM)', doing: 'Memeriksa memori', group: 'hardware' },
  storage: { name: 'Penyimpanan', doing: 'Memeriksa penyimpanan', group: 'hardware' },
  gpu: { name: 'Kartu grafis', doing: 'Memeriksa kartu grafis', group: 'hardware' },
  battery: { name: 'Baterai', doing: 'Memeriksa baterai', group: 'hardware' },
  system: { name: 'Windows', doing: 'Memeriksa Windows', group: 'windows' },
  programs: { name: 'Program', doing: 'Memeriksa program', group: 'windows' },
  cleanup: { name: 'Kebersihan', doing: 'Menghitung file sampah', group: 'cleanup' },
  security: { name: 'Keamanan', doing: 'Memeriksa antivirus', group: 'security' },
  network: { name: 'Jaringan', doing: 'Mengetes internet', group: 'network' },
}

/** Kelompok besar yang dilihat pengguna di layar scan. */
export const groupOrder: Group[] = ['hardware', 'windows', 'cleanup', 'security', 'network']

export const groupInfo: Record<Group, { name: string; hint: string }> = {
  // Pendek supaya muat utuh (tanpa "…") di kartu kategori layar awal.
  hardware: { name: 'Hardware', hint: 'Prosesor, RAM, disk, baterai' },
  windows: { name: 'Windows', hint: 'Versi, update, driver' },
  cleanup: { name: 'Kebersihan', hint: 'File sampah' },
  security: { name: 'Keamanan', hint: 'Antivirus & firewall' },
  network: { name: 'Jaringan', hint: 'Internet & Wi-Fi' },
}

export const categoriesOf = (g: Group): Category[] => categoryOrder.filter((c) => categoryInfo[c].group === g)

export const statusWord: Record<Status, string> = {
  sehat: 'Sehat',
  perhatian: 'Perhatian',
  bermasalah: 'Bermasalah',
}

/** "Masalah berat", "Perhatian ringan" — label keparahan di kartu temuan. */
export function severityLabel(status: Status, severity: Severity | null): string {
  if (!severity) return statusWord[status]
  return `${status === 'bermasalah' ? 'Masalah' : 'Perhatian'} ${severity}`
}

export const severityLevel: Record<Severity, number> = { ringan: 1, sedang: 2, berat: 3 }

export function finishedText(ms: number, durationMs: number): string {
  const d = new Date(ms)
  const jam = d.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' })
  const detik = Math.max(1, Math.round(durationMs / 1000))
  return `Diperiksa pukul ${jam}, selesai dalam ${detik} detik.`
}

// ---------------------------------------------------------------- Mode Performa

export const modeName: Record<PresetMode, string> = {
  kentang: 'Kentang',
  seimbang: 'Seimbang',
  normal: 'Normal',
}

export const effectText: Record<Effect, string> = {
  langsung: 'Terasa langsung',
  setelahSignOut: 'Terasa setelah sign out',
  setelahRestart: 'Terasa setelah restart',
}

export function waktuText(ms: number): string {
  return new Date(ms).toLocaleString('id-ID', { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}

// ---------------------------------------------------------------- Bersihkan File Sampah

const angka = (n: number, digits: number) => n.toLocaleString('id-ID', { maximumFractionDigits: digits })

/** Ukuran untuk pengguna awam: "1,3 GB", "640 MB", "12 KB". */
export function sizeText(bytes: number): string {
  const KB = 1024
  const MB = KB * 1024
  const GB = MB * 1024
  if (bytes >= GB) return `${angka(bytes / GB, 1)} GB`
  if (bytes >= MB) return `${angka(bytes / MB, bytes >= 100 * MB ? 0 : 1)} MB`
  if (bytes >= KB) return `${angka(bytes / KB, 0)} KB`
  return `${bytes} byte`
}

export function countText(n: number): string {
  return angka(n, 0)
}

export function tanggalText(ms: number): string {
  return new Date(ms).toLocaleDateString('id-ID', { day: 'numeric', month: 'long', year: 'numeric' })
}
