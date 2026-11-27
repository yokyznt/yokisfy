<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, go } from '../lib/state.svelte.js'
  import { like } from '../lib/actions.js'
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

  $effect(() => {
    const tr = t
    lyr = null
    if (!tr || tr.live) return
    let alive = true
    api.lyrics(tr).then((l) => alive && (lyr = l))
    return () => (alive = false)
  })

  $effect(() => {
    const id = setInterval(() => {
      let p = player.position
      if (player.playing && player.at) p += (performance.now() - player.at) / 1000
      now = p
    }, 250)
    return () => clearInterval(id)
  })

  let lines = $derived.by(() => {
    const ls = lyr?.lines?.filter((l) => l.text.trim()) || []
    if (!ls.length) return []
    let i = ls.findLastIndex((l) => l.start <= now + 0.1)
    if (i < 0) i = 0
    return ls.slice(i, i + 3)
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
    {#if lines.length}
      <button class="now-lyr" style={`--lc:${colors[0] || '#444'}`} onclick={openLyrics}>
        <span class="now-lyr-h">Letra</span>
        {#each lines as l, i (l.start)}
          <span class="now-lyr-l" class:cur={i === 0}>{l.text}</span>
        {/each}
      </button>
    {/if}
  </aside>
{/if}
