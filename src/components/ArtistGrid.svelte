<script>
  import Cover from './Cover.svelte'
  import { go } from '../lib/state.svelte.js'

  let { artists = [], limit = 0 } = $props()
  let shown = $derived(limit ? artists.slice(0, limit) : artists)
</script>

<div class="grid">
  {#each shown as a (a.id)}
    <button class="card artist-card" onclick={() => go('artist', a.id)} aria-label={`Abrir ${a.name}`}>
      <span class="card-art round"><Cover src={a.cover} size="100%" radius={999} /></span>
      <span class="card-title">{a.name}</span>
      <span class="card-sub">{a.subtitle || 'Artista'}</span>
    </button>
  {/each}
</div>
