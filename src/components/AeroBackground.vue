<script setup lang="ts">
// Latar Frutiger Aero: langit, matahari & lens flare lembut, awan, bukit hijau, gelembung.
// Semuanya statis (tanpa animasi) supaya tidak membebani PC lambat.
const bubbles = [
  { x: 6, y: 58, s: 54 },
  { x: 11, y: 48, s: 22 },
  { x: 3, y: 40, s: 14 },
  { x: 90, y: 30, s: 64 },
  { x: 85, y: 44, s: 26 },
  { x: 94, y: 52, s: 16 },
  { x: 72, y: 12, s: 18 },
  { x: 24, y: 16, s: 12 },
]
</script>

<template>
  <div class="aero-bg" aria-hidden="true">
    <div class="sun" />
    <div class="flare f1" />
    <div class="flare f2" />
    <div class="flare f3" />
    <div class="cloud c1" />
    <div class="cloud c2" />
    <div class="cloud c3" />
    <span
      v-for="(b, i) in bubbles"
      :key="i"
      class="bubble"
      :style="{ left: `${b.x}%`, top: `${b.y}%`, width: `${b.s}px`, height: `${b.s}px` }"
    />
    <svg class="hills" viewBox="0 0 1440 300" preserveAspectRatio="none">
      <defs>
        <linearGradient id="hill-back" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#bdeb93" />
          <stop offset="1" stop-color="#74c64c" />
        </linearGradient>
        <linearGradient id="hill-front" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#93dc62" />
          <stop offset="0.5" stop-color="#52ae37" />
          <stop offset="1" stop-color="#2d8626" />
        </linearGradient>
      </defs>
      <path d="M0 140 C 230 62, 430 66, 650 124 S 1080 196, 1440 84 L1440 300 L0 300 Z" fill="url(#hill-back)" />
      <path d="M0 140 C 230 62, 430 66, 650 124 S 1080 196, 1440 84" fill="none" stroke="rgba(255,255,255,0.6)" stroke-width="2" />
      <path d="M0 206 C 250 140, 520 146, 770 196 S 1190 250, 1440 166 L1440 300 L0 300 Z" fill="url(#hill-front)" />
      <path d="M0 206 C 250 140, 520 146, 770 196 S 1190 250, 1440 166" fill="none" stroke="rgba(255,255,255,0.65)" stroke-width="3" />
    </svg>
    <svg class="sprig" viewBox="0 0 120 120">
      <path d="M18 116 C 34 82, 50 60, 86 30" fill="none" stroke="#2d8a27" stroke-width="4" stroke-linecap="round" />
      <path d="M86 30 C 104 8, 116 14, 116 14 C 116 14, 112 40, 86 30 Z" fill="#6cc84a" stroke="#fff" stroke-width="2" />
      <path d="M50 66 C 46 40, 64 26, 64 26 C 64 26, 76 52, 50 66 Z" fill="#59b83c" stroke="#fff" stroke-width="2" />
      <path d="M38 84 C 14 80, 8 60, 8 60 C 8 60, 34 56, 38 84 Z" fill="#7ad457" stroke="#fff" stroke-width="2" />
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
  background: linear-gradient(180deg, #1677c9 0%, #2e98e2 9%, #5bbdf2 24%, #a6defa 46%, #dff5ff 66%, #effaff 80%);
}

.sun {
  position: absolute;
  left: -160px;
  top: -200px;
  width: 620px;
  height: 620px;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.95) 0%, rgba(255, 254, 228, 0.55) 16%, rgba(255, 255, 255, 0.12) 42%, rgba(255, 255, 255, 0) 62%);
}

.flare {
  position: absolute;
  border-radius: 50%;
}

.f1 {
  left: 22%;
  top: 15%;
  width: 70px;
  height: 70px;
  background: radial-gradient(circle, rgba(255, 255, 255, 0) 52%, rgba(214, 255, 236, 0.45) 68%, rgba(255, 255, 255, 0) 74%);
}

.f2 {
  left: 30%;
  top: 25%;
  width: 26px;
  height: 26px;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.4), rgba(255, 255, 255, 0) 70%);
}

.f3 {
  left: 15%;
  top: 8%;
  width: 120px;
  height: 120px;
  background: radial-gradient(circle, rgba(255, 255, 255, 0) 58%, rgba(255, 246, 206, 0.28) 66%, rgba(255, 255, 255, 0) 72%);
}

.cloud {
  position: absolute;
  width: 380px;
  height: 140px;
  background:
    radial-gradient(closest-side, rgba(255, 255, 255, 0.95), rgba(255, 255, 255, 0)) 0 46px / 210px 92px no-repeat,
    radial-gradient(closest-side, rgba(255, 255, 255, 0.98), rgba(255, 255, 255, 0)) 92px 0 / 190px 130px no-repeat,
    radial-gradient(closest-side, rgba(255, 255, 255, 0.92), rgba(255, 255, 255, 0)) 178px 40px / 200px 96px no-repeat;
}

.c1 {
  right: 6%;
  top: 9%;
}

.c2 {
  left: 38%;
  top: 4%;
  transform: scale(0.6);
  opacity: 0.85;
}

.c3 {
  left: -60px;
  top: 34%;
  transform: scale(0.8);
  opacity: 0.8;
}

.bubble {
  position: absolute;
  border-radius: 50%;
  background:
    radial-gradient(circle at 32% 28%, rgba(255, 255, 255, 0.95) 0 7%, rgba(255, 255, 255, 0) 16%),
    radial-gradient(circle, rgba(255, 255, 255, 0.04) 55%, rgba(255, 255, 255, 0.55) 72%, rgba(186, 238, 255, 0.75) 100%);
  box-shadow: inset 0 -4px 10px rgba(110, 215, 255, 0.4);
}

.hills {
  position: absolute;
  left: 0;
  bottom: 0;
  width: 100%;
  height: 30vh;
  min-height: 170px;
}

.sprig {
  position: absolute;
  right: 2%;
  bottom: 3vh;
  width: 120px;
  height: 120px;
  opacity: 0.95;
}
</style>
