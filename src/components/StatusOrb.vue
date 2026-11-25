<script setup lang="ts">
import type { Status } from '../lib/types'
import Icon from './Icon.vue'

withDefaults(defineProps<{ status: Status; size?: number }>(), { size: 32 })
const icon = { sehat: 'check', perhatian: 'alert', bermasalah: 'cross' } as const
</script>

<template>
  <span class="orb-status" :class="status" :style="{ width: `${size}px`, height: `${size}px` }" aria-hidden="true">
    <Icon :name="icon[status]" />
  </span>
</template>
