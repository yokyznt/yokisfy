export const ui = $state({
  view: { name: 'home' },
  stack: [],
  queueOpen: false,
  fullPlayer: false,
  menu: null,
  toasts: [],
  playlists: [],
  liked: {},
  local: {},
  downloads: {},
  settings: {},
  theme: typeof document !== 'undefined' ? document.documentElement.dataset.theme : 'dark',
  homeCache: null,
  importing: null,
  scanning: null,
  lyrics: false,
  fpMode: 'cover',
  update: null,
  sleep: null,
  searchCache: { q: '', results: null }
})

export const player = $state({
  track: null,
  index: null,
  queueLen: 0,
  playing: false,
  loading: false,
  position: 0,
  duration: 0,
  volume: 0.8,
  shuffle: false,
  repeat: 'off',
  error: null,
  external: false,
  queue: [],
  at: 0
})

export function ask(title, value = '', confirmLabel = 'Guardar') {
  return new Promise((resolve) => {
    ui.prompt = { title, value, confirmLabel, resolve }
  })
}

let toastId = 0
export function toast(text) {
  const id = ++toastId
  ui.toasts = [...ui.toasts.slice(-2), { id, text }]
  setTimeout(() => {
    ui.toasts = ui.toasts.filter((t) => t.id !== id)
  }, 3200)
}

export function go(name, param = null) {
  if (ui.view.name === name && ui.view.param === param) return
  ui.stack = [...ui.stack.slice(-30), ui.view]
  ui.view = { name, param }
  ui.fullPlayer = false
  document.querySelector('.view')?.scrollTo({ top: 0 })
}

export function root(name) {
  ui.stack = []
  ui.view = { name, param: null }
  ui.fullPlayer = false
  document.querySelector('.view')?.scrollTo({ top: 0 })
}

export function back() {
  const prev = ui.stack.at(-1)
  if (!prev) return
  ui.stack = ui.stack.slice(0, -1)
  ui.view = prev
}

export function openMenu(event, items) {
  const r = event.currentTarget?.getBoundingClientRect?.()
  const x = r ? r.right : event.clientX
  const y = r ? r.bottom + 4 : event.clientY
  ui.menu = { x, y, items, stack: [] }
}

export function setTheme(t) {
  ui.theme = t
  document.documentElement.dataset.theme = t
  localStorage.setItem('yk-theme', t)
}

export function fmt(s) {
  if (!s || !isFinite(s) || s < 0) return '0:00'
  s = Math.floor(s)
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = String(s % 60).padStart(2, '0')
  return h ? `${h}:${String(m).padStart(2, '0')}:${sec}` : `${m}:${sec}`
}

export function totalTime(tracks) {
  const s = tracks.reduce((a, t) => a + (t.duration || 0), 0)
  const h = Math.floor(s / 3600)
  const m = Math.round((s % 3600) / 60)
  return h ? `${h} h ${m} min` : `${m} min`
}

export function sourceName(s) {
  return { jamendo: 'Jamendo', archive: 'Internet Archive', audius: 'Audius', radio: 'Radio', local: 'Local', youtube: 'YouTube Music' }[s] || s
}

export function bestLabel(t) {
  if (ui.local[t.id] || t.localPath) return null
  const f = t.formats?.find((f) => f.kind !== 'wav' && f.kind !== 'yt')
  return f ? f.label.split(' ')[0] : null
}

export function isLocal(t) {
  return !!(ui.local[t.id] || t.localPath)
}
