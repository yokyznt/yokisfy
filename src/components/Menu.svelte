<script>
  import Icon from './Icon.svelte'
  import { ui } from '../lib/state.svelte.js'

  let el = $state()
  let pos = $state({ left: 0, top: 0 })

  let items = $derived(ui.menu ? (ui.menu.stack.length ? ui.menu.stack.at(-1).items : ui.menu.items) : [])
  let title = $derived(ui.menu?.stack.length ? ui.menu.stack.at(-1).label : null)

  $effect(() => {
    if (!ui.menu || !el) return
    items
    const w = el.offsetWidth
    const h = el.offsetHeight
    let left = ui.menu.x - w
    let top = ui.menu.y
    if (left < 8) left = 8
    if (left + w > innerWidth - 8) left = innerWidth - w - 8
    if (top + h > innerHeight - 8) top = Math.max(8, ui.menu.y - h - 40)
    pos = { left, top }
  })

  function close() {
    ui.menu = null
  }

  function pick(it) {
    if (it.disabled) return
    if (it.sub) {
      ui.menu.stack = [...ui.menu.stack, { label: it.label, items: it.sub() }]
      return
    }
    close()
    it.action?.()
  }

  function onKey(e) {
    if (e.key === 'Escape' && ui.menu) close()
  }
</script>

<svelte:window onkeydown={onKey} onresize={close} />

{#if ui.menu}
  <div class="menu-back" onpointerdown={close} aria-hidden="true"></div>
  <div class="menu" bind:this={el} style={`left:${pos.left}px;top:${pos.top}px`} role="menu">
    {#if title}
      <button class="menu-item head" onclick={() => (ui.menu.stack = ui.menu.stack.slice(0, -1))}>
        <Icon name="back" size={16} /><span>{title}</span>
      </button>
    {/if}
    {#each items as it}
      <button class="menu-item" class:mono={it.mono} disabled={it.disabled} role="menuitem" onclick={() => pick(it)}>
        {#if it.icon}<Icon name={it.icon} size={16} />{/if}
        <span>{it.label}</span>
        {#if it.sub}<span class="chev"><Icon name="back" size={14} /></span>{/if}
      </button>
    {/each}
    {#if !items.length}<div class="menu-empty">Nada por aquí</div>{/if}
  </div>
{/if}
