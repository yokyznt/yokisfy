<script>
  import Icon from './Icon.svelte'
  import { api } from '../lib/api.js'
  import { player } from '../lib/state.svelte.js'

  let { big = false } = $props()

  let drag = $state(null)
  let boost = $state(Number(localStorage.getItem('yk-boost') || 0))
  let lastVol = 0.8
  let kick = $state(0)
  let push = $state(0)
  let hit = $state(false)
  let hitTimer
  let sess = null
  let tries = 0
  const MAX = 18
  const T = 0.24

  api.getBoost().then((db) => (boost = db)).catch(() => {})

  function tellYt(d) {
    document.querySelector('.yt iframe')?.contentWindow?.postMessage(JSON.stringify({ ykBoost: d }), '*')
  }

  let val = $derived(drag ?? (player.volume >= 0.999 && boost > 0 ? 1 + (boost / MAX) * 0.5 : player.volume))
  let over = $derived(val > 1.001)
  let db = $derived(over ? Math.round(((val - 1) / 0.5) * MAX) : 0)

  function apply(v) {
    if (v <= 1) {
      api.volume(v)
      if (boost) setBoost(0)
    } else {
      if (player.volume < 1) api.volume(1)
      setBoost(Math.round(((v - 1) / 0.5) * MAX))
    }
  }

  function setBoost(d) {
    if (d === boost) return
    if (d > boost) kick++
    boost = d
    localStorage.setItem('yk-boost', String(d))
    api.boost(d)
    tellYt(d)
  }

  function stuck() {
    push++
    hit = true
    clearTimeout(hitTimer)
    hitTimer = setTimeout(() => (hit = false), 450)
    navigator.vibrate?.(10)
  }

  function input(e) {
    const raw = Number(e.currentTarget.value)
    if (!sess) sess = { open: over }
    let v = raw
    if (!sess.open && raw > 1) {
      if (raw < 1 + T) {
        v = 1
        if (drag == null || drag < 1 || push % 4 === 0) stuck()
      } else v = 1 + ((raw - 1 - T) * 0.5) / (0.5 - T)
    }
    v = Math.round(v * 100) / 100
    drag = v
    e.currentTarget.value = String(v)
    apply(v)
  }

  function commit() {
    if (drag != null) player.volume = Math.min(drag, 1)
    drag = null
    sess = null
  }

  function key(e) {
    const upKey = ['ArrowRight', 'ArrowUp', 'PageUp'].includes(e.key)
    const downKey = ['ArrowLeft', 'ArrowDown', 'PageDown'].includes(e.key)
    if (over && (upKey || downKey)) {
      e.preventDefault()
      const d = Math.max(0, Math.min(MAX, boost + (upKey ? 1 : -1)))
      if (d === 0) {
        setBoost(0)
        api.volume(1)
      } else setBoost(d)
      return
    }
    if (!upKey) return
    if (val < 0.995) return (tries = 0)
    e.preventDefault()
    stuck()
    if (++tries < 4) return
    tries = 0
    apply(1.03)
  }

  function mute() {
    if (player.volume > 0) {
      lastVol = player.volume
      player.volume = 0
      api.volume(0)
      if (boost) setBoost(0)
    } else {
      player.volume = lastVol || 0.8
      api.volume(player.volume)
    }
  }
</script>

<div class="vol-wrap" class:big class:over>
  <button class="icon-btn sm vol-ic" onclick={mute} aria-label={val > 0 ? 'Silenciar' : 'Activar sonido'}>
    <Icon name={val > 0 ? 'volume' : 'mute'} size={18} />
    {#if over}
      {#key kick}<span class="vol-wave" style={`--k:${db / MAX}`}></span><span class="vol-wave w2" style={`--k:${db / MAX}`}></span>{/key}
    {/if}
  </button>
  <div class="vol-track">
    <input type="range" class="range vol vol-boost" min="0" max="1.5" step="0.01" value={val} style={`--p:${(Math.min(val, 1) / 1.5) * 100}%;--b:${(val / 1.5) * 100}%`} oninput={input} onchange={commit} onpointerdown={() => (sess = { open: over })} onpointerup={() => (sess = null)} onkeydown={key} aria-label="Volumen" aria-valuetext={over ? `100% más ${db} dB de amplificador` : `${Math.round(val * 100)}%`} />
    <span class="vol-mark" title="Más allá de aquí se amplifica"></span>
    {#if !over}
      {#key push}<span class="vol-lock" class:hit><Icon name="lock" size={10} /></span>{/key}
    {/if}
  </div>
</div>
