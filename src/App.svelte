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
  import TopBar from './components/TopBar.svelte'
  import NowPanel from './components/NowPanel.svelte'
  import Home from './views/Home.svelte'
  import Search from './views/Search.svelte'
  import Album from './views/Album.svelte'
  import Collection from './views/Collection.svelte'
  import Library from './views/Library.svelte'
  import Downloads from './views/Downloads.svelte'
  import Settings from './views/Settings.svelte'
  import Pc from './views/Pc.svelte'
  import Stats from './views/Stats.svelte'
  import Party from './views/Party.svelte'
  import Equalizer from './components/Equalizer.svelte'
  import MiniPlayer from './components/MiniPlayer.svelte'
  import { boot, api } from './lib/api.js'
  import { ui, player, go, root, back, setTheme, isLight, look, paintAccent } from './lib/state.svelte.js'
  import { paletteFor } from './lib/palette.js'
  import { newPlaylist, importSpotify } from './lib/actions.js'

  if (typeof window !== 'undefined') {
    window.__ykBack = () => {
      if (ui.menu) return !(ui.menu = null)
      if (ui.eqOpen) return !(ui.eqOpen = false)
      if (ui.fullPlayer) return !(ui.fullPlayer = false)
      if (ui.queueOpen) return !(ui.queueOpen = false)
      if (ui.stack.length) {
        back()
        return true
      }
      if (ui.view.name !== 'home') {
        root('home')
        return true
      }
      return false
    }
  }

  $effect(() => {
    if (!look.dynamic) {
      paintAccent(look.accent || null)
      return
    }
    const c = player.track?.cover
    if (!c) return paintAccent(look.accent || null)
    let alive = true
    paletteFor(c).then((p) => alive && look.dynamic && paintAccent(p[0]))
    return () => (alive = false)
  })

  let ready = $state(false)
  const isMini = new URLSearchParams(location.search).has('mini')
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
    if (name === 'library') return ['library', 'liked', 'recent', 'playlist', 'stats', 'party', 'pc'].includes(v)
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
{#if isMini}
  {#if ready}<MiniPlayer />{/if}
{:else}

<div class="app" class:queue-open={ui.queueOpen} class:now-open={!ui.queueOpen && ui.nowOpen !== false && !!player.track}>
  <TopBar />
  <aside class="side">
    <button class="brand" onclick={() => root('home')} aria-label="Yokisfy, inicio">
      <svg class="logo" viewBox="5 15 54 34" aria-hidden="true">
        <path class="lb" d="M8 36h8l4-9 6 18 6-26 5 17h18M49 30v14" />
      </svg>
      <span>yokisfy</span>
    </button>
    <nav class="side-nav">
      {#each nav as [k, label, icon]}
        <button class="side-link" class:on={isRoot(k)} onclick={() => root(k)} title={label}>
          <Icon name={icon} size={20} /><span>{label}</span>
          {#if k === 'downloads' && activeDownloads}<span class="badge mono">{activeDownloads}</span>{/if}
        </button>
      {/each}
    </nav>
    <div class="side-sec">
      <span class="label">Playlists</span>
      <span class="side-sec-btns">
        <button class="icon-btn sm" onclick={() => importSpotify()} disabled={!!ui.importing} aria-label="Importar playlist de Spotify o YouTube Music" title="Importar playlist de Spotify o YouTube Music"><Icon name="list" size={17} /></button>
        <button class="icon-btn sm" onclick={() => newPlaylist()} aria-label="Nueva playlist" title="Nueva playlist"><Icon name="plus" size={17} /></button>
      </span>
    </div>
    <div class="side-lists">
      <button class="side-pl" class:on={ui.view.name === 'liked'} onclick={() => go('liked')} title="Tus me gusta">
        <span class="pl-art liked"><Icon name="heartFill" size={14} /></span><span class="side-name">Tus me gusta</span>
      </button>
      {#each ui.playlists as p (p.id)}
        <button class="side-pl" class:on={ui.view.name === 'playlist' && ui.view.param === p.id} onclick={() => go('playlist', p.id)} title={p.name}>
          <Cover src={p.cover} size={32} radius={5} /><span class="side-name">{p.name}</span>
        </button>
      {/each}
    </div>
    <div class="side-foot">
      <button class="side-link" class:on={ui.view.name === 'settings'} onclick={() => root('settings')}><Icon name="settings" size={20} /><span>Ajustes</span></button>
      <button class="icon-btn sm" onclick={() => setTheme(isLight(ui.theme) ? 'dark' : 'light')} aria-label={isLight(ui.theme) ? 'Cambiar a tema oscuro' : 'Cambiar a tema claro'} title={isLight(ui.theme) ? 'Cambiar a tema oscuro' : 'Cambiar a tema claro'}>
        <Icon name={isLight(ui.theme) ? 'moon' : 'sun'} size={18} />
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
          {:else if ui.view.name === 'stats'}<Stats />
          {:else if ui.view.name === 'party'}<Party />
          {/if}
        </div>
      {/key}
    {/if}
  </main>

  <Queue />
  {#if !ui.queueOpen && ui.nowOpen !== false}<NowPanel />{/if}
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
<Equalizer />
{/if}
