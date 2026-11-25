<script setup lang="ts">
import Icon from './Icon.vue'

defineProps<{ items: string[]; busy: boolean; message: string | null }>()
defineEmits<{ elevate: [] }>()
</script>

<template>
  <aside class="admin glass" aria-labelledby="admin-title">
    <h3 id="admin-title">Sebagian pemeriksaan butuh izin administrator</h3>
    <p>Yang belum bisa diperiksa:</p>
    <ul>
      <li v-for="(it, i) in items" :key="i">{{ it }}</li>
    </ul>
    <button type="button" class="btn" :disabled="busy" @click="$emit('elevate')">
      <Icon name="shield" />
      {{ busy ? 'Menunggu izin…' : 'Minta izin administrator' }}
    </button>
    <p v-if="message" class="msg" role="status">{{ message }}</p>
  </aside>
</template>

<style scoped>
.admin {
  display: grid;
  justify-items: start;
  gap: 8px;
  padding: 18px 20px;
}

h3 {
  font-size: 16px;
  font-weight: 600;
}

p,
li {
  font-size: 14px;
  color: var(--ink-soft);
}

ul {
  padding-left: 18px;
  display: grid;
  gap: 2px;
}

li::first-letter {
  text-transform: uppercase;
}

.msg {
  color: var(--warn-ink);
  font-weight: 600;
}
</style>
