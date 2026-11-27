<script setup lang="ts">
// Latar Frutiger Aero versi "bersih": putih-biru muda yang terang ala menu konsol game era
// 2010-an — cahaya biru lembut di sudut, berkas cahaya diagonal, gelembung kaca yang
// melayang, dan ombak air mengkilap di bawah. Dipilih pemilik dari 3 alternatif
// (langit-aurora, bersih, bawah air).
// Statis (tanpa animasi, tanpa filter blur) supaya ringan di PC kentang.
// Zona aman: tidak ada gelembung di 36% atas bagian kiri-tengah (judul & tombol).
type Dot = { x: number; y: number; s: number; o?: number }
const glows: Dot[] = [
  { x: 92, y: 10, s: 520, o: 0.9 },
  { x: 2, y: 62, s: 560, o: 0.75 },
  { x: 60, y: 96, s: 700, o: 0.6 },
]
const bubbles: Dot[] = [
  { x: 88, y: 18, s: 34 },
  { x: 94, y: 30, s: 64 },
  { x: 83, y: 33, s: 16 },
  { x: 4, y: 47, s: 70 },
  { x: 11, y: 58, s: 28 },
  { x: 3, y: 66, s: 16 },
  { x: 91, y: 52, s: 40 },
  { x: 97, y: 60, s: 20 },
  { x: 18, y: 72, s: 14 },
  { x: 78, y: 70, s: 22 },
]
const sparkles: Dot[] = [
  { x: 89.5, y: 26, s: 16 },
  { x: 7, y: 43, s: 14 },
  { x: 72, y: 76, s: 14 },
  { x: 26, y: 80, s: 12 },
]
</script>

<template>
  <div class="aero-bg" aria-hidden="true">
    <span
      v-for="(g, i) in glows"
      :key="`g${i}`"
      class="glow"
      :style="{ left: `${g.x}%`, top: `${g.y}%`, width: `${g.s}px`, height: `${g.s}px`, opacity: g.o ?? 1 }"
    />
    <div class="beams" />
    <span
      v-for="(b, i) in bubbles"
      :key="`b${i}`"
      class="bubble"
      :style="{ left: `${b.x}%`, top: `${b.y}%`, width: `${b.s}px`, height: `${b.s}px` }"
    />

    <svg class="waves" viewBox="0 0 1440 320" preserveAspectRatio="none">
      <defs>
        <linearGradient id="br-w1" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#b9ecff" />
          <stop offset="1" stop-color="#5cc6f5" />
        </linearGradient>
        <linearGradient id="br-w2" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#6fd0fb" />
          <stop offset="0.5" stop-color="#22a6e8" />
          <stop offset="1" stop-color="#0b78c4" />
        </linearGradient>
        <linearGradient id="br-shine" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#fff" stop-opacity="0.8" />
          <stop offset="1" stop-color="#fff" stop-opacity="0" />
        </linearGradient>
      </defs>
      <path d="M0 150 C 240 90, 470 96, 720 150 S 1180 214, 1440 120 L1440 320 L0 320 Z" fill="url(#br-w1)" opacity="0.75" />
      <path d="M0 150 C 240 90, 470 96, 720 150 S 1180 214, 1440 120" fill="none" stroke="#fff" stroke-opacity="0.9" stroke-width="2.5" />
      <path d="M0 220 C 280 168, 540 176, 800 222 S 1230 270, 1440 196 L1440 320 L0 320 Z" fill="url(#br-w2)" />
      <path d="M0 220 C 280 168, 540 176, 800 222 S 1230 270, 1440 196 L1440 226 C 1230 300, 960 262, 800 248 S 280 200, 0 252 Z" fill="url(#br-shine)" opacity="0.7" />
      <path d="M0 220 C 280 168, 540 176, 800 222 S 1230 270, 1440 196" fill="none" stroke="#fff" stroke-opacity="0.95" stroke-width="3" />
      <path d="M0 268 C 300 236, 600 250, 880 272 S 1260 300, 1440 262" fill="none" stroke="#fff" stroke-opacity="0.35" stroke-width="2" />
    </svg>

    <svg
      v-for="(s, i) in sparkles"
      :key="`s${i}`"
      class="sparkle"
      viewBox="0 0 20 20"
      :style="{ left: `${s.x}%`, top: `${s.y}%`, width: `${s.s}px`, height: `${s.s}px` }"
    >
      <path d="M10 0 C 10.8 7, 13 9.2, 20 10 C 13 10.8, 10.8 13, 10 20 C 9.2 13, 7 10.8, 0 10 C 7 9.2, 9.2 7, 10 0 Z" fill="#7fd6ff" />
    </svg>
  </div>
</template>

<style scoped>
.aero-bg {
  position: fixed;
  inset: 0;
  z-index: -1;
  overflow: hidden;
  pointer-events: none;
  background: linear-gradient(180deg, #ffffff 0%, #f4fbff 30%, #e4f6ff 62%, #d2efff 100%);
}

/* Cahaya biru lembut di sudut (tanpa filter blur: hanya gradasi radial). */
.glow {
  position: absolute;
  border-radius: 50%;
  transform: translate(-50%, -50%);
  background: radial-gradient(circle, rgba(126, 214, 255, 0.45) 0%, rgba(126, 214, 255, 0.18) 40%, rgba(126, 214, 255, 0) 70%);
}

/* Berkas cahaya diagonal yang sangat halus. */
.beams {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(115deg, rgba(255, 255, 255, 0) 30%, rgba(255, 255, 255, 0.7) 38%, rgba(255, 255, 255, 0) 46%),
    linear-gradient(115deg, rgba(255, 255, 255, 0) 52%, rgba(255, 255, 255, 0.55) 57%, rgba(255, 255, 255, 0) 62%);
}

/* Gelembung kaca biru: lebih "berisi" dari versi langit supaya terlihat di latar putih. */
.bubble {
  position: absolute;
  border-radius: 50%;
  transform: translate(-50%, -50%);
  background:
    radial-gradient(ellipse 36% 24% at 35% 26%, rgba(255, 255, 255, 0.98), rgba(255, 255, 255, 0)),
    radial-gradient(circle at 70% 76%, rgba(255, 255, 255, 0.85) 0 5%, rgba(255, 255, 255, 0) 12%),
    radial-gradient(circle, rgba(160, 228, 255, 0.18) 50%, rgba(90, 200, 250, 0.45) 72%, rgba(40, 165, 230, 0.6) 88%, rgba(150, 225, 255, 0.5) 100%);
  box-shadow:
    inset 0 -6px 12px rgba(30, 160, 230, 0.3),
    0 8px 16px -8px rgba(10, 100, 170, 0.35);
}

.waves {
  position: absolute;
  left: 0;
  bottom: 0;
  width: 100%;
  height: 30vh;
  min-height: 170px;
}

.sparkle {
  position: absolute;
  transform: translate(-50%, -50%);
}
</style>
