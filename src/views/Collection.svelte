<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import TrackList from '../components/TrackList.svelte'
  import { api, refreshPlaylists } from '../lib/api.js'
  import { ui, player, totalTime, back, ask, toast, isLocal } from '../lib/state.svelte.js'
  import { smartDownload, canDownload, downloadAll } from '../lib/actions.js'

  let { kind, id = null } = $props()
  let tracks = $state([])
  let name = $state('')
  let loaded = $state(false)

  async function load() {
    if (kind === 'liked') {
      tracks = await api.liked()
      name = 'Tus me gusta'
    } else if (kind === 'recent') {
      tracks = await api.recent()
      name = 'Escuchado recientemente'
    } else {
      const d = await api.playlist(id)
      tracks = d.tracks
      name = d.playlist.name
    }
    loaded = true
  }

  $effect(() => {
    kind
    id
    loaded = false
    load()
  })

  $effect(() => {
    if (kind === 'liked' && loaded) {
      const n = Object.keys(ui.liked).length
      if (n !== tracks.length) load()
    }
  })

  let here = $derived(tracks.length && tracks.some((t) => t.id === player.track?.id))
  let pending = $derived(tracks.filter((t) => !isLocal(t) && canDownload(t)))
  let online = $derived(tracks.filter((t) => !isLocal(t) && !canDownload(t) && !t.live))

  async function rename() {
    const n = await ask('Renombrar playlist', name)
    if (!n?.trim()) return
    await api.playlistRename(id, n.trim())
    name = n.trim()
    refreshPlaylists()
  }

  async function remove() {
    const n = await ask(`Escribe “borrar” para eliminar ${name}`, '', 'Eliminar')
    if (n?.trim().toLowerCase() !== 'borrar') return
    await api.playlistDelete(id)
    await refreshPlaylists()
    toast(`Eliminada: ${name}`)
    back()
  }

  function extra(t, i) {
    if (kind !== 'playlist') return []
    return [{ label: 'Quitar de esta playlist', icon: 'trash', action: async () => { await api.playlistRemove(id, i); await load(); refreshPlaylists() } }]
  }
</script>

<div class="page">
  <header class="hero">
    {#if kind === 'liked'}
      <span class="hero-art liked"><Icon name="heartFill" size={72} /></span>
    {:else if kind === 'recent'}
      <span class="hero-art"><Icon name="radio" size={64} stroke={1.3} /></span>
    {:else}
      <Cover src={tracks[0]?.cover} size={208} radius={10} />
    {/if}
    <div class="hero-text">
      <span class="label">{kind === 'playlist' ? 'Playlist' : 'Colección'}</span>
      <h1 class="hero-title">{name}</h1>
      <p class="hero-sub"><span>{tracks.length} canciones{tracks.length ? `, ${totalTime(tracks)}` : ''}</span></p>
    </div>
  </header>
  <div class="actions">
    <button class="play-btn lg" disabled={!tracks.length} onclick={() => (here ? api.toggle() : api.play(tracks, 0))} aria-label="Reproducir">
      <Icon name={here && player.playing ? 'pause' : 'play'} size={22} />
    </button>
    <button class="icon-btn" disabled={!tracks.length} onclick={() => { api.shuffle(true); player.shuffle = true; api.play(tracks, Math.floor(Math.random() * tracks.length)) }} aria-label="Aleatorio" title="Aleatorio"><Icon name="shuffle" size={22} /></button>
    <button class="icon-btn" onclick={(e) => (online.length ? downloadAll(tracks) : smartDownload(e, pending))} disabled={(!pending.length && !online.length) || !!ui.scanning} aria-label="Descargar todo lo posible" title="Descargar todo lo posible"><Icon name={pending.length || online.length || !tracks.length ? 'download' : 'check'} size={22} /></button>
    {#if kind === 'playlist'}
      <button class="icon-btn" onclick={rename} aria-label="Renombrar" title="Renombrar"><Icon name="edit" size={20} /></button>
      <button class="icon-btn" onclick={remove} aria-label="Eliminar playlist" title="Eliminar playlist"><Icon name="trash" size={20} /></button>
    {/if}
  </div>
  {#if loaded && !tracks.length}
    <p class="empty">{kind === 'liked' ? 'Toca el corazón de una canción para guardarla aquí.' : kind === 'playlist' ? 'Busca canciones y usa “Añadir a playlist”.' : 'Aún no escuchas nada.'}</p>
  {:else}
    <TrackList {tracks} extraMenu={extra} />
  {/if}
</div>
