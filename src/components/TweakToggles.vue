<script setup lang="ts">
import { effectText, modeName } from '../lib/labels'
import type { TweakInfo } from '../lib/types'

const props = defineProps<{ tweaks: TweakInfo[]; disabled: string[] }>()
const emit = defineEmits<{ 'update:disabled': [value: string[]] }>()

function toggle(id: string) {
  const off = props.disabled.includes(id)
  emit('update:disabled', off ? props.disabled.filter((d) => d !== id) : [...props.disabled, id])
}
</script>

<template>
  <details class="tweaks glass">
    <summary>
      <span class="sum-head">
        <svg class="chev" viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5L10 8l-5 4.5" /></svg>
        Atur satu per satu
        <span v-if="disabled.length" class="off-count">{{ disabled.length }} dimatikan</span>
      </span>
      <span class="sum-desc">Pilih pengaturan mana yang boleh diubah RepairDoc. Semuanya dipakai, kecuali yang kamu matikan di sini.</span>
      <span class="sum-state">
        {{ disabled.length ? `${disabled.length} dari ${tweaks.length} pengaturan dimatikan.` : `Semua ${tweaks.length} pengaturan dipakai.` }}
      </span>
      <span class="btn sum-btn" aria-hidden="true">Tampilkan daftar pengaturan</span>
    </summary>
    <p class="note">Pilihan ini berlaku saat menerapkan Mode Kentang dan Mode Seimbang.</p>
    <ul>
      <li v-for="t in tweaks" :key="t.id">
        <label class="row" :class="{ off: disabled.includes(t.id) }">
          <input type="checkbox" role="switch" :checked="!disabled.includes(t.id)" @change="toggle(t.id)" />
          <span class="switch" aria-hidden="true" />
          <span class="text">
            <strong>{{ t.title }} <em v-if="disabled.includes(t.id)" class="off-tag">Tidak dipakai</em></strong>
            <span>{{ t.description }}</span>
          </span>
          <span class="meta">
            <span class="modes">
              <span v-for="m in t.modes" :key="m" class="chip">{{ modeName[m] }}</span>
            </span>
            <span class="when">{{ effectText[t.effect] }}</span>
          </span>
        </label>
      </li>
    </ul>
  </details>
</template>

<style scoped>
.tweaks {
  padding: 0 20px;
}

summary {
  display: grid;
  gap: 4px;
  cursor: pointer;
  padding: 16px 0;
}

.sum-head {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
  color: var(--aqua-ink);
}

.chev {
  width: 14px;
  height: 14px;
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 2.2;
  stroke-linecap: round;
  stroke-linejoin: round;
  transition: transform 0.15s;
}

.tweaks[open] .chev {
  transform: rotate(90deg);
}

summary::-webkit-details-marker {
  display: none;
}

.sum-state {
  font-size: 14px;
  font-weight: 600;
}

/* Penanda aksi yang jelas (seluruh judul tetap bisa diklik). Disembunyikan saat terbuka. */
.sum-btn {
  justify-self: start;
  margin-top: 8px;
  pointer-events: none;
}

.tweaks[open] .sum-btn {
  display: none;
}

.sum-desc {
  font-size: 14px;
  color: var(--ink-soft);
}

.off-count,
.off-tag {
  padding: 1px 10px;
  border-radius: 999px;
  background: var(--warn-soft);
  color: var(--warn-ink);
  font-size: 13px;
  font-style: normal;
  font-weight: 600;
}

.tweaks[open] {
  padding-bottom: 12px;
}

.note {
  font-size: 14px;
  color: var(--ink-soft);
  margin-bottom: 6px;
}

ul {
  list-style: none;
  padding: 0;
}

li + li {
  border-top: 1px solid rgba(64, 150, 205, 0.18);
}

.row {
  position: relative;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
  padding: 12px 0;
  cursor: pointer;
}

.row input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
}

/* Saklar kaca */
.switch {
  position: relative;
  width: 46px;
  height: 26px;
  border-radius: 999px;
  border: 1px solid #0a5c9e;
  background: linear-gradient(180deg, #167fc6, #0a5c9e);
  box-shadow: inset 0 2px 4px rgba(0, 40, 90, 0.25);
  transition: background 0.15s;
}

.switch::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 22px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: radial-gradient(circle at 50% 30%, #fff, #e3f4fd);
  box-shadow: 0 1px 3px rgba(0, 40, 90, 0.4);
  transition: left 0.15s;
}

.row.off .switch {
  background: linear-gradient(180deg, #e3eef5, #c9d9e4);
  border-color: #a9c3d4;
}

.row.off .switch::after {
  left: 2px;
}

.row input:focus-visible + .switch {
  outline: 3px solid var(--aqua-ink);
  outline-offset: 3px;
}

.text {
  display: grid;
  line-height: 1.35;
}

.text strong {
  font-weight: 600;
}

.text span {
  font-size: 14px;
  color: var(--ink-soft);
}

/* Saklar mati ditandai lewat saklar + label "Tidak dipakai", bukan teks yang dipudarkan. */

.meta {
  display: grid;
  justify-items: end;
  gap: 4px;
}

.modes {
  display: flex;
  gap: 4px;
}

.chip {
  padding: 1px 9px;
  border-radius: 999px;
  background: #eaf7ff;
  border: 1px solid #b9dff3;
  color: var(--aqua-ink);
  font-size: 13px;
  font-weight: 600;
}

.when {
  font-size: 13px;
  color: var(--ink-soft);
  white-space: nowrap;
}

@media (max-width: 720px) {
  .row {
    grid-template-columns: auto minmax(0, 1fr);
  }

  .meta {
    grid-column: 2;
    justify-items: start;
  }
}
</style>
