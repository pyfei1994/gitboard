const tauri = window.__TAURI__
export const isNative = Boolean(tauri && tauri.core && tauri.core.invoke)

const invoke = isNative ? tauri.core.invoke : null

/* ------------------------------------------------------------------ mock
 * 浏览器里（没有 Tauri 注入时）用假数据渲染，方便截图和调样式。
 */
const now = Date.now()
const mock = {
  projects: [
    { key: '/w/rsl-hexaos', path: '/w/rsl-hexaos', name: 'rsl-hexaos', state: 'ok', branch: 'main', ahead: 0, behind: 0, staged: 0, unstaged: 5, untracked: 0, total: 5, dirty: true, pinned: true,
      files: [{ path: 'apps/crm', x: 'M', y: '.', from: null }, { path: 'apps/delivery', x: 'M', y: '.', from: null }, { path: 'apps/ops', x: 'M', y: '.', from: null }, { path: 'apps/studio', x: 'M', y: '.', from: null }, { path: 'pnpm-lock.yaml', x: 'M', y: '.', from: null }],
      today: { commits: 0, added: 0, deleted: 0 }, lastCommit: { time: now - 22 * 3600e3, subject: 'docs(database): 增加 028 入口镜像 CRM 013 实施阶段自动里程碑来源字段' } },
    { key: '/w/rsl-hexaos-pi-agent', path: '/w/rsl-hexaos-pi-agent', name: 'rsl-hexaos-pi-agent', state: 'ok', branch: 'main', ahead: 0, behind: 1, staged: 0, unstaged: 7, untracked: 5, total: 12, dirty: true, pinned: false,
      files: [{ path: '.dockerignore', x: 'M', y: '.', from: null }, { path: 'Dockerfile', x: 'M', y: '.', from: null }, { path: 'README.md', x: 'M', y: '.', from: null }, { path: 'codeup.mjs', x: 'M', y: '.', from: null }],
      today: { commits: 0, added: 0, deleted: 0 }, lastCommit: { time: now - 7 * 86400e3, subject: 'feat(agent): add Delivery assistant conversation flow' } },
    { key: '/w/rsl-hexaos-crm', path: '/w/rsl-hexaos-crm', name: 'rsl-hexaos-crm', state: 'ok', branch: 'main', ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, total: 0, dirty: false, pinned: false, files: [],
      today: { commits: 0, added: 0, deleted: 0 }, lastCommit: { time: now - 22 * 3600e3, subject: "Merge remote-tracking branch 'origin/main'" } },
    { key: '/w/rsl-hexaos-delivery', path: '/w/rsl-hexaos-delivery', name: 'rsl-hexaos-delivery', state: 'ok', branch: 'main', ahead: 2, behind: 0, staged: 0, unstaged: 0, untracked: 0, total: 0, dirty: false, pinned: false, files: [],
      today: { commits: 7, added: 2678, deleted: 1732 }, lastCommit: { time: now - 20 * 60e3, subject: 'feat(delivery): add shared manifest pipeline' } },
    { key: '/w/rsl-hexaos-iam', path: '/w/rsl-hexaos-iam', name: 'rsl-hexaos-iam', state: 'ok', branch: 'main', ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, total: 0, dirty: false, pinned: false, files: [],
      today: { commits: 1, added: 34, deleted: 1 }, lastCommit: { time: now - 4 * 3600e3, subject: 'Restore Studio Pi Agent settings in IAM' } },
    { key: '/w/rsl-hexaos-ops', path: '/w/rsl-hexaos-ops', name: 'rsl-hexaos-ops', state: 'ok', branch: 'main', ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, total: 0, dirty: false, pinned: false, files: [],
      today: { commits: 3, added: 456, deleted: 568 }, lastCommit: { time: now - 19 * 60e3, subject: 'feat(ops): uninstall unused argocd components' } },
    { key: '/w/rsl-hexaos-ops-center', path: '/w/rsl-hexaos-ops-center', name: 'rsl-hexaos-ops-center', state: 'ok', branch: 'main', ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, total: 0, dirty: false, pinned: false, files: [],
      today: { commits: 3, added: 237, deleted: 73 }, lastCommit: { time: now - 26 * 60e3, subject: 'Expose manual template reconciliation' } },
    { key: '/w/rsl-hexaos-studio', path: '/w/rsl-hexaos-studio', name: 'rsl-hexaos-studio', state: 'ok', branch: 'feat/ui', ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, total: 0, dirty: false, pinned: false, files: [],
      today: { commits: 3, added: 342, deleted: 482 }, lastCommit: { time: now - 41 * 60e3, subject: 'Send pinned sanitized repository metadata' } },
  ],
  pollMs: 3000,
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function mockCall(cmd, args) {
  await sleep(80)
  switch (cmd) {
    case 'state':
    case 'refresh':
      return mock
    case 'branches':
      return { branches: ['main', 'feat/ui', 'release/2.0'], current: 'main' }
    case 'pull':
      return { results: [{ name: 'mock', ok: true, message: '已是最新' }] }
    case 'app_info':
      return { version: '3.1.1', repo: 'pyfei1994/gitboard', arch: 'arm64' }
    case 'check_update':
      return { current: '3.0.1', latest: '3.0.1', hasUpdate: false, url: '', notes: '', dmg: null }
    case 'download_update':
      return { path: '/tmp/mock.dmg', size: 1 }
    case 'add_project':
      return { added: [], already: [], ignored: [], state: mock }
    case 'pick_projects':
      return { canceled: true }
    default:
      return {}
  }
}

export async function call(cmd, args = {}) {
  if (!isNative) return mockCall(cmd, args)
  return invoke(cmd, args)
}

export function onState(handler) {
  if (!isNative) {
    const timer = setInterval(() => handler(mock), 5000)
    return () => clearInterval(timer)
  }
  let unlisten = null
  let stopped = false
  tauri.event.listen('state', (event) => handler(event.payload)).then((fn) => {
    if (stopped) fn()
    else unlisten = fn
  })
  return () => {
    stopped = true
    if (unlisten) unlisten()
  }
}

export const mockState = mock
