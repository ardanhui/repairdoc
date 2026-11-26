<script setup lang="ts">
import { computed } from 'vue'
import type { Preview, PreviewChange } from '../lib/types'
import Icon from './Icon.vue'

const props = defineProps<{ preview: Preview; skipItems: string[]; title: string }>()
const emit = defineEmits<{ apply: []; cancel: []; 'update:skipItems': [value: string[]] }>()

/** Satu baris per item (satu aplikasi bisa terdiri dari dua nilai registry). */
function uniqueItems(changes: PreviewChange[]): PreviewChange[] {
  const seen = new Set<string>()
  return changes.filter((c) => (seen.has(c.itemKey) ? false : (seen.add(c.itemKey), true)))
}

const groups = computed(() =>
  props.preview.groups.map((g) => {
    const items = uniqueItems(g.changes)
    return { ...g, changing: items.filter((c) => c.willChange), unchanged: items.filter((c) => !c.willChange) }
  }),
)

const effectiveCount = computed(
  () => new Set(props.preview.groups.flatMap((g) => g.changes.filter((c) => c.willChange && !props.skipItems.includes(c.itemKey)).map((c) => c.itemKey))).size,
)

function toggleItem(key: string) {
  const off = props.skipItems.includes(key)
  emit('update:skipItems', off ? props.skipItems.filter((k) => k !== key) : [...props.skipItems, key])
}
</script>

<template>
  <section class="preview glass glass-window" aria-labelledby="preview-title">
    <header class="head">
      <h2 id="preview-title">Periksa dulu: {{ title }}</h2>
      <p class="summary">
        <strong>{{ effectiveCount }} pengaturan akan diubah</strong
        ><template v-if="preview.sameCount">, {{ preview.sameCount }} sudah sesuai dan dibiarkan</template>. Belum ada yang diubah sampai kamu menekan
        Terapkan.
      </p>
      <ul v-if="preview.notes.length" class="notes">
        <li v-for="n in preview.notes" :key="n">{{ n }}</li>
      </ul>
      <p v-if="effectiveCount" class="rp">
        <Icon name="shield" />
        Sebelum mengubah apa pun, RepairDoc membuat restore point: <strong>{{ preview.restorePointName }}</strong>
      </p>
    </header>

    <div class="groups">
      <section v-for="g in groups" :key="g.tweakId" class="group" :class="{ skipped: g.skippedByUser }">
        <h3>
          {{ g.title }}
          <span v-if="g.skippedByUser" class="tag">Dimatikan</span>
        </h3>
        <p v-if="g.description" class="desc">{{ g.description }}</p>

        <ul v-if="g.changing.length" class="rows">
          <li v-for="c in g.changing" :key="c.itemKey" class="row" :class="{ excluded: skipItems.includes(c.itemKey) }">
            <label v-if="c.optional" class="pick">
              <input type="checkbox" :checked="!skipItems.includes(c.itemKey)" @change="toggleItem(c.itemKey)" />
              <span class="sr-only">Ubah {{ c.label }}</span>
            </label>
            <span v-else class="pick-space" aria-hidden="true" />
            <span class="label" :title="c.targetText">{{ c.label }}</span>
            <span class="values">
              <span class="now">{{ c.current }}</span>
              <Icon name="arrow" />
              <span class="new">{{ c.new }}</span>
            </span>
          </li>
        </ul>

        <details v-if="g.unchanged.length" class="same">
          <summary>{{ g.unchanged.length }} tidak diubah</summary>
          <ul class="rows">
            <li v-for="c in g.unchanged" :key="c.itemKey" class="row muted">
              <span class="pick-space" aria-hidden="true" />
              <span class="label" :title="c.targetText">{{ c.label }}</span>
              <span class="values">
                <span class="now">{{ c.current }}</span>
                <span class="why">{{ c.note }}</span>
              </span>
            </li>
          </ul>
        </details>
      </section>
    </div>

    <footer class="actions">
      <button type="button" class="btn" @click="emit('cancel')">Batal</button>
      <button type="button" class="btn btn-primary" :disabled="effectiveCount === 0" @click="emit('apply')">Terapkan</button>
    </footer>
  </section>
</template>

<style scoped>
.preview {
  display: grid;
  gap: 18px;
  padding: 24px 26px;
}

.preview > * {
  position: relative;
}

.head {
  display: grid;
  gap: 8px;
}

h2 {
  font-size: 28px;
  font-weight: 300;
  letter-spacing: -0.015em;
}

.summary {
  font-size: 16px;
}

.notes {
  padding-left: 18px;
  font-size: 14px;
  color: var(--ink-soft);
}

.rp {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 14px;
  border-radius: 12px;
  background: var(--ok-soft);
  color: var(--ok-ink);
  font-size: 14px;
}

.rp svg {
  flex: none;
  width: 18px;
  height: 18px;
  margin-top: 2px;
}

.groups {
  display: grid;
  gap: 4px;
}

.group {
  padding: 12px 0;
  border-top: 1px solid rgba(64, 150, 205, 0.2);
}

.group.skipped {
  opacity: 0.6;
}

h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
}

.tag {
  padding: 1px 9px;
  border-radius: 999px;
  background: var(--warn-soft);
  color: var(--warn-ink);
  font-size: 12px;
}

.desc {
  font-size: 14px;
  color: var(--ink-soft);
  margin-bottom: 6px;
}

.rows {
  list-style: none;
  padding: 0;
}

.row {
  display: grid;
  grid-template-columns: 26px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 5px 0;
  font-size: 14px;
}

.row.excluded .label,
.row.excluded .values {
  text-decoration: line-through;
  opacity: 0.55;
}

.pick input {
  width: 18px;
  height: 18px;
  accent-color: var(--aqua-deep);
}

.label {
  overflow-wrap: anywhere;
}

.values {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  justify-self: end;
  text-align: right;
}

.values svg {
  width: 16px;
  height: 16px;
  flex: none;
  color: var(--aqua-deep);
}

.now {
  color: var(--ink-soft);
}

.new {
  font-weight: 600;
  color: var(--aqua-ink);
}

.why {
  color: var(--ink-soft);
  font-style: italic;
}

.row.muted .label {
  color: var(--ink-soft);
}

.same summary {
  cursor: pointer;
  font-size: 13px;
  color: var(--aqua-ink);
  padding: 4px 0 2px 36px;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding-top: 6px;
}

@media (max-width: 720px) {
  .row {
    grid-template-columns: 26px minmax(0, 1fr);
  }

  .values {
    grid-column: 2;
    justify-self: start;
    text-align: left;
  }
}
</style>
