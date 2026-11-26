<script>
  import Icon from '../components/Icon.svelte'
  import TrackList from '../components/TrackList.svelte'
  import { api } from '../lib/api.js'
  import Cover from '../components/Cover.svelte'
  import { ui, player, fmt, toast } from '../lib/state.svelte.js'
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

  let remote = $state(null)
  let rAt = $state(0)
  let tick = $state(performance.now())
  let syncing = $state(false)

  async function poll() {
    try {
      const r = await api.pcCall('now')
      remote = r
      rAt = performance.now()
    } catch {
      remote = null
    }
  }

  $effect(() => {
    poll()
    const a = setInterval(poll, 1500)
    const b = setInterval(() => (tick = performance.now()), 250)
    return () => {
      clearInterval(a)
      clearInterval(b)
    }
  })

  let rp = $derived(remote?.player)
  let rpos = $derived(rp ? Math.min(rp.position + (rp.playing ? (tick - rAt) / 1000 : 0), rp.duration || 1e9) : 0)

  async function cmd(c, v = 0) {
    try {
      await api.pcCall('cmd', { c, v })
      setTimeout(poll, 250)
    } catch {
      toast('No pude mandar la orden a tu PC')
    }
  }

  async function listenHere() {
    const q = remote?.queue?.tracks
    if (!rp?.track || !q?.length) return
    const at = rpos
    const idx = Math.max(0, q.findIndex((t) => t.id === rp.track.id))
    await cmd('pause')
    await api.play(q, idx)
    const id = rp.track.id
    const t0 = Date.now()
    const wait = setInterval(() => {
      if (player.track?.id === id && !player.loading) {
        clearInterval(wait)
        if (at > 3) api.seek(at)
      } else if (Date.now() - t0 > 15000) clearInterval(wait)
    }, 300)
    toast('Sigues escuchando en el celular')
  }

  async function sync() {
    syncing = true
    try {
      const r = await api.pcSync()
      toast(r.received || r.sent ? `Sincronizado con ${r.name}: ${r.received} nuevos aquí, ${r.sent} en la PC` : 'Todo está igual en los dos')
      const { refreshPlaylists } = await import('../lib/api.js')
      await refreshPlaylists()
      const ids = await api.likedIds()
      for (const id of ids) ui.liked[id] = true
    } catch (e) {
      toast(String(e))
    }
    syncing = false
  }

  let shown = $derived(q.trim() ? tracks.filter((t) => `${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(q.trim().toLowerCase())) : tracks)
  let missing = $derived(tracks.filter((t) => !ui.local[t.id]))

  function saveAll(e) {
    smartDownload(e, missing)
  }
</script>

<div class="page">
  <div class="sec-head">
    <h1 class="h1">En mi PC</h1>
    <div class="sec-actions">
      <button class="btn ghost" onclick={sync} disabled={syncing}><Icon name="sync" size={16} /> {syncing ? 'Sincronizando…' : 'Sincronizar'}</button>
      <button class="btn ghost" onclick={load} disabled={state === 'loading'}><Icon name="repeat" size={16} /> Actualizar</button>
    </div>
  </div>
  {#if rp?.track}
    <div class="rc">
      <span class="label"><Icon name="remote" size={14} /> Sonando en tu PC</span>
      <div class="rc-main">
        <Cover src={rp.track.cover} size={72} radius={10} />
        <div class="rc-text"><b>{rp.track.title}</b><span>{rp.track.artist}</span></div>
      </div>
      {#if !rp.track.live}<div class="rc-bar" style={`--p:${rp.duration ? (rpos / rp.duration) * 100 : 0}%`}><i></i></div>
      <div class="fp-times mono"><span>{fmt(rpos)}</span><span>{fmt(rp.duration)}</span></div>{/if}
      <div class="rc-ctl">
        <button class="icon-btn" onclick={() => cmd('prev')} aria-label="Anterior en la PC"><Icon name="prev" size={24} /></button>
        <button class="play-btn" onclick={() => cmd('toggle')} aria-label={rp.playing ? 'Pausar la PC' : 'Reproducir en la PC'}><Icon name={rp.playing ? 'pause' : 'play'} size={22} /></button>
        <button class="icon-btn" onclick={() => cmd('next')} aria-label="Siguiente en la PC"><Icon name="next" size={24} /></button>
      </div>
      <label class="rc-vol"><Icon name="volume" size={18} /><input type="range" class="range" min="0" max="1" step="0.02" value={rp.volume} style={`--p:${rp.volume * 100}%`} onchange={(e) => cmd('volume', Number(e.currentTarget.value))} aria-label="Volumen de la PC" /></label>
      <button class="btn primary" onclick={listenHere}><Icon name="play" size={16} /> Seguir escuchando aquí</button>
    </div>
  {/if}
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
