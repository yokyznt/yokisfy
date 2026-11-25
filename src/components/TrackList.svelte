<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, fmt, bestLabel, isLocal, go } from '../lib/state.svelte.js'
  import { like, trackMenu } from '../lib/actions.js'

  let { tracks = [], numbered = false, showAlbum = true, showCover = true, extraMenu = null, limit = 0 } = $props()

  let shown = $derived(limit ? tracks.slice(0, limit) : tracks)

  function play(i) {
    const t = tracks[i]
    if (player.track?.id === t.id) return api.toggle()
    api.play(tracks, i)
  }

  function key(e, i) {
    if (e.key === 'Enter') play(i)
  }
</script>

<div class="tl" class:nocover={!showCover} class:noalbum={!showAlbum} role="list">
  {#each shown as t, i (t.id + ':' + i)}
    {@const current = player.track?.id === t.id}
    {@const dl = ui.downloads[t.id]}
    <div class="tl-row" class:current role="listitem">
      <button class="tl-main" onclick={() => play(i)} onkeydown={(e) => key(e, i)} aria-label={`Reproducir ${t.title}`}>
        <span class="tl-n">
          {#if current && player.playing && !player.loading}
            <span class="eq" aria-hidden="true"><i></i><i></i><i></i></span>
          {:else if current && player.loading}
            <span class="spin" aria-hidden="true"></span>
          {:else}
            <span class="num">{i + 1}</span>
          {/if}
          <span class="hover-play"><Icon name={current && player.playing ? 'pause' : 'play'} size={14} /></span>
        </span>
        {#if showCover}<Cover src={t.cover} size={40} />{/if}
        <span class="tl-text">
          <span class="tl-title">{t.title}</span>
          <span class="tl-artist">
            {#if isLocal(t)}<span class="dot-ok" title="Descargada"><Icon name="check" size={11} stroke={3} /></span>{/if}
            {t.artist}
          </span>
        </span>
      </button>
      {#if showAlbum}
        <button class="tl-album" onclick={() => t.albumId && go('album', t.albumId)} disabled={!t.albumId}>{t.album}</button>
      {/if}
      <span class="tl-tags">
        {#if dl && dl.state !== 'done' && dl.state !== 'error'}
          <span class="ring" style={`--p:${dl.total ? (dl.received / dl.total) * 100 : 8}`} title="Descargando"></span>
        {:else if bestLabel(t)}
          <span class="tag">{bestLabel(t)}</span>
        {/if}
      </span>
      <button class="icon-btn like" class:on={ui.liked[t.id]} onclick={() => like(t)} aria-label={ui.liked[t.id] ? 'Quitar de Me gusta' : 'Me gusta'}>
        <Icon name={ui.liked[t.id] ? 'heartFill' : 'heart'} size={17} />
      </button>
      <span class="tl-dur mono">{fmt(t.duration)}</span>
      <button class="icon-btn" onclick={(e) => trackMenu(e, t, extraMenu ? extraMenu(t, i) : [])} aria-label="Más opciones">
        <Icon name="more" size={18} />
      </button>
    </div>
  {/each}
</div>
