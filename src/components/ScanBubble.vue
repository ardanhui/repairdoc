<script setup lang="ts">
// Gelembung utama. Diam: tombol "Mulai Scan". Saat scan: gelembung kaca yang
// terisi air sesuai kemajuan pemeriksaan.
const props = defineProps<{ scanning: boolean; level: number; label: string }>()
defineEmits<{ start: [] }>()

const pct = () => Math.round(Math.min(1, Math.max(0, props.level)) * 100)
</script>

<template>
  <button v-if="!scanning" type="button" class="bubble-btn" @click="$emit('start')">
    <span class="bubble-text">Mulai Scan</span>
  </button>

  <div
    v-else
    class="bubble-glass"
    role="progressbar"
    :aria-valuenow="pct()"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-valuetext="label"
  >
    <div class="water" :style="{ height: `${pct()}%` }">
      <svg class="wave" viewBox="0 0 200 16" preserveAspectRatio="none" aria-hidden="true">
        <path d="M0 8 Q 12.5 0 25 8 T 50 8 T 75 8 T 100 8 T 125 8 T 150 8 T 175 8 T 200 8 V16 H0 Z" />
      </svg>
    </div>
    <span class="shine" aria-hidden="true" />
  </div>
</template>

<style scoped>
.bubble-btn,
.bubble-glass {
  position: relative;
  width: 236px;
  height: 236px;
  border-radius: 50%;
}

/* ---- Tombol: bola air biru mengkilap ---- */
.bubble-btn {
  display: grid;
  place-items: center;
  border: 3px solid rgba(255, 255, 255, 0.95);
  cursor: pointer;
  /* Biru di belakang tulisan cukup gelap: teks putih ≥ 3:1 (teks besar) di titik terburuk. */
  background: radial-gradient(circle at 50% 118%, #8eeaff 0%, #1c98de 34%, #0a6ab3 72%, #075291 100%);
  box-shadow:
    0 0 0 8px rgba(255, 255, 255, 0.4),
    0 22px 44px -14px rgba(6, 80, 145, 0.7),
    inset 0 -18px 30px rgba(150, 240, 255, 0.55),
    inset 0 8px 16px rgba(0, 40, 90, 0.25);
  transition: transform 0.15s ease, filter 0.15s ease;
}

.bubble-btn::before {
  content: "";
  position: absolute;
  top: 5%;
  left: 13%;
  width: 74%;
  height: 46%;
  border-radius: 50% 50% 46% 46% / 62% 62% 38% 38%;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(255, 255, 255, 0.06));
}

.bubble-btn::after {
  content: "";
  position: absolute;
  bottom: 12%;
  left: 30%;
  width: 40%;
  height: 12%;
  border-radius: 50%;
  background: radial-gradient(closest-side, rgba(210, 255, 255, 0.7), rgba(210, 255, 255, 0));
}

.bubble-btn:hover {
  transform: translateY(-2px);
  filter: brightness(1.06) saturate(1.08);
}

.bubble-btn:active {
  transform: translateY(1px) scale(0.985);
}

.bubble-btn:focus-visible {
  outline: 4px solid var(--aqua-ink);
  outline-offset: 10px;
}

.bubble-text {
  position: relative;
  z-index: 1;
  /* Sedikit di bawah pusat lingkaran (kilau besar di atas menggeser pusat visual),
     tapi tidak sejauh dulu (26 px) yang membuatnya tampak turun. */
  margin-top: 18px;
  color: #fff;
  font-size: 26px;
  font-weight: 600;
  letter-spacing: -0.01em;
  text-shadow: 0 1px 3px rgba(0, 45, 95, 0.6);
}

/* ---- Saat scan: kaca yang terisi air ---- */
.bubble-glass {
  overflow: hidden;
  border: 3px solid rgba(255, 255, 255, 0.98);
  background: radial-gradient(circle at 50% 30%, rgba(255, 255, 255, 0.92), rgba(222, 245, 255, 0.85) 58%, rgba(170, 224, 250, 0.9) 100%);
  box-shadow:
    0 0 0 8px rgba(255, 255, 255, 0.4),
    0 22px 44px -14px rgba(6, 80, 145, 0.55),
    inset 0 -12px 26px rgba(40, 150, 220, 0.3);
}

.water {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(180deg, #4cc4f5 0%, #1797dc 45%, #0a6cae 100%);
  transition: height 0.9s cubic-bezier(0.3, 0.7, 0.3, 1);
}

.wave {
  position: absolute;
  left: 0;
  bottom: calc(100% - 1px);
  width: 200%;
  height: 12px;
  fill: #4cc4f5;
  animation: wave 2.4s linear infinite;
}

@keyframes wave {
  to {
    transform: translateX(-50%);
  }
}

.shine {
  position: absolute;
  top: 5%;
  left: 13%;
  width: 74%;
  height: 44%;
  border-radius: 50% 50% 46% 46% / 62% 62% 38% 38%;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.85), rgba(255, 255, 255, 0));
  pointer-events: none;
}
</style>
