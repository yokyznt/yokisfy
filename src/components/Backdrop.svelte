<script>
  import { levels, useLevels } from '../lib/levels.js'

  let { cover = null, colors = [], intense = false } = $props()
  let el = $state()

  $effect(() => {
    const stop = useLevels()
    let raf
    const tick = () => {
      const l = levels()
      if (el) {
        el.style.setProperty('--lv', (intense ? l.low : l.rms * 0.6).toFixed(3))
        el.style.setProperty('--beat', l.beat.toFixed(3))
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => {
      cancelAnimationFrame(raf)
      stop()
    }
  })
</script>

<div class="bd" bind:this={el} style={`--c1:${colors[0] || '#7c5cff'};--c2:${colors[1] || '#ff4d8d'};--c3:${colors[2] || '#2bd4c5'};--c4:${colors[3] || '#111'}`} aria-hidden="true">
  {#if cover}<img class="bd-cover" src={cover} alt="" />{/if}
  <i class="bd-blob b1"></i>
  <i class="bd-blob b2"></i>
  <i class="bd-blob b3"></i>
  <i class="bd-shade"></i>
</div>
