<script setup lang="ts">
import { modeName } from '../lib/labels'
import type { Outcome } from '../lib/types'
import StatusOrb from './StatusOrb.vue'

const props = defineProps<{ outcome: Outcome; kind: 'apply' | 'revert' }>()
defineEmits<{ done: [] }>()

function headline(): string {
  const o = props.outcome
  if (o.error?.kind === 'restorePoint') return 'Restore point tidak bisa dibuat'
  if (o.error?.kind === 'nothingToRevert') return 'Tidak ada yang perlu dikembalikan'
  if (o.error && !o.entry) return 'Belum bisa dijalankan'
  if (props.kind === 'revert') return o.failed ? 'Sebagian perubahan dikembalikan' : 'Perubahan sudah dikembalikan'
  const name = o.mode ? `Mode ${modeName[o.mode]}` : 'Mode'
  if (o.changed === 0 && !o.failed) return `${name} sudah berlaku — tidak ada yang perlu diubah`
  return o.failed ? `${name} diterapkan sebagian` : `${name} diterapkan`
}

const status = () => (props.outcome.error && !props.outcome.entry ? 'perhatian' : props.outcome.failed ? 'perhatian' : 'sehat')
</script>

<template>
  <section class="result glass glass-window" aria-live="polite">
    <StatusOrb :status="status()" :size="72" />
    <h2>{{ headline() }}</h2>

    <template v-if="outcome.entry">
      <p class="counts">
        {{ outcome.changed }} pengaturan {{ kind === 'revert' ? 'dikembalikan' : 'diubah' }}<template v-if="outcome.skipped"
          >, {{ outcome.skipped }} dilewati</template
        ><template v-if="outcome.failed">, {{ outcome.failed }} gagal</template>.
      </p>
      <div v-if="outcome.failures.length" class="failures">
        <h3>Yang gagal</h3>
        <ul>
          <li v-for="f in outcome.failures" :key="f.label">
            <strong>{{ f.label }}</strong> — {{ f.reason }}
          </li>
        </ul>
        <p>Pengaturan lain tetap diterapkan. Yang gagal tidak diubah sama sekali.</p>
      </div>
      <p v-if="outcome.needsRestart && kind === 'apply'" class="restart">
        Sebagian perubahan baru terasa setelah keluar-masuk akun (sign out) atau restart. Simpan pekerjaan dulu, lalu restart PC saat sempat.
      </p>
      <p v-if="outcome.restorePoint" class="rp">
        Restore point dibuat: <strong>{{ outcome.restorePoint }}</strong>
      </p>
    </template>

    <div v-if="outcome.error" class="error">
      <p>{{ outcome.error.message }}</p>
      <template v-if="outcome.error.help.length">
        <h3>{{ outcome.error.kind === 'restorePoint' ? 'Cara menyalakan System Restore' : 'Yang bisa dilakukan' }}</h3>
        <ol>
          <li v-for="h in outcome.error.help" :key="h">{{ h }}</li>
        </ol>
      </template>
    </div>

    <button type="button" class="btn btn-primary" @click="$emit('done')">Selesai</button>
  </section>
</template>

<style scoped>
.result {
  display: grid;
  justify-items: center;
  gap: 12px;
  max-width: 640px;
  margin: 0 auto;
  padding: 30px 32px;
  text-align: center;
}

.result > * {
  position: relative;
}

h2 {
  font-size: 28px;
  font-weight: 300;
  line-height: 1.2;
  letter-spacing: -0.015em;
}

.counts {
  font-size: 17px;
}

h3 {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 4px;
}

.failures,
.error {
  justify-self: stretch;
  padding: 14px 16px;
  border-radius: 12px;
  background: var(--warn-soft);
  color: var(--warn-ink);
  text-align: left;
  font-size: 14px;
}

.failures ul,
.error ol {
  padding-left: 20px;
  margin-bottom: 6px;
}

.error > p {
  margin-bottom: 8px;
}

.restart {
  color: var(--ink-soft);
  font-size: 14px;
  max-width: 52ch;
}

.rp {
  font-size: 14px;
  color: var(--ok-ink);
}

.btn {
  margin-top: 6px;
  min-width: 140px;
}
</style>
