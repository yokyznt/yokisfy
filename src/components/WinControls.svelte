<script>
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { ui } from '../lib/state.svelte.js'
  import { api } from '../lib/api.js'

  const win = getCurrentWindow()
  let max = $state(false)

  $effect(() => {
    win.isMaximized().then((m) => (max = m))
    const un = win.onResized(() => win.isMaximized().then((m) => (max = m)))
    return () => un.then((f) => f())
  })
</script>

<div class="winbar" class:hide={ui.fpIdle} data-tauri-drag-region>
  <div class="wc">
    <button class="wc-b" onclick={() => api.newWindow()} aria-label="Nueva ventana" title="Abrir otra ventana (misma música)">
      <svg viewBox="0 0 12 12"><rect x="1.5" y="3.5" width="7" height="7" rx="1.5" /><path d="M4 1.5h5a1.5 1.5 0 0 1 1.5 1.5v5" /></svg>
    </button>
    <i class="wc-sep"></i>
    <button class="wc-b" onclick={() => win.minimize()} aria-label="Minimizar" title="Minimizar">
      <svg viewBox="0 0 12 12"><path d="M2.5 6h7" /></svg>
    </button>
    <button class="wc-b" onclick={() => win.toggleMaximize()} aria-label={max ? 'Restaurar' : 'Maximizar'} title={max ? 'Restaurar' : 'Maximizar'}>
      {#if max}<svg viewBox="0 0 12 12"><path d="M4 2.5h5.5V8M2.5 4h5.5v5.5H2.5z" /></svg>{:else}<svg viewBox="0 0 12 12"><rect x="2.5" y="2.5" width="7" height="7" rx="1.5" /></svg>{/if}
    </button>
    <button class="wc-b close" onclick={() => win.close()} aria-label="Cerrar" title="Cerrar">
      <svg viewBox="0 0 12 12"><path d="M3 3l6 6M9 3l-6 6" /></svg>
    </button>
  </div>
</div>
