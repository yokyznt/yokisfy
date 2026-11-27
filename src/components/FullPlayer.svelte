<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import Backdrop from './Backdrop.svelte'
  import Lyrics from './Lyrics.svelte'
  import Visualizer from './Visualizer.svelte'
  import Volume from './Volume.svelte'
  import CoverFlow from './CoverFlow.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, fmt, go, openMenu } from '../lib/state.svelte.js'
  import { like, trackMenu } from '../lib/actions.js'
  import { sleepItems } from '../lib/sleep.js'
  import { paletteFor } from '../lib/palette.js'

  let now = $state(performance.now())
  let drag = $state(null)
  let idle = $state(false)
  let colors = $state([])
  let preset = $state(0)
  let idleTimer

  let mode = $derived(ui.fpMode || 'cover')
  let immersive = $derived(mode !== 'cover')
  let t = $derived(player.track)

  $effect(() => {
    if (!ui.fullPlayer) return
    let raf
    const tick = () => {
      now = performance.now()
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  })

  let pos = $derived.by(() => {
    if (drag !== null) return drag
    let p = player.position
    if (player.playing && !player.loading && player.at) p += (now - player.at) / 1000
    return player.duration ? Math.min(p, player.duration) : p
  })
  let pct = $derived(player.duration ? (pos / player.duration) * 100 : 0)

  let lyr = $state(null)
  let lyrFor = $state(null)

  $effect(() => {
    if (mode !== 'lyrics' || !t || t.live || lyrFor === t.id) return
    const id = t.id
    lyrFor = id
    lyr = 'loading'
    api.lyrics(t).then((l) => {
      if (lyrFor === id) lyr = l
    })
  })

  $effect(() => {
    const c = t?.cover
    if (!ui.fullPlayer) return
    paletteFor(c).then((p) => {
      if (t?.cover === c) colors = p
    })
  })

  function wake() {
    idle = false
    clearTimeout(idleTimer)
    if (immersive) idleTimer = setTimeout(() => (idle = true), 2600)
  }

  $effect(() => {
    if (!ui.fullPlayer) return
    immersive
    wake()
    return () => clearTimeout(idleTimer)
  })

  $effect(() => {
    ui.fpIdle = !!ui.fullPlayer && idle && immersive
  })

  function setMode(m) {
    ui.fpMode = mode === m ? 'cover' : m
    ui.lyrics = ui.fpMode === 'lyrics'
  }

  function key(e) {
    if (!ui.fullPlayer) return
    wake()
    if (e.key === 'Escape') ui.fullPlayer = false
    else if (e.key === ' ' && e.target === document.body) {
      e.preventDefault()
      api.toggle()
    }
  }
</script>

<svelte:window onkeydown={key} />

{#if ui.fullPlayer && t}
  <section class="fp" class:vbg={t.source === 'youtube' && mode !== 'visual' && ui.settings.yt_video !== 'off'} class:immersive class:idle={idle && immersive} class:m-lyrics={mode === 'lyrics'} class:m-visual={mode === 'visual'} aria-label="Reproductor" onmousemove={wake} onpointerdown={wake} ontouchstart={wake}>
    {#if mode === 'visual'}
      <Visualizer {colors} {preset} />
    {:else if mode === 'lyrics'}
      <CoverFlow cover={t.cover} {colors} />
    {:else}
      <Backdrop cover={t.cover} {colors} intense={mode === 'lyrics'} />
    {/if}

    <header class="fp-head fp-chrome">
      <button class="icon-btn" onclick={() => (ui.fullPlayer = false)} aria-label="Cerrar"><Icon name="down" size={24} /></button>
      <span class="label">{t.album}</span>
      <button class="icon-btn" onclick={(e) => trackMenu(e, t)} aria-label="Más opciones"><Icon name="more" size={20} /></button>
    </header>

    {#if mode === 'lyrics'}
      <div class="fp-lyr">
        {#if lyr === 'loading'}
          <p class="lyr-note">Buscando la letra…</p>
        {:else if !lyr}
          <p class="lyr-note">No encontré la letra de esta canción.</p>
        {:else if lyr.instrumental}
          <p class="lyr-note">Instrumental ♪</p>
        {:else if lyr.lines?.length}
          <Lyrics {lyr} time={pos} />
        {:else}
          <p class="lyr-plain">{lyr.plain}</p>
        {/if}
      </div>
    {:else if mode === 'visual'}
      <button class="fp-viz-hit" onclick={() => (preset = (preset + 1) % 3)} aria-label="Cambiar animación"></button>
      <div class="fp-viz-meta fp-chrome">
        <Cover src={t.cover} size={64} radius={10} />
        <div>
          <strong>{t.title}</strong>
          <span>{t.artist}</span>
        </div>
      </div>
    {:else}
      <div class="fp-art"><Cover src={t.cover} size="min(78vw, 46vh)" radius={14} /></div>
    {/if}

    <div class="fp-bottom fp-chrome">
      {#if mode === 'cover'}
        <div class="fp-meta">
          <div>
            <h2>{#if t.albumId}<button onclick={() => go('album', t.albumId)}>{t.title}</button>{:else}{t.title}{/if}</h2>
            <p><button class="link-artist" onclick={() => !t.live && go('artist', t.artist)}>{t.artist}</button></p>
            {#if t.album && t.albumId}<button class="fp-album link-artist" onclick={() => go('album', t.albumId)}>{t.album}</button>{/if}
          </div>
          <button class="icon-btn like" class:on={ui.liked[t.id]} onclick={() => like(t)} aria-label="Me gusta"><Icon name={ui.liked[t.id] ? 'heartFill' : 'heart'} size={24} /></button>
        </div>
      {:else}
        <div class="fp-mini">
          <strong>{t.title}</strong>
          <span>{t.artist}</span>
        </div>
      {/if}
      {#if t.live}<span class="live-pill mono"><i></i>EN VIVO · {fmt(pos)}</span>{:else}<input type="range" class="range big" min="0" max={player.duration || 1} step="0.1" value={pos} style={`--p:${pct}%`} oninput={(e) => (drag = Number(e.currentTarget.value))} onchange={(e) => { const v = Number(e.currentTarget.value); player.position = v; player.at = performance.now(); drag = null; api.seek(v) }} aria-label="Posición" />
      <div class="fp-times mono"><span>{fmt(pos)}</span><span>{fmt(player.duration)}</span></div>{/if}
      <div class="fp-controls">
        <button class="icon-btn" class:active={player.shuffle} onclick={() => { player.shuffle = !player.shuffle; api.shuffle(player.shuffle) }} aria-label="Aleatorio"><Icon name="shuffle" size={22} /></button>
        <button class="icon-btn" onclick={() => api.prev()} aria-label="Anterior"><Icon name="prev" size={30} /></button>
        <button class="play-btn xl" onclick={() => api.toggle()} aria-label={player.playing ? 'Pausar' : 'Reproducir'}>
          {#if player.loading}<span class="spin dark"></span>{:else}<Icon name={player.playing ? 'pause' : 'play'} size={28} />{/if}
        </button>
        <button class="icon-btn" onclick={() => api.next()} aria-label="Siguiente"><Icon name="next" size={30} /></button>
        <button class="icon-btn rep" class:active={player.repeat !== 'off'} onclick={() => { player.repeat = { off: 'all', all: 'one', one: 'off' }[player.repeat]; api.repeat(player.repeat) }} aria-label="Repetir"><Icon name="repeat" size={22} />{#if player.repeat === 'one'}<b>1</b>{/if}</button>
      </div>
      <Volume big />
      <div class="fp-foot">
        <button class="icon-btn" class:active={!!ui.sleep} onclick={(e) => openMenu(e, sleepItems())} aria-label="Temporizador para dormir" title={ui.sleep ? `Se pausa ${ui.sleep.label === 'al terminar la canción' ? 'al terminar la canción' : 'en ' + ui.sleep.label}` : 'Temporizador para dormir'}><Icon name="moon" size={22} /></button>
        <button class="icon-btn" class:active={mode === 'visual'} onclick={() => setMode('visual')} aria-label="Animaciones" title="Animaciones"><Icon name="spark" size={22} /></button>
        {#if !t.live}<button class="icon-btn" class:active={mode === 'lyrics'} onclick={() => setMode('lyrics')} aria-label="Letra" title="Letra"><Icon name="lyrics" size={22} /></button>{/if}
        <button class="icon-btn" onclick={() => { ui.fullPlayer = false; ui.queueOpen = true }} aria-label="Cola"><Icon name="queue" size={22} /></button>
      </div>
    </div>
  </section>
{/if}
