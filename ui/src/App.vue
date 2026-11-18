<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import Icon from './components/Icon.vue'
import ProjectCard from './components/ProjectCard.vue'
import ProjectRow from './components/ProjectRow.vue'
import SkeletonCard from './components/SkeletonCard.vue'
import AddModal from './components/AddModal.vue'
import BranchModal from './components/BranchModal.vue'
import Toast from './components/Toast.vue'
import { call, onState } from './lib/api'

const projects = ref([])
const loading = ref(true)
const refreshing = ref(false)
const busy = ref(false)
const progress = ref({ done: 0, total: 0, name: '' })
const toast = ref(null)
const showAdd = ref(false)
const branchTarget = ref(null)
const params = new URLSearchParams(location.search)
const view = ref(params.get('view') || localStorage.getItem('gb-view') || 'grid')
const theme = ref(params.get('theme') || localStorage.getItem('gb-theme') || 'dark')
const query = ref('')
const filter = ref('all')
const now = ref(Date.now())

let timer = null
let unlisten = null
let toastTimer = null

document.documentElement.dataset.theme = theme.value

const summary = computed(() => {
  const total = projects.value.length
  const dirty = projects.value.filter((p) => p.dirty).length
  const added = projects.value.reduce((n, p) => n + (p.today?.added || 0), 0)
  const deleted = projects.value.reduce((n, p) => n + (p.today?.deleted || 0), 0)
  const commits = projects.value.reduce((n, p) => n + (p.today?.commits || 0), 0)
  return { total, dirty, added, deleted, commits }
})

const visible = computed(() => {
  const q = query.value.trim().toLowerCase()
  const list = projects.value.filter((p) => {
    if (filter.value === 'dirty' && !p.dirty) return false
    if (filter.value === 'clean' && (p.dirty || p.state !== 'ok')) return false
    if (!q) return true
    return (p.name || '').toLowerCase().includes(q) || (p.branch || '').toLowerCase().includes(q)
  })
  return list.sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
    if (a.dirty !== b.dirty) return a.dirty ? -1 : 1
    return (a.name || '').localeCompare(b.name || '')
  })
})

function showToast(text, error = false) {
  toast.value = { text, error }
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => (toast.value = null), 3600)
}

function applyState(payload) {
  if (!payload || !Array.isArray(payload.projects)) return
  projects.value = payload.projects
}

async function refresh() {
  if (refreshing.value) return
  refreshing.value = true
  try {
    applyState(await call('refresh'))
  } catch (err) {
    showToast(String(err), true)
  } finally {
    refreshing.value = false
  }
}

async function pullAll() {
  if (busy.value) return
  busy.value = true
  progress.value = { done: 0, total: projects.value.length, name: '' }
  try {
    const res = await call('pull', { path: null })
    const results = res.results || []
    const failed = results.filter((r) => !r.ok)
    const updated = results.filter((r) => r.ok && r.message !== '已是最新').length
    const same = results.filter((r) => r.ok && r.message === '已是最新').length
    if (failed.length) showToast(`拉取完成：${updated} 个更新 · ${failed.length} 个失败（${failed[0].name}）`, true)
    else showToast(`拉取完成：${updated} 个更新 · ${same} 个已是最新`)
    if (res.state) applyState(res.state)
    else await refresh()
  } catch (err) {
    showToast(String(err), true)
  } finally {
    busy.value = false
  }
}

async function pullOne(project) {
  busy.value = true
  progress.value = { done: 0, total: 1, name: project.name }
  try {
    const res = await call('pull', { path: project.path })
    const result = (res.results || [])[0]
    showToast(`${project.name}：${result?.message || '已更新'}`, !(result?.ok ?? true))
    if (res.state) applyState(res.state)
    else await refresh()
  } catch (err) {
    showToast(String(err), true)
  } finally {
    busy.value = false
  }
}

async function togglePin(project) {
  const next = !project.pinned
  try {
    const res = await call('pin_project', { path: project.key || project.path, pinned: next })
    if (res.state) applyState(res.state)
    showToast(next ? `已置顶 ${project.name}` : `已取消置顶 ${project.name}`)
  } catch (err) {
    showToast(String(err), true)
  }
}

async function removeProject(project) {
  try {
    const res = await call('remove_project', { path: project.key || project.path })
    if (res.state) applyState(res.state)
    showToast('已从看板移除')
  } catch (err) {
    showToast(String(err), true)
  }
}

async function reveal(project) {
  try {
    await call('reveal', { path: project.path })
  } catch (err) {
    showToast(String(err), true)
  }
}

async function onAdded(res) {
  const added = res?.added?.length || 0
  const already = res?.already?.length || 0
  if (res?.error) showToast(String(res.error), true)
  else if (added) showToast(`已添加 ${added} 个项目`)
  else if (already) showToast('该项目已经在看板中')
  else showToast('这个文件夹里没有找到 Git 仓库', true)
  if (res?.state) applyState(res.state)
  else await refresh()
  showAdd.value = false
}

function toggleView() {
  view.value = view.value === 'grid' ? 'list' : 'grid'
  localStorage.setItem('gb-view', view.value)
}

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
  document.documentElement.dataset.theme = theme.value
  localStorage.setItem('gb-theme', theme.value)
}

function onKeydown(e) {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') {
    e.preventDefault()
    refresh()
  }
  if (e.key === 'Escape') {
    showAdd.value = false
    branchTarget.value = null
  }
}

onMounted(async () => {
  try {
    applyState(await call('state'))
  } catch (err) {
    showToast(String(err), true)
  } finally {
    loading.value = false
  }
  call('boot_ok', { info: 'state-loaded projects=' + projects.value.length }).catch(() => {})
  unlisten = onState((payload) => {
    applyState(payload)
    loading.value = false
    if (busy.value) {
      const total = payload.projects?.length || 0
      if (progress.value.total !== total) progress.value = { done: 0, total, name: '' }
    }
  })
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('pointermove', onPointerMove, { passive: true })
  timer = setInterval(() => (now.value = Date.now()), 15000)
})

let raf = 0
let pendingEvent = null

function onPointerMove(event) {
  pendingEvent = event
  if (raf) return
  raf = requestAnimationFrame(() => {
    raf = 0
    const ev = pendingEvent
    if (!ev) return
    const target = ev.target instanceof Element ? ev.target.closest('.card') : null
    if (!target) return
    const rect = target.getBoundingClientRect()
    target.style.setProperty('--mx', ev.clientX - rect.left + 'px')
    target.style.setProperty('--my', ev.clientY - rect.top + 'px')
  })
}

onUnmounted(() => {
  if (unlisten) unlisten()
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('pointermove', onPointerMove)
  clearInterval(timer)
})
</script>

<template>
  <div class="app">
    <div class="aurora" aria-hidden="true">
      <span class="blob b1"></span>
      <span class="blob b2"></span>
      <span class="blob b3"></span>
    </div>

    <header class="topbar">
      <div class="brand">
        <div class="logo"><Icon name="branch" :size="18" /></div>
        <div class="titles">
          <h1>Git 看板</h1>
          <p>
            {{ summary.total }} 个项目 · {{ summary.dirty }} 个有改动 · 今日
            <b class="add">+{{ summary.added }}</b> <b class="del">−{{ summary.deleted }}</b> 行 ·
            {{ summary.commits }} 次提交
          </p>
        </div>
      </div>

      <div class="spacer"></div>

      <div class="toolbar">
        <div class="search">
          <Icon name="search" :size="13" />
          <input v-model="query" placeholder="搜索项目 / 分支" />
        </div>

        <div class="segmented">
          <button :class="{ active: filter === 'all' }" @click="filter = 'all'">全部</button>
          <button :class="{ active: filter === 'dirty' }" @click="filter = 'dirty'">有改动</button>
          <button :class="{ active: filter === 'clean' }" @click="filter = 'clean'">干净</button>
        </div>

        <button class="btn primary" @click="showAdd = true"><Icon name="plus" :size="14" />添加项目</button>
        <button class="btn" :disabled="refreshing" @click="refresh">
          <Icon name="refresh" :size="14" :spin="refreshing" />{{ refreshing ? '刷新中' : '刷新' }}
        </button>
        <button class="btn" :disabled="busy" @click="pullAll">
          <Icon name="download" :size="14" :spin="busy" />
          {{ busy ? `拉取中 ${progress.done}/${progress.total}` : '拉取全部' }}
        </button>
        <button class="icon-btn" :title="view === 'grid' ? '切换列表视图' : '切换卡片视图'" @click="toggleView">
          <Icon :name="view === 'grid' ? 'list' : 'grid'" :size="15" />
        </button>
        <button class="icon-btn" :title="theme === 'dark' ? '浅色模式' : '深色模式'" @click="toggleTheme">
          <Icon :name="theme === 'dark' ? 'sun' : 'moon'" :size="15" />
        </button>
      </div>
    </header>

    <main class="content">
      <div v-if="loading" class="grid">
        <SkeletonCard v-for="i in 6" :key="i" />
      </div>

      <div v-else-if="!projects.length" class="empty">
        <div class="empty-icon"><Icon name="branch" :size="30" /></div>
        <h2>还没有项目</h2>
        <p>添加一个 Git 仓库，或选一个装着多个仓库的文件夹</p>
        <button class="btn primary" @click="showAdd = true"><Icon name="plus" :size="14" />添加项目</button>
      </div>

      <div v-else-if="!visible.length" class="empty small">
        <h2>没有匹配的项目</h2>
        <p>换个关键词，或把筛选切回「全部」</p>
      </div>

      <div v-else-if="view === 'grid'" class="grid">
        <ProjectCard
          v-for="p in visible"
          :key="p.key"
          :project="p"
          @reveal="reveal(p)"
          @branches="branchTarget = p"
          @pin="togglePin(p)"
          @remove="removeProject(p)"
          @pull="pullOne(p)"
        />
      </div>

      <div v-else class="list">
        <div class="list-head row">
          <span>项目</span><span>分支</span><span>改动</span><span>今日</span><span>最近提交</span><span></span>
        </div>
        <ProjectRow
          v-for="p in visible"
          :key="p.key"
          :project="p"
          @reveal="reveal(p)"
          @branches="branchTarget = p"
          @pin="togglePin(p)"
          @remove="removeProject(p)"
        />
      </div>
    </main>

    <AddModal v-if="showAdd" @close="showAdd = false" @done="onAdded" @toast="(t) => showToast(t.text, t.error)" />
    <BranchModal
      v-if="branchTarget"
      :project="branchTarget"
      @close="branchTarget = null"
      @toast="(t) => showToast(t.text, t.error)"
      @changed="refresh"
    />
    <Toast :toast="toast" />
  </div>
</template>
