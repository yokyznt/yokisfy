<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { api } from '../lib/api.js'
  import { player, fmt } from '../lib/state.svelte.js'
  import { like } from '../lib/actions.js'
  import { ui } from '../lib/state.svelte.js'

  const win = getCurrentWindow()
  let now = $state(performance.now())

  $effect(() => {
    let raf
    const tick = () => {
      now = performance.now()
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  })

  let t = $derived(player.track)
  let pos = $derived.by(() => {
    let p = player.position
    if (player.playing && !player.loading && player.at) p += (now - player.at) / 1000
    return player.duration ? Math.min(p, player.duration) : p
  })
  let pct = $derived(player.duration ? (pos / player.duration) * 100 : 0)
</script>

<div class="mini" data-tauri-drag-region>
  {#if t}
    <Cover src={t.cover} size={96} radius={10} />
    <div class="mini-body" data-tauri-drag-region>
      <div class="mini-text" data-tauri-drag-region>
        <b>{t.title}</b>
        <span>{t.artist}</span>
      </div>
      <div class="mini-ctl">
        <button class="icon-btn sm" onclick={() => api.prev()} aria-label="Anterior"><Icon name="prev" size={18} /></button>
        <button class="play-btn sm" onclick={() => api.toggle()} aria-label={player.playing ? 'Pausar' : 'Reproducir'}>
          {#if player.loading}<span class="spin dark"></span>{:else}<Icon name={player.playing ? 'pause' : 'play'} size={16} />{/if}
        </button>
        <button class="icon-btn sm" onclick={() => api.next()} aria-label="Siguiente"><Icon name="next" size={18} /></button>
        <button class="icon-btn sm like" class:on={ui.liked[t.id]} onclick={() => like(t)} aria-label="Me gusta"><Icon name={ui.liked[t.id] ? 'heartFill' : 'heart'} size={16} /></button>
      </div>
      {#if !t.live}<div class="mini-bar" style={`--p:${pct}%`}><i></i></div><div class="mini-time mono"><span>{fmt(pos)}</span><span>{fmt(player.duration)}</span></div>{/if}
    </div>
  {:else}
    <p class="mini-empty" data-tauri-drag-region>Nada sonando</p>
  {/if}
  <button class="mini-x" onclick={() => win.close()} aria-label="Cerrar mini reproductor"><Icon name="close" size={13} /></button>
</div>
