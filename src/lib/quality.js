import { api } from './api.js'
import { ui } from './state.svelte.js'

export const TIERS = [
  { k: 'hires', name: 'Hi-Res', tag: 'FLAC 24-bit', desc: 'Más detalle que un CD, tal cual salió del estudio. Solo vale la pena con audífonos o DAC buenos; pesa muchísimo.' },
  { k: 'lossless', name: 'Sin pérdida', tag: 'FLAC / WAV · calidad CD', desc: 'Idéntica al original, no se pierde nada. Lo mejor para audífonos o bocinas buenas. Pesa unas 4 a 5 veces más que MP3.' },
  { k: 'high', name: 'Alta', tag: 'MP3 320 / VBR · OGG', desc: 'Casi imposible distinguirla del original al oído. El mejor equilibrio entre calidad y espacio.' },
  { k: 'normal', name: 'Ligera', tag: 'MP3 64–128', desc: 'Ocupa poco. Suena más plana en graves y agudos; bien si tienes poco espacio.' }
]

const LOSSLESS = ['flac', 'wav', 'aif', 'aiff', 'alac']

export function info(f) {
  const kind = (f.kind || '').toLowerCase()
  const label = f.label || ''
  const num = Number((/(\d{2,3})\b/.exec(label) || [])[1]) || 0
  if (LOSSLESS.includes(kind)) {
    if (/24/.test(label)) return { tier: 'hires', kbps: 2300, rank: 0 }
    return { tier: 'lossless', kbps: kind === 'wav' || kind.startsWith('aif') ? 1411 : 900, rank: kind === 'flac' ? 1 : 2 }
  }
  if (kind === 'mp3') {
    if (num >= 256) return { tier: 'high', kbps: num, rank: 3 }
    if (/vbr/i.test(label)) return { tier: 'high', kbps: 210, rank: 4 }
    if (num && num < 256) return { tier: 'normal', kbps: num, rank: 8 - num / 100 }
    if (/original/i.test(label)) return { tier: 'high', kbps: 320, rank: 3 }
    return { tier: 'normal', kbps: 128, rank: 7 }
  }
  if (kind === 'ogg' || kind === 'opus') return { tier: 'high', kbps: kind === 'opus' ? 160 : 192, rank: 5 }
  if (kind === 'm4a' || kind === 'aac') return { tier: 'high', kbps: 256, rank: 4.5 }
  return { tier: 'high', kbps: 256, rank: 6 }
}

const order = TIERS.map((t) => t.k)

export function pick(track, tier) {
  const fs = (track.formats || []).filter((f) => f.kind !== 'yt').map((f) => ({ f, ...info(f) }))
  if (!fs.length) return null
  const want = order.indexOf(tier)
  const exact = fs.filter((x) => x.tier === tier).sort((a, b) => a.rank - b.rank)
  if (exact.length) return { ...exact[0], exact: true }
  const below = fs.filter((x) => order.indexOf(x.tier) > want).sort((a, b) => order.indexOf(a.tier) - order.indexOf(b.tier) || a.rank - b.rank)
  if (below.length) return { ...below[0], exact: false }
  const above = fs.sort((a, b) => order.indexOf(b.tier) - order.indexOf(a.tier) || a.rank - b.rank)
  return { ...above[0], exact: false }
}

const sizeCache = new Map()

export function sizeOf(track, p) {
  const real = p.f.size || sizeCache.get(p.f.url)
  if (real) return { bytes: real, exact: true }
  const secs = track.duration || 210
  return { bytes: Math.round((p.kbps * 1000 * secs) / 8), exact: false }
}

export async function measure(tracks) {
  const urls = []
  for (const t of tracks.slice(0, 60)) for (const f of t.formats || []) if (f.kind !== 'yt' && !f.size && !sizeCache.has(f.url)) urls.push(f.url)
  if (!urls.length) return false
  try {
    const sizes = await api.formatSizes(urls)
    urls.forEach((u, i) => sizes[i] && sizeCache.set(u, sizes[i]))
    return true
  } catch {
    return false
  }
}

export function human(b) {
  if (b >= 1e9) return `${(b / 1e9).toFixed(2)} GB`
  if (b >= 1e6) return `${(b / 1e6).toFixed(b >= 1e8 ? 0 : 1)} MB`
  return `${Math.max(1, Math.round(b / 1e3))} KB`
}

export function summary(tracks) {
  const out = []
  for (const tier of TIERS) {
    let bytes = 0
    let exact = true
    let have = 0
    for (const t of tracks) {
      const p = pick(t, tier.k)
      if (!p) continue
      if (p.exact) have++
      const s = sizeOf(t, p)
      bytes += s.bytes
      if (!s.exact) exact = false
    }
    if (have) out.push({ ...tier, bytes, exact, have, total: tracks.length })
  }
  return out
}

const legacy = { flac: 'lossless', mp3: 'high', ogg: 'high' }

export function savedTier() {
  const v = ui.settings.download_format || 'ask'
  return legacy[v] || v
}

export function chooseQuality(tracks) {
  return new Promise((resolve) => {
    ui.quality = { tracks, resolve }
  })
}
