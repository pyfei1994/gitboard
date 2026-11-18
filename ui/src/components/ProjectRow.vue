<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'
import { relTime, truncate } from '../lib/format'

const props = defineProps({ project: { type: Object, required: true } })
defineEmits(['reveal', 'branches', 'pin', 'remove'])
const tone = computed(() => {
  const p = props.project
  if (p.state === 'error') return 'red'
  if (p.state === 'missing' || p.state === 'not-repo') return 'muted'
  return p.dirty ? 'amber' : 'green'
})
</script>

<template>
  <div class="row" :class="{ pinned: project.pinned }">
    <div class="cell name-cell">
      <span class="dot" :class="tone"></span>
      <span class="name">{{ project.name }}</span>
      <Icon v-if="project.pinned" name="star" :size="11" class="pinned-icon" />
    </div>
    <div class="cell branch-cell">
      <Icon name="branch" :size="11" />
      <span>{{ project.branch || '—' }}</span>
    </div>
    <div class="cell">
      <span v-if="project.dirty" class="pill amber">{{ project.total }} 个改动</span>
      <span v-else-if="project.state === 'ok'" class="pill green"><Icon name="check" :size="10" />干净</span>
      <span v-else class="pill muted">—</span>
    </div>
    <div class="cell mono">
      <span class="add">+{{ project.today.added }}</span>
      <span class="del">−{{ project.today.deleted }}</span>
    </div>
    <div class="cell commit-cell">
      <template v-if="project.lastCommit">
        <Icon name="commit" :size="11" />
        <span class="subject">{{ truncate(project.lastCommit.subject, 32) }}</span>
        <span class="time">{{ relTime(project.lastCommit.time) }}</span>
      </template>
      <span v-else class="muted">—</span>
    </div>
    <div class="cell actions-cell">
      <button class="mini" title="打开目录" @click="$emit('reveal')"><Icon name="folder" :size="13" /></button>
      <button class="mini" title="切换分支" @click="$emit('branches')"><Icon name="branch" :size="13" /></button>
      <button class="mini" :class="{ active: project.pinned }" :title="project.pinned ? '取消置顶' : '置顶（排到最前）'" @click="$emit('pin')"><Icon name="star" :size="13" /></button>
      <button class="mini danger" title="移出看板" @click="$emit('remove')"><Icon name="x" :size="13" /></button>
    </div>
  </div>
</template>
