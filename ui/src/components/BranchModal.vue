<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import Icon from './Icon.vue'
import { call } from '../lib/api'

const props = defineProps({ project: { type: Object, required: true } })
const emit = defineEmits(['close', 'toast', 'changed'])

const branches = ref([])
const current = ref(props.project.branch || '')
const loading = ref(true)
const busy = ref(false)

async function load() {
  loading.value = true
  try {
    const res = await call('branches', { path: props.project.path })
    branches.value = res.branches || []
    current.value = res.current || current.value
  } catch (err) {
    emit('toast', { text: String(err), error: true })
    emit('close')
  } finally {
    loading.value = false
  }
}

async function checkout(branch) {
  if (busy.value || branch === current.value) return
  busy.value = true
  try {
    await call('checkout', { path: props.project.path, branch })
    emit('toast', { text: '已切换到 ' + branch, error: false })
    emit('changed')
    emit('close')
  } catch (err) {
    emit('toast', { text: String(err), error: true })
  } finally {
    busy.value = false
  }
}

function onKey(e) {
  if (e.key === 'Escape') emit('close')
}
onMounted(() => {
  load()
  window.addEventListener('keydown', onKey)
})
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="backdrop" @mousedown.self="emit('close')">
    <div class="modal">
      <header class="modal-head">
        <span class="modal-icon"><Icon name="branch" :size="15" /></span>
        <h2>切换分支 · {{ project.name }}</h2>
        <button class="mini" @click="emit('close')"><Icon name="x" :size="14" /></button>
      </header>

      <div class="branch-list">
        <div v-if="loading" class="branch-loading">
          <span class="spinner"></span> 加载分支中…
        </div>
        <template v-else>
          <div v-if="!branches.length" class="branch-loading">没有本地分支</div>
          <button
            v-for="b in branches"
            :key="b"
            class="branch-item"
            :class="{ current: b === current }"
            :disabled="busy"
            @click="checkout(b)"
          >
            <Icon name="branch" :size="12" />
            <span class="branch-name">{{ b }}</span>
            <Icon v-if="b === current" name="check" :size="12" class="tick" />
          </button>
        </template>
      </div>

      <footer class="modal-foot">
        <button class="btn" @click="call('reveal', { path: project.path })"><Icon name="folder" :size="13" />打开目录</button>
      </footer>
    </div>
  </div>
</template>
