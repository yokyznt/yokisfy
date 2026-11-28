<script>
  import { levels, useLevels } from '../lib/levels.js'

  let { cover = null, colors = [] } = $props()
  let el = $state()

  $effect(() => {
    const stop = useLevels()
    let raf
    let lv = 0
    const tick = () => {
      const l = levels()
      lv += ((l.low * 0.7 + l.rms * 0.3) - lv) * 0.12
      if (el) {
        el.style.setProperty('--lv', lv.toFixed(3))
        el.style.setProperty('--bt', l.beat.toFixed(3))
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

<div class="cf" bind:this={el} style={`--c1:${colors[0] || '#333'};--c4:${colors[3] || '#111'}`} aria-hidden="true">
  {#if cover}
    {#key cover}
      <div class="cf-in">
        <img class="cf-base" src={cover} alt="" />
        <img class="cf-l l1" src={cover} alt="" />
        <img class="cf-l l2" src={cover} alt="" />
        <img class="cf-l l3" src={cover} alt="" />
        <img class="cf-l l4" src={cover} alt="" />
      </div>
    {/key}
  {/if}
  <i class="cf-shade"></i>
</div>
