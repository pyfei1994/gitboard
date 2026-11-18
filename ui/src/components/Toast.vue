<script setup>
import Icon from './Icon.vue'
defineProps({ toast: { type: Object, default: null } })
</script>

<template>
  <transition name="toast">
    <div v-if="toast" class="toast" :class="toast.error ? 'error' : 'ok'">
      <Icon :name="toast.error ? 'alert' : 'check-circle'" :size="14" />
      <span>{{ toast.text }}</span>
    </div>
  </transition>
</template>
