import { api } from './api.js'
import { player, ui, toast } from './state.svelte.js'
import { like } from './actions.js'

const plain = (s) => s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '').replace(/[¿?¡!.,]/g, '').trim()

export async function runVoice(raw) {
  const t = plain(raw || '')
  if (!t) return
  if (/^(siguiente|salta|saltala|otra|cambia(la)?|la que sigue)\b/.test(t)) return api.next()
  if (/^(anterior|regresa|atras|la de antes)\b/.test(t)) return api.prev()
  if (/^(pausa|pausala|para|detente|alto|callate)\b/.test(t)) return api.setPlaying(false)
  if (/^(sigue|continua|play|dale|reanuda)$/.test(t)) return api.setPlaying(true)
  if (/^(sube|mas fuerte|subele)/.test(t)) return api.volume(Math.min(1, (player.volume || 0.5) + 0.15)).then(() => (player.volume = Math.min(1, (player.volume || 0.5) + 0.15)))
  if (/^(baja|mas bajo|bajale)/.test(t)) return api.volume(Math.max(0, (player.volume || 0.5) - 0.15)).then(() => (player.volume = Math.max(0, (player.volume || 0.5) - 0.15)))
  if (/^(me gusta|guardala|guarda esta)/.test(t)) return player.track && like(player.track)
  const q = t.replace(/^(oye yokisfy |yokisfy )?(pon(me|le)?|reproduce|toca|quiero (escuchar|oir)|escuchar|busca)\s+/, '').replace(/^(a |al |el |la |los |las |musica de |canciones de |algo de |una de |unas de )/, '').replace(/ en yokisfy$/, '').trim()
  if (!q) return
  toast(`Buscando «${q}»…`)
  const r = await api.search(q)
  let tracks = (r.tracks || []).filter((x) => !x.live)
  const art = (r.artists || []).find((a) => plain(a.name) === q || plain(a.name).includes(q) || q.includes(plain(a.name)))
  if (art) {
    const own = tracks.filter((x) => plain(x.artist).includes(plain(art.name)))
    if (own.length >= 3) tracks = [...own, ...tracks.filter((x) => !own.includes(x))]
  }
  if (!tracks.length) return toast(`No encontré «${q}»`)
  await api.play(tracks.slice(0, 30), 0)
  toast(`Poniendo ${art ? art.name : tracks[0].title}`)
}
