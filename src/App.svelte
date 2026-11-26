<script>
  import Icon from './components/Icon.svelte'
  import Cover from './components/Cover.svelte'
  import PlayerBar from './components/PlayerBar.svelte'
  import Queue from './components/Queue.svelte'
  import FullPlayer from './components/FullPlayer.svelte'
  import Menu from './components/Menu.svelte'
  import Overlays from './components/Overlays.svelte'
  import YtPlayer from './components/YtPlayer.svelte'
  import Artist from './views/Artist.svelte'
  import WinControls from './components/WinControls.svelte'
  import Home from './views/Home.svelte'
  import Search from './views/Search.svelte'
  import Album from './views/Album.svelte'
  import Collection from './views/Collection.svelte'
  import Library from './views/Library.svelte'
  import Downloads from './views/Downloads.svelte'
  import Settings from './views/Settings.svelte'
  import Pc from './views/Pc.svelte'
  import { boot, api } from './lib/api.js'
  import { ui, player, go, root, back, setTheme } from './lib/state.svelte.js'
  import { newPlaylist, importSpotify } from './lib/actions.js'

  let ready = $state(false)
  const isMainWin = (() => {
    try {
      return (window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label || 'main') === 'main'
    } catch {
      return true
    }
  })()
  $effect(() => {
    boot().finally(() => (ready = true))
  })

  const nav = [
    ['home', 'Inicio', 'home'],
    ['search', 'Buscar', 'search'],
    ['library', 'Biblioteca', 'library'],
    ['downloads', 'Descargas', 'download']
  ]

  let activeDownloads = $derived(Object.values(ui.downloads).filter((d) => d.state === 'downloading' || d.state === 'queued').length)

  function isRoot(name) {
    const v = ui.view.name
    if (name === 'library') return ['library', 'liked', 'recent', 'playlist'].includes(v)
    return v === name
  }

  function onKey(e) {
    const tag = e.target?.tagName
    if (tag === 'INPUT' || tag === 'TEXTAREA' || ui.prompt) return
    if (e.code === 'Space') {
      e.preventDefault()
      if (player.track) api.toggle()
    } else if (e.key === '/' || (e.ctrlKey && e.key.toLowerCase() === 'k')) {
      e.preventDefault()
      root('search')
    } else if (e.altKey && e.key === 'ArrowLeft') {
      back()
    } else if (e.ctrlKey && e.key === 'ArrowRight') {
      api.next()
    } else if (e.ctrlKey && e.key === 'ArrowLeft') {
      api.prev()
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app" class:queue-open={ui.queueOpen}>
  <aside class="side">
    <button class="brand" onclick={() => root('home')} aria-label="Yokisfy, inicio">
      <svg class="logo" viewBox="0 0 32 32" aria-hidden="true">
        <rect x="1" y="1" width="30" height="30" rx="9" fill="currentColor" />
        <rect x="8" y="13" width="3.2" height="10" rx="1.6" class="lb" />
        <rect x="14.4" y="8" width="3.2" height="15" rx="1.6" class="lb" />
        <rect x="20.8" y="11" width="3.2" height="12" rx="1.6" class="lb ac" />
      </svg>
      <span>yokisfy</span>
    </button>
    <nav class="side-nav">
      {#each nav as [k, label, icon]}
        <button class="side-link" class:on={isRoot(k)} onclick={() => root(k)}>
          <Icon name={icon} size={20} /><span>{label}</span>
          {#if k === 'downloads' && activeDownloads}<span class="badge mono">{activeDownloads}</span>{/if}
        </button>
      {/each}
    </nav>
    <div class="side-sec">
      <span class="label">Playlists</span>
      <span class="side-sec-btns">
        <button class="icon-btn sm" onclick={() => importSpotify()} disabled={!!ui.importing} aria-label="Importar playlist de Spotify o YouTube Music" title="Importar playlist de Spotify o YouTube Music"><Icon name="download" size={17} /></button>
        <button class="icon-btn sm" onclick={() => newPlaylist()} aria-label="Nueva playlist" title="Nueva playlist"><Icon name="plus" size={17} /></button>
      </span>
    </div>
    <div class="side-lists">
      <button class="side-pl" class:on={ui.view.name === 'liked'} onclick={() => go('liked')}>
        <span class="pl-art liked"><Icon name="heartFill" size={14} /></span><span>Tus me gusta</span>
      </button>
      {#each ui.playlists as p (p.id)}
        <button class="side-pl" class:on={ui.view.name === 'playlist' && ui.view.param === p.id} onclick={() => go('playlist', p.id)}>
          <Cover src={p.cover} size={32} radius={5} /><span>{p.name}</span>
        </button>
      {/each}
    </div>
    <div class="side-foot">
      <button class="side-link" class:on={ui.view.name === 'settings'} onclick={() => root('settings')}><Icon name="settings" size={20} /><span>Ajustes</span></button>
      <button class="icon-btn sm" onclick={() => setTheme(ui.theme === 'dark' ? 'light' : 'dark')} aria-label={ui.theme === 'dark' ? 'Cambiar a tema claro' : 'Cambiar a tema oscuro'} title={ui.theme === 'dark' ? 'Cambiar a tema claro' : 'Cambiar a tema oscuro'}>
        <Icon name={ui.theme === 'dark' ? 'sun' : 'moon'} size={18} />
      </button>
    </div>
  </aside>

  <main class="view">
    <div class="topbar" data-tauri-drag-region>
      <button class="icon-btn round" onclick={back} disabled={!ui.stack.length} aria-label="Atrás"><Icon name="back" size={18} /></button>
      <button class="icon-btn round mob-only" onclick={() => root('settings')} aria-label="Ajustes"><Icon name="settings" size={18} /></button>
    </div>
    {#if ready}
      {#key ui.view.name + ':' + ui.view.param}
        <div class="view-in">
          {#if ui.view.name === 'home'}<Home />
          {:else if ui.view.name === 'search'}<Search />
          {:else if ui.view.name === 'album'}<Album id={ui.view.param} />
          {:else if ui.view.name === 'artist'}<Artist id={ui.view.param} />
          {:else if ui.view.name === 'library'}<Library />
          {:else if ui.view.name === 'liked'}<Collection kind="liked" />
          {:else if ui.view.name === 'recent'}<Collection kind="recent" />
          {:else if ui.view.name === 'playlist'}<Collection kind="playlist" id={ui.view.param} />
          {:else if ui.view.name === 'downloads'}<Downloads />
          {:else if ui.view.name === 'settings'}<Settings />
          {:else if ui.view.name === 'pc'}<Pc />
          {/if}
        </div>
      {/key}
    {/if}
  </main>

  <Queue />
  <PlayerBar />

  <nav class="tabbar">
    {#each nav as [k, label, icon]}
      <button class:on={isRoot(k)} onclick={() => root(k)}>
        <Icon name={icon} size={22} /><span>{label}</span>
        {#if k === 'downloads' && activeDownloads}<span class="badge mono">{activeDownloads}</span>{/if}
      </button>
    {/each}
  </nav>
</div>

<FullPlayer />
<Menu />
<Overlays />
{#if isMainWin}<YtPlayer />{/if}
{#if ready && !ui.settings.mobile}<WinControls />{/if}
