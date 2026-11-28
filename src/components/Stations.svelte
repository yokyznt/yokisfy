<script>
  import Icon from './Icon.svelte'
  import { api } from '../lib/api.js'
  import { player } from '../lib/state.svelte.js'

  let { stations = [], limit = 0 } = $props()
  let shown = $derived(limit ? stations.slice(0, limit) : stations)
  let broken = $state({})

  function play(i) {
    const s = shown[i]
    if (player.track?.id === s.id) return api.toggle()
    api.play([s], 0)
  }
</script>

<div class="st-grid">
  {#each shown as s, i (s.id)}
    {@const on = player.track?.id === s.id}
    <button class="st" class:on onclick={() => play(i)} aria-label={`Escuchar ${s.title}`}>
      <span class="st-logo">
        {#if s.cover && !broken[s.id]}<img src={s.cover} alt="" loading="lazy" onerror={() => (broken[s.id] = true)} />{:else}<Icon name="radio" size={22} />{/if}
        {#if on && player.playing}<span class="eq" aria-hidden="true"><i></i><i></i><i></i></span>{/if}
      </span>
      <span class="st-text">
        <span class="st-name">{s.title}</span>
        <span class="st-sub">{s.album}</span>
      </span>
    </button>
  {/each}
</div>
