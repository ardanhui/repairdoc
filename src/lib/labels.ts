import type { Category, Group, Severity, Status } from './types'

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
  hardware: { name: 'Hardware', hint: 'Prosesor, memori, penyimpanan, grafis, baterai' },
  windows: { name: 'Windows', hint: 'Versi, update, driver, program saat menyala' },
  cleanup: { name: 'Kebersihan', hint: 'File sampah dan sisa-sisa' },
  security: { name: 'Keamanan', hint: 'Antivirus, firewall, program mencurigakan' },
  network: { name: 'Jaringan', hint: 'Internet, Wi-Fi, kecepatan unduh' },
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
