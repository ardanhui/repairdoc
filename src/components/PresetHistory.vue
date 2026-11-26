<script setup lang="ts">
import { ref } from 'vue'
import { modeName, waktuText } from '../lib/labels'
import type { LogEntry, PresetStatus } from '../lib/types'
import Icon from './Icon.vue'

const props = defineProps<{ status: PresetStatus; disabled: boolean }>()
const emit = defineEmits<{ revert: [] }>()

const confirming = ref(false)

function counts(e: LogEntry): string {
  const items = new Map<string, string>()
  for (const c of e.perubahan) {
    const k = c.itemKey || c.targetTeks
    const prev = items.get(k)
    if (prev === 'gagal' || (prev === 'berhasil' && c.status === 'dilewati')) continue
    items.set(k, c.status)
  }
  const n = (s: string) => [...items.values()].filter((v) => v === s).length
  const parts = [`${n('berhasil')} diubah`]
  if (n('dilewati')) parts.push(`${n('dilewati')} dilewati`)
  if (n('gagal')) parts.push(`${n('gagal')} gagal`)
  return parts.join(', ')
}

function what(e: LogEntry): string {
  return e.jenis === 'kembalikan' ? 'Mode Normal (membatalkan perubahan sebelumnya)' : `Mode ${modeName[e.mode]}`
}

function confirm() {
  confirming.value = false
  emit('revert')
}

const last = () => props.status.lastEntry
const target = () => props.status.revertTarget
</script>

<template>
  <section class="history glass" aria-labelledby="history-title">
    <h2 id="history-title">Riwayat perubahan</h2>

    <div v-if="last()" class="entry">
      <p class="what">{{ what(last()!) }}</p>
      <p>Diterapkan {{ waktuText(last()!.waktu) }} di {{ last()!.namaPc }}</p>
      <p>{{ counts(last()!) }}</p>
      <p v-if="last()!.restorePoint">Restore point: {{ last()!.restorePoint }}</p>
      <p v-if="last()!.dikembalikan && last()!.waktuDikembalikan" class="undone">Sudah dikembalikan {{ waktuText(last()!.waktuDikembalikan!) }}</p>
    </div>
    <p v-else class="empty">Belum ada perubahan RepairDoc di PC ini.</p>

    <div class="revert">
      <template v-if="!confirming">
        <button type="button" class="btn big" :disabled="disabled || !target()" @click="confirming = true">
          <Icon name="undo" />
          Kembalikan Semua
        </button>
        <p v-if="target()" class="hint">
          Membatalkan Mode {{ modeName[target()!.mode] }} dari {{ waktuText(target()!.waktu) }}. Setiap pengaturan kembali ke nilai sebelum diubah.
        </p>
        <p v-else class="hint">Belum ada perubahan yang bisa dikembalikan.</p>
      </template>
      <div v-else class="confirm" role="group" aria-label="Konfirmasi kembalikan semua">
        <p>Kembalikan semua pengaturan dari Mode {{ modeName[target()!.mode] }} ke nilai sebelumnya?</p>
        <div class="confirm-actions">
          <button type="button" class="btn" @click="confirming = false">Batal</button>
          <button type="button" class="btn btn-primary" @click="confirm">Ya, kembalikan</button>
        </div>
      </div>
    </div>

    <details class="rp-help">
      <summary>Jaring pengaman kedua: System Restore</summary>
      <p>
        Setiap kali Mode Performa diterapkan, RepairDoc membuat restore point lebih dulu. Kalau Windows bermasalah dan tombol di atas tidak bisa
        dipakai:
      </p>
      <ol>
        <li>Ketik "Create a restore point" di menu Start, lalu buka.</li>
        <li>Klik tombol System Restore, lalu Next.</li>
        <li>Pilih restore point yang namanya diawali "RepairDoc", lalu Next dan Finish. PC akan restart.</li>
      </ol>
      <p class="path">Catatan perubahan disimpan di {{ status.logPath }}</p>
    </details>
  </section>
</template>

<style scoped>
.history {
  display: grid;
  gap: 14px;
  padding: 20px 22px;
}

h2 {
  font-size: 19px;
  font-weight: 600;
}

.entry {
  display: grid;
  gap: 2px;
  font-size: 14px;
  color: var(--ink-soft);
}

.entry .what {
  font-size: 16px;
  font-weight: 600;
  color: var(--ink);
}

.undone {
  color: var(--ok-ink);
  font-weight: 600;
}

.empty {
  font-size: 14px;
  color: var(--ink-soft);
}

.revert {
  display: grid;
  gap: 8px;
}

.big {
  min-height: 52px;
  font-size: 17px;
}

.big svg {
  width: 22px;
  height: 22px;
}

.hint {
  font-size: 13px;
  color: var(--ink-soft);
}

.confirm {
  display: grid;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 12px;
  background: var(--warn-soft);
  color: var(--warn-ink);
  font-size: 14px;
}

.confirm-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.rp-help {
  font-size: 13px;
  color: var(--ink-soft);
}

.rp-help summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--aqua-ink);
  padding: 4px 0;
}

.rp-help ol {
  padding-left: 18px;
  margin: 6px 0;
}

.path {
  overflow-wrap: anywhere;
}
</style>
