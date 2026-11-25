<script setup lang="ts">
import type { TechSection } from '../lib/types'

defineProps<{ sections: TechSection[] }>()
</script>

<template>
  <details class="tech glass">
    <summary>Info untuk teknisi</summary>
    <p class="note">Data teknis untuk teknisi, atau untuk dikirim saat minta bantuan. Tidak perlu dipahami untuk memakai RepairDoc.</p>
    <section v-for="s in sections" :key="s.title">
      <h4>{{ s.title }}</h4>
      <dl>
        <template v-for="(it, i) in s.items" :key="i">
          <dt>{{ it.label }}</dt>
          <dd>{{ it.value }}</dd>
        </template>
      </dl>
    </section>
  </details>
</template>

<style scoped>
.tech {
  padding: 0 20px;
}

summary {
  cursor: pointer;
  padding: 14px 0;
  font-weight: 600;
  color: var(--aqua-ink);
}

.tech[open] {
  padding-bottom: 18px;
}

.note {
  font-size: 13px;
  color: var(--ink-soft);
  margin-bottom: 8px;
}

section {
  padding-top: 12px;
}

h4 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 6px;
}

dl {
  display: grid;
  grid-template-columns: minmax(150px, 34%) minmax(0, 1fr);
  gap: 4px 16px;
  font-size: 13px;
}

dt {
  color: var(--ink-soft);
}

dd {
  overflow-wrap: anywhere;
}
</style>
