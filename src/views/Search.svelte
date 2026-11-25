<script>
  import Icon from '../components/Icon.svelte'
  import TrackList from '../components/TrackList.svelte'
  import AlbumGrid from '../components/AlbumGrid.svelte'
  import { api } from '../lib/api.js'
  import { ui } from '../lib/state.svelte.js'

  let q = $state(ui.searchCache.q)
  let results = $state(ui.searchCache.results)
  let loading = $state(false)
  let filter = $state('all')
  let showAll = $state(false)
  let input = $state()
  let timer
  let seq = 0

  $effect(() => {
    input?.focus()
  })

  function run(value) {
    clearTimeout(timer)
    const v = value.trim()
    if (!v) {
      results = null
      ui.searchCache = { q: '', results: null }
      return
    }
    timer = setTimeout(async () => {
      const my = ++seq
      loading = true
      try {
        const r = await api.search(v)
        if (my !== seq) return
        results = r
        showAll = false
        ui.searchCache = { q: v, results: r }
      } finally {
        if (my === seq) loading = false
      }
    }, 450)
  }

  let tracks = $derived(results ? results.tracks.filter((t) => filter === 'all' || t.source === filter) : [])
  let albums = $derived(results ? results.albums.filter((a) => filter === 'all' || a.source === filter) : [])
  const genres = ['jazz', 'lo-fi', 'rock', 'electrónica', 'clásica', 'hip hop', 'ambient', 'folk', 'funk', 'reggae', 'blues', 'piano']
</script>

<div class="page">
  <div class="search-bar">
    <Icon name="search" size={20} />
    <input bind:this={input} bind:value={q} oninput={() => run(q)} placeholder="Canciones, artistas o álbumes" aria-label="Buscar" spellcheck="false" />
    {#if loading}<span class="spin"></span>{:else if q}<button class="icon-btn sm" onclick={() => { q = ''; run('') }} aria-label="Borrar"><Icon name="close" size={16} /></button>{/if}
  </div>

  {#if !results}
    <h2 class="h2">Explorar</h2>
    <div class="genres">
      {#each genres as g, i}
        <button class="genre" style={`--h:${(i * 37) % 360}`} onclick={() => { q = g; run(g) }}>{g}</button>
      {/each}
    </div>
  {:else}
    <div class="chips" role="tablist">
      {#each [['all', 'Todo'], ['jamendo', 'Jamendo'], ['archive', 'Internet Archive']] as [k, l]}
        <button class="chip" class:on={filter === k} onclick={() => (filter = k)} role="tab" aria-selected={filter === k}>{l}</button>
      {/each}
    </div>
    {#each results.warnings as w}<p class="warn">{w}</p>{/each}

    {#if tracks.length}
      <section class="sec">
        <div class="sec-head">
          <h2 class="h2">Canciones</h2>
          {#if tracks.length > 6}<button class="link" onclick={() => (showAll = !showAll)}>{showAll ? 'Ver menos' : `Ver las ${tracks.length}`}</button>{/if}
        </div>
        <TrackList {tracks} limit={showAll ? 0 : 6} />
      </section>
    {/if}
    {#if albums.length}
      <section class="sec">
        <h2 class="h2">Álbumes y conciertos</h2>
        <AlbumGrid {albums} />
      </section>
    {/if}
    {#if !tracks.length && !albums.length && !loading}
      <p class="empty">Sin resultados para “{ui.searchCache.q}”.</p>
    {/if}
  {/if}
</div>
