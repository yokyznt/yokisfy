import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ui, player, toast } from './state.svelte.js'

export async function call(cmd, args = {}) {
  try {
    return await invoke(cmd, args)
  } catch (e) {
    toast(String(e))
    throw e
  }
}

export const api = {
  search: (q) => call('search', { q }),
  album: (id) => call('album', { id }),
  home: () => call('home'),
  stations: (q) => call('stations', { q }),
  serverInfo: () => call('server_info'),
  pcLibrary: () => call('pc_library'),
  startRadio: (track) => call('start_radio', { track: plain(track) }),
  play: (tracks, start = 0) => call('play', { tracks: plain(tracks), start }),
  toggle: () => call('toggle'),
  next: () => call('next'),
  prev: () => call('prev'),
  seek: (secs) => call('seek', { secs }),
  volume: (v) => call('volume', { v }),
  enqueue: (tracks) => call('enqueue', { tracks: plain(tracks) }),
  playNext: (track) => call('play_next', { track: plain(track) }),
  jump: (index) => call('jump', { index }),
  queueRemove: (index) => call('queue_remove', { index }),
  shuffle: (on) => call('shuffle', { on }),
  repeat: (mode) => call('repeat', { mode }),
  playlists: () => call('playlists'),
  playlist: (id) => call('playlist', { id }),
  playlistCreate: (name, tracks = []) => call('playlist_create', { name, tracks: plain(tracks) }),
  playlistRename: (id, name) => call('playlist_rename', { id, name }),
  playlistDelete: (id) => call('playlist_delete', { id }),
  playlistAdd: (id, tracks) => call('playlist_add', { id, tracks: plain(tracks) }),
  playlistRemove: (id, index) => call('playlist_remove', { id, index }),
  liked: () => call('liked'),
  likedIds: () => call('liked_ids'),
  toggleLike: (track) => call('toggle_like', { track: plain(track) }),
  recent: () => call('recent'),
  downloads: () => call('downloads'),
  download: (track, kind = null) => call('download', { track: plain(track), kind }),
  deleteDownload: (id) => call('delete_download', { id }),
  importSpotify: (link) => call('import_spotify', { link }),
  extProgress: (pos, dur) => invoke('ext_progress', { pos, dur }).catch(() => {}),
  extEnded: () => invoke('ext_ended').catch(() => {}),
  extFailed: (error) => invoke('ext_failed', { error }).catch(() => {}),
  settings: () => call('settings'),
  setSetting: (key, value) => call('set_setting', { key, value: String(value) })
}

function plain(v) {
  return JSON.parse(JSON.stringify(v))
}

export async function boot() {
  await listen('player', (e) => {
    Object.assign(player, e.payload, { at: performance.now() })
  })
  await listen('queue', (e) => {
    player.queue = e.payload.tracks
  })
  await listen('import', (e) => {
    if (ui.importing) ui.importing = e.payload
  })
  await listen('autoradio', (e) => {
    toast(`Radio automática: música parecida a ${e.payload}`)
  })
  await listen('download', (e) => {
    const p = e.payload
    ui.downloads[p.id] = p
    if (p.state === 'done') {
      ui.local[p.id] = p.path
      toast(`Descargada: ${p.title}`)
    } else if (p.state === 'error') {
      toast(`No se pudo descargar ${p.title}: ${p.error}`)
    }
  })
  const [ids, pls, settings, dls] = await Promise.all([api.likedIds(), api.playlists(), api.settings(), api.downloads()])
  for (const id of ids) ui.liked[id] = true
  ui.playlists = pls
  ui.settings = settings
  for (const t of dls) ui.local[t.id] = t.localPath
}

export async function refreshPlaylists() {
  ui.playlists = await api.playlists()
}
