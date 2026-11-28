<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import { ui, go } from '../lib/state.svelte.js'
  import { newPlaylist, importSpotify } from '../lib/actions.js'

  let likedCount = $derived(Object.keys(ui.liked).length)
  let localCount = $derived(Object.keys(ui.local).length)
</script>

<div class="page">
  <div class="sec-head">
    <h1 class="h1">Tu biblioteca</h1>
    <div class="sec-actions">
      <button class="btn ghost" onclick={() => importSpotify()} disabled={!!ui.importing} aria-label="Importar playlist" title="Importar playlist"><Icon name="download" size={16} /> <span class="btn-label">Importar playlist</span></button>
      <button class="btn ghost" onclick={() => newPlaylist()} aria-label="Nueva playlist" title="Nueva playlist"><Icon name="plus" size={16} /> <span class="btn-label">Nueva playlist</span></button>
    </div>
  </div>
  <div class="rows">
    <button class="lrow" onclick={() => go('liked')}>
      <span class="lrow-art liked"><Icon name="heartFill" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">Tus me gusta</span><span class="lrow-meta">{likedCount} canciones</span></span>
      <Icon name="back" size={16} />
    </button>
    <button class="lrow" onclick={() => go('downloads')}>
      <span class="lrow-art"><Icon name="download" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">Descargas</span><span class="lrow-meta">{localCount} canciones sin conexión</span></span>
      <Icon name="back" size={16} />
    </button>
    <button class="lrow" onclick={() => go('pc')}>
      <span class="lrow-art"><Icon name="disc" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">En mi PC</span><span class="lrow-meta">Control remoto, sincronizar y bajar tu música</span></span>
      <Icon name="back" size={16} />
    </button>
    <button class="lrow" onclick={() => go('stats')}>
      <span class="lrow-art"><Icon name="stats" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">Tus estadísticas</span><span class="lrow-meta">Tus artistas y canciones top, horas escuchadas</span></span>
      <Icon name="back" size={16} />
    </button>
    <button class="lrow" onclick={() => go('party')}>
      <span class="lrow-art"><Icon name="party" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">Fiesta</span><span class="lrow-meta">Que tus amigos agreguen canciones a la cola</span></span>
      <Icon name="back" size={16} />
    </button>
    <button class="lrow" onclick={() => (ui.eqOpen = true)}>
      <span class="lrow-art"><Icon name="eq" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">Ecualizador</span><span class="lrow-meta">Graves, agudos, volumen extra y karaoke</span></span>
      <Icon name="back" size={16} />
    </button>
    <button class="lrow" onclick={() => go('recent')}>
      <span class="lrow-art"><Icon name="radio" size={22} /></span>
      <span class="lrow-text"><span class="lrow-name">Escuchado recientemente</span><span class="lrow-meta">Historial</span></span>
      <Icon name="back" size={16} />
    </button>
    {#each ui.playlists as p (p.id)}
      <button class="lrow" onclick={() => go('playlist', p.id)}>
        <Cover src={p.cover} size={52} />
        <span class="lrow-text"><span class="lrow-name">{p.name}</span><span class="lrow-meta">Playlist · {p.count} canciones</span></span>
        <Icon name="back" size={16} />
      </button>
    {/each}
  </div>
</div>
