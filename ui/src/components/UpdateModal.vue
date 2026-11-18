<script setup>
import { computed, onMounted, onUnmounted } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  update: { type: Object, required: true },
  busy: { type: Boolean, default: false },
})
const emit = defineEmits(['close', 'download', 'release'])

const notes = computed(() => {
  const text = (props.update.notes || '').trim()
  return text.length > 900 ? text.slice(0, 900) + '…' : text || '（这个版本没有写更新说明）'
})

const sizeText = computed(() => {
  const bytes = props.update.dmg?.size || 0
  return bytes ? (bytes / 1024 / 1024).toFixed(1) + ' MB' : ''
})

function onKey(e) {
  if (e.key === 'Escape') emit('close')
}
onMounted(() => window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="backdrop" @mousedown.self="emit('close')">
    <div class="modal update-modal">
      <header class="modal-head">
        <span class="modal-icon spark"><Icon name="download" :size="15" /></span>
        <h2>发现新版本 v{{ update.latest }}</h2>
        <button class="mini" @click="emit('close')"><Icon name="x" :size="14" /></button>
      </header>

      <p class="hint">
        当前 v{{ update.current }} → 最新 v{{ update.latest }}
        <span v-if="sizeText"> · {{ sizeText }}</span>
      </p>

      <div class="notes">{{ notes }}</div>

      <footer class="modal-foot">
        <button class="btn" @click="emit('release')"><Icon name="grid" :size="13" />打开发布页</button>
        <span class="spacer"></span>
        <button class="btn" :disabled="busy" @click="emit('close')">稍后</button>
        <button class="btn primary" :disabled="busy || !update.dmg" @click="emit('download')">
          <Icon name="download" :size="13" :spin="busy" />{{ busy ? '下载中…' : '下载更新' }}
        </button>
      </footer>
    </div>
  </div>
</template>
