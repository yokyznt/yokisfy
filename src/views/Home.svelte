<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import TrackList from '../components/TrackList.svelte'
  import AlbumGrid from '../components/AlbumGrid.svelte'
  import Stations from '../components/Stations.svelte'
  import { api } from '../lib/api.js'
  import { ui, go } from '../lib/state.svelte.js'

  let sections = $state(ui.homeCache || [])
  let recent = $state([])
  let loading = $state(!ui.homeCache)

  const h = new Date().getHours()
  const greet = h < 6 ? 'Buenas noches' : h < 12 ? 'Buenos días' : h < 19 ? 'Buenas tardes' : 'Buenas noches'

  $effect(() => {
    api.recent().then((r) => (recent = r))
    api.home().then((s) => {
      sections = s
      ui.homeCache = s
      loading = false
    }).catch(() => (loading = false))
  })

  let tiles = $derived.by(() => {
    const seen = new Set()
    const out = []
    for (const t of recent) {
      const k = t.albumId || t.id
      if (seen.has(k)) continue
      seen.add(k)
      out.push(t)
      if (out.length === 6) break
    }
    return out
  })
</script>

<div class="page">
  <h1 class="h1">{greet}</h1>

  {#if tiles.length}
    <div class="tiles">
      {#each tiles as t (t.id)}
        <div class="tile">
          <button class="tile-hit" onclick={() => (t.albumId ? go('album', t.albumId) : api.play([t], 0))} aria-label={t.album || t.title}></button>
          <Cover src={t.cover} size={56} radius={0} />
          <span class="tile-name">{t.albumId ? t.album : t.title}</span>
          <button class="tile-play" onclick={() => api.play(recent, recent.indexOf(t))} aria-label={`Reproducir ${t.title}`}><Icon name="play" size={16} /></button>
        </div>
      {/each}
    </div>
  {/if}


  {#if recent.length}
    <section class="sec">
      <h2 class="h2">Escuchado recientemente</h2>
      <TrackList tracks={recent} limit={5} />
    </section>
  {/if}

  {#if loading}
    <section class="sec">
      <div class="skel-title"></div>
      <div class="grid">{#each Array(6) as _}<div class="skel-card"></div>{/each}</div>
    </section>
  {/if}

  {#each sections as s (s.title)}
    <section class="sec">
      <h2 class="h2">{s.title}</h2>
      {#if s.tracks.length && s.tracks[0].live}<Stations stations={s.tracks} limit={12} />{:else if s.tracks.length}<TrackList tracks={s.tracks} limit={8} />{/if}
      {#if s.albums.length}<AlbumGrid albums={s.albums} limit={12} />{/if}
    </section>
  {/each}
</div>
