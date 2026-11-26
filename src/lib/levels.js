import { invoke } from '@tauri-apps/api/core'
import { player } from './state.svelte.js'

const lv = { rms: 0, low: 0, high: 0, beat: 0 }
let users = 0
let timer = null
let busy = false
let peak = 0.2

function synth(now) {
  const t = now / 1000
  const b = Math.pow(Math.max(0, Math.sin(t * Math.PI * 2 * (118 / 60))), 10)
  const w = 0.5 + 0.5 * Math.sin(t * 0.7) * Math.sin(t * 1.3)
  return { rms: 0.25 + 0.2 * w + 0.35 * b, low: 0.3 + 0.6 * b, high: 0.2 + 0.3 * w + 0.2 * Math.random() * b }
}

async function poll() {
  if (!player.playing) {
    lv.rms *= 0.9
    lv.low *= 0.9
    lv.high *= 0.9
    return
  }
  let v
  if (player.external || player.track?.source === 'youtube') v = synth(performance.now())
  else if (!busy) {
    busy = true
    try {
      const [r, l, h] = await invoke('levels')
      peak = Math.max(peak * 0.995, r, 0.08)
      v = { rms: r / peak, low: l / peak, high: h / peak }
    } catch {
      v = synth(performance.now())
    } finally {
      busy = false
    }
  }
  if (!v) return
  const k = (a, b) => (b > a ? a + (b - a) * 0.6 : a + (b - a) * 0.18)
  const prevLow = lv.low
  lv.rms = k(lv.rms, Math.min(v.rms, 1.6))
  lv.low = k(lv.low, Math.min(v.low, 1.6))
  lv.high = k(lv.high, Math.min(v.high, 1.6))
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
