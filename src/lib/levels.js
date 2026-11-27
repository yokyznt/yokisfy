import { invoke } from '@tauri-apps/api/core'
import { player } from './state.svelte.js'

const lv = { rms: 0, low: 0, high: 0, beat: 0, mid: 0, tone: 0.5, flux: 0, mood: 0.6, spec: new Array(8).fill(0) }
let users = 0
let timer = null
let busy = false
let peak = 0.2
const bpk = new Array(9).fill(0.05)
let prev = new Array(8).fill(0)
let slow = 0.3
let long = 0.3
let silent = 0
const android = /android/i.test(navigator.userAgent)

function synth(now) {
  const t = now / 1000
  const b = Math.pow(Math.max(0, Math.sin(t * Math.PI * 2 * (118 / 60))), 10)
  const w = 0.5 + 0.5 * Math.sin(t * 0.7) * Math.sin(t * 1.3)
  const spec = Array.from({ length: 8 }, (_, i) => Math.max(0, (i < 2 ? 0.3 + 0.6 * b : 0.25 + 0.3 * w) * (0.8 + 0.4 * Math.sin(t * (1 + i * 0.37) + i))))
  return { rms: 0.25 + 0.2 * w + 0.35 * b, low: 0.3 + 0.6 * b, high: 0.2 + 0.3 * w + 0.2 * Math.random() * b, spec }
}

function fromSpec(s) {
  const n = s.map((v, i) => {
    bpk[i] = Math.max(bpk[i] * 0.997, v, 0.02)
    return Math.min(v / bpk[i], 1.6)
  })
  const b = n.slice(0, 8)
  return { rms: n[8], low: (b[0] + b[1]) / 2, high: (b[5] + b[6] + b[7]) / 3, spec: b }
}

async function read() {
  const ext = player.external || player.track?.source === 'youtube'
  if (ext && android) return synth(performance.now())
  try {
    const s = await invoke('spectrum', { external: !!ext })
    if (s.some((x) => x > 0.0005)) {
      silent = 0
      return fromSpec(s)
    }
    silent++
  } catch {
    silent++
  }
  if (ext) return silent > 20 ? synth(performance.now()) : null
  const [r, l, h] = await invoke('levels')
  peak = Math.max(peak * 0.995, r, 0.08)
  return { rms: r / peak, low: l / peak, high: h / peak, spec: [l, l, (l + h) / 2, r, r, h, h, h].map((x) => x / peak) }
}

async function poll() {
  if (!player.playing) {
    for (const k of ['rms', 'low', 'high', 'mid', 'flux']) lv[k] *= 0.9
    lv.spec = lv.spec.map((x) => x * 0.9)
    return
  }
  if (busy) return
  busy = true
  let v
  try {
    v = await read()
  } catch {
    v = synth(performance.now())
  } finally {
    busy = false
  }
  if (!v) return
  const k = (a, b) => (b > a ? a + (b - a) * 0.6 : a + (b - a) * 0.18)
  const prevLow = lv.low
  lv.rms = k(lv.rms, Math.min(v.rms, 1.6))
  lv.low = k(lv.low, Math.min(v.low, 1.6))
  lv.high = k(lv.high, Math.min(v.high, 1.6))
  lv.spec = lv.spec.map((x, i) => k(x, v.spec[i] || 0))
  const s = lv.spec
  lv.mid = (s[2] + s[3] + s[4]) / 3
  const tot = s.reduce((a, b) => a + b, 0) || 1
  const cen = s.reduce((a, b, i) => a + b * i, 0) / tot / 7
  lv.tone += (cen - lv.tone) * 0.04
  let fl = 0
  for (let i = 3; i < 8; i++) fl += Math.max(0, (v.spec[i] || 0) - prev[i])
  prev = v.spec.slice()
  lv.flux = Math.max(lv.flux * 0.8, Math.min(fl / 2, 1.2))
  slow += (v.rms - slow) * 0.02
  long = Math.max(long * 0.9995, slow, 0.1)
  lv.mood += (Math.min(slow / long, 1) - lv.mood) * 0.05
  lv.beat = Math.max(lv.beat * 0.86, lv.low - prevLow > 0.12 ? 1 : 0)
}

export function useLevels() {
  users++
  if (!timer) timer = setInterval(poll, 33)
  return () => {
    users--
    if (users <= 0 && timer) {
      clearInterval(timer)
      timer = null
    }
  }
}

export function levels() {
  return lv
}
