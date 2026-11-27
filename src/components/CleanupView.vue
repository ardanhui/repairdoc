<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { cleanupApply, cleanupPreview, cleanupStatus } from '../lib/api'
import { countText, sizeText, tanggalText, waktuText } from '../lib/labels'
import type { CleanEvent, CleanJenis, CleanOutcome, CleanPreview, CleanStatus, PresetError } from '../lib/types'
import Icon from './Icon.vue'
import ScanBubble from './ScanBubble.vue'
import StatusOrb from './StatusOrb.vue'

defineProps<{ elevating: boolean; elevateMsg: string | null }>()
const emit = defineEmits<{ back: []; elevate: [] }>()

type Stage = 'memuat' | 'pilih' | 'berjalan' | 'hasil'

const status = ref<CleanStatus | null>(null)
const stage = ref<Stage>('memuat')
const preview = ref<CleanPreview | null>(null)
const error = ref<PresetError | null>(null)
const selected = ref<CleanJenis[]>([])
const windowsOld = ref(false)
const confirming = ref(false)
const outcome = ref<CleanOutcome | null>(null)
const level = ref(0)
const stepText = ref('')
const live = ref('')

const isAdmin = computed(() => status.value?.isAdmin ?? false)

const chosen = computed(() => preview.value?.kinds.filter((k) => selected.value.includes(k.jenis)) ?? [])
const chosenCount = computed(() => chosen.value.length + (windowsOld.value && preview.value?.windowsOld ? 1 : 0))
const chosenBytes = computed(() => chosen.value.reduce((a, k) => a + k.bytes, 0) + (windowsOld.value ? (preview.value?.windowsOld?.bytes ?? 0) : 0))

const TEN_DAYS = 10 * 24 * 3600 * 1000
const rollbackUntil = computed(() => {
  const c = preview.value?.windowsOld?.createdMs
  return c ? c + TEN_DAYS : null
})

async function measure() {
  stage.value = 'memuat'
  confirming.value = false
  error.value = null
  live.value = 'Mengukur file sampah. Tidak ada yang dihapus.'
  try {
    const res = await cleanupPreview()
    preview.value = res.preview
    error.value = res.error
    if (res.preview) {
      selected.value = res.preview.kinds.filter((k) => k.selectedByDefault && k.bytes > 0).map((k) => k.jenis)
      windowsOld.value = false
    }
  } catch (e) {
    error.value = { kind: 'other', message: e instanceof Error ? e.message : String(e), help: [] }
  }
  stage.value = 'pilih'
}

async function load() {
  try {
    status.value = await cleanupStatus()
  } catch (e) {
    error.value = { kind: 'other', message: e instanceof Error ? e.message : String(e), help: [] }
    stage.value = 'pilih'
    return
  }
  if (status.value.isAdmin) await measure()
  else stage.value = 'pilih'
}

onMounted(load)

function toggle(j: CleanJenis) {
  confirming.value = false
  selected.value = selected.value.includes(j) ? selected.value.filter((x) => x !== j) : [...selected.value, j]
}

// ---- kemajuan: gelembung air yang sama dengan layar scan
let creep: number | undefined
let target = 0
function onEvent(e: CleanEvent) {
  if (e.step === 'preparing') {
    target = 0.08
    stepText.value = 'Bersiap…'
  } else if (e.step === 'cleaning') {
    target = (e.index + 0.9) / e.total
    stepText.value =
      e.jenis === 'windowsOld'
        ? `Disk Cleanup sedang menghapus Windows.old… bisa lebih dari 10 menit (${e.index + 1} dari ${e.total})`
        : `Membersihkan ${e.label.toLowerCase()}… (${e.index + 1} dari ${e.total})`
  } else {
    target = 0.98
    stepText.value = 'Menyimpan catatan pembersihan…'
  }
  live.value = stepText.value
}

function stopCreep() {
  if (creep !== undefined) window.clearInterval(creep)
  creep = undefined
}
onBeforeUnmount(stopCreep)

async function run() {
  if (!preview.value || chosenCount.value === 0) return
  confirming.value = false
  stage.value = 'berjalan'
  level.value = 0.04
  target = 0.08
  stepText.value = 'Bersiap…'
  window.scrollTo({ top: 0 })
  stopCreep()
  creep = window.setInterval(() => (level.value = Math.max(level.value, level.value + (target - level.value) * 0.1)), 250)
  try {
    outcome.value = await cleanupApply({ kinds: selected.value, windowsOld: windowsOld.value && !!preview.value.windowsOld }, onEvent)
  } catch (e) {
    outcome.value = {
      ok: false,
      error: { kind: 'other', message: e instanceof Error ? e.message : String(e), help: [] },
      entry: null,
      freedBytes: 0,
      deletedFiles: 0,
      skippedFiles: 0,
      failures: [],
    }
  }
  stopCreep()
  level.value = 1
  await new Promise((r) => setTimeout(r, 500))
  stage.value = 'hasil'
  live.value = outcome.value?.error?.message ?? `Selesai. ${sizeText(outcome.value?.freedBytes ?? 0)} dibebaskan.`
  try {
    status.value = await cleanupStatus()
  } catch {
    /* status hanya untuk riwayat */
  }
}

const statusWord: Record<string, string> = { berhasil: 'Bersih', sebagian: 'Sebagian dilewati', gagal: 'Gagal' }
</script>

<template>
  <main class="bersihkan">
    <p class="sr-only" aria-live="polite">{{ live }}</p>

    <div class="title-row">
      <button v-if="stage !== 'berjalan'" type="button" class="btn back" @click="emit('back')">
        <Icon name="back" />
        Kembali
      </button>
    </div>

    <!-- Sedang membersihkan -->
    <section v-if="stage === 'berjalan'" class="running">
      <h1>Membersihkan file sampah</h1>
      <ScanBubble :scanning="true" :level="level" :label="stepText" />
      <p class="step">{{ stepText }}</p>
      <p class="hint">File yang sedang dipakai program dilewati. Jangan matikan PC.</p>
    </section>

    <!-- Hasil -->
    <section v-else-if="stage === 'hasil' && outcome" class="result glass glass-window" aria-labelledby="clean-result">
      <StatusOrb :status="outcome.entry && !outcome.failures.length ? 'sehat' : 'perhatian'" :size="72" />
      <h2 id="clean-result">
        <template v-if="outcome.entry && outcome.freedBytes > 0">{{ sizeText(outcome.freedBytes) }} berhasil dibebaskan</template>
        <template v-else-if="outcome.entry">Tidak ada yang bisa dihapus saat ini</template>
        <template v-else>Belum bisa dibersihkan</template>
      </h2>
      <p v-if="outcome.entry" class="counts">
        {{ countText(outcome.deletedFiles) }} file terhapus<template v-if="outcome.skippedFiles">,
          {{ countText(outcome.skippedFiles) }} dilewati (sedang dipakai, masih baru, atau browsernya terbuka)</template
        >.
      </p>

      <ul v-if="outcome.entry" class="kinds-result">
        <li v-for="k in outcome.entry.jenis" :key="k.jenis" :class="k.status">
          <div class="kr-head">
            <strong>{{ k.label }}</strong>
            <span class="chip">{{ statusWord[k.status] }}</span>
            <span class="kr-size">{{ sizeText(k.byteDibebaskan) }}</span>
          </div>
          <p class="kr-count">
            {{ countText(k.fileTerhapus) }} file terhapus<template v-if="k.dilewati">, {{ countText(k.dilewati) }} dilewati</template>
          </p>
          <ul v-if="k.catatan.length" class="kr-notes">
            <li v-for="n in k.catatan" :key="n">{{ n }}</li>
          </ul>
        </li>
      </ul>

      <div v-if="outcome.failures.length" class="failures">
        <h3>Yang gagal</h3>
        <ul>
          <li v-for="f in outcome.failures" :key="f.label">
            <strong>{{ f.label }}</strong> — {{ f.reason }}
          </li>
        </ul>
        <p>Jenis lain tetap dibersihkan.</p>
      </div>

      <div v-if="outcome.error" class="error-box">
        <p>{{ outcome.error.message }}</p>
        <ol v-if="outcome.error.help.length">
          <li v-for="h in outcome.error.help" :key="h">{{ h }}</li>
        </ol>
      </div>

      <p v-if="outcome.entry && status" class="logpath">Catatan pembersihan disimpan di {{ status.logPath }}</p>
      <button type="button" class="btn btn-primary" @click="measure">Selesai</button>
    </section>

    <template v-else>
      <header class="intro">
        <h1>Bersihkan File Sampah</h1>
        <p class="lead">
          File sementara, cache browser, dan sisa unduhan update menumpuk tanpa terasa. Pilih jenis yang mau dibersihkan — hanya isi folder sampah
          yang dihapus, file kamu yang lain tidak disentuh.
        </p>
      </header>

      <section v-if="status && !isAdmin && status.isWindows" class="admin glass" aria-labelledby="clean-admin">
        <StatusOrb status="perhatian" :size="40" />
        <div>
          <h2 id="clean-admin">Membersihkan file sampah butuh izin administrator</h2>
          <p>
            Folder C:\Windows\Temp dan sisa unduhan Windows Update hanya bisa dibersihkan dengan izin itu. RepairDoc akan dibuka ulang dengan izin
            tersebut.
          </p>
          <button type="button" class="btn btn-primary" :disabled="elevating" @click="emit('elevate')">
            <Icon name="shield" />
            {{ elevating ? 'Menunggu izin…' : 'Minta izin administrator' }}
          </button>
          <p v-if="elevateMsg" class="msg" role="status">{{ elevateMsg }}</p>
        </div>
      </section>

      <p v-if="status?.error" class="glass warn-box">{{ status.error }}</p>
      <div v-if="error" class="glass warn-box" role="alert">
        <p>{{ error.message }}</p>
        <ul v-if="error.help.length">
          <li v-for="h in error.help" :key="h">{{ h }}</li>
        </ul>
      </div>

      <section v-if="stage === 'memuat'" class="measuring glass glass-window" role="status">
        <span class="spinner" aria-hidden="true" />
        <div>
          <h2>Mengukur file sampah…</h2>
          <p>Biasanya kurang dari setengah menit. Belum ada yang dihapus.</p>
        </div>
      </section>

      <div v-else-if="preview" class="layout">
        <section class="list glass glass-window" aria-labelledby="clean-list">
          <h2 id="clean-list" class="sr-only">Jenis file sampah</h2>
          <ul class="kinds">
            <li v-for="k in preview.kinds" :key="k.jenis" class="kind" :class="{ off: !selected.includes(k.jenis), empty: k.bytes === 0 }">
              <label class="pick">
                <input type="checkbox" :checked="selected.includes(k.jenis)" :disabled="k.bytes === 0" @change="toggle(k.jenis)" />
                <span class="sr-only">Bersihkan {{ k.label }}</span>
              </label>
              <div class="info">
                <h3>{{ k.label }}</h3>
                <p class="desc">{{ k.description }}</p>
                <ul v-if="k.notes.length" class="notes">
                  <li v-for="n in k.notes" :key="n">{{ n }}</li>
                </ul>
                <details v-if="k.locations.length" class="where">
                  <summary><Icon name="folder" /> {{ k.locations.length }} lokasi folder</summary>
                  <ul>
                    <li v-for="l in k.locations" :key="l">{{ l }}</li>
                  </ul>
                </details>
              </div>
              <div class="size">
                <strong>{{ k.approximate ? '±' : '' }}{{ sizeText(k.bytes) }}</strong>
                <span>{{ countText(k.files) }} file</span>
                <em v-if="!selected.includes(k.jenis) && k.bytes > 0" class="not-picked">Tidak dipilih</em>
              </div>
            </li>
          </ul>

          <section v-if="preview.windowsOld" class="winold" :class="{ on: windowsOld }" aria-labelledby="winold-title">
            <div class="wo-head">
              <Icon name="alert" />
              <h3 id="winold-title">Sisa Windows lama (Windows.old)</h3>
              <strong class="wo-size">{{ preview.windowsOld.approximate ? '±' : '' }}{{ sizeText(preview.windowsOld.bytes) }}</strong>
            </div>
            <p>{{ preview.windowsOld.warning }}</p>
            <p v-if="preview.windowsOld.createdMs && rollbackUntil" class="wo-date">
              Folder ini dibuat {{ tanggalText(preview.windowsOld.createdMs) }}.
              <template v-if="rollbackUntil > Date.now()">Kalau upgrade-nya baru, kamu biasanya masih bisa kembali ke Windows lama sampai sekitar {{ tanggalText(rollbackUntil) }}.</template>
              <template v-else>Batas 10 hari untuk kembali ke Windows lama biasanya sudah lewat.</template>
            </p>
            <p class="wo-how">Dihapus memakai Disk Cleanup bawaan Windows. Tidak ikut tercentang bersama yang lain.</p>
            <label class="wo-pick">
              <input v-model="windowsOld" type="checkbox" @change="confirming = false" />
              Ya, hapus Windows.old. Saya mengerti ini tidak bisa dibatalkan.
            </label>
          </section>

          <p v-if="preview.hiberfil || preview.pagefile" class="sysfiles">
            <Icon name="info" />
            <span>
              Tidak pernah dihapus:
              <template v-if="preview.hiberfil">hiberfil.sys ({{ sizeText(preview.hiberfil) }})</template
              ><template v-if="preview.hiberfil && preview.pagefile"> dan </template
              ><template v-if="preview.pagefile">pagefile.sys ({{ sizeText(preview.pagefile) }})</template>. Keduanya file sistem Windows, bukan
              sampah.
            </span>
          </p>
        </section>

        <aside class="summary glass glass-window" aria-labelledby="clean-summary">
          <h2 id="clean-summary">Akan dibersihkan</h2>
          <p class="total">
            <strong>±{{ sizeText(chosenBytes) }}</strong>
            <span>dari {{ chosenCount }} jenis</span>
          </p>
          <ul v-if="chosenCount" class="chosen">
            <li v-for="k in chosen" :key="k.jenis">{{ k.label }}</li>
            <li v-if="windowsOld && preview.windowsOld" class="danger">Windows.old</li>
          </ul>
          <p v-else class="none">Belum ada yang dicentang.</p>

          <p class="permanent">
            <Icon name="alert" />
            File yang dihapus tidak bisa dikembalikan. Angka di atas perkiraan; hasil sebenarnya ditampilkan setelah selesai.
          </p>

          <div v-if="confirming" class="confirm" role="alertdialog" aria-labelledby="confirm-text">
            <p id="confirm-text">
              Akan menghapus ±{{ sizeText(chosenBytes) }} dari {{ chosenCount }} jenis<template v-if="windowsOld && preview.windowsOld"
                >, termasuk Windows.old</template
              >. File yang dihapus tidak bisa dikembalikan. Lanjutkan?
            </p>
            <div class="confirm-actions">
              <button type="button" class="btn" @click="confirming = false">Batal</button>
              <button type="button" class="btn btn-primary" @click="run">Ya, bersihkan</button>
            </div>
          </div>
          <button v-else type="button" class="btn btn-primary go" :disabled="!isAdmin || chosenCount === 0" @click="confirming = true">
            <Icon name="cleanup" />
            Bersihkan
          </button>

          <p v-if="status?.last" class="last">
            Terakhir dibersihkan {{ waktuText(status.last.waktu) }}:
            {{ sizeText(status.last.jenis.reduce((a, k) => a + k.byteDibebaskan, 0)) }} dibebaskan.
          </p>
        </aside>
      </div>
    </template>
  </main>
</template>

<style scoped>
.bersihkan {
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

.measuring {
  display: flex;
  gap: 16px;
  align-items: center;
  padding: 22px 24px;
}

.measuring > * {
  position: relative;
}

.measuring h2 {
  font-size: 18px;
  font-weight: 600;
}

.measuring p {
  font-size: 14px;
  color: var(--ink-soft);
}

.spinner {
  flex: none;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  border: 4px solid rgba(18, 155, 219, 0.2);
  border-top-color: var(--aqua);
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .spinner {
    animation-duration: 3s;
  }
}

.layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: 16px;
  align-items: start;
}

.list {
  display: grid;
  gap: 10px;
  padding: 10px 22px 18px;
}

.list > *,
.summary > *,
.result > * {
  position: relative;
}

.kinds {
  list-style: none;
  padding: 0;
}

.kind {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr) auto;
  gap: 12px;
  align-items: start;
  padding: 14px 0;
  border-bottom: 1px solid rgba(64, 150, 205, 0.18);
}

/* Jenis yang tidak dicentang: ditandai checkbox kosong + label "Tidak dipilih".
   Angka & teksnya TIDAK dipudarkan, supaya tetap terbaca. */
.not-picked {
  margin-top: 4px;
  padding: 1px 9px;
  border-radius: 999px;
  background: #eef4f8;
  color: var(--ink-soft);
  font-size: 13px;
  font-style: normal;
  font-weight: 600;
}

.kind.empty .size strong {
  color: var(--ink-soft);
}

/* Area klik 32×32 px (kotak centangnya sendiri 20 px). */
.pick {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  margin-top: -2px;
  cursor: pointer;
}

.pick input,
.wo-pick input {
  width: 20px;
  height: 20px;
  margin: 0;
  accent-color: var(--aqua-deep);
  cursor: pointer;
}

.info h3 {
  font-size: 17px;
  font-weight: 600;
}

.desc {
  font-size: 14px;
  color: var(--ink-soft);
}

.notes {
  margin-top: 6px;
  padding-left: 18px;
  font-size: 13px;
  color: var(--ink-soft);
}

.where {
  margin-top: 6px;
  font-size: 13px;
}

.where summary {
  display: inline-flex;
  align-items: center;
  min-height: 32px;
  gap: 6px;
  cursor: pointer;
  color: var(--aqua-ink);
  font-weight: 600;
}

.where summary svg {
  width: 15px;
  height: 15px;
}

.where ul {
  margin-top: 4px;
  padding-left: 22px;
  color: var(--ink-soft);
  overflow-wrap: anywhere;
}

.size {
  display: grid;
  justify-items: end;
  text-align: right;
  min-width: 96px;
}

.size strong {
  font-size: 21px;
  font-weight: 600;
  color: var(--aqua-ink);
  line-height: 1.2;
}

.size span {
  font-size: 13px;
  color: var(--ink-soft);
}

.winold {
  display: grid;
  gap: 6px;
  margin-top: 6px;
  padding: 14px 16px;
  border-radius: 14px;
  background: var(--warn-soft);
  border: 1px solid rgba(240, 161, 14, 0.45);
  color: var(--warn-ink);
  font-size: 14px;
}

.winold.on {
  background: var(--bad-soft);
  border-color: rgba(223, 68, 62, 0.5);
  color: var(--bad-ink);
}

.wo-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.wo-head svg {
  width: 20px;
  height: 20px;
  flex: none;
}

.wo-head h3 {
  font-size: 16px;
  font-weight: 600;
  flex: 1;
}

.wo-size {
  font-size: 19px;
}

.wo-how {
  font-size: 13px;
  opacity: 0.9;
}

.wo-pick {
  display: flex;
  align-items: center;
  min-height: 36px;
  gap: 10px;
  margin-top: 4px;
  font-weight: 600;
  cursor: pointer;
}

.sysfiles {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  font-size: 13px;
  color: var(--ink-soft);
}

.sysfiles svg {
  flex: none;
  width: 16px;
  height: 16px;
  margin-top: 2px;
  color: var(--aqua-deep);
}

.summary {
  position: sticky;
  top: 84px;
  display: grid;
  gap: 12px;
  padding: 20px 20px 18px;
}

.summary h2 {
  font-size: 15px;
  font-weight: 600;
  color: var(--ink-soft);
}

.total {
  display: grid;
}

.total strong {
  font-size: 34px;
  font-weight: 300;
  letter-spacing: -0.02em;
  color: var(--aqua-ink);
  line-height: 1.1;
}

.total span {
  font-size: 14px;
  color: var(--ink-soft);
}

.chosen {
  padding-left: 18px;
  font-size: 14px;
}

.chosen .danger {
  color: var(--bad-ink);
  font-weight: 600;
}

.none {
  font-size: 14px;
  color: var(--ink-soft);
}

.permanent {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(255, 242, 207, 0.85);
  color: var(--warn-ink);
  font-size: 13px;
}

.permanent svg {
  flex: none;
  width: 17px;
  height: 17px;
  margin-top: 1px;
}

.go {
  width: 100%;
  justify-content: center;
}

.confirm {
  display: grid;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 14px;
  background: var(--bad-soft);
  color: var(--bad-ink);
  font-size: 14px;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.last {
  font-size: 13px;
  color: var(--ink-soft);
}

.running {
  display: grid;
  justify-items: center;
  gap: 18px;
  padding-top: 2vh;
  text-align: center;
}

.step {
  font-size: 17px;
  font-weight: 600;
  color: var(--aqua-ink);
}

.hint {
  font-size: 14px;
  color: var(--ink-soft);
}

.result {
  display: grid;
  justify-items: center;
  gap: 12px;
  max-width: 760px;
  margin: 0 auto;
  padding: 28px 30px 26px;
  text-align: center;
}

.result h2 {
  font-size: 28px;
  font-weight: 300;
  letter-spacing: -0.015em;
}

.counts {
  font-size: 16px;
}

.kinds-result {
  list-style: none;
  width: 100%;
  padding: 0;
  text-align: left;
}

.kinds-result > li {
  padding: 10px 0;
  border-top: 1px solid rgba(64, 150, 205, 0.18);
}

.kr-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.kr-head strong {
  font-size: 15px;
}

.chip {
  padding: 1px 9px;
  border-radius: 999px;
  font-size: 13px;
  background: var(--ok-soft);
  color: var(--ok-ink);
}

.sebagian .chip {
  background: var(--warn-soft);
  color: var(--warn-ink);
}

.gagal .chip {
  background: var(--bad-soft);
  color: var(--bad-ink);
}

.kr-size {
  margin-left: auto;
  font-weight: 600;
  color: var(--aqua-ink);
}

.kr-count,
.kr-notes {
  font-size: 13px;
  color: var(--ink-soft);
}

.kr-notes {
  padding-left: 18px;
}

.failures,
.error-box {
  width: 100%;
  padding: 12px 16px;
  border-radius: 14px;
  background: var(--warn-soft);
  color: var(--warn-ink);
  font-size: 14px;
  text-align: left;
}

.failures h3 {
  font-size: 15px;
}

.failures ul,
.error-box ol {
  padding-left: 18px;
}

.logpath {
  font-size: 13px;
  color: var(--ink-soft);
  overflow-wrap: anywhere;
}

@media (max-width: 860px) {
  .layout {
    grid-template-columns: 1fr;
  }

  .summary {
    position: static;
  }
}
</style>
