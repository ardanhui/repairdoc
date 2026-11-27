<script setup lang="ts">
import { ref } from 'vue'
import { severityLabel, severityLevel } from '../lib/labels'
import type { Finding } from '../lib/types'
import StatusOrb from './StatusOrb.vue'

const props = withDefaults(defineProps<{ finding: Finding; defaultOpen?: boolean }>(), { defaultOpen: false })
defineEmits<{ bersihkan: [] }>()
/** Temuan kebersihan (file sampah, Windows.old) punya tombol ke layar Bersihkan (tahap 4). */
const cleanable = props.finding.category === 'cleanup' && props.finding.status !== 'sehat'
const open = ref(props.defaultOpen)
const bodyId = `detail-${props.finding.id.replace(/[^a-zA-Z0-9_-]/g, '-')}`
const hasBody = !!props.finding.why || props.finding.steps.length > 0
</script>

<template>
  <article class="finding glass" :class="finding.status">
    <div class="head">
      <StatusOrb :status="finding.status" :size="36" />
      <h3>{{ finding.title }}</h3>
      <p v-if="finding.severity" class="severity">
        <span class="drops" aria-hidden="true">
          <span v-for="n in 3" :key="n" class="drop" :class="{ on: n <= severityLevel[finding.severity] }" />
        </span>
        {{ severityLabel(finding.status, finding.severity) }}
      </p>
      <p class="summary">{{ finding.summary }}</p>
    </div>

    <div v-if="hasBody && open" :id="bodyId" class="body">
      <div v-if="finding.why" class="block">
        <h4>Kenapa ini penting?</h4>
        <p>{{ finding.why }}</p>
      </div>
      <div v-if="finding.steps.length" class="block">
        <h4>Yang bisa kamu lakukan</h4>
        <ol>
          <li v-for="(s, i) in finding.steps" :key="i">{{ s }}</li>
        </ol>
      </div>
    </div>

    <button v-if="hasBody" type="button" class="toggle" :aria-expanded="open" :aria-controls="bodyId" @click="open = !open">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5L10 8l-5 4.5" /></svg>
      {{ open ? 'Sembunyikan penjelasan' : 'Lihat penjelasan dan cara memperbaiki' }}
    </button>
    <button v-if="cleanable" type="button" class="btn btn-primary clean" @click="$emit('bersihkan')">Bersihkan</button>
  </article>
</template>

<style scoped>
.clean {
  justify-self: start;
  margin: 0 0 6px;
}

.finding {
  display: grid;
  gap: 12px;
  padding: 18px 22px 14px;
  border-left: 5px solid var(--ok);
}

.finding.perhatian {
  border-left-color: var(--warn);
}

.finding.bermasalah {
  border-left-color: var(--bad);
}

.head {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  column-gap: 14px;
  row-gap: 2px;
}

h3 {
  font-size: 18px;
  font-weight: 600;
  line-height: 1.3;
}

.summary {
  grid-column: 2 / -1;
  font-size: 15px;
  max-width: 72ch;
}

.severity {
  /* Sejajar baris pertama judul, juga saat judulnya dua baris. */
  align-self: start;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 12px 4px 10px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  background: var(--warn-soft);
  color: var(--warn-ink);
}

.bermasalah .severity {
  background: var(--bad-soft);
  color: var(--bad-ink);
}

.drops {
  display: inline-flex;
  gap: 3px;
}

/* Tetes kecil: jumlah yang terisi = tingkat keparahan (1 ringan, 2 sedang, 3 berat). */
.drop {
  width: 8px;
  height: 10px;
  border-radius: 50% 50% 50% 50% / 60% 60% 40% 40%;
  border: 1.5px solid currentColor;
  opacity: 0.45;
}

.drop.on {
  background: currentColor;
  opacity: 1;
}

.body {
  display: grid;
  gap: 12px;
  padding-left: 50px;
}

h4 {
  font-size: 14px;
  font-weight: 600;
  color: var(--aqua-ink);
  margin-bottom: 2px;
}

.block p,
.block li {
  font-size: 15px;
  max-width: 68ch;
}

ol {
  padding-left: 20px;
  display: grid;
  gap: 4px;
}

li::marker {
  color: var(--aqua-deep);
  font-weight: 600;
}

.toggle {
  justify-self: start;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 32px;
  margin-left: 40px;
  padding: 6px 10px;
  border: 0;
  border-radius: 8px;
  background: none;
  color: var(--aqua-ink);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}

.toggle:hover {
  background: rgba(18, 155, 219, 0.1);
}

.toggle svg {
  width: 14px;
  height: 14px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2.2;
  stroke-linecap: round;
  stroke-linejoin: round;
  transition: transform 0.15s;
}

.toggle[aria-expanded="true"] svg {
  transform: rotate(90deg);
}

@media (max-width: 720px) {
  .head {
    grid-template-columns: auto minmax(0, 1fr);
  }

  .severity {
    grid-column: 2;
    justify-self: start;
  }

  .body {
    padding-left: 0;
  }

  .toggle {
    margin-left: -8px;
  }
}
</style>
