import { api } from './api.js'
import { player, toast } from './state.svelte.js'

export const together = $state({ on: false, url: '', name: '', drift: 0 })

let timer = null
let cur = null
let fails = 0
let busy = false

function mine() {
  return player.playing && !player.loading && player.at ? player.position + (performance.now() - player.at) / 1000 : player.position
}

async function tick() {
  if (busy) return
  busy = true
  try {
    const t0 = performance.now()
    let n
    try {
      n = await api.partyCall(together.url, 'party_now')
      fails = 0
    } catch {
      if (++fails > 6) {
        stopTogether()
        toast('Se perdió la conexión con la fiesta')
      }
      return
    }
    const half = (performance.now() - t0) / 2000
    if (!n.on) {
      stopTogether()
      toast('La fiesta se cerró')
      return
    }
    if (!n.track) return
    const target = n.position + (n.playing && !n.loading ? half : 0)
    if (n.track.id !== cur) {
      cur = n.track.id
      const t = { ...n.track }
      if (n.src?.audio) {
        t.stream_url = together.url.replace(/\/$/, '') + n.src.audio
        t.local_path = null
      }
      await api.play([t], 0)
      return
    }
    if (player.loading || n.loading || player.track?.id !== cur) return
    together.drift = mine() - target
    if (Math.abs(together.drift) > 1.8) await api.seek(Math.max(0, target + 0.3))
    if (!!n.playing !== player.playing) await api.setPlaying(!!n.playing)
  } finally {
    busy = false
  }
}

export function startTogether(url, name) {
  stopTogether()
  Object.assign(together, { on: true, url, name, drift: 0 })
  cur = null
  fails = 0
  tick()
  timer = setInterval(tick, 2000)
}

export function stopTogether() {
  clearInterval(timer)
  timer = null
  together.on = false
}
