<script>
  import { ui } from '../lib/state.svelte.js'
  let input = $state()
  let value = $state('')

  $effect(() => {
    if (ui.prompt) {
      value = ui.prompt.value
      queueMicrotask(() => {
        input?.focus()
        input?.select()
      })
    }
  })

  function done(ok) {
    const p = ui.prompt
    ui.prompt = null
    p?.resolve(ok ? value : null)
  }
</script>

{#if ui.prompt}
  <div class="dlg-back" onpointerdown={() => done(false)} aria-hidden="true"></div>
  <form class="dlg" onsubmit={(e) => { e.preventDefault(); done(true) }} role="dialog" aria-modal="true" aria-label={ui.prompt.title}>
    <h3>{ui.prompt.title}</h3>
    <input class="field" bind:this={input} bind:value onkeydown={(e) => e.key === 'Escape' && done(false)} />
    <div class="dlg-actions">
      <button type="button" class="btn ghost" onclick={() => done(false)}>Cancelar</button>
      <button type="submit" class="btn primary" disabled={!value.trim()}>{ui.prompt.confirmLabel}</button>
    </div>
  </form>
{/if}

{#if ui.scanning}
  <div class="import-card" role="status" aria-live="polite">
    <span>Buscando versiones gratis para descargar{ui.scanning.total ? ` · ${ui.scanning.done} de ${ui.scanning.total}` : '…'}</span>
    <div class="import-bar"><span style={`width:${ui.scanning.total ? (ui.scanning.done / ui.scanning.total) * 100 : 4}%`}></span></div>
  </div>
{/if}

{#if ui.importing}
  <div class="import-card" role="status" aria-live="polite">
    <span>Importando playlist{ui.importing.total ? ` · ${ui.importing.done} de ${ui.importing.total}` : '…'}</span>
    <div class="import-bar"><span style={`width:${ui.importing.total ? (ui.importing.done / ui.importing.total) * 100 : 4}%`}></span></div>
  </div>
{/if}

{#if ui.toasts.length}
  <div class="toasts" role="status" aria-live="polite">
    {#each ui.toasts as t (t.id)}<div class="toast">{t.text}</div>{/each}
  </div>
{/if}
