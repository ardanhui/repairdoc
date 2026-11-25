<script setup lang="ts">
import { computed } from 'vue'
import { categoryInfo, groupInfo, groupOrder } from '../lib/labels'
import type { Finding, Group } from '../lib/types'
import StatusOrb from './StatusOrb.vue'

const props = defineProps<{ findings: Finding[] }>()

const grouped = computed(() =>
  groupOrder
    .map((g: Group) => ({ group: g, items: props.findings.filter((f) => categoryInfo[f.category].group === g) }))
    .filter((x) => x.items.length > 0),
)
</script>

<template>
  <div class="healthy glass">
    <section v-for="g in grouped" :key="g.group">
      <h3>{{ groupInfo[g.group].name }}</h3>
      <ul>
        <li v-for="f in g.items" :key="f.id">
          <StatusOrb status="sehat" :size="22" />
          <p>
            <strong>{{ f.title }}</strong>
            <span>{{ f.summary }}</span>
          </p>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.healthy {
  padding: 6px 20px 10px;
}

section + section {
  border-top: 1px solid rgba(64, 150, 205, 0.18);
}

h3 {
  padding: 12px 0 4px;
  font-size: 13px;
  font-weight: 600;
  color: var(--aqua-ink);
}

ul {
  list-style: none;
  padding: 0;
}

li {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 8px 0;
}

li :deep(.orb-status) {
  margin-top: 2px;
}

p {
  display: grid;
  line-height: 1.4;
}

strong {
  font-weight: 600;
}

span {
  font-size: 14px;
  color: var(--ink-soft);
}
</style>
