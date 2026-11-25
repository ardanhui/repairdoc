<script setup lang="ts">
import { finishedText } from '../lib/labels'
import type { ScanReport } from '../lib/types'
import Icon from './Icon.vue'
import StatusOrb from './StatusOrb.vue'

defineProps<{ report: ScanReport }>()
defineEmits<{ rescan: [] }>()
</script>

<template>
  <section class="verdict glass glass-window" :class="report.verdict.status" aria-labelledby="verdict-title">
    <StatusOrb :status="report.verdict.status" :size="84" />
    <h2 id="verdict-title">{{ report.verdict.headline }}</h2>
    <p class="detail">{{ report.verdict.detail }}</p>
    <p v-if="report.verdict.topPriority" class="priority">{{ report.verdict.topPriority }}</p>
    <p class="when">{{ finishedText(report.finishedAtMs, report.durationMs) }}</p>
    <button type="button" class="btn" @click="$emit('rescan')">
      <Icon name="refresh" />
      Scan ulang
    </button>
  </section>
</template>

<style scoped>
.verdict {
  display: grid;
  justify-items: start;
  gap: 10px;
  padding: 26px 26px 24px;
}

.verdict > * {
  position: relative;
}

h2 {
  margin-top: 8px;
  font-size: 29px;
  font-weight: 300;
  line-height: 1.2;
  letter-spacing: -0.015em;
}

.detail {
  font-size: 16px;
}

.priority {
  padding: 10px 14px;
  border-radius: 12px;
  font-weight: 600;
  background: var(--ok-soft);
  color: var(--ok-ink);
}

.perhatian .priority {
  background: var(--warn-soft);
  color: var(--warn-ink);
}

.bermasalah .priority {
  background: var(--bad-soft);
  color: var(--bad-ink);
}

.when {
  font-size: 13px;
  color: var(--ink-soft);
}

.btn {
  margin-top: 6px;
}
</style>
