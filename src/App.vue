<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { getAppInfo, isTauri, relaunchAsAdmin, startScan } from './lib/api'
import { setupMusic } from './lib/music'
import { categoriesOf, categoryInfo, categoryOrder, groupOrder } from './lib/labels'
import type { AppInfo, Category, CategoryState, ScanReport } from './lib/types'
import AeroBackground from './components/AeroBackground.vue'
import AppHeader from './components/AppHeader.vue'
import ScanBubble from './components/ScanBubble.vue'
import CategoryRow from './components/CategoryRow.vue'
import VerdictPanel from './components/VerdictPanel.vue'
import AdminNote from './components/AdminNote.vue'
import FindingCard from './components/FindingCard.vue'
import HealthyList from './components/HealthyList.vue'
import TechInfo from './components/TechInfo.vue'
import StatusOrb from './components/StatusOrb.vue'
import Icon from './components/Icon.vue'
import PerformaView from './components/PerformaView.vue'
import CleanupView from './components/CleanupView.vue'

type Phase = 'idle' | 'scanning' | 'done' | 'error' | 'performa' | 'bersihkan'

const info = ref<AppInfo | null>(null)
const phase = ref<Phase>('idle')
const blankStates = (): Record<Category, CategoryState> =>
  Object.fromEntries(categoryOrder.map((c) => [c, 'menunggu'])) as Record<Category, CategoryState>
const states = reactive<Record<Category, CategoryState>>(blankStates())
const current = ref<Category | null>(null)
const level = ref(0)
const report = ref<ScanReport | null>(null)
const errorMsg = ref<string | null>(null)
const elevating = ref(false)
const elevateMsg = ref<string | null>(null)
const live = ref('')
const preview = !isTauri()

// Tiga keranjang hasil: yang harus dibereskan, hal kecil yang bisa menunggu, dan yang sudah baik.
const urgent = computed(() => report.value?.findings.filter((f) => f.status === 'bermasalah' || (f.status === 'perhatian' && f.severity !== 'ringan')) ?? [])
const minor = computed(() => report.value?.findings.filter((f) => f.status === 'perhatian' && f.severity === 'ringan') ?? [])
const healthy = computed(() => report.value?.findings.filter((f) => f.status === 'sehat') ?? [])
const showAdminNote = computed(() => !!report.value && !report.value.isAdmin && report.value.needsAdmin.length > 0 && (info.value?.isWindows || preview))
const stepLabel = computed(() => (current.value ? `${categoryInfo[current.value].doing}…` : 'Bersiap…'))
// Kemajuan yang bisa dihitung: berapa dari 5 kelompok (Hardware, Windows, …) yang sudah selesai.
const groupsDone = computed(
  () => groupOrder.filter((g) => categoriesOf(g).every((c) => states[c] !== 'menunggu' && states[c] !== 'berjalan')).length,
)

onMounted(async () => {
  setupMusic()
  try {
    info.value = await getAppInfo()
  } catch {
    info.value = null
  }
})

// Air di gelembung naik pelan selama satu kategori berjalan, lalu melompat saat selesai.
let creepTimer: number | undefined
const doneCount = () => categoryOrder.filter((c) => states[c] !== 'menunggu' && states[c] !== 'berjalan').length
const raise = (to: number) => (level.value = Math.max(level.value, Math.min(1, to)))

function startCreep() {
  stopCreep()
  creepTimer = window.setInterval(() => {
    const target = (doneCount() + 0.85) / categoryOrder.length
    raise(level.value + (target - level.value) * 0.12)
  }, 300)
}

function stopCreep() {
  if (creepTimer !== undefined) window.clearInterval(creepTimer)
  creepTimer = undefined
}

onBeforeUnmount(stopCreep)

const reducedMotion = () => window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

async function scan() {
  phase.value = 'scanning'
  report.value = null
  errorMsg.value = null
  elevateMsg.value = null
  current.value = null
  Object.assign(states, blankStates())
  level.value = 0.04
  live.value = 'Scan dimulai.'
  window.scrollTo({ top: 0 })
  startCreep()

  try {
    const result = await startScan((ev) => {
      if (ev.kind === 'categoryStarted') {
        states[ev.category] = 'berjalan'
        current.value = ev.category
        live.value = `${categoryInfo[ev.category].doing}…`
      } else {
        states[ev.category] = ev.kind === 'categoryFinished' ? 'selesai' : ev.kind === 'categorySkipped' ? 'dilewati' : 'gagal'
        raise(doneCount() / categoryOrder.length)
      }
    })
    stopCreep()
    raise(1)
    // Biarkan gelembung terlihat penuh sejenak sebelum hasil muncul.
    await new Promise((r) => setTimeout(r, reducedMotion() ? 0 : 900))
    report.value = result
    phase.value = 'done'
    live.value = `${result.verdict.headline}. ${result.verdict.detail}`
  } catch (e) {
    stopCreep()
    errorMsg.value = e instanceof Error ? e.message : String(e)
    phase.value = 'error'
  }
}

// Mode Performa (tahap 3) dan Bersihkan File Sampah (tahap 4): layar penuh,
// tombol Kembali membawa ke layar sebelumnya (layar awal atau hasil scan).
const beforePerforma = ref<Phase>('idle')
function openTool(tool: 'performa' | 'bersihkan') {
  if (phase.value !== 'performa' && phase.value !== 'bersihkan') beforePerforma.value = phase.value
  phase.value = tool
  window.scrollTo({ top: 0 })
}
const openPerforma = () => openTool('performa')
const openBersihkan = () => openTool('bersihkan')
function closePerforma() {
  phase.value = beforePerforma.value === 'done' && !report.value ? 'idle' : beforePerforma.value
  window.scrollTo({ top: 0 })
}

async function elevate() {
  elevating.value = true
  elevateMsg.value = null
  try {
    await relaunchAsAdmin()
  } catch (e) {
    elevateMsg.value = e instanceof Error ? e.message : String(e)
  } finally {
    elevating.value = false
  }
}
</script>

<template>
  <AeroBackground />
  <AppHeader :preview="preview" />
  <p class="sr-only" aria-live="polite">{{ live }}</p>

  <main v-if="phase === 'idle' || phase === 'scanning'" class="home">
    <div class="hero">
      <h1>{{ phase === 'idle' ? 'Periksa kesehatan PC ini' : 'Sedang memeriksa PC ini' }}</h1>
      <p class="lead">
        {{
          phase === 'idle'
            ? 'Butuh sekitar satu menit. RepairDoc hanya melihat kondisi PC, tidak ada pengaturan yang diubah.'
            : 'Tunggu sebentar ya. Selama pemeriksaan, tidak ada pengaturan yang diubah.'
        }}
      </p>
    </div>

    <ScanBubble :scanning="phase === 'scanning'" :level="level" :label="stepLabel" @start="scan" />
    <div class="step" :class="{ hidden: phase !== 'scanning' }" aria-hidden="true">
      <p>{{ stepLabel }}</p>
      <p class="progress-count">{{ groupsDone }} dari {{ groupOrder.length }} kelompok selesai</p>
    </div>

    <CategoryRow :states="states" :scanning="phase === 'scanning'" />

    <div v-if="phase === 'idle'" class="tools">
      <button type="button" class="btn tool-btn" @click="openPerforma">
        <Icon name="gauge" />
        <span><strong>Mode Performa</strong><small>Buat Windows lebih ringan</small></span>
      </button>
      <button type="button" class="btn tool-btn" @click="openBersihkan">
        <Icon name="cleanup" />
        <span><strong>Bersihkan File Sampah</strong><small>Kosongkan ruang di drive</small></span>
      </button>
    </div>
  </main>

  <PerformaView v-else-if="phase === 'performa'" :elevating="elevating" :elevate-msg="elevateMsg" @back="closePerforma" @elevate="elevate" />

  <CleanupView v-else-if="phase === 'bersihkan'" :elevating="elevating" :elevate-msg="elevateMsg" @back="closePerforma" @elevate="elevate" />

  <main v-else-if="phase === 'done' && report" class="results">
    <div class="side">
      <VerdictPanel :report="report" @rescan="scan" @performa="openPerforma" @bersihkan="openBersihkan" />
      <AdminNote v-if="showAdminNote && report" :items="report.needsAdmin" :busy="elevating" :message="elevateMsg" @elevate="elevate" />
    </div>

    <div class="main-col">
      <section aria-labelledby="todo-title" class="group">
        <h2 id="todo-title" class="section-title">Yang perlu dibereskan</h2>
        <div v-if="urgent.length" class="cards">
          <!-- Hanya kartu pertama (yang terberat) yang langsung terbuka; sisanya cukup ringkasannya. -->
          <FindingCard v-for="(f, i) in urgent" :key="f.id" :finding="f" :default-open="i === 0" @bersihkan="openBersihkan" />
        </div>
        <p v-else class="empty glass">Tidak ada yang mendesak. Semua yang diperiksa dalam kondisi baik atau hanya butuh perhatian kecil.</p>
      </section>

      <section v-if="minor.length" aria-labelledby="minor-title" class="group">
        <h2 id="minor-title" class="section-title">Hal kecil yang bisa dirapikan nanti</h2>
        <div class="cards">
          <FindingCard v-for="f in minor" :key="f.id" :finding="f" @bersihkan="openBersihkan" />
        </div>
      </section>

      <section v-if="healthy.length" aria-labelledby="ok-title" class="group">
        <h2 id="ok-title" class="section-title">Yang sudah baik</h2>
        <HealthyList :findings="healthy" />
      </section>

      <TechInfo :sections="report.tech" />
    </div>
  </main>

  <main v-else class="home">
    <section class="error glass glass-window" aria-labelledby="err-title">
      <StatusOrb status="perhatian" :size="64" />
      <h1 id="err-title">Scan berhenti sebelum selesai</h1>
      <p>Coba jalankan lagi. Kalau tetap gagal, tutup RepairDoc lalu buka lagi dengan klik kanan, pilih "Run as administrator".</p>
      <p v-if="errorMsg" class="err-detail">Pesan dari sistem: {{ errorMsg }}</p>
      <button type="button" class="btn" @click="scan">
        <Icon name="refresh" />
        Coba lagi
      </button>
    </section>
  </main>
</template>

<style scoped>
.home {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 22px;
  padding: 4vh 24px 26vh;
  text-align: center;
}

.hero {
  display: grid;
  gap: 8px;
  max-width: 680px;
  text-wrap: balance;
}

h1 {
  font-size: 40px;
  font-weight: 300;
  line-height: 1.15;
  letter-spacing: -0.02em;
}

.lead {
  font-size: 17px;
  color: var(--ink);
}

.step {
  display: grid;
  gap: 2px;
  margin-top: -4px;
  font-size: 17px;
  font-weight: 600;
  color: var(--aqua-ink);
}

.progress-count {
  font-size: 14px;
  font-weight: 400;
  color: var(--ink-soft);
}

.step.hidden {
  visibility: hidden;
}

.tools {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 16px;
  width: min(100%, 696px);
}

/* Dua tombol alat sama lebar (simetris), judul + keterangan singkat. */
.tool-btn {
  flex: 1 1 0;
  min-width: 260px;
  max-width: 340px;
  justify-content: flex-start;
  gap: 12px;
  padding: 10px 22px;
  border-radius: 18px;
  text-align: left;
}

.tool-btn span {
  display: grid;
  line-height: 1.3;
}

.tool-btn small {
  font-size: 13px;
  font-weight: 400;
  color: var(--ink-soft);
}

.tool-btn svg {
  width: 22px;
  height: 22px;
}

.results {
  display: grid;
  grid-template-columns: minmax(290px, 350px) minmax(0, 1fr);
  align-items: start;
  gap: 24px;
  max-width: 1180px;
  margin: 0 auto;
  padding: 18px 24px 22vh;
}

.side {
  position: sticky;
  top: 84px;
  display: grid;
  gap: 16px;
}

.main-col {
  display: grid;
  gap: 26px;
  min-width: 0;
}

.group {
  display: grid;
  gap: 12px;
}

.section-title {
  font-size: 21px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.cards {
  display: grid;
  gap: 14px;
}

.empty {
  padding: 18px 22px;
}

.error {
  display: grid;
  justify-items: center;
  gap: 12px;
  max-width: 560px;
  margin-top: 6vh;
  padding: 32px;
}

.error > * {
  position: relative;
}

.error h1 {
  font-size: 30px;
}

.err-detail {
  font-size: 13px;
  color: var(--ink-soft);
  overflow-wrap: anywhere;
}

@media (max-width: 980px) {
  .results {
    grid-template-columns: 1fr;
  }

  .side {
    position: static;
  }
}
</style>
