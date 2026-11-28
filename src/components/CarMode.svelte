<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, player, fmt, toast } from '../lib/state.svelte.js'
  import { like } from '../lib/actions.js'
  import { runVoice } from '../lib/voice.js'
  import { listen } from '@tauri-apps/api/event'

  let hearing = $state(false)
  const cars = () => JSON.parse(localStorage.getItem('yk-cars') || '[]')
  const nope = () => JSON.parse(localStorage.getItem('yk-not-cars') || '[]')

  listen('bt', (e) => {
    const { name, on } = e.payload || {}
    if (!name) return
    if (on) {
      if (cars().includes(name)) {
        ui.car = true
        ui.carAuto = name
      } else if (!nope().includes(name)) ui.carAsk = name
    } else {
      if (ui.carAsk === name) ui.carAsk = null
      if (ui.carAuto === name) {
        ui.car = false
        ui.carAuto = null
      }
    }
  })

  listen('voice', (e) => runVoice(e.payload?.text))

  function answer(yes) {
    const n = ui.carAsk
    ui.carAsk = null
    if (!n) return
    if (yes) {
      localStorage.setItem('yk-cars', JSON.stringify([...new Set([...cars(), n])]))
      ui.car = true
      ui.carAuto = n
    } else localStorage.setItem('yk-not-cars', JSON.stringify([...new Set([...nope(), n])]))
  }

  async function mic() {
    hearing = true
    try {
      const txt = await api.speech()
      if (txt) await runVoice(txt)
    } catch (e) {
      toast(String(e))
    }
    hearing = false
  }

  let t = $derived(player.track)
  let now = $state(Date.now())
  let x0 = null
  let dx = $state(0)
  let lock = null

  let clock = $derived(new Date(now).toLocaleTimeString('es-MX', { hour: 'numeric', minute: '2-digit' }))
  let pct = $derived.by(() => {
    now
    let p = player.position
    if (player.playing && !player.loading && player.at) p += (performance.now() - player.at) / 1000
    return player.duration ? Math.min(100, (p / player.duration) * 100) : 0
  })

  $effect(() => {
    if (!ui.car) return
    const id = setInterval(() => (now = Date.now()), 1000)
    const grab = async () => {
      try {
        lock = await navigator.wakeLock?.request('screen')
      } catch {}
    }
    grab()
    const vis = () => document.visibilityState === 'visible' && grab()
    document.addEventListener('visibilitychange', vis)
    return () => {
      clearInterval(id)
      document.removeEventListener('visibilitychange', vis)
      lock?.release?.().catch(() => {})
      lock = null
    }
  })

  function down(e) {
    x0 = e.clientX
  }

  function move(e) {
    if (x0 != null) dx = e.clientX - x0
  }

  function up() {
    if (x0 == null) return
    const d = dx
    x0 = null
    dx = 0
    if (d < -80) api.next()
    else if (d > 80) api.prev()
  }
</script>

{#if ui.carAsk && !ui.car}
  <div class="car-ask" role="dialog" aria-label="Modo coche">
    <Icon name="car" size={26} />
    <p>Te conectaste a <b>{ui.carAsk}</b>. ¿Es el Bluetooth de tu carro?</p>
    <div>
      <button class="btn primary" onclick={() => answer(true)}>Sí, abrir modo coche</button>
      <button class="btn ghost" onclick={() => answer(false)}>No</button>
    </div>
  </div>
{/if}

{#if ui.car}
  <section class="car" aria-label="Modo coche">
    <header class="car-top">
      <span class="car-clock mono">{clock}</span>
      <button class="car-exit" onclick={() => { ui.car = false; ui.carAuto = null }}><Icon name="close" size={22} /> Salir</button>
    </header>

    {#if t}
      <div class="car-main" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} role="presentation">
        <div class="car-art" style={`transform: translateX(${dx * 0.6}px) rotate(${dx * 0.02}deg)`}>
          <Cover src={t.cover} size={520} radius={18} />
        </div>
        <div class="car-info">
          <strong>{t.title}</strong>
          <span>{t.artist}</span>
          <div class="car-bar"><i style={`width:${pct}%`}></i></div>
          <small class="car-hint">Desliza la portada para cambiar de canción · o di «Ok Google, pon Bad Bunny en Yokisfy»</small>
        </div>
      </div>

      <div class="car-ctl">
        <button class="car-btn" class:on={ui.liked[t.id]} onclick={() => like(t)} aria-label="Me gusta"><Icon name={ui.liked[t.id] ? 'heartFill' : 'heart'} size={34} /></button>
        <button class="car-btn" onclick={() => api.prev()} aria-label="Anterior"><Icon name="prev" size={44} /></button>
        <button class="car-btn car-play" onclick={() => api.toggle()} aria-label={player.playing ? 'Pausar' : 'Reproducir'}>
          {#if player.loading}<span class="spin dark"></span>{:else}<Icon name={player.playing ? 'pause' : 'play'} size={52} />{/if}
        </button>
        <button class="car-btn" onclick={() => api.next()} aria-label="Siguiente"><Icon name="next" size={44} /></button>
        <button class="car-btn" onclick={() => { ui.car = false; ui.queueOpen = true }} aria-label="Cola"><Icon name="queue" size={34} /></button>
      </div>
      {#if ui.settings.mobile}
        <button class="car-mic" class:on={hearing} onclick={mic} aria-label="Hablar"><Icon name="mic" size={30} /> {hearing ? 'Te escucho…' : 'Di qué poner: «pon a Bad Bunny», «siguiente», «pausa»'}</button>
      {/if}
    {:else}
      <p class="car-empty">Pon algo de música para empezar.</p>
    {/if}
  </section>
{/if}
