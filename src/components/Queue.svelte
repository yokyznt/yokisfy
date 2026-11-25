<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, fmt } from '../lib/state.svelte.js'

  let upcoming = $derived(player.queue.map((t, i) => ({ t, i })).filter(({ i }) => i !== player.index))
</script>

<aside class="queue" class:open={ui.queueOpen} aria-label="Cola de reproducción">
  <header class="queue-head">
    <h2>Cola</h2>
    <button class="icon-btn sm" onclick={() => (ui.queueOpen = false)} aria-label="Cerrar cola"><Icon name="close" size={18} /></button>
  </header>
  {#if player.track}
    <p class="label">Sonando</p>
    <div class="q-row current">
      <Cover src={player.track.cover} size={40} />
      <span class="q-text"><span class="q-title">{player.track.title}</span><span class="q-artist">{player.track.artist}</span></span>
      <span class="mono q-dur">{fmt(player.track.duration)}</span>
    </div>
  {/if}
  {#if upcoming.length}
    <p class="label">{player.shuffle ? 'En la cola · aleatorio' : 'A continuación'}</p>
    <div class="q-list">
      {#each upcoming as { t, i } (t.id + i)}
        <div class="q-row">
          <button class="q-hit" onclick={() => api.jump(i)} aria-label={`Reproducir ${t.title}`}></button>
          <Cover src={t.cover} size={40} />
          <span class="q-text"><span class="q-title">{t.title}</span><span class="q-artist">{t.artist}</span></span>
          <button class="icon-btn sm q-x" onclick={() => api.queueRemove(i)} aria-label="Quitar de la cola"><Icon name="close" size={15} /></button>
        </div>
      {/each}
    </div>
  {:else if !player.track}
    <p class="muted pad">La cola está vacía.</p>
  {/if}
</aside>
