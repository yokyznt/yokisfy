<script>
  import Icon from '../components/Icon.svelte'
  import TrackList from '../components/TrackList.svelte'
  import { api } from '../lib/api.js'
  import { ui, toast } from '../lib/state.svelte.js'
  import { revealItemInDir, openPath } from '@tauri-apps/plugin-opener'

  let tracks = $state([])
  let active = $derived(Object.values(ui.downloads).filter((d) => d.state === 'downloading' || d.state === 'queued'))
  let failed = $derived(Object.values(ui.downloads).filter((d) => d.state === 'error'))
  let doneCount = $derived(Object.keys(ui.local).length)

  async function load() {
    tracks = await api.downloads()
  }

  $effect(() => {
    doneCount
    load()
  })

  function mb(b) {
    return b ? (b / 1048576).toFixed(1) + ' MB' : '—'
  }

  function extra(t) {
    return [
      {
        label: 'Eliminar descarga',
        icon: 'trash',
        action: async () => {
          await api.deleteDownload(t.id)
          delete ui.local[t.id]
          delete ui.downloads[t.id]
          toast(`Eliminada: ${t.title}`)
        }
      }
    ]
  }
</script>

<div class="page">
  <div class="sec-head">
    <h1 class="h1">Descargas</h1>
    <button class="btn ghost" onclick={() => openPath(ui.settings.download_dir_effective)}><Icon name="folder" size={16} /> Abrir carpeta</button>
  </div>

  {#if active.length}
    <section class="sec">
      <h2 class="h2">En curso</h2>
      <div class="dl-list">
        {#each active as d (d.id)}
          {@const p = d.total ? Math.min(100, (d.received / d.total) * 100) : 0}
          <div class="dl">
            <div class="dl-top">
              <span class="dl-name"><strong>{d.title}</strong> · {d.artist}</span>
              <span class="mono dl-meta">{d.format} · {mb(d.received)}{d.total ? ` de ${mb(d.total)}` : ''}</span>
            </div>
            <div class="bar" class:indeterminate={!d.total}><i style={`width:${p}%`}></i></div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if failed.length}
    <section class="sec">
      {#each failed as d (d.id)}<p class="warn">{d.title}: {d.error}</p>{/each}
    </section>
  {/if}

  <section class="sec">
    {#if tracks.length}
      <TrackList {tracks} extraMenu={extra} />
    {:else if !active.length}
      <p class="empty">Las canciones que descargues quedan aquí, con portada y licencia, y suenan sin internet.</p>
    {/if}
  </section>
</div>
