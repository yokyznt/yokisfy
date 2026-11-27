<script>
  import { untrack } from 'svelte'
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, go } from '../lib/state.svelte.js'
  import { like, outputMenu } from '../lib/actions.js'
  import Lyrics from './Lyrics.svelte'
  import { paletteFor } from '../lib/palette.js'

  let t = $derived(player.track)
  let colors = $state([])
  let lyr = $state(null)
  let now = $state(0)

  $effect(() => {
    const c = t?.cover
    if (!c) return
    let alive = true
    paletteFor(c).then((p) => alive && (colors = p))
    return () => (alive = false)
  })

  let tid = $derived(t && !t.live ? t.id : null)

  $effect(() => {
    const id = tid
    lyr = null
    if (!id) return
    let alive = true
    api.lyrics(untrack(() => player.track)).then((l) => alive && (lyr = l?.lines?.length ? l : null))
    return () => (alive = false)
  })

  $effect(() => {
    if (!lyr || ui.fullPlayer || !player.playing) return
    let raf
    let last = 0
    const tick = (ts) => {
      raf = requestAnimationFrame(tick)
      if (ts - last < 50) return
      last = ts
      let p = player.position
      if (player.at) p += (performance.now() - player.at) / 1000
      now = p
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  })
  let liked = $derived(t ? !!ui.liked[t.id] : false)

  function openLyrics() {
    ui.fpMode = 'lyrics'
    ui.lyrics = true
    ui.fullPlayer = true
  }
</script>

{#if t}
  <aside class="now" aria-label="Ahora suena">
    <header class="now-head">
      <strong>{t.album || 'Ahora suena'}</strong>
      <button class="icon-btn sm" onclick={outputMenu} aria-label="Salida de audio" title="Salida de audio"><Icon name="output" size={16} /></button>
      <button class="icon-btn sm" onclick={() => (ui.nowOpen = false)} aria-label="Cerrar panel"><Icon name="close" size={16} /></button>
    </header>
    <button class="now-cover" onclick={() => (ui.fullPlayer = true)} aria-label="Abrir reproductor">
      {#key t.cover}<Cover src={t.cover} size="100%" radius={10} />{/key}
    </button>
    <div class="now-meta">
      <div class="now-txt">
        <h2>{#if t.albumId}<button onclick={() => go('album', t.albumId)}>{t.title}</button>{:else}{t.title}{/if}</h2>
        <button class="link-artist" onclick={() => !t.live && go('artist', t.artist)}>{t.artist}</button>
      </div>
      {#if !t.live}<button class="icon-btn like" class:on={liked} onclick={() => like(t)} aria-label={liked ? 'Quitar de me gusta' : 'Me gusta'}><Icon name={liked ? 'heartFill' : 'heart'} size={20} /></button>{/if}
    </div>
    {#if lyr}
      <div class="now-lyr" style={`--lc:${colors[0] || '#444'}`}>
        <button class="now-lyr-h" onclick={openLyrics}>Letra</button>
        <div class="now-lyr-box"><Lyrics {lyr} time={now} compact /></div>
      </div>
    {/if}
  </aside>
{/if}
