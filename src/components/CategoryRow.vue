<script setup lang="ts">
import { computed } from 'vue'
import { categoriesOf, categoryInfo, groupInfo, groupOrder } from '../lib/labels'
import type { Category, CategoryState, Group } from '../lib/types'
import Icon from './Icon.vue'

const props = defineProps<{ states: Record<Category, CategoryState>; scanning: boolean }>()

type GroupView = { group: Group; state: CategoryState; text: string }

const finished = (s: CategoryState) => s === 'selesai' || s === 'gagal' || s === 'dilewati'

const views = computed<GroupView[]>(() =>
  groupOrder.map((group) => {
    const cats = categoriesOf(group)
    const running = cats.find((c) => props.states[c] === 'berjalan')
    if (!props.scanning) return { group, state: 'menunggu', text: groupInfo[group].hint }
    if (running) return { group, state: 'berjalan', text: `${categoryInfo[running].doing}…` }
    if (cats.every((c) => finished(props.states[c]))) {
      const failed = cats.filter((c) => props.states[c] === 'gagal').length
      return { group, state: failed ? 'gagal' : 'selesai', text: failed ? 'Sebagian tidak bisa diperiksa' : 'Selesai' }
    }
    const done = cats.filter((c) => finished(props.states[c])).length
    return { group, state: done ? 'berjalan' : 'menunggu', text: done ? `${done} dari ${cats.length} selesai` : 'Menunggu giliran' }
  }),
)
</script>

<template>
  <ul class="cats">
    <li v-for="v in views" :key="v.group" class="cat glass" :class="scanning ? v.state : ''">
      <span class="cat-orb">
        <Icon v-if="scanning && v.state === 'selesai'" name="check" />
        <Icon v-else :name="v.group" />
      </span>
      <span class="cat-text">
        <strong>{{ groupInfo[v.group].name }}</strong>
        <span>{{ v.text }}</span>
      </span>
    </li>
  </ul>
</template>

<style scoped>
.cats {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 12px;
  padding: 0;
  list-style: none;
  width: min(100%, 1000px);
}

.cat {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 999px;
  text-align: left;
  transition: opacity 0.3s;
}

.cat-orb {
  position: relative;
  display: grid;
  place-items: center;
  flex: none;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  color: #fff;
  background: radial-gradient(circle at 50% 115%, #8ee6ff 0%, #23a1e3 50%, #0a69ab 100%);
  box-shadow: 0 0 0 2px #fff, 0 3px 8px -2px rgba(10, 70, 120, 0.4);
}

.cat-orb::before {
  content: "";
  position: absolute;
  top: 5%;
  left: 16%;
  width: 68%;
  height: 45%;
  border-radius: 50%;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.8), rgba(255, 255, 255, 0.05));
}

.cat-orb svg {
  position: relative;
  width: 21px;
  height: 21px;
}

.cat-text {
  display: grid;
  line-height: 1.25;
  min-width: 0;
}

.cat-text strong {
  font-size: 14px;
  font-weight: 600;
}

.cat-text span {
  font-size: 12px;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

/* Status saat scan */
.cat.menunggu {
  opacity: 0.6;
}

.cat.berjalan .cat-orb::after {
  content: "";
  position: absolute;
  inset: -6px;
  border-radius: 50%;
  border: 3px solid rgba(18, 155, 219, 0.25);
  border-top-color: var(--aqua);
  animation: spin 1s linear infinite;
}

.cat.selesai .cat-orb {
  background: radial-gradient(circle at 50% 115%, #a3ec80 0%, var(--ok) 50%, #1f7a25 100%);
}

.cat.gagal .cat-orb {
  background: radial-gradient(circle at 50% 115%, #ffe08a 0%, var(--warn) 50%, #c46f00 100%);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 1040px) {
  .cats {
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  }
}
</style>
