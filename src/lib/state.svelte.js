export const ui = $state({
  view: { name: 'home' },
  stack: [],
  queueOpen: false,
  fullPlayer: false,
  car: false,
  carAsk: null,
  carAuto: null,
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
  fpIdle: false,
  quality: null,
  eqOpen: false,
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

export const THEMES = [
  ['dark', 'Negro', '#09090a', '#f4f4f3', '#f4f4f3'],
  ['light', 'Blanco', '#fafaf8', '#0b0b0c', '#0b0b0c'],
  ['amoled', 'Negro puro', '#000000', '#ffffff', '#ffffff'],
  ['graphite', 'Grafito', '#1c1c1e', '#ececec', '#d6d6d6'],
  ['paper', 'Papel', '#efe9dd', '#1d1a14', '#1d1a14'],
  ['lime', 'Lima', '#09090a', '#f4f4f3', '#d4ff3a'],
  ['crimson', 'Carmesí', '#0d0809', '#f4f4f3', '#ff3b4f'],
  ['ocean', 'Océano', '#070b12', '#f4f4f3', '#3ba7ff'],
  ['violet', 'Violeta', '#0b0812', '#f4f4f3', '#a374ff'],
  ['sunset', 'Atardecer', '#0f0a06', '#f4f4f3', '#ff9f2e'],
  ['sakura', 'Sakura', '#fff5f7', '#2a0d16', '#e8466f']
]

export const isLight = (t) => ['light', 'paper', 'sakura'].includes(t)

export const FONTS = {
  geist: ['Geist', "'Geist Sans', 'Geist', system-ui, sans-serif"],
  system: ['Sistema', "system-ui, 'Segoe UI', Roboto, sans-serif"],
  rounded: ['Redonda', "ui-rounded, 'SF Pro Rounded', 'Nunito', 'Varela Round', system-ui, sans-serif"],
  serif: ['Clásica', "'Iowan Old Style', 'Palatino Linotype', Georgia, serif"],
  mono: ['Máquina', "'Geist Mono', ui-monospace, monospace"]
}

export const ACCENTS = ['#ffffff', '#ff3b4f', '#ff7a1a', '#ffd21f', '#d4ff3a', '#2fe07a', '#1fd6e0', '#3ba7ff', '#7a6cff', '#c86bff', '#ff5fa8']
export const BACKGROUNDS = ['#000000', '#141416', '#0b1020', '#160b10', '#0b140e', '#1a1206', '#efe9dd', '#ffffff']

function readLook() {
  try {
    return JSON.parse(localStorage.getItem('yk-look') || '{}') || {}
  } catch {
    return {}
  }
}

export const look = $state(readLook())

function lum(hex) {
  const v = parseInt(String(hex).replace('#', '').slice(0, 6), 16)
  const c = [(v >> 16) & 255, (v >> 8) & 255, v & 255].map((x) => {
    x /= 255
    return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

export function paintAccent(hex) {
  const r = document.documentElement.style
  if (!hex) {
    for (const p of ['--ac', '--ac-ink', '--on-ac']) r.removeProperty(p)
    return
  }
  r.setProperty('--ac', hex)
  r.setProperty('--ac-ink', hex)
  r.setProperty('--on-ac', lum(hex) > 0.4 ? '#0a0a0a' : '#ffffff')
}

export function applyLook() {
  const r = document.documentElement.style
  for (const p of ['--bg', '--bg-2', '--bg-3', '--bg-4', '--tx', '--tx-2', '--tx-3', '--line', '--line-2', '--font']) r.removeProperty(p)
  r.removeProperty('color-scheme')
  if (look.bg) {
    const light = lum(look.bg) > 0.4
    const tx = light ? '#0b0b0c' : '#f4f4f3'
    r.setProperty('--bg', look.bg)
    r.setProperty('--bg-2', `color-mix(in oklab, ${look.bg}, ${tx} 5%)`)
    r.setProperty('--bg-3', `color-mix(in oklab, ${look.bg}, ${tx} 10%)`)
    r.setProperty('--bg-4', `color-mix(in oklab, ${look.bg}, ${tx} 16%)`)
    r.setProperty('--tx', tx)
    r.setProperty('--tx-2', `color-mix(in oklab, ${tx}, ${look.bg} 38%)`)
    r.setProperty('--tx-3', `color-mix(in oklab, ${tx}, ${look.bg} 56%)`)
    r.setProperty('--line', `color-mix(in oklab, ${tx} 9%, transparent)`)
    r.setProperty('--line-2', `color-mix(in oklab, ${tx} 16%, transparent)`)
    r.setProperty('color-scheme', light ? 'light' : 'dark')
  }
  if (look.font && FONTS[look.font]) r.setProperty('--font', FONTS[look.font][1])
  r.zoom = look.scale && look.scale !== 1 ? String(look.scale) : ''
  if (!look.dynamic) paintAccent(look.accent || null)
}

export function setLook(k, v) {
  if (v === null || v === undefined || v === '') delete look[k]
  else look[k] = v
  localStorage.setItem('yk-look', JSON.stringify($state.snapshot(look)))
  applyLook()
}

export function resetLook() {
  for (const k of Object.keys(look)) delete look[k]
  localStorage.removeItem('yk-look')
  applyLook()
}

if (typeof window !== 'undefined') {
  window.addEventListener('storage', (e) => {
    if (e.key === 'yk-look') {
      const n = readLook()
      for (const k of Object.keys(look)) delete look[k]
      Object.assign(look, n)
      applyLook()
    }
    if (e.key === 'yk-theme' && e.newValue) {
      ui.theme = e.newValue
      document.documentElement.dataset.theme = e.newValue
    }
  })
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
  return { jamendo: 'Jamendo', archive: 'Internet Archive', audius: 'Audius', ccmixter: 'ccMixter', commons: 'Wikimedia Commons', radio: 'Radio', local: 'Local', youtube: '', mix: 'Para ti' }[s] ?? s
}

export function bestLabel(t) {
  if (ui.local[t.id] || t.localPath) return null
  const f = t.formats?.find((f) => f.kind !== 'wav' && f.kind !== 'yt')
  return f ? f.label.split(' ')[0] : null
}

export function isLocal(t) {
  return !!(ui.local[t.id] || t.localPath)
}
