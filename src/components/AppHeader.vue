<script setup lang="ts">
import LogoDrop from './LogoDrop.vue'

defineProps<{ preview: boolean }>()
</script>

<template>
  <header class="app-header">
    <div class="brand">
      <LogoDrop :size="38" />
      <p class="wordmark"><span class="name">RepairDoc</span> <span class="tag">Dokter PC</span></p>
    </div>
    <p v-if="preview" class="preview" title="Dibuka di browser biasa, bukan aplikasi Windows. Hasil scan memakai data contoh.">
      Mode pratinjau: data contoh
    </p>
  </header>
</template>

<style scoped>
.app-header {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 28px;
  /* Sama dengan langit di belakangnya, supaya konten yang di-scroll lewat di bawahnya. */
  background: linear-gradient(180deg, #1677c9 0%, #2590dd 100%);
  border-bottom: 1px solid rgba(255, 255, 255, 0.45);
  box-shadow: 0 6px 16px -10px rgba(4, 50, 100, 0.5);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.wordmark {
  line-height: 1.1;
  color: #fff;
  text-shadow: 0 1px 2px rgba(5, 60, 110, 0.55);
}

.name {
  font-size: 21px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.tag {
  font-size: 16px;
  font-weight: 300;
  margin-left: 4px;
}

.preview {
  padding: 5px 14px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.85);
  border: 1px solid #fff;
  color: var(--aqua-ink);
  font-size: 13px;
  font-weight: 600;
}
</style>
