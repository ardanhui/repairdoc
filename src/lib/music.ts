// Musik latar RepairDoc — lagu orisinal yang dibuat khusus untuk aplikasi ini (lihat
// src/assets/LISENSI-MUSIK.md dan tools/musik-latar/). Bukan lagu pihak lain.
//
// - Diputar lewat Web Audio (bukan <audio loop>) supaya sambungan loop tanpa jeda.
// - Mulai setelah klik/tombol pertama: aturan browser melarang suara sebelum ada interaksi.
// - Volume pelan, bisa dimatikan dari tombol "Musik" di kepala aplikasi; pilihan diingat.
// - Berhenti saat jendela diminimize/tersembunyi, supaya tidak memakai prosesor percuma.
// - Musik hanya hiasan: galat apa pun diabaikan diam-diam, aplikasi tetap jalan.
import { ref } from 'vue'
import musikUrl from '../assets/musik-latar.ogg?url'

const KEY = 'repairdoc.musik'
/** 16 birama × 4 ketukan pada 108 BPM — sama dengan tools/musik-latar/buat_musik.py. */
const LOOP_SECONDS = (16 * 4 * 60) / 108
const VOLUME = 0.22

function readPref(): boolean {
  try {
    return localStorage.getItem(KEY) !== 'mati'
  } catch {
    return true
  }
}

function savePref(on: boolean) {
  try {
    localStorage.setItem(KEY, on ? 'nyala' : 'mati')
  } catch {
    /* penyimpanan tidak tersedia: pilihan berlaku sampai aplikasi ditutup */
  }
}

/** Pilihan pengguna (nyala/mati). */
export const musicOn = ref(readPref())

let ctx: AudioContext | null = null
let gain: GainNode | null = null
let buffer: AudioBuffer | null = null
let loading: Promise<void> | null = null
let source: AudioBufferSourceNode | null = null

function fade(to: number, seconds: number) {
  if (!ctx || !gain) return
  const t = ctx.currentTime
  gain.gain.cancelScheduledValues(t)
  gain.gain.setValueAtTime(gain.gain.value, t)
  gain.gain.linearRampToValueAtTime(to, t + seconds)
}

async function load(c: AudioContext) {
  if (buffer) return
  loading ??= (async () => {
    const res = await fetch(musikUrl)
    buffer = await c.decodeAudioData(await res.arrayBuffer())
  })()
  await loading
}

async function start() {
  if (!musicOn.value || source) return
  try {
    if (!ctx) {
      ctx = new AudioContext()
      gain = ctx.createGain()
      gain.gain.value = 0
      gain.connect(ctx.destination)
    }
    if (ctx.state === 'suspended') await ctx.resume()
    await load(ctx)
    // Pengguna bisa saja sudah mematikan musik selama file dimuat.
    if (!musicOn.value || source || !buffer || !gain) return
    const s = ctx.createBufferSource()
    s.buffer = buffer
    s.loop = true
    s.loopStart = 0
    s.loopEnd = Math.min(LOOP_SECONDS, buffer.duration)
    s.connect(gain)
    s.start()
    source = s
    fade(VOLUME, 1.5)
  } catch {
    source = null
  }
}

function stop() {
  const s = source
  if (!ctx || !s) return
  source = null
  fade(0, 0.6)
  window.setTimeout(() => {
    try {
      s.stop()
    } catch {
      /* sudah berhenti */
    }
    if (!source) void ctx?.suspend()
  }, 700)
}

export function toggleMusic() {
  musicOn.value = !musicOn.value
  savePref(musicOn.value)
  if (musicOn.value) void start()
  else stop()
}

/** Dipanggil sekali saat aplikasi mulai. */
export function setupMusic() {
  const first = () => {
    window.removeEventListener('pointerdown', first)
    window.removeEventListener('keydown', first)
    void start()
  }
  window.addEventListener('pointerdown', first)
  window.addEventListener('keydown', first)
  document.addEventListener('visibilitychange', () => {
    if (!ctx) return
    if (document.hidden) void ctx.suspend()
    else if (musicOn.value && source) void ctx.resume()
  })
}
