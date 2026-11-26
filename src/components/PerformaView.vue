<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { presetApply, presetPreview, presetRevertAll, presetStatus } from '../lib/api'
import { modeName, waktuText } from '../lib/labels'
import type { Outcome, PresetError, PresetEvent, PresetMode, PresetStatus, Preview } from '../lib/types'
import Icon from './Icon.vue'
import ModeCard from './ModeCard.vue'
import PresetHistory from './PresetHistory.vue'
import PresetPreview from './PresetPreview.vue'
import PresetResult from './PresetResult.vue'
import ScanBubble from './ScanBubble.vue'
import StatusOrb from './StatusOrb.vue'
import TweakToggles from './TweakToggles.vue'

defineProps<{ elevating: boolean; elevateMsg: string | null }>()
const emit = defineEmits<{ back: []; elevate: [] }>()

type Stage = 'pilih' | 'memuat' | 'pratinjau' | 'berjalan' | 'hasil'

const status = ref<PresetStatus | null>(null)
const loadError = ref<string | null>(null)
const stage = ref<Stage>('pilih')
const disabledTweaks = ref<string[]>([])
const skipItems = ref<string[]>([])
const previewMode = ref<PresetMode>('kentang')
const preview = ref<Preview | null>(null)
const stageError = ref<PresetError | null>(null)
const outcome = ref<Outcome | null>(null)
const outcomeKind = ref<'apply' | 'revert'>('apply')
const running = ref<'apply' | 'revert'>('apply')
const level = ref(0)
const stepText = ref('')
const live = ref('')

const isAdmin = computed(() => status.value?.isAdmin ?? false)
const activeInfo = computed(() => status.value?.modes.find((m) => m.mode === status.value?.activeMode))

async function load() {
  try {
    status.value = await presetStatus()
    loadError.value = status.value.error
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e)
  }
}

onMounted(load)

function enabledCount(ids: string[]) {
  return ids.filter((id) => !disabledTweaks.value.includes(id)).length
}

async function openPreview(mode: PresetMode) {
  previewMode.value = mode
  stageError.value = null
  stage.value = 'memuat'
  live.value = `Memeriksa pengaturan untuk Mode ${modeName[mode]}…`
  try {
    const res = await presetPreview(mode, { disabledTweaks: disabledTweaks.value, skipItems: [] })
    if (res.error || !res.preview) {
      stageError.value = res.error ?? { kind: 'other', message: 'Pratinjau tidak bisa dibuat.', help: [] }
      stage.value = 'pilih'
      return
    }
    preview.value = res.preview
    skipItems.value = []
    stage.value = 'pratinjau'
    window.scrollTo({ top: 0 })
  } catch (e) {
    stageError.value = { kind: 'other', message: e instanceof Error ? e.message : String(e), help: [] }
    stage.value = 'pilih'
  }
}

// ---- kemajuan (gelembung air yang sama dengan layar scan)
let creep: number | undefined
let target = 0
function setStep(e: PresetEvent) {
  const total = e.step === 'applying' ? e.total : 0
  const steps: Record<PresetEvent['step'], [number, string]> = {
    reading: [0.2, 'Membaca pengaturan sekarang…'],
    restorePoint: [0.6, 'Membuat restore point… bisa 1–3 menit, jangan matikan PC'],
    applying: [0.9, running.value === 'revert' ? `Mengembalikan ${total} pengaturan…` : `Menerapkan ${total} perubahan…`],
    saving: [0.97, 'Menyimpan catatan perubahan…'],
  }
  const [t, text] = steps[e.step]
  target = t
  stepText.value = text
  live.value = text
}

function startCreep() {
  stopCreep()
  level.value = 0.05
  target = 0.15
  creep = window.setInterval(() => {
    level.value = Math.max(level.value, level.value + (target - level.value) * 0.08)
  }, 250)
}

function stopCreep() {
  if (creep !== undefined) window.clearInterval(creep)
  creep = undefined
}

onBeforeUnmount(stopCreep)

async function run(kind: 'apply' | 'revert') {
  running.value = kind
  stage.value = 'berjalan'
  stepText.value = kind === 'revert' ? 'Bersiap mengembalikan…' : 'Bersiap…'
  window.scrollTo({ top: 0 })
  startCreep()
  try {
    outcome.value =
      kind === 'apply'
        ? await presetApply(previewMode.value, { disabledTweaks: disabledTweaks.value, skipItems: skipItems.value }, setStep)
        : await presetRevertAll(setStep)
  } catch (e) {
    outcome.value = {
      ok: false,
      mode: null,
      error: { kind: 'other', message: e instanceof Error ? e.message : String(e), help: [] },
      entry: null,
      changed: 0,
      skipped: 0,
      failed: 0,
      failures: [],
      restorePoint: null,
      needsRestart: false,
    }
  }
  stopCreep()
  level.value = 1
  outcomeKind.value = kind
  await new Promise((r) => setTimeout(r, 500))
  stage.value = 'hasil'
  live.value = outcome.value?.error?.message ?? 'Selesai.'
  await load()
}

function done() {
  outcome.value = null
  preview.value = null
  stage.value = 'pilih'
}
</script>

<template>
  <main class="performa">
    <p class="sr-only" aria-live="polite">{{ live }}</p>

    <div class="title-row">
      <button v-if="stage !== 'berjalan'" type="button" class="btn back" @click="stage === 'pratinjau' ? done() : emit('back')">
        <Icon name="back" />
        {{ stage === 'pratinjau' ? 'Kembali ke pilihan mode' : 'Kembali' }}
      </button>
    </div>

    <!-- Menerapkan / mengembalikan -->
    <section v-if="stage === 'berjalan'" class="running">
      <h1>{{ running === 'revert' ? 'Mengembalikan pengaturan' : `Menerapkan Mode ${modeName[previewMode]}` }}</h1>
      <ScanBubble :scanning="true" :level="level" :label="stepText" />
      <p class="step">{{ stepText }}</p>
    </section>

    <PresetResult v-else-if="stage === 'hasil' && outcome" :outcome="outcome" :kind="outcomeKind" @done="done" />

    <PresetPreview
      v-else-if="stage === 'pratinjau' && preview"
      v-model:skip-items="skipItems"
      :preview="preview"
      :title="`Mode ${modeName[preview.mode]}`"
      @apply="run('apply')"
      @cancel="done"
    />

    <!-- Pilih mode -->
    <template v-else>
      <header class="intro">
        <h1>Mode Performa</h1>
        <p class="lead">
          Seperti pengaturan grafis di game: pilih satu mode, RepairDoc yang mengatur detailnya. Sebelum mengubah apa pun, kamu melihat daftarnya dulu,
          dan semua perubahan bisa dikembalikan.
        </p>
      </header>

      <p v-if="loadError" class="glass warn-box">{{ loadError }}</p>

      <section v-if="status && !isAdmin" class="admin glass" aria-labelledby="perf-admin">
        <StatusOrb status="perhatian" :size="40" />
        <div>
          <h2 id="perf-admin">Mode Performa butuh izin administrator</h2>
          <p>
            Mengubah pengaturan Windows, service, dan power plan hanya bisa dilakukan dengan izin administrator. RepairDoc akan dibuka ulang dengan
            izin itu. Kamu masih bisa melihat isi tiap mode di bawah.
          </p>
          <button type="button" class="btn btn-primary" :disabled="elevating" @click="emit('elevate')">
            <Icon name="shield" />
            {{ elevating ? 'Menunggu izin…' : 'Minta izin administrator' }}
          </button>
          <p v-if="elevateMsg" class="msg" role="status">{{ elevateMsg }}</p>
        </div>
      </section>

      <div v-if="stageError" class="glass warn-box" role="alert">
        <p>{{ stageError.message }}</p>
        <ul v-if="stageError.help.length">
          <li v-for="h in stageError.help" :key="h">{{ h }}</li>
        </ul>
      </div>

      <p v-if="status" class="active-line">
        <span class="dot" aria-hidden="true" />
        Sekarang berlaku: <strong>{{ activeInfo?.title ?? 'Mode Normal' }}</strong>
        <template v-if="status.activeSince"> sejak {{ waktuText(status.activeSince) }}</template>
        <template v-else> (bawaan Windows)</template>
      </p>

      <div v-if="status" class="cards" :class="{ busy: stage === 'memuat' }">
        <ModeCard
          v-for="m in status.modes"
          :key="m.mode"
          :info="m"
          :active="status.activeMode === m.mode"
          :disabled="!isAdmin || stage === 'memuat'"
          :enabled-count="enabledCount(m.tweakIds)"
          @apply="openPreview(m.mode)"
        />
      </div>
      <p v-if="stage === 'memuat'" class="loading" role="status">Memeriksa pengaturan sekarang… (tidak ada yang diubah)</p>

      <div v-if="status" class="lower">
        <TweakToggles v-model:disabled="disabledTweaks" :tweaks="status.tweaks" />
        <PresetHistory :status="status" :disabled="!isAdmin" @revert="run('revert')" />
      </div>
    </template>
  </main>
</template>

<style scoped>
.performa {
  display: grid;
  gap: 18px;
  max-width: 1180px;
  margin: 0 auto;
  padding: 14px 24px 22vh;
}

.title-row {
  min-height: 40px;
}

.back svg {
  width: 16px;
  height: 16px;
}

.intro {
  display: grid;
  gap: 6px;
  max-width: 760px;
}

h1 {
  font-size: 38px;
  font-weight: 300;
  letter-spacing: -0.02em;
  line-height: 1.15;
}

.lead {
  font-size: 16px;
}

.admin {
  display: flex;
  gap: 16px;
  align-items: flex-start;
  padding: 18px 20px;
}

.admin h2 {
  font-size: 17px;
  font-weight: 600;
}

.admin p {
  font-size: 14px;
  color: var(--ink-soft);
  margin: 4px 0 10px;
}

.admin .msg {
  color: var(--warn-ink);
  font-weight: 600;
}

.warn-box {
  padding: 14px 18px;
  background: var(--warn-soft);
  color: var(--warn-ink);
  font-size: 14px;
}

.warn-box ul {
  padding-left: 18px;
  margin-top: 6px;
}

.active-line {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  justify-self: start;
  padding: 7px 16px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.88);
  border: 1px solid #fff;
  font-size: 15px;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: radial-gradient(circle at 40% 35%, #b6f39a, var(--ok));
  box-shadow: 0 0 0 2px rgba(52, 163, 59, 0.2);
}

.cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
  align-items: stretch;
}

.cards.busy {
  opacity: 0.6;
  pointer-events: none;
}

.loading {
  font-weight: 600;
  color: var(--aqua-ink);
}

.lower {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(300px, 1fr);
  gap: 16px;
  align-items: start;
}

.running {
  display: grid;
  justify-items: center;
  gap: 20px;
  padding-top: 2vh;
  text-align: center;
}

.step {
  font-size: 17px;
  font-weight: 600;
  color: var(--aqua-ink);
}

@media (max-width: 980px) {
  .cards,
  .lower {
    grid-template-columns: 1fr;
  }
}
</style>
