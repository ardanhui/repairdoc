<script setup lang="ts">
// Logo RepairDoc: tetesan air dengan garis detak jantung — "dokter" untuk PC.
withDefaults(defineProps<{ size?: number }>(), { size: 36 })
</script>

<template>
  <svg :width="size" :height="size" viewBox="0 0 64 64" aria-hidden="true">
    <defs>
      <radialGradient id="rd-drop" cx="50%" cy="95%" r="80%">
        <stop offset="0" stop-color="#8be9ff" />
        <stop offset="0.5" stop-color="#1ba3e6" />
        <stop offset="1" stop-color="#0a64a8" />
      </radialGradient>
      <linearGradient id="rd-shine" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#fff" stop-opacity="0.9" />
        <stop offset="1" stop-color="#fff" stop-opacity="0" />
      </linearGradient>
    </defs>
    <path d="M32 3C32 3 11 27 11 41.5a21 21 0 0 0 42 0C53 27 32 3 32 3z" fill="url(#rd-drop)" stroke="#fff" stroke-width="2.5" />
    <ellipse cx="24" cy="30" rx="6" ry="11" transform="rotate(24 24 30)" fill="url(#rd-shine)" />
    <polyline points="17,43 25,43 28.5,35 34,51 38.5,39 41.5,43 47,43" fill="none" stroke="#fff" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
</template>
