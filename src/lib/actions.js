import { api, refreshPlaylists } from './api.js'
import { ui, toast, go, openMenu, ask } from './state.svelte.js'
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'

export async function like(t) {
  const on = await api.toggleLike(t)
  if (on) ui.liked[t.id] = true
  else delete ui.liked[t.id]
}

export function canDownload(t) {
  return !!(t.downloadable && t.formats?.length)
}

export async function download(t, kind = null) {
  if (!canDownload(t)) return toast('El artista solo permite escucharla en streaming')
  ui.downloads[t.id] = { id: t.id, title: t.title, artist: t.artist, format: kind || t.formats[0].label, received: 0, total: null, state: 'queued' }
  try {
    await api.download(t, kind)
  } catch {
    delete ui.downloads[t.id]
  }
}

export function downloadItems(tracks) {
  const list = (Array.isArray(tracks) ? tracks : [tracks]).filter(canDownload)
  const labels = []
  for (const t of list) for (const f of t.formats || []) if (!labels.includes(f.label)) labels.push(f.label)
  return labels.map((label) => ({
    label,
    mono: true,
    action: () => {
      for (const t of list) {
        const f = t.formats.find((x) => x.label === label) || t.formats.find((x) => x.kind !== 'wav')
        if (f) download(t, f.label)
      }
      toast(list.length > 1 ? `Descargando ${list.length} canciones` : `Descargando ${list[0].title}`)
    }
  }))
}

export function smartDownload(event, tracks) {
  const pref = ui.settings.download_format || 'ask'
  const list = (Array.isArray(tracks) ? tracks : [tracks]).filter(canDownload)
  if (!list.length) return toast('El artista solo permite escuchar este álbum en streaming')
  if (pref === 'ask') return openMenu(event, downloadItems(list))
  for (const t of list) {
    const f = t.formats?.find((x) => x.kind === pref) || t.formats?.find((x) => x.kind !== 'wav')
    if (f) download(t, f.label)
  }
  toast(list.length > 1 ? `Descargando ${list.length} canciones` : `Descargando ${list[0].title}`)
}

export async function newPlaylist(tracks = []) {
  const name = await ask('Nueva playlist', `Playlist ${ui.playlists.length + 1}`, 'Crear')
  if (!name?.trim()) return
  const id = await api.playlistCreate(name.trim(), tracks)
  await refreshPlaylists()
  toast(`Creada: ${name.trim()}`)
  return id
}

export async function importSpotify() {
  const link = await ask('Pega el link de una playlist, álbum o canción de Spotify', '', 'Importar')
  if (!link?.trim()) return
  ui.importing = { done: 0, total: 0 }
  try {
    const r = await api.importSpotify(link.trim())
    await refreshPlaylists()
    go('playlist', r.playlistId)
    const parts = [`${r.free} gratis para descargar`, `${r.youtube} desde YouTube`]
    if (r.missing.length) parts.push(`${r.missing.length} sin encontrar`)
    toast(`«${r.name}»: ${parts.join(', ')}`)
  } catch {
  } finally {
    ui.importing = null
  }
}

export function playlistItems(tracks) {
  return [
    { label: 'Nueva playlist', icon: 'plus', action: () => newPlaylist(tracks) },
    ...ui.playlists.map((p) => ({
      label: p.name,
      icon: 'list',
      action: async () => {
        await api.playlistAdd(p.id, tracks)
        await refreshPlaylists()
        toast(`Añadida a ${p.name}`)
      }
    }))
  ]
}

export async function startRadio(t) {
  toast('Buscando música parecida…')
  try {
    const n = await api.startRadio(t)
    toast(n ? `Radio de ${t.artist}: ${n} canciones` : 'No encontré canciones parecidas')
  } catch (e) {
    toast(String(e))
  }
}

export function trackMenu(event, t, extra = []) {
  const items = [
    !t.live ? { label: 'Iniciar radio', icon: 'radio', action: () => startRadio(t) } : null,
    { label: 'Reproducir a continuación', icon: 'next', action: () => api.playNext(t).then(() => toast('Sonará a continuación')) },
    { label: 'Añadir a la cola', icon: 'queue', action: () => api.enqueue([t]).then(() => toast('Añadida a la cola')) },
    { label: 'Añadir a playlist', icon: 'list', sub: () => playlistItems([t]) },
    ui.local[t.id] || t.localPath
      ? { label: 'Mostrar en carpeta', icon: 'folder', action: () => revealItemInDir(ui.local[t.id] || t.localPath) }
      : canDownload(t)
        ? { label: 'Descargar', icon: 'download', sub: () => downloadItems(t) }
        : { label: 'Solo streaming', icon: 'download', disabled: true },
    t.albumId ? { label: 'Ir al álbum', icon: 'disc', action: () => go('album', t.albumId) } : null,
    t.license ? { label: 'Ver licencia', icon: 'info', action: () => openLicense(t.license) } : null,
    ...extra
  ].filter(Boolean)
  openMenu(event, items)
}

export function openLicense(l) {
  if (/^https?:\/\//.test(l)) openUrl(l)
  else toast(l)
}
