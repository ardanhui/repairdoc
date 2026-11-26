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

/** Sumber program startup (tahap 4), dalam urutan tampil. */
const SOURCES: { key: string; hint: string }[] = [
  { key: 'Registry', hint: 'daftar startup Windows, sama dengan tab Startup di Task Manager' },
  { key: 'Folder Startup', hint: 'shortcut dipindah ke folder cadangan RepairDoc, tidak dihapus' },
  { key: 'Tugas Terjadwal', hint: 'tugas yang jalan saat masuk Windows; dinonaktifkan, tidak dihapus' },
]

const groups = computed(() =>
  props.preview.groups.map((g) => {
    const items = uniqueItems(g.changes)
    const changing = items.filter((c) => c.willChange)
    const bySource = changing.some((c) => c.source)
      ? SOURCES.map((s) => ({ ...s, items: changing.filter((c) => c.source === s.key) })).filter((s) => s.items.length)
      : []
    return { ...g, changing, bySource, hasEffects: changing.some((c) => c.effect), unchanged: items.filter((c) => !c.willChange) }
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

        <p v-if="g.hasEffects" class="vendor">
          <Icon name="info" />
          Untuk aplikasi tambahan vendor (suara, touchpad, kartu grafis, tombol Fn): yang dimatikan hanya aplikasi pembuka atau ikon
          tray-nya, bukan drivernya. Driver tetap jalan sebagai bagian dari Windows, jadi suara, touchpad dasar, dan tampilan tetap
          berfungsi. Semuanya bisa dikembalikan.
        </p>

        <template v-if="g.bySource.length">
          <div v-for="src in g.bySource" :key="src.key" class="source">
            <h4>
              {{ src.key }} <span class="hint">— {{ src.hint }}</span>
            </h4>
            <ul class="rows">
              <li v-for="c in src.items" :key="c.itemKey" class="row" :class="{ excluded: skipItems.includes(c.itemKey) }">
                <label v-if="c.optional" class="pick">
                  <input type="checkbox" :checked="!skipItems.includes(c.itemKey)" @change="toggleItem(c.itemKey)" />
                  <span class="sr-only">Matikan {{ c.label }}</span>
                </label>
                <span v-else class="pick-space" aria-hidden="true" />
                <span class="label">
                  <span :title="c.targetText">{{ c.label }}</span>
                  <span v-if="c.detail" class="detail" :title="c.detail">{{ c.detail }}</span>
                  <span v-if="c.effect" class="effect">{{ c.effect }}</span>
                </span>
                <span class="values">
                  <span class="now">{{ c.current }}</span>
                  <Icon name="arrow" />
                  <span class="new">{{ c.new }}</span>
                </span>
              </li>
            </ul>
          </div>
        </template>

        <ul v-else-if="g.changing.length" class="rows">
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
  display: grid;
  gap: 1px;
  overflow-wrap: anywhere;
}

.detail {
  font-size: 12px;
  color: var(--ink-soft);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.effect {
  margin-top: 2px;
  padding: 3px 9px;
  border-radius: 8px;
  background: rgba(255, 246, 214, 0.9);
  color: var(--warn-ink);
  font-size: 12.5px;
  line-height: 1.4;
}

.vendor {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  margin: 2px 0 8px;
  padding: 9px 12px;
  border-radius: 12px;
  background: rgba(225, 243, 255, 0.85);
  border: 1px solid rgba(64, 150, 205, 0.25);
  font-size: 13.5px;
  color: var(--ink);
}

.vendor svg {
  flex: none;
  width: 18px;
  height: 18px;
  margin-top: 1px;
  color: var(--aqua-deep);
}

.source {
  margin-top: 8px;
}

.source h4 {
  font-size: 14px;
  font-weight: 600;
  color: var(--aqua-ink);
  padding: 2px 0 2px 36px;
}

.source .hint {
  font-weight: 400;
  color: var(--ink-soft);
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
