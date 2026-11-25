<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import TrackList from '../components/TrackList.svelte'
  import { api } from '../lib/api.js'
  import { player, sourceName, totalTime, openMenu, isLocal } from '../lib/state.svelte.js'
  import { smartDownload, playlistItems, openLicense, canDownload } from '../lib/actions.js'

  let { id } = $props()
  let data = $state(null)
  let error = $state(null)

  $effect(() => {
    data = null
    error = null
    api.album(id).then((d) => (data = d)).catch((e) => (error = String(e)))
  })

  let playingHere = $derived(data && player.track?.albumId === data.album.id)
  let pending = $derived(data ? data.tracks.filter((t) => !isLocal(t) && canDownload(t)) : [])

  function playAll(shuffle = false) {
    if (playingHere && !shuffle) return api.toggle()
    if (shuffle) {
      api.shuffle(true)
      player.shuffle = true
      api.play(data.tracks, Math.floor(Math.random() * data.tracks.length))
    } else api.play(data.tracks, 0)
  }
</script>

<div class="page">
  {#if error}
    <p class="empty">{error}</p>
  {:else if !data}
    <div class="hero skel"><div class="skel-cover"></div><div class="skel-lines"><i></i><i></i><i></i></div></div>
  {:else}
    <header class="hero">
      <Cover src={data.album.cover} size={208} radius={10} />
      <div class="hero-text">
        <span class="label">Álbum · {sourceName(data.album.source)}</span>
        <h1 class="hero-title">{data.album.title}</h1>
        <p class="hero-sub">
          <strong>{data.album.artist}</strong>
          {#if data.album.year}<span>· {data.album.year}</span>{/if}
          <span>· {data.tracks.length} canciones, {totalTime(data.tracks)}</span>
        </p>
        {#if data.album.license}
          <button class="license mono" onclick={() => openLicense(data.album.license)}>{data.album.license.replace(/^https?:\/\/(www\.)?/, '')}</button>
        {/if}
      </div>
    </header>
    <div class="actions">
      <button class="play-btn lg" onclick={() => playAll()} disabled={!data.tracks.length} aria-label={playingHere && player.playing ? 'Pausar' : 'Reproducir'}>
        <Icon name={playingHere && player.playing ? 'pause' : 'play'} size={22} />
      </button>
      <button class="icon-btn" onclick={() => playAll(true)} aria-label="Aleatorio" title="Aleatorio"><Icon name="shuffle" size={22} /></button>
      <button class="icon-btn" onclick={(e) => openMenu(e, playlistItems(data.tracks))} aria-label="Añadir a playlist" title="Añadir a playlist"><Icon name="plus" size={22} /></button>
      <button class="icon-btn" onclick={(e) => smartDownload(e, pending)} disabled={!pending.length} aria-label="Descargar álbum" title={pending.length ? 'Descargar álbum' : data.tracks.some(canDownload) ? 'Ya está descargado' : 'Solo streaming'}>
        <Icon name={pending.length || !data.tracks.some(canDownload) ? 'download' : 'check'} size={22} />
      </button>
      <button class="icon-btn" onclick={() => api.enqueue(data.tracks)} aria-label="Añadir a la cola" title="Añadir a la cola"><Icon name="queue" size={22} /></button>
    </div>
    <TrackList tracks={data.tracks} numbered showAlbum={false} showCover={false} />
  {/if}
</div>
