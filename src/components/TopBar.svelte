<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { ui, back, root, go } from '../lib/state.svelte.js'
  import { api } from '../lib/api.js'

  let q = $state(ui.searchCache?.q || '')
  let open = $state(false)
  let res = $state(null)
  let busy = $state(false)
  let sel = $state(-1)
  let box = $state()
  let timer
  let seq = 0
  const cache = new Map()

  let items = $derived(res ? [...res.tracks.map((t) => ({ k: 't', t })), ...res.artists.map((a) => ({ k: 'a', a }))] : [])

  function fetchQuick(v) {
    clearTimeout(timer)
    const key = v.trim().toLowerCase()
    sel = -1
    if (!key) {
      res = null
      busy = false
      return
    }
    if (cache.has(key)) {
      res = cache.get(key)
      busy = false
      return
    }
    busy = true
    timer = setTimeout(async () => {
      const my = ++seq
      try {
        const r = await api.quickSearch(v.trim())
        cache.set(key, r)
        if (cache.size > 60) cache.delete(cache.keys().next().value)
        if (my === seq) res = r
      } catch {}
      if (my === seq) busy = false
    }, 140)
  }

  function input() {
    open = true
    fetchQuick(q)
  }

  function full() {
    open = false
    ui.searchQ = q
    root('search')
  }

  function pick(it) {
    if (!it) return full()
    open = false
    if (it.k === 't') {
      const list = res.tracks
      api.play(list, list.indexOf(it.t))
    } else go('artist', it.a.id)
  }

  function key(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      open = true
      sel = Math.min(items.length - 1, sel + 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      sel = Math.max(-1, sel - 1)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      sel >= 0 ? pick(items[sel]) : full()
    } else if (e.key === 'Escape') {
      open = false
    }
  }

  function outside(e) {
    if (box && !box.contains(e.target)) open = false
  }
</script>

<svelte:window onpointerdown={outside} />

<header class="top" data-tauri-drag-region>
  <button class="icon-btn round" onclick={back} disabled={!ui.stack.length} aria-label="Atrás"><Icon name="back" size={18} /></button>
  <div class="top-mid" data-tauri-drag-region>
    <button class="top-home" class:on={ui.view.name === 'home'} onclick={() => root('home')} aria-label="Inicio" title="Inicio"><Icon name="home" size={22} /></button>
    <div class="top-wrap" bind:this={box}>
      <label class="top-search">
        <Icon name="search" size={20} />
        <input bind:value={q} oninput={input} onfocus={() => q.trim() && (open = true)} onkeydown={key} placeholder="¿Qué quieres escuchar?" aria-label="Buscar" spellcheck="false" autocomplete="off" role="combobox" aria-expanded={open && !!q.trim()} aria-controls="qs-list" />
        {#if busy}<span class="spin sm"></span>{:else if q}<button class="icon-btn sm" onclick={() => { q = ''; input() }} aria-label="Borrar"><Icon name="close" size={16} /></button>{/if}
      </label>
      {#if open && q.trim()}
        <div class="qs" id="qs-list" role="listbox">
          {#if res}
            {#each res.tracks as t, i (t.id)}
              <button class="qs-row" class:on={sel === i} role="option" aria-selected={sel === i} onpointerenter={() => (sel = i)} onclick={() => pick({ k: 't', t })}>
                <Cover src={t.cover} size={40} radius={6} />
                <span class="qs-t"><b>{t.title}</b><small>{i < res.local ? 'En tu biblioteca · ' : ''}{t.artist}</small></span>
                <Icon name="play" size={16} />
              </button>
            {/each}
            {#each res.artists as a, j (a.id)}
              <button class="qs-row" class:on={sel === res.tracks.length + j} role="option" aria-selected={sel === res.tracks.length + j} onpointerenter={() => (sel = res.tracks.length + j)} onclick={() => pick({ k: 'a', a })}>
                <Cover src={a.cover} size={40} radius={20} />
                <span class="qs-t"><b>{a.name}</b><small>Artista</small></span>
              </button>
            {/each}
            {#if !res.tracks.length && !res.artists.length && !busy}<p class="qs-empty">Nada para “{q.trim()}”</p>{/if}
          {:else}
            <p class="qs-empty"><span class="spin sm"></span> Buscando…</p>
          {/if}
          <button class="qs-all" onclick={full}><Icon name="search" size={16} /> Ver todos los resultados de “{q.trim()}”</button>
        </div>
      {/if}
    </div>
  </div>
</header>
