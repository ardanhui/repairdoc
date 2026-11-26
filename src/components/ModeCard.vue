<script setup lang="ts">
import type { ModeInfo, PresetMode } from '../lib/types'
import Icon from './Icon.vue'

defineProps<{ info: ModeInfo; active: boolean; disabled: boolean; enabledCount: number }>()
defineEmits<{ apply: [] }>()

// Seperti pengaturan grafis game: makin ringan, makin sederhana tampilannya.
const meters: Record<PresetMode, { ringan: number; tampilan: number }> = {
  kentang: { ringan: 3, tampilan: 1 },
  seimbang: { ringan: 2, tampilan: 2 },
  normal: { ringan: 1, tampilan: 3 },
}
const icons = { kentang: 'bolt', seimbang: 'balance', normal: 'refresh' } as const
</script>

<template>
  <article class="mode glass" :class="[info.mode, { active }]" :aria-current="active ? 'true' : undefined">
    <header class="top">
      <span class="mode-orb" aria-hidden="true"><Icon :name="icons[info.mode]" /></span>
      <h3>{{ info.title }}</h3>
      <span class="level">{{ info.level }}</span>
    </header>
    <p v-if="active" class="badge">Aktif</p>
    <p class="tagline">{{ info.tagline }}</p>

    <dl class="meters">
      <div>
        <dt>Ringan</dt>
        <dd :aria-label="`${meters[info.mode].ringan} dari 3`">
          <span v-for="n in 3" :key="n" class="bar" :class="{ on: n <= meters[info.mode].ringan }" />
        </dd>
      </div>
      <div>
        <dt>Tampilan</dt>
        <dd :aria-label="`${meters[info.mode].tampilan} dari 3`">
          <span v-for="n in 3" :key="n" class="bar look" :class="{ on: n <= meters[info.mode].tampilan }" />
        </dd>
      </div>
    </dl>

    <p class="for">Cocok untuk: {{ info.forWhom }}</p>
    <ul class="bullets">
      <li v-for="b in info.bullets" :key="b">{{ b }}</li>
    </ul>
    <p v-if="info.mode !== 'normal'" class="count">{{ enabledCount }} dari {{ info.tweakIds.length }} pengaturan dipakai</p>

    <button type="button" class="btn" :class="{ 'btn-primary': !active }" :disabled="disabled" @click="$emit('apply')">Terapkan</button>
  </article>
</template>

<style scoped>
.mode {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 20px 20px 18px;
  border-radius: 20px;
}

.mode.active {
  box-shadow:
    0 0 0 3px var(--ok),
    inset 0 1px 0 #fff,
    0 16px 32px -16px rgba(9, 76, 130, 0.5);
}

.top {
  display: flex;
  align-items: center;
  gap: 12px;
}

.mode-orb {
  position: relative;
  display: grid;
  place-items: center;
  flex: none;
  width: 46px;
  height: 46px;
  border-radius: 50%;
  color: #fff;
  background: radial-gradient(circle at 50% 115%, #8ee6ff 0%, #23a1e3 50%, #0a69ab 100%);
  box-shadow: 0 0 0 2px #fff, 0 3px 8px -2px rgba(10, 70, 120, 0.4);
}

.kentang .mode-orb {
  background: radial-gradient(circle at 50% 115%, #ffe08a 0%, #f0a10e 50%, #c46f00 100%);
}

.normal .mode-orb {
  background: radial-gradient(circle at 50% 115%, #a3ec80 0%, var(--ok) 50%, #1f7a25 100%);
}

.mode-orb::before {
  content: "";
  position: absolute;
  top: 5%;
  left: 16%;
  width: 68%;
  height: 45%;
  border-radius: 50%;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.8), rgba(255, 255, 255, 0.05));
}

.mode-orb svg {
  position: relative;
  width: 24px;
  height: 24px;
}

h3 {
  flex: 1;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.level {
  padding: 2px 10px;
  border-radius: 999px;
  border: 1px solid #8cc9ea;
  background: #eaf7ff;
  color: var(--aqua-ink);
  font-size: 13px;
  font-weight: 600;
}

.badge {
  align-self: flex-start;
  padding: 3px 12px;
  border-radius: 999px;
  background: var(--ok-soft);
  color: var(--ok-ink);
  font-size: 13px;
  font-weight: 700;
}

.tagline {
  font-size: 16px;
  line-height: 1.4;
}

.meters {
  display: grid;
  gap: 6px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(225, 244, 255, 0.7);
}

.meters div {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

dt {
  font-size: 13px;
  color: var(--ink-soft);
}

dd {
  display: flex;
  gap: 4px;
}

.bar {
  width: 26px;
  height: 8px;
  border-radius: 999px;
  background: rgba(64, 150, 205, 0.18);
}

.bar.on {
  background: linear-gradient(180deg, #7fd8ff, #129bdb);
}

.bar.look.on {
  background: linear-gradient(180deg, #a6e98a, #34a33b);
}

.for {
  font-size: 14px;
  color: var(--ink-soft);
}

.bullets {
  display: grid;
  gap: 3px;
  padding-left: 18px;
  font-size: 14px;
}

.count {
  margin-top: auto;
  font-size: 13px;
  color: var(--ink-soft);
}

.mode .btn {
  align-self: stretch;
}

.normal .bullets {
  margin-bottom: auto;
}
</style>
