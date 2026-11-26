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

export async function downloadAll(tracks) {
  const todo = tracks.filter((t) => !t.live && !isLocalTrack(t))
  if (!todo.length) return toast('Ya está todo descargado')
  if (ui.scanning) return toast('Ya estoy buscando otras canciones, espera un momento')
  ui.scanning = { done: 0, total: todo.length }
  let found
  try {
    found = await api.freeVersions(todo)
  } catch {
    ui.scanning = null
    return
  }
  ui.scanning = null
  const pairs = todo.map((t, i) => [t, found[i]]).filter(([, f]) => f)
  const online = todo.length - pairs.length
  toast(pairs.length ? `Descargando ${pairs.length} en la mejor calidad${online ? ` · ${online} solo se pueden escuchar en línea` : ''}` : `Ninguna tiene versión gratis para descargar; ${online} se escuchan en línea`)
  let next = 0
  const worker = async () => {
    while (next < pairs.length) {
      const [orig, free] = pairs[next++]
      const f = free.formats?.find((x) => x.kind !== 'wav') || free.formats?.[0]
      ui.downloads[orig.id] = { id: orig.id, title: orig.title, artist: orig.artist, format: f?.label || '', received: 0, total: null, state: 'queued' }
      try {
        const t = await api.downloadAs(orig, free)
        if (t.localPath) ui.local[orig.id] = t.localPath
        if (ui.downloads[orig.id]?.state !== 'done') delete ui.downloads[orig.id]
      } catch {
        delete ui.downloads[orig.id]
      }
    }
  }
  await Promise.all([worker(), worker()])
}

function isLocalTrack(t) {
  return !!(ui.local[t.id] || t.localPath)
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
  const link = await ask('Pega el link de una playlist de Spotify o YouTube Music', '', 'Importar')
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
    t.artist && !t.live ? { label: 'Ir al artista', icon: 'mic', action: () => go('artist', t.artist) } : null,
    t.source === 'youtube' ? { label: 'Copiar link', icon: 'list', action: () => copyLink(t) } : null,
    t.license ? { label: 'Ver licencia', icon: 'info', action: () => openLicense(t.license) } : null,
    ...extra
  ].filter(Boolean)
  openMenu(event, items)
}

export async function copyLink(t) {
  const url = `https://music.youtube.com/watch?v=${t.stream_url || t.streamUrl}`
  try {
    await navigator.clipboard.writeText(url)
    toast('Link copiado')
  } catch {
    toast(url)
  }
}

export function openLicense(l) {
  if (/^https?:\/\//.test(l)) openUrl(l)
  else toast(l)
}
