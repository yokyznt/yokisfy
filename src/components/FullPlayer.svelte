<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, fmt } from '../lib/state.svelte.js'
  import { like, trackMenu } from '../lib/actions.js'

  let now = $state(performance.now())
  let drag = $state(null)
  $effect(() => {
    let raf
    const tick = () => {
      now = performance.now()
      raf = requestAnimationFrame(tick)
    }
    if (ui.fullPlayer && player.playing && !player.loading) raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  })
  let pos = $derived.by(() => {
    if (drag !== null) return drag
    let p = player.position
    if (player.playing && !player.loading && player.at) p += (now - player.at) / 1000
    return player.duration ? Math.min(p, player.duration) : p
  })
  let pct = $derived(player.duration ? (pos / player.duration) * 100 : 0)
  let t = $derived(player.track)
</script>

{#if ui.fullPlayer && t}
  <section class="fp" aria-label="Reproductor">
    <header class="fp-head">
      <button class="icon-btn" onclick={() => (ui.fullPlayer = false)} aria-label="Cerrar"><Icon name="down" size={24} /></button>
      <span class="label">{t.album}</span>
      <button class="icon-btn" onclick={(e) => trackMenu(e, t)} aria-label="Más opciones"><Icon name="more" size={20} /></button>
    </header>
    <div class="fp-art"><Cover src={t.cover} size="min(78vw, 46vh)" radius={14} /></div>
    <div class="fp-meta">
      <div>
        <h2>{t.title}</h2>
        <p>{t.artist}</p>
      </div>
      <button class="icon-btn like" class:on={ui.liked[t.id]} onclick={() => like(t)} aria-label="Me gusta"><Icon name={ui.liked[t.id] ? 'heartFill' : 'heart'} size={24} /></button>
    </div>
    {#if player.track?.live}<span class="live-pill mono"><i></i>EN VIVO · {fmt(pos)}</span>{:else}<input type="range" class="range big" min="0" max={player.duration || 1} step="0.1" value={pos} style={`--p:${pct}%`} oninput={(e) => (drag = Number(e.currentTarget.value))} onchange={(e) => { const v = Number(e.currentTarget.value); player.position = v; player.at = performance.now(); drag = null; api.seek(v) }} aria-label="Posición" />
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
    <div class="fp-foot">
      <button class="icon-btn" onclick={() => { ui.fullPlayer = false; ui.queueOpen = true }} aria-label="Cola"><Icon name="queue" size={22} /></button>
    </div>
  </section>
{/if}
