<script>
  import Icon from './Icon.svelte'
  import { ui, back, root } from '../lib/state.svelte.js'

  let q = $state(ui.searchCache?.q || '')

  function input() {
    ui.searchQ = q
    if (ui.view.name !== 'search') root('search')
  }

  function focus() {
    if (ui.view.name !== 'search') root('search')
  }
</script>

<header class="top" data-tauri-drag-region>
  <button class="icon-btn round" onclick={back} disabled={!ui.stack.length} aria-label="Atrás"><Icon name="back" size={18} /></button>
  <div class="top-mid" data-tauri-drag-region>
    <button class="top-home" class:on={ui.view.name === 'home'} onclick={() => root('home')} aria-label="Inicio" title="Inicio"><Icon name="home" size={22} /></button>
    <label class="top-search">
      <Icon name="search" size={20} />
      <input bind:value={q} oninput={input} onfocus={focus} placeholder="¿Qué quieres escuchar?" aria-label="Buscar" spellcheck="false" />
      {#if q}<button class="icon-btn sm" onclick={() => { q = ''; input() }} aria-label="Borrar"><Icon name="close" size={16} /></button>{/if}
    </label>
  </div>
</header>
