<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import { api } from '../lib/api.js'
  import { go } from '../lib/state.svelte.js'

  const RANGES = [
    [7, 'Semana'],
    [30, 'Mes'],
    [365, 'Año'],
    [0, 'Siempre']
  ]
  let days = $state(30)
  let s = $state(null)

  $effect(() => {
    const d = days
    s = null
    api.stats(d).then((r) => {
      if (days === d) s = r
    })
  })

  let hours = $derived(s ? Math.max(1, ...s.hours) : 1)
  let dayMax = $derived(s ? Math.max(1, ...s.perDay.map((d) => d.minutes)) : 1)
  let peak = $derived(s ? s.hours.indexOf(Math.max(...s.hours)) : 0)
  let label = $derived(RANGES.find((r) => r[0] === days)?.[1].toLowerCase())

  function h(n) {
    if (n < 60) return `${n} min`
    const hh = Math.floor(n / 60)
    return `${hh} h ${n % 60 ? `${Math.round(n % 60)} min` : ''}`.trim()
  }

  function when(hr) {
    if (hr < 6) return 'de madrugada'
    if (hr < 12) return 'en la mañana'
    if (hr < 19) return 'en la tarde'
    return 'en la noche'
  }
</script>

<div class="page stats">
  <div class="sec-head">
    <h1 class="h1">Tus estadísticas</h1>
  </div>
  <div class="seg">
    {#each RANGES as [d, l]}<button class:on={days === d} onclick={() => (days = d)}>{l}</button>{/each}
  </div>

  {#if !s}
    <p class="empty"><span class="spin"></span> Contando…</p>
  {:else if !s.plays}
    <div class="empty-box"><Icon name="stats" size={28} /><p>Todavía no hay nada que contar en este periodo. Pon música y regresa.</p></div>
  {:else}
    <div class="st-hero">
      <div class="st-big"><b class="mono">{h(s.minutes)}</b><span>escuchando música {label === 'siempre' ? 'desde que usas Yokisfy' : `este ${label === 'semana' ? 'semana' : label}`}</span></div>
      <div class="st-kpis">
        <div><b class="mono">{s.plays}</b><span>reproducciones</span></div>
        <div><b class="mono">{s.uniqueTracks}</b><span>canciones distintas</span></div>
        <div><b class="mono">{s.uniqueArtists}</b><span>artistas</span></div>
      </div>
    </div>

    {#if s.topArtists[0]}
      <button class="st-top" onclick={() => go('artist', s.topArtists[0].name)}>
        <Cover src={s.topArtists[0].cover} size={120} radius={60} />
        <div>
          <span class="label">Tu artista número 1</span>
          <h2>{s.topArtists[0].name}</h2>
          <p>{s.topArtists[0].plays} reproducciones · {h(s.topArtists[0].minutes)}</p>
        </div>
      </button>
    {/if}

    <div class="st-grid">
      <section>
        <h2 class="h2">Artistas que más escuchas</h2>
        <ol class="st-list">
          {#each s.topArtists as a, i}
            <li><button onclick={() => go('artist', a.name)}><span class="mono st-n">{i + 1}</span><Cover src={a.cover} size={40} radius={20} /><span class="st-t"><b>{a.name}</b><span>{a.plays} veces · {h(a.minutes)}</span></span></button></li>
          {/each}
        </ol>
      </section>
      <section>
        <h2 class="h2">Canciones que más pones</h2>
        <ol class="st-list">
          {#each s.topTracks as t, i}
            <li><button onclick={() => api.play(s.topTracks.map((x) => x.track), i)}><span class="mono st-n">{i + 1}</span><Cover src={t.track.cover} size={40} radius={6} /><span class="st-t"><b>{t.track.title}</b><span>{t.track.artist} · {t.plays} veces</span></span></button></li>
          {/each}
        </ol>
      </section>
    </div>

    <section>
      <h2 class="h2">Minutos por día</h2>
      <div class="st-bars">
        {#each s.perDay as d}
          <i style={`--h:${(d.minutes / dayMax) * 100}%`} title={`${new Date(d.day * 86400000).toLocaleDateString('es-MX', { day: 'numeric', month: 'short', timeZone: 'UTC' })}: ${d.minutes} min`}></i>
        {/each}
      </div>
    </section>

    <section>
      <h2 class="h2">A qué hora escuchas</h2>
      <p class="sub">Escuchas más {when(peak)}, sobre todo a las {peak}:00.</p>
      <div class="st-bars hours">
        {#each s.hours as n, i}
          <i class:pk={i === peak} style={`--h:${(n / hours) * 100}%`} title={`${i}:00 · ${n}`}></i>
        {/each}
      </div>
      <div class="st-axis mono"><span>0</span><span>6</span><span>12</span><span>18</span><span>23</span></div>
    </section>
  {/if}
</div>
