import { invoke } from '@tauri-apps/api/core'

const cache = new Map()
const fallback = ['#7c5cff', '#ff4d8d', '#2bd4c5', '#1a1a2e']

function vivid(hex, minS, l0, l1) {
  const v = parseInt(hex.slice(1), 16)
  let r = ((v >> 16) & 255) / 255
  let g = ((v >> 8) & 255) / 255
  let b = (v & 255) / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  let h = 0
  let s = 0
  let l = (max + min) / 2
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4
    h /= 6
  }
  s = Math.max(s, minS)
  l = Math.min(Math.max(l, l0), l1)
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const pp = 2 * l - q
  const f = (t) => {
    t = (t + 1) % 1
    return t < 1 / 6 ? pp + (q - pp) * 6 * t : t < 1 / 2 ? q : t < 2 / 3 ? pp + (q - pp) * (2 / 3 - t) * 6 : pp
  }
  const x = (n) => Math.round(n * 255).toString(16).padStart(2, '0')
  return `#${x(f(h + 1 / 3))}${x(f(h))}${x(f(h - 1 / 3))}`
}

export async function paletteFor(url) {
  if (!url) return fallback
  if (cache.has(url)) return cache.get(url)
  try {
    const p = await invoke('palette', { url })
    const out = p?.length >= 4 ? [vivid(p[0], 0.62, 0.42, 0.6), vivid(p[1], 0.6, 0.38, 0.58), vivid(p[2], 0.55, 0.36, 0.6), vivid(p[3], 0.3, 0.06, 0.14)] : fallback
    cache.set(url, out)
    return out
  } catch {
    return fallback
  }
}
