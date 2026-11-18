<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'
import { relTime, truncate, fileBadge } from '../lib/format'

const props = defineProps({ project: { type: Object, required: true } })
defineEmits(['reveal', 'branches', 'pin', 'remove', 'pull'])

const branch = computed(() => props.project.branch || (props.project.detached ? 'detached' : '—'))
const hasCounts = computed(() => props.project.staged || props.project.unstaged || props.project.untracked)
const shortPath = computed(() => {
  const p = props.project.path || ''
  return p.length > 34 ? '…' + p.slice(-32) : p
})
const tone = computed(() => {
  const p = props.project
  if (p.state === 'error') return 'red'
  if (p.state === 'missing' || p.state === 'not-repo') return 'muted'
  return p.dirty ? 'amber' : 'green'
})
</script>

<template>
  <article class="card" :class="[tone, { pinned: project.pinned }]">
    <header class="card-head">
      <span class="dot" :class="tone"></span>
      <h3 class="name" :title="project.name">{{ project.name }}</h3>
      <span v-if="project.pinned" class="pin-flag" title="已置顶"><Icon name="star" :size="11" /></span>
      <span class="spacer"></span>
      <button class="mini" :class="{ active: project.pinned }" :title="project.pinned ? '取消置顶' : '置顶（排到最前）'" @click="$emit('pin')">
        <Icon name="star" :size="13" />
      </button>
      <button class="mini danger" title="移出看板" @click="$emit('remove')">
        <Icon name="x" :size="13" />
      </button>
    </header>

    <template v-if="project.state === 'ok'">
      <div class="meta">
        <span class="branch"><Icon name="branch" :size="12" />{{ truncate(branch, 18) }}</span>
        <span v-if="project.ahead" class="pill blue"><Icon name="arrow-up" :size="10" />{{ project.ahead }}</span>
        <span v-if="project.behind" class="pill red"><Icon name="arrow-down" :size="10" />{{ project.behind }}</span>
        <span class="spacer"></span>
        <span v-if="project.dirty" class="pill amber">{{ project.total }} 个改动</span>
        <span v-else class="pill green"><Icon name="check" :size="10" />干净</span>
      </div>

      <div v-if="hasCounts" class="chips">
        <span v-if="project.staged" class="chip green">已暂存 {{ project.staged }}</span>
        <span v-if="project.unstaged" class="chip amber">未暂存 {{ project.unstaged }}</span>
        <span v-if="project.untracked" class="chip muted">未跟踪 {{ project.untracked }}</span>
      </div>

      <div class="files">
        <template v-if="project.files.length">
          <div v-for="f in project.files.slice(0, 4)" :key="f.path" class="file">
            <span class="badge" :class="fileBadge(f).tone">{{ fileBadge(f).letter }}</span>
            <span class="path" :title="f.path">{{ f.path }}</span>
          </div>
          <div v-if="project.files.length > 4" class="more">还有 {{ project.files.length - 4 }} 个文件…</div>
        </template>
        <div v-else class="clean-line"><Icon name="check-circle" :size="12" />没有未提交的改动</div>
      </div>

      <div class="stats">
        <span class="stat">
          <Icon name="clock" :size="11" />
          <b class="add">+{{ project.today.added }}</b>
          <b class="del">−{{ project.today.deleted }}</b>
          <span class="muted">· {{ project.today.commits }} 次提交</span>
        </span>
      </div>

      <div v-if="project.lastCommit" class="commit">
        <Icon name="commit" :size="11" />
        <span class="subject">{{ truncate(project.lastCommit.subject, 26) }}</span>
        <span class="time">{{ relTime(project.lastCommit.time) }}</span>
      </div>

      <footer class="card-actions">
        <button class="ghost" @click="$emit('reveal')"><Icon name="folder" :size="12" />打开</button>
        <button class="ghost" @click="$emit('branches')"><Icon name="branch" :size="12" />分支</button>
        <button v-if="project.dirty || project.ahead" class="ghost" @click="$emit('pull')">
          <Icon name="download" :size="12" />拉取
        </button>
      </footer>
    </template>

    <div v-else class="state-note" :class="tone">
      <Icon :name="project.state === 'error' ? 'alert' : 'folder'" :size="13" />
      <span v-if="project.state === 'missing'">路径不存在</span>
      <span v-else-if="project.state === 'not-repo'">不是 Git 仓库</span>
      <span v-else>{{ truncate(project.error || '读取失败', 60) }}</span>
    </div>
  </article>
</template>
