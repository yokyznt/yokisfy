<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import TrackList from '../components/TrackList.svelte'
  import AlbumGrid from '../components/AlbumGrid.svelte'
  import ArtistGrid from '../components/ArtistGrid.svelte'
  import { api } from '../lib/api.js'
  import { ui, player } from '../lib/state.svelte.js'
  import { startRadio, downloadAll } from '../lib/actions.js'

  let { id } = $props()
  let data = $state(null)
  let error = $state(null)

  $effect(() => {
    data = null
    error = null
    api.artist(id).then((d) => (data = d)).catch((e) => (error = String(e)))
  })

  let playingHere = $derived(data && data.top.some((t) => t.id === player.track?.id))

  function playAll(shuffle = false) {
    if (playingHere && !shuffle) return api.toggle()
    if (shuffle) {
      api.shuffle(true)
      player.shuffle = true
      api.play(data.top, Math.floor(Math.random() * data.top.length))
    } else api.play(data.top, 0)
  }
</script>

<div class="page">
  {#if error}
    <p class="empty">{error}</p>
  {:else if !data}
    <div class="hero skel"><div class="skel-cover round"></div><div class="skel-lines"><i></i><i></i><i></i></div></div>
  {:else}
    <header class="hero">
      <Cover src={data.artist.cover} size={208} radius={999} />
      <div class="hero-text">
        <span class="label">Artista</span>
        <h1 class="hero-title">{data.artist.name}</h1>
        {#if data.artist.subtitle}<p class="hero-sub"><span>{data.artist.subtitle} suscriptores</span></p>{/if}
      </div>
    </header>
    <div class="actions">
      <button class="play-btn lg" onclick={() => playAll()} disabled={!data.top.length} aria-label={playingHere && player.playing ? 'Pausar' : 'Reproducir'}>
        <Icon name={playingHere && player.playing ? 'pause' : 'play'} size={22} />
      </button>
      <button class="icon-btn" onclick={() => playAll(true)} disabled={!data.top.length} aria-label="Aleatorio" title="Aleatorio"><Icon name="shuffle" size={22} /></button>
      <button class="icon-btn" onclick={() => downloadAll(data.top)} disabled={!data.top.length || !!ui.scanning} aria-label="Descargar todo lo posible" title="Descargar todo lo posible"><Icon name="download" size={22} /></button>
      <button class="icon-btn" onclick={() => data.top[0] && startRadio(data.top[0])} disabled={!data.top.length} aria-label="Radio del artista" title="Radio del artista"><Icon name="radio" size={22} /></button>
    </div>
    {#if data.top.length}
      <section class="sec">
        <h2 class="h2">Populares</h2>
        <TrackList tracks={data.top} />
      </section>
    {/if}
    {#if data.albums.length}
      <section class="sec">
        <h2 class="h2">Álbumes</h2>
        <AlbumGrid albums={data.albums} />
      </section>
    {/if}
    {#if data.singles.length}
      <section class="sec">
        <h2 class="h2">Sencillos y EP</h2>
        <AlbumGrid albums={data.singles} />
      </section>
    {/if}
    {#if data.related.length}
      <section class="sec">
        <h2 class="h2">Artistas parecidos</h2>
        <ArtistGrid artists={data.related} />
      </section>
    {/if}
  {/if}
</div>
