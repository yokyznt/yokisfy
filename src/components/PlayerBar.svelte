<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, fmt, go, sourceName, isLocal } from '../lib/state.svelte.js'
  import { like, trackMenu } from '../lib/actions.js'

  let now = $state(performance.now())
  let drag = $state(null)
  let volDrag = $state(null)
  let lastVol = 0.8

  $effect(() => {
    let raf
    const tick = () => {
      now = performance.now()
      raf = requestAnimationFrame(tick)
    }
    if (player.playing && !player.loading) raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  })

  let pos = $derived.by(() => {
    if (drag !== null) return drag
    let p = player.position
    if (player.playing && !player.loading && player.at) p += (now - player.at) / 1000
    return player.duration ? Math.min(p, player.duration) : p
  })
  let pct = $derived(player.duration ? (pos / player.duration) * 100 : 0)
  let vol = $derived(volDrag ?? player.volume)
  let t = $derived(player.track)
  let quality = $derived.by(() => {
    if (!t) return ''
    if (isLocal(t)) return 'LOCAL'
    if (t.source === 'youtube') return ''
    const hq = (ui.settings.stream_quality || 'alta') !== 'normal'
    const f = hq ? t.formats?.find((f) => f.kind !== 'wav') : t.formats?.find((f) => f.kind === 'mp3' || f.kind === 'ogg')
    return f ? f.label.split(' ')[0] : 'STREAM'
  })

  function seekInput(e) {
    drag = Number(e.currentTarget.value)
  }
  function seekCommit(e) {
    const v = Number(e.currentTarget.value)
    player.position = v
    player.at = performance.now()
    drag = null
    api.seek(v)
  }
  function volInput(e) {
    volDrag = Number(e.currentTarget.value)
    api.volume(volDrag)
  }
  function volCommit() {
    player.volume = volDrag
    volDrag = null
  }
  function mute() {
    if (player.volume > 0) {
      lastVol = player.volume
      player.volume = 0
      api.volume(0)
    } else {
      player.volume = lastVol || 0.8
      api.volume(player.volume)
    }
  }
  function cycleRepeat() {
    const m = { off: 'all', all: 'one', one: 'off' }[player.repeat] || 'off'
    player.repeat = m
    api.repeat(m)
  }
  function toggleShuffle() {
    player.shuffle = !player.shuffle
    api.shuffle(player.shuffle)
  }
</script>

<footer class="pb" class:empty={!t}>
  <div class="pb-progress-mobile" style={`--p:${pct}%`} aria-hidden="true"></div>
  <div class="pb-left">
    {#if t}
      <button class="pb-cover" onclick={() => (ui.fullPlayer = true)} aria-label="Abrir reproductor">
        <Cover src={t.cover} size={56} />
      </button>
      <div class="pb-meta">
        <button class="pb-title" onclick={() => t.albumId && go('album', t.albumId)}>{t.title}</button>
        <button class="pb-artist link-artist" onclick={() => !t.live && go('artist', t.artist)}>{t.artist}</button>
      </div>
      <button class="icon-btn like" class:on={ui.liked[t.id]} onclick={() => like(t)} aria-label="Me gusta">
        <Icon name={ui.liked[t.id] ? 'heartFill' : 'heart'} size={18} />
      </button>
    {:else}
      <span class="pb-idle">Elige algo para escuchar</span>
    {/if}
  </div>

  <div class="pb-center">
    <div class="pb-controls">
      <button class="icon-btn sm" class:active={player.shuffle} onclick={toggleShuffle} aria-label="Aleatorio" title="Aleatorio"><Icon name="shuffle" size={17} /></button>
      <button class="icon-btn" onclick={() => api.prev()} disabled={!t} aria-label="Anterior"><Icon name="prev" size={20} /></button>
      <button class="play-btn" onclick={() => api.toggle()} disabled={!t} aria-label={player.playing ? 'Pausar' : 'Reproducir'}>
        {#if player.loading}<span class="spin dark"></span>{:else}<Icon name={player.playing ? 'pause' : 'play'} size={18} />{/if}
      </button>
      <button class="icon-btn" onclick={() => api.next()} disabled={!t} aria-label="Siguiente"><Icon name="next" size={20} /></button>
      <button class="icon-btn sm rep" class:active={player.repeat !== 'off'} onclick={cycleRepeat} aria-label="Repetir" title={{ off: 'Repetir', all: 'Repetir todo', one: 'Repetir una' }[player.repeat]}>
        <Icon name="repeat" size={17} />{#if player.repeat === 'one'}<b>1</b>{/if}
      </button>
    </div>
    <div class="pb-seek">
      {#if t?.live}
        <span class="live-pill mono"><i></i>EN VIVO · {fmt(pos)}</span>
      {:else}
        <span class="mono time">{fmt(pos)}</span>
        <input type="range" class="range" min="0" max={player.duration || 1} step="0.1" value={pos} style={`--p:${pct}%`} oninput={seekInput} onchange={seekCommit} disabled={!t} aria-label="Posición" />
        <span class="mono time">{fmt(player.duration)}</span>
      {/if}
    </div>
  </div>

  <div class="pb-right">
    {#if t}
      {#if quality}<span class="tag" title={`Fuente: ${sourceName(t.source)}`}>{quality}</span>{/if}
      <button class="icon-btn sm" onclick={(e) => trackMenu(e, t)} aria-label="Más opciones"><Icon name="more" size={17} /></button>
    {/if}
    <button class="icon-btn sm" class:active={ui.queueOpen} onclick={() => (ui.queueOpen = !ui.queueOpen)} aria-label="Cola" title="Cola"><Icon name="queue" size={18} /></button>
    <button class="icon-btn sm" onclick={mute} aria-label={vol > 0 ? 'Silenciar' : 'Activar sonido'}><Icon name={vol > 0 ? 'volume' : 'mute'} size={18} /></button>
    <input type="range" class="range vol" min="0" max="1" step="0.01" value={vol} style={`--p:${vol * 100}%`} oninput={volInput} onchange={volCommit} aria-label="Volumen" />
  </div>

  <div class="pb-mobile-ctl">
    {#if t}
      <button class="icon-btn" onclick={() => api.toggle()} aria-label={player.playing ? 'Pausar' : 'Reproducir'}>
        {#if player.loading}<span class="spin"></span>{:else}<Icon name={player.playing ? 'pause' : 'play'} size={22} />{/if}
      </button>
    {/if}
  </div>
</footer>

{#if player.error}
  <div class="pb-error" role="status">{player.error}</div>
{/if}
