<script>
  import Icon from './Icon.svelte'
  import { ui } from '../lib/state.svelte.js'
  import { api } from '../lib/api.js'
  import { summary, measure, human } from '../lib/quality.js'

  let tick = $state(0)
  let sel = $state(null)
  let remember = $state(false)
  let measuring = $state(false)

  let q = $derived(ui.quality)
  let rows = $derived.by(() => {
    tick
    return q ? summary(q.tracks) : []
  })

  $effect(() => {
    if (!q) return
    sel = null
    remember = false
    measuring = true
    measure(q.tracks).then(() => {
      measuring = false
      tick++
    })
  })

  $effect(() => {
    if (rows.length && !rows.some((r) => r.k === sel)) sel = rows[0].k
  })

  async function done(ok) {
    const p = ui.quality
    ui.quality = null
    if (ok && remember && sel) await api.setSetting('download_format', sel).then(() => (ui.settings.download_format = sel)).catch(() => {})
    p?.resolve(ok ? sel : null)
  }
</script>

{#if q}
  <div class="dlg-back" onpointerdown={() => done(false)} aria-hidden="true"></div>
  <div class="dlg qd" role="dialog" aria-modal="true" aria-label="Calidad de descarga" onkeydown={(e) => e.key === 'Escape' && done(false)} tabindex="-1">
    <h3>{q.tracks.length > 1 ? `Descargar ${q.tracks.length} canciones` : `Descargar «${q.tracks[0].title}»`}</h3>
    <p class="qd-sub">Elige la calidad. {measuring ? 'Midiendo el peso real…' : 'Los pesos con ~ son aproximados.'}</p>
    <div class="qd-list" role="radiogroup">
      {#each rows as r, i (r.k)}
        <button class="qd-row" class:on={sel === r.k} role="radio" aria-checked={sel === r.k} onclick={() => (sel = r.k)} ondblclick={() => { sel = r.k; done(true) }}>
          <span class="qd-dot"></span>
          <span class="qd-main">
            <span class="qd-name">{r.name}{#if i === 0}<em>Mejor disponible</em>{/if}</span>
            <span class="qd-tag mono">{r.tag}</span>
            <span class="qd-desc">{r.desc}</span>
            {#if r.total > 1 && r.have < r.total}<span class="qd-note">{r.have} de {r.total} en esta calidad; las demás en la mejor que tengan</span>{/if}
          </span>
          <span class="qd-size mono">{r.exact ? '' : '~'}{human(r.bytes)}</span>
        </button>
      {/each}
    </div>
    <label class="qd-rem"><input type="checkbox" bind:checked={remember} /> Usar siempre esta calidad sin preguntar</label>
    <div class="dlg-actions">
      <button type="button" class="btn ghost" onclick={() => done(false)}>Cancelar</button>
      <button type="button" class="btn primary" disabled={!sel} onclick={() => done(true)}><Icon name="download" size={16} /> Descargar</button>
    </div>
  </div>
{/if}
