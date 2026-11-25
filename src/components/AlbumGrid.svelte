<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { go, sourceName } from '../lib/state.svelte.js'

  let { albums = [], limit = 0 } = $props()
  let shown = $derived(limit ? albums.slice(0, limit) : albums)
  let busy = $state(null)

  async function quickPlay(e, a) {
    e.stopPropagation()
    busy = a.id
    try {
      const d = await api.album(a.id)
      if (d.tracks.length) await api.play(d.tracks, 0)
    } finally {
      busy = null
    }
  }
</script>

<div class="grid">
  {#each shown as a (a.id)}
    <div class="card">
      <button class="card-hit" onclick={() => go('album', a.id)} aria-label={`Abrir ${a.title}`}></button>
      <div class="card-art">
        <Cover src={a.cover} size="100%" radius={8} />
        <button class="fab" class:busy={busy === a.id} onclick={(e) => quickPlay(e, a)} aria-label={`Reproducir ${a.title}`}>
          {#if busy === a.id}<span class="spin dark"></span>{:else}<Icon name="play" size={18} />{/if}
        </button>
      </div>
      <span class="card-title">{a.title}</span>
      <span class="card-sub">{a.artist}{a.year ? ` · ${a.year}` : ''}</span>
      <span class="card-src mono">{sourceName(a.source)}</span>
    </div>
  {/each}
</div>
