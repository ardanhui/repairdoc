<script setup lang="ts">
// Ikon garis sederhana (tanpa library, supaya jalan offline dari flashdisk).
defineProps<{ name: 'hardware' | 'windows' | 'cleanup' | 'security' | 'network' | 'check' | 'alert' | 'cross' | 'shield' | 'refresh' }>()
</script>

<template>
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <template v-if="name === 'hardware'">
      <rect x="6" y="6" width="12" height="12" rx="2" />
      <rect x="9.5" y="9.5" width="5" height="5" rx="1" />
      <path d="M9 2.5V6M15 2.5V6M9 18v3.5M15 18v3.5M2.5 9H6M2.5 15H6M18 9h3.5M18 15h3.5" />
    </template>
    <template v-else-if="name === 'windows'">
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M3 9h18M9 9v11" />
    </template>
    <template v-else-if="name === 'cleanup'">
      <path d="M14 3l-5.5 9.5" />
      <path d="M8.5 12.5l4 2.3-2.2 3.8c-1.1 1.9-3.3 2.6-5.4 1.9l-1.9-.6 3.5-7.4z" />
      <path d="M17.5 4.5l.6 1.6 1.6.6-1.6.6-.6 1.6-.6-1.6-1.6-.6 1.6-.6z" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'security'">
      <path d="M12 2.8l7.5 3v5.6c0 4.6-3.1 8.4-7.5 9.8-4.4-1.4-7.5-5.2-7.5-9.8V5.8z" />
      <path d="M8.5 12l2.4 2.4 4.6-4.8" />
    </template>
    <template v-else-if="name === 'network'">
      <path d="M2.5 9.5a14 14 0 0 1 19 0" />
      <path d="M5.8 13a9.5 9.5 0 0 1 12.4 0" />
      <path d="M9 16.4a5 5 0 0 1 6 0" />
      <circle cx="12" cy="19.5" r="1.3" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'check'">
      <path d="M5 12.5l4.2 4.2L19 7" stroke-width="3" />
    </template>
    <template v-else-if="name === 'alert'">
      <path d="M12 5.5v8" stroke-width="3" />
      <circle cx="12" cy="18" r="1.2" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'cross'">
      <path d="M6.5 6.5l11 11M17.5 6.5l-11 11" stroke-width="3" />
    </template>
    <template v-else-if="name === 'shield'">
      <path d="M12 2.8l7.5 3v5.6c0 4.6-3.1 8.4-7.5 9.8-4.4-1.4-7.5-5.2-7.5-9.8V5.8z" />
      <path d="M8.5 12l2.4 2.4 4.6-4.8" />
    </template>
    <template v-else-if="name === 'refresh'">
      <path d="M20 11a8 8 0 1 0-2.3 5.7" />
      <path d="M20 4.5V11h-6.5" />
    </template>
  </svg>
</template>
