<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import Icon from './Icon.vue'
import { call } from '../lib/api'

const emit = defineEmits(['close', 'done', 'toast'])
const path = ref('')
const input = ref(null)
const busy = ref(false)

onMounted(() => input.value && input.value.focus())

function onKey(e) {
  if (e.key === 'Escape') emit('close')
}
onMounted(() => window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))

async function submit() {
  const value = path.value.trim()
  if (!value || busy.value) return
  busy.value = true
  try {
    const res = await call('add_project', { path: value })
    emit('done', res)
  } catch (err) {
    emit('toast', { text: String(err), error: true })
  } finally {
    busy.value = false
  }
}

async function pick() {
  busy.value = true
  try {
    const res = await call('pick_projects')
    if (!res.canceled) emit('done', res)
  } catch (err) {
    emit('toast', { text: String(err), error: true })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="backdrop" @mousedown.self="emit('close')">
    <div class="modal" @keydown="onKey">
      <header class="modal-head">
        <span class="modal-icon"><Icon name="folder" :size="15" /></span>
        <h2>添加项目</h2>
        <button class="mini" @click="emit('close')"><Icon name="x" :size="14" /></button>
      </header>
      <p class="hint">输入仓库路径，或选一个装着多个仓库的文件夹</p>
      <input ref="input" v-model="path" class="input" placeholder="~/work/my-repo" @keydown.enter="submit" />
      <footer class="modal-foot">
        <button class="btn" :disabled="busy" @click="pick"><Icon name="folder" :size="13" />选择文件夹</button>
        <span class="spacer"></span>
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn primary" :disabled="busy" @click="submit">
          <Icon name="check" :size="13" />添加
        </button>
      </footer>
    </div>
  </div>
</template>
