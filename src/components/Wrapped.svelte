<script>
  import Icon from './Icon.svelte'
  import Cover from './Cover.svelte'
  import { api } from '../lib/api.js'
  import { toast } from '../lib/state.svelte.js'
  import { paletteFor } from '../lib/palette.js'
  import { onMount } from 'svelte'

  let { onclose } = $props()

  const year = new Date().getFullYear()
  let s = $state(null)
  let i = $state(0)
  let cols = $state(['#3b2a6b', '#b3325e', '#f0a23a'])
  let saving = $state(false)

  onMount(async () => {
    const r = await api.stats(365)
    s = r
    const c = r.topArtists[0]?.cover || r.topTracks[0]?.track.cover
    if (c) {
      const p = await paletteFor(c).catch(() => null)
      if (p?.length >= 2) cols = [p[0], p[1], p[2] || p[0]]
    }
  })

  let slides = $derived(s && s.plays ? ['intro', 'minutes', 'artist', 'songs', 'hours', 'card'] : ['empty'])
  let peak = $derived(s ? s.hours.indexOf(Math.max(...s.hours)) : 0)
  let slide = $derived(slides[Math.min(i, slides.length - 1)])

  function when(hr) {
    if (hr < 6) return 'de madrugada'
    if (hr < 12) return 'en la mañana'
    if (hr < 19) return 'en la tarde'
    return 'en la noche'
  }

  function portal(node) {
    document.body.appendChild(node)
    return { destroy: () => node.remove() }
  }

  function span(m) {
    if (m < 1440) return `como ${Math.max(1, Math.round(m / 60))} horas seguidas`
    const d = Math.round((m / 1440) * 10) / 10
    return `${d.toLocaleString('es-MX')} días seguidos`
  }

  function next() {
    if (i < slides.length - 1) i++
  }

  function prev() {
    if (i > 0) i--
  }

  function key(e) {
    if (e.key === 'ArrowRight' || e.key === ' ') next()
    else if (e.key === 'ArrowLeft') prev()
    else if (e.key === 'Escape') onclose()
  }

  async function img(url) {
    if (!url) return null
    try {
      const d = await api.imageData(url)
      return await new Promise((res) => {
        const im = new Image()
        im.onload = () => res(im)
        im.onerror = () => res(null)
        im.src = d
      })
    } catch {
      return null
    }
  }

  function fit(ctx, text, max) {
    let t = text
    while (ctx.measureText(t).width > max && t.length > 1) t = t.slice(0, -1)
    return t === text ? t : t.trimEnd() + '…'
  }

  async function save() {
    saving = true
    try {
      const W = 1080, H = 1920
      const cv = document.createElement('canvas')
      cv.width = W
      cv.height = H
      const ctx = cv.getContext('2d')
      const font = getComputedStyle(document.body).fontFamily
      const g = ctx.createLinearGradient(0, 0, W * 0.4, H)
      g.addColorStop(0, cols[0])
      g.addColorStop(0.55, cols[1])
      g.addColorStop(1, '#0a0a0c')
      ctx.fillStyle = g
      ctx.fillRect(0, 0, W, H)
      ctx.fillStyle = 'rgba(0,0,0,0.28)'
      ctx.fillRect(0, 0, W, H)
      ctx.fillStyle = '#fff'
      ctx.textBaseline = 'alphabetic'
      ctx.font = `800 54px ${font}`
      ctx.fillText(`Mi ${year} en Yokisfy`, 80, 150)
      const a = s.topArtists[0]
      const pic = await img(a?.cover)
      if (pic) {
        ctx.save()
        ctx.beginPath()
        ctx.arc(W / 2, 470, 230, 0, Math.PI * 2)
        ctx.clip()
        const k = Math.max(460 / pic.width, 460 / pic.height)
        ctx.drawImage(pic, W / 2 - (pic.width * k) / 2, 470 - (pic.height * k) / 2, pic.width * k, pic.height * k)
        ctx.restore()
      }
      ctx.textAlign = 'center'
      ctx.font = `600 34px ${font}`
      ctx.fillStyle = 'rgba(255,255,255,0.75)'
      ctx.fillText('Mi artista número 1', W / 2, 790)
      ctx.font = `800 78px ${font}`
      ctx.fillStyle = '#fff'
      ctx.fillText(fit(ctx, a?.name || '', W - 160), W / 2, 880)
      ctx.textAlign = 'left'
      const col = (x, title, items) => {
        ctx.font = `700 34px ${font}`
        ctx.fillStyle = 'rgba(255,255,255,0.7)'
        ctx.fillText(title, x, 1010)
        items.slice(0, 5).forEach((t, n) => {
          ctx.font = `800 40px ${font}`
          ctx.fillStyle = '#fff'
          ctx.fillText(`${n + 1}`, x, 1080 + n * 74)
          ctx.font = `600 38px ${font}`
          ctx.fillText(fit(ctx, t, 400), x + 50, 1080 + n * 74)
        })
      }
      col(80, 'Artistas', s.topArtists.map((x) => x.name))
      col(560, 'Canciones', s.topTracks.map((x) => x.track.title))
      ctx.font = `700 34px ${font}`
      ctx.fillStyle = 'rgba(255,255,255,0.7)'
      ctx.fillText('Minutos escuchados', 80, 1530)
      ctx.fillText('Hora favorita', 560, 1530)
      ctx.font = `800 96px ${font}`
      ctx.fillStyle = '#fff'
      ctx.fillText(s.minutes.toLocaleString('es-MX'), 80, 1630)
      ctx.fillText(`${peak}:00`, 560, 1630)
      ctx.font = `700 36px ${font}`
      ctx.fillStyle = 'rgba(255,255,255,0.85)'
      ctx.fillText('yokisfy · música libre', 80, 1810)
      const p = await api.saveImage(`Mi ${year} en Yokisfy`, cv.toDataURL('image/png'))
      toast(`Imagen guardada en ${p}`)
    } catch (e) {
      toast('No pude guardar la imagen: ' + e)
    }
    saving = false
  }
</script>

<svelte:window onkeydown={key} />

<div class="wr" use:portal style={`--c0:${cols[0]};--c1:${cols[1]};--c2:${cols[2]}`} role="dialog" aria-modal="true" aria-label={`Tu ${year} en Yokisfy`}>
  <div class="wr-bars">
    {#each slides as _, n}<i class:on={n <= i}></i>{/each}
  </div>
  <button class="icon-btn wr-x" onclick={onclose} aria-label="Cerrar"><Icon name="close" size={22} /></button>
  <button class="wr-prev" onclick={prev} aria-label="Anterior"></button>
  <button class="wr-next" onclick={next} aria-label="Siguiente"></button>
  {#if !s}
    <div class="wr-s"><span class="spin"></span></div>
  {:else}
    {#key slide}
      <div class="wr-s">
        {#if slide === 'empty'}
          <h1>Aún no hay suficiente música este año</h1>
          <p>Pon canciones y regresa: aquí verás tu resumen.</p>
        {:else if slide === 'intro'}
          <p class="wr-k">Tu año en música</p>
          <h1 class="wr-huge">{year}</h1>
          <p>Toca a la derecha para avanzar</p>
        {:else if slide === 'minutes'}
          <p class="wr-k">Este año escuchaste</p>
          <h1 class="wr-huge mono">{s.minutes.toLocaleString('es-MX')}</h1>
          <h2>minutos de música</h2>
          <p>Eso es {span(s.minutes)} de música, con {s.uniqueTracks} canciones y {s.uniqueArtists} artistas distintos.</p>
        {:else if slide === 'artist'}
          <p class="wr-k">Tu artista número 1</p>
          <div class="wr-pic"><Cover src={s.topArtists[0].cover} size={240} radius={120} /></div>
          <h1>{s.topArtists[0].name}</h1>
          <p>{s.topArtists[0].plays} reproducciones · {s.topArtists[0].minutes} minutos juntos</p>
        {:else if slide === 'songs'}
          <p class="wr-k">Las que no soltaste</p>
          <ol class="wr-list">
            {#each s.topTracks.slice(0, 5) as t, n}
              <li style={`--d:${n * 90}ms`}><span class="mono">{n + 1}</span><Cover src={t.track.cover} size={56} radius={8} /><span><b>{t.track.title}</b><small>{t.track.artist} · {t.plays} veces</small></span></li>
            {/each}
          </ol>
        {:else if slide === 'hours'}
          <p class="wr-k">Tu hora de música</p>
          <h1 class="wr-huge mono">{peak}:00</h1>
          <p>Escuchas más {when(peak)}.</p>
          <div class="wr-hours">
            {#each s.hours as n, h}<i class:pk={h === peak} style={`--h:${(n / Math.max(1, ...s.hours)) * 100}%`}></i>{/each}
          </div>
        {:else if slide === 'card'}
          <p class="wr-k">Tu resumen</p>
          <div class="wr-card">
            <b>Mi {year} en Yokisfy</b>
            <div class="wr-cols">
              <div><small>Artistas</small>{#each s.topArtists.slice(0, 5) as a, n}<span>{n + 1}. {a.name}</span>{/each}</div>
              <div><small>Canciones</small>{#each s.topTracks.slice(0, 5) as t, n}<span>{n + 1}. {t.track.title}</span>{/each}</div>
            </div>
            <div class="wr-cols"><div><small>Minutos</small><strong class="mono">{s.minutes.toLocaleString('es-MX')}</strong></div><div><small>Hora favorita</small><strong class="mono">{peak}:00</strong></div></div>
          </div>
          <button class="btn primary wr-save" onclick={save} disabled={saving}>{saving ? 'Guardando…' : 'Guardar imagen para compartir'}</button>
        {/if}
      </div>
    {/key}
  {/if}
</div>
