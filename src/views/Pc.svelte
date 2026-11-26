<script>
  import Icon from '../components/Icon.svelte'
  import TrackList from '../components/TrackList.svelte'
  import { api } from '../lib/api.js'
  import { ui } from '../lib/state.svelte.js'
  import { smartDownload } from '../lib/actions.js'

  let state = $state(ui.pcCache ? 'ok' : 'loading')
  let name = $state(ui.pcCache?.name || '')
  let tracks = $state(ui.pcCache?.tracks || [])
  let error = $state('')
  let q = $state('')

  async function load() {
    state = 'loading'
    try {
      const r = await api.pcLibrary()
      name = r.name
      tracks = r.tracks
      ui.pcCache = r
      state = 'ok'
    } catch (e) {
      error = String(e)
      state = 'error'
    }
  }

  if (!ui.pcCache) load()

  let shown = $derived(q.trim() ? tracks.filter((t) => `${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(q.trim().toLowerCase())) : tracks)
  let missing = $derived(tracks.filter((t) => !ui.local[t.id]))

  function saveAll(e) {
    smartDownload(e, missing)
  }
</script>

<div class="page">
  <div class="sec-head">
    <h1 class="h1">En mi PC</h1>
    <button class="btn ghost" onclick={load} disabled={state === 'loading'}><Icon name="repeat" size={16} /> Actualizar</button>
  </div>
  {#if state === 'loading'}
    <p class="empty"><span class="spin"></span> Buscando tu PC en la red de casa…</p>
  {:else if state === 'error'}
    <div class="empty-box">
      <Icon name="info" size={28} />
      <p>{error}</p>
      <button class="btn primary" onclick={load}>Buscar otra vez</button>
    </div>
  {:else}
    <p class="sub">{name} · {tracks.length} canciones</p>
    <div class="actions">
      <button class="play-btn" onclick={() => tracks.length && api.play(tracks, 0)} aria-label="Reproducir todo" disabled={!tracks.length}><Icon name="play" size={20} /></button>
      {#if missing.length}<button class="btn ghost" onclick={saveAll}><Icon name="download" size={16} /> Guardar {missing.length} en el celular</button>{/if}
    </div>
    {#if tracks.length > 12}
      <div class="search-bar small"><Icon name="search" size={18} /><input bind:value={q} placeholder="Filtrar" aria-label="Filtrar" /></div>
    {/if}
    {#if tracks.length}<TrackList tracks={shown} />{:else}<p class="empty">Tu PC aún no tiene descargas.</p>{/if}
  {/if}
</div>
