<script>
  import { api } from '../lib/api.js'
  import { player } from '../lib/state.svelte.js'

  let { lyr, time = 0, compact = false } = $props()
  let box = $state()
  let rail = $state()
  let offset = $state(0)

  let lines = $derived(lyr?.lines?.length ? lyr.lines : [])
  let cur = $derived.by(() => {
    let i = -1
    for (let k = 0; k < lines.length; k++) {
      if (lines[k].start <= time + 0.08) i = k
      else break
    }
    return i
  })
  let gap = $derived(cur >= 0 && lines[cur] && time > lines[cur].end + 1.2 && (!lines[cur + 1] || lines[cur + 1].start - time > 2.5))
  let intro = $derived(lines.length && cur < 0 && lines[0].start - time > 2)

  function fill(line, wi) {
    const w = line.words[wi]
    const next = line.words[wi + 1]?.[0] ?? line.end
    const s = w[0]
    if (time >= next) return 100
    if (time <= s) return 0
    return Math.min(100, ((time - s) / Math.max(0.12, next - s)) * 100)
  }

  function emph(line, wi) {
    const w = line.words[wi]
    const next = line.words[wi + 1]?.[0] ?? line.end
    return Math.max(0, Math.min(1, (next - w[0] - 0.25) / 1.1)).toFixed(2)
  }

  $effect(() => {
    const i = cur < 0 ? 0 : cur
    if (!rail || !box) return
    const el = rail.querySelector(`[data-l="${i}"]`)
    if (!el) return
    offset = box.clientHeight * 0.5 - el.offsetTop - el.offsetHeight / 2
  })

  function jump(t) {
    player.position = t
    player.at = performance.now()
    api.seek(Math.max(0, t - 0.05))
  }
</script>

<div class="ly" class:compact bind:this={box}>
  <div class="ly-rail" bind:this={rail} style={`transform:translate3d(0,${offset}px,0)`}>
    {#each lines as line, i (i)}
      <button class="ly-line" class:on={i === cur && !gap} class:past={i < cur || (i === cur && gap)} class:near={Math.abs(i - cur) === 1} data-l={i} style={`--d:${Math.min(4, Math.abs(i - (cur < 0 ? 0 : cur)))}`} onclick={() => jump(line.start)}>
        {#if !line.text.trim()}
          <span class="ly-dots"><i></i><i></i><i></i></span>
        {:else}
          {#each line.words as w, wi}
            {@const p = i === cur ? fill(line, wi) : i < cur ? 100 : 0}
            <span class="ly-w" class:lit={p >= 100} class:ly-cur={p > 0 && p < 100} style={`--p:${p}%;--e:${emph(line, wi)}`}>{w[1]}</span>
          {/each}
        {/if}
      </button>
    {/each}
  </div>
  {#if intro || gap}<div class="ly-wait"><span class="ly-dots big"><i></i><i></i><i></i></span></div>{/if}
</div>
