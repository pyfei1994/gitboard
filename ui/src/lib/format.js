export function relTime(ms) {
  if (!ms) return '—'
  const secs = Math.max(0, Math.floor((Date.now() - ms) / 1000))
  if (secs < 60) return '刚刚'
  if (secs < 3600) return Math.floor(secs / 60) + ' 分钟前'
  if (secs < 86400) return Math.floor(secs / 3600) + ' 小时前'
  if (secs < 86400 * 30) return Math.floor(secs / 86400) + ' 天前'
  return Math.floor(secs / (86400 * 30)) + ' 个月前'
}

export function truncate(text, max) {
  if (!text) return ''
  const chars = [...text]
  return chars.length <= max ? text : chars.slice(0, max - 1).join('') + '…'
}

export function fileBadge(file) {
  const x = file.x || '.'
  const y = file.y || '.'
  if (x === '?' && y === '?') return { letter: 'U', tone: 'muted' }
  if (x === 'U' || y === 'U') return { letter: '!', tone: 'red' }
  const ch = x !== '.' && x !== '?' ? x : y
  const tone = { A: 'green', D: 'red', R: 'blue', C: 'blue', M: 'amber' }[ch] || 'muted'
  return { letter: ch, tone }
}

export function fileSummary(file) {
  const badge = fileBadge(file)
  const from = file.from ? file.from + ' → ' : ''
  return { ...badge, text: from + file.path }
}
