<script>
  import Icon from './Icon.svelte'
  import { ui, player } from '../lib/state.svelte.js'
  import { api } from '../lib/api.js'

  const FREQS = ['31', '62', '125', '250', '500', '1k', '2k', '4k', '8k', '16k']
  const PRESETS = [
    ['Plano', [0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0],
    ['Bass boost', [7, 6, 4.5, 2, 0, 0, 0, 0, 0.5, 1], 0],
    ['Graves fuertes', [9, 8, 6, 3, 0, -1, 0, 0, 1, 1.5], 2],
    ['Más volumen', [2, 2, 1.5, 1, 0.5, 0.5, 1, 1.5, 2, 2], 6],
    ['Vocal', [-2, -2, -1, 1, 3, 4, 3.5, 2, 0.5, 0], 0],
    ['Rock', [5, 4, 2.5, -1, -2, -1, 2, 3.5, 4.5, 5], 0],
    ['Pop', [-1, 0.5, 2.5, 4, 4, 2, 0, -0.5, -1, -1], 0],
    ['Reggaetón', [6, 6.5, 4, 1, -1, 0, 1.5, 2.5, 3, 3], 1],
    ['Electrónica', [6, 5, 1.5, 0, -2, 1.5, 0.5, 1.5, 5, 5.5], 0],
    ['Acústico', [3.5, 3, 2.5, 1, 1.5, 1.5, 2.5, 3, 2.5, 1.5], 0],
    ['Noche', [-3, -2.5, -1, 0, 1, 1, 0.5, -1, -2.5, -3.5], -2]
  ]

  let cfg = $state({ enabled: false, gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0], preamp: 0, karaoke: false })
  let timer

  $effect(() => {
    if (!ui.eqOpen) return
    try {
      const s = JSON.parse(ui.settings.eq || '{}')
      cfg = { enabled: !!s.enabled, gains: s.gains?.length === 10 ? [...s.gains] : [0, 0, 0, 0, 0, 0, 0, 0, 0, 0], preamp: s.preamp || 0, karaoke: !!s.karaoke }
    } catch {}
  })

  function save() {
    clearTimeout(timer)
    timer = setTimeout(() => {
      const v = JSON.stringify($state.snapshot(cfg))
      ui.settings.eq = v
      api.setSetting('eq', v)
    }, 60)
  }

  function preset(p) {
    cfg.gains = [...p[1]]
    cfg.preamp = p[2]
    cfg.enabled = true
    save()
  }

  let active = $derived(PRESETS.find((p) => p[1].every((g, i) => Math.abs(g - cfg.gains[i]) < 0.01) && Math.abs(p[2] - cfg.preamp) < 0.01)?.[0])
  let yt = $derived(player.track?.source === 'youtube' && !player.track?.localPath)

  function curve() {
    const w = 600
    const h = 120
    const pts = cfg.gains.map((g, i) => [(i / 9) * w, h / 2 - (g / 12) * (h / 2 - 8)])
    let d = `M${pts[0][0]},${pts[0][1]}`
    for (let i = 1; i < pts.length; i++) {
      const [x0, y0] = pts[i - 1]
      const [x1, y1] = pts[i]
      const mx = (x0 + x1) / 2
      d += ` C${mx},${y0} ${mx},${y1} ${x1},${y1}`
    }
    return d
  }
</script>

{#if ui.eqOpen}
  <div class="dlg-back" onpointerdown={() => (ui.eqOpen = false)} aria-hidden="true"></div>
  <div class="dlg eqd" role="dialog" aria-modal="true" aria-label="Ecualizador" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && (ui.eqOpen = false)}>
    <div class="eq-head">
      <h3>Ecualizador</h3>
      <button class="switch" role="switch" aria-checked={cfg.enabled} aria-label="Activar ecualizador" onclick={() => { cfg.enabled = !cfg.enabled; save() }}><i></i></button>
      <button class="icon-btn sm" onclick={() => (ui.eqOpen = false)} aria-label="Cerrar"><Icon name="close" size={16} /></button>
    </div>
    {#if yt}<p class="eq-note">Esta canción viene de YouTube y ahí el ecualizador no aplica. Funciona con todo lo demás: descargas, música libre, radios y tu carpeta Música.</p>{/if}
    <div class="eq-presets">
      {#each PRESETS as p}
        <button class="chip" class:on={cfg.enabled && active === p[0]} onclick={() => preset(p)}>{p[0]}</button>
      {/each}
    </div>
    <div class="eq-board" class:off={!cfg.enabled}>
      <svg class="eq-curve" viewBox="0 0 600 120" preserveAspectRatio="none" aria-hidden="true">
        <path d="M0,60 H600" class="eq-zero" />
        <path d={curve()} class="eq-line" />
      </svg>
      <div class="eq-bands">
        {#each cfg.gains as g, i}
          <label class="eq-band">
            <span class="mono eq-val">{g > 0 ? '+' : ''}{Math.round(g)}</span>
            <input type="range" min="-12" max="12" step="0.5" value={g} oninput={(e) => { cfg.gains[i] = Number(e.currentTarget.value); cfg.enabled = true; save() }} aria-label={`${FREQS[i]} Hz`} />
            <span class="mono eq-f">{FREQS[i]}</span>
          </label>
        {/each}
      </div>
    </div>
    <label class="eq-pre">
      <span>Volumen extra <b class="mono">{cfg.preamp > 0 ? '+' : ''}{cfg.preamp} dB</b></span>
      <input type="range" class="range" min="-6" max="12" step="0.5" value={cfg.preamp} style={`--p:${((cfg.preamp + 6) / 18) * 100}%`} oninput={(e) => { cfg.preamp = Number(e.currentTarget.value); cfg.enabled = true; save() }} />
    </label>
    <p class="eq-hint">Sube el volumen sin que truene: un limitador suave evita que se sature.</p>
    <div class="set-row">
      <span class="set-name">Modo karaoke · le quita la voz a la canción</span>
      <button class="switch" role="switch" aria-checked={cfg.karaoke} aria-label="Modo karaoke" onclick={() => { cfg.karaoke = !cfg.karaoke; save() }}><i></i></button>
    </div>
  </div>
{/if}
