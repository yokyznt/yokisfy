<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, toast } from '../lib/state.svelte.js'
  import QRCode from 'qrcode'
  import { together, startTogether, stopTogether } from '../lib/together.svelte.js'
  import { onDestroy } from 'svelte'

  let host = $derived(ui.settings.party === 'on')
  let info = $state(null)
  let qr = $state('')
  let hosts = $state(null)
  let pick = $state(null)
  let q = $state('')
  let results = $state([])
  let busy = $state(false)
  let added = $state({})
  let who = $state(localStorage.getItem('yk-who') || '')
  let code = $state('')
  let joining = $state(false)
  let timer
  let poll

  async function refreshHost() {
    if (ui.settings.mobile) return
    const i = await api.serverInfo()
    if (i?.on) {
      const url = (i.public || i.url).replace(/\/$/, '') + '/fiesta'
      i.party = url
      if (url !== info?.party) qr = await QRCode.toString(url, { type: 'svg', margin: 1, color: { dark: '#000000', light: '#ffffff' } })
    }
    info = i
    clearTimeout(poll)
    if (host && !info?.public) poll = setTimeout(refreshHost, 3000)
  }

  onDestroy(() => clearTimeout(poll))

  async function share() {
    const text = `Únete a mi fiesta en Yokisfy: ${info.party}`
    try {
      if (navigator.share) await navigator.share({ title: 'Fiesta en Yokisfy', text, url: info.party })
      else {
        await navigator.clipboard.writeText(info.party)
        toast('Enlace copiado')
      }
    } catch {}
  }

  async function join() {
    joining = true
    try {
      const s = await api.partyJoin(code)
      hosts = [...(hosts || []).filter((h) => h.url !== s.url), s]
      pick = s
      code = ''
      results = []
    } catch (e) {
      toast(String(e))
    }
    joining = false
  }

  function listen() {
    if (together.on && together.url === pick.url) stopTogether()
    else startTogether(pick.url, pick.name)
  }

  async function toggleHost() {
    await api.setSetting('party', host ? 'off' : 'on')
    ui.settings = await api.settings()
    await refreshHost()
  }

  async function findHosts() {
    hosts = null
    try {
      hosts = await api.partyHosts()
    } catch {
      hosts = []
    }
    if (hosts.length === 1) pick = hosts[0]
  }

  refreshHost()
  findHosts()

  function search() {
    clearTimeout(timer)
    timer = setTimeout(async () => {
      if (!q.trim() || !pick) return (results = [])
      busy = true
      try {
        const r = await api.partyCall(pick.url, 'party_search', { q: q.trim() })
        results = r.tracks || []
      } catch {
        toast('No pude buscar en esa fiesta')
      }
      busy = false
    }, 350)
  }

  async function add(t) {
    localStorage.setItem('yk-who', who)
    try {
      const r = await api.partyCall(pick.url, 'party_add', { track: t, from: who.trim() || 'Alguien' })
      if (r.ok) {
        added[t.id] = true
        toast(`Agregada a la fiesta: ${t.title}`)
      }
    } catch {
      toast('No se pudo agregar')
    }
  }
</script>

<div class="page party">
  <div class="sec-head">
    <h1 class="h1">Fiesta</h1>
  </div>
  <p class="sub">Tus amigos, estén donde estén, escuchan lo mismo que tú al mismo tiempo y le meten canciones a la cola, desde Yokisfy o desde el navegador.</p>

  {#if !ui.settings.mobile}
    <section class="pt-host">
      <div class="set-row">
        <span class="set-name">Abrir mi fiesta en esta PC</span>
        <button class="switch" role="switch" aria-checked={host} aria-label="Abrir fiesta" onclick={toggleHost}><i></i></button>
      </div>
      {#if host && info?.party}
        <div class="pt-share">
          <div class="pt-qr" aria-label="Código QR de la fiesta">{@html qr}</div>
          <div>
            <h2 class="h2">Invita a tus amigos</h2>
            {#if info.public}
              <p>Que escaneen el código o mándales el enlace. Funciona desde cualquier lugar, con datos o en otro Wi‑Fi, y no necesitan instalar nada: escuchan contigo y agregan canciones.</p>
            {:else}
              <p>Por ahora solo funciona en tu Wi‑Fi. <span class="spin"></span> {info.tunnel || 'Conectando por internet…'}</p>
            {/if}
            <p class="mono pt-url">{info.party}</p>
            <div class="pt-act">
              <button class="btn primary" onclick={share}>{navigator.share ? 'Compartir enlace' : 'Copiar enlace'}</button>
              {#if info.party_code}<span class="pt-code">Código para Yokisfy: <b class="mono">{info.party_code.slice(0, 3)}-{info.party_code.slice(3)}</b></span>{/if}
            </div>
            <p class="sub">Cuando alguien agregue una canción te sale un aviso.</p>
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <section class="pt-guest">
    <div class="sec-head">
      <h2 class="h2">Unirme a una fiesta</h2>
      <button class="btn ghost" onclick={findHosts}><Icon name="repeat" size={16} /> Buscar</button>
    </div>
    <form class="pt-join" onsubmit={(e) => { e.preventDefault(); join() }}>
      <input class="field mono" bind:value={code} placeholder="Código de la fiesta (abc-def)" aria-label="Código de la fiesta" autocapitalize="off" spellcheck="false" />
      <button class="btn primary" disabled={joining || code.replace(/[^a-z0-9]/gi, '').length < 6}>{joining ? 'Buscando…' : 'Unirme'}</button>
    </form>
    {#if hosts === null}
      <p class="empty"><span class="spin"></span> Buscando fiestas en tu Wi‑Fi…</p>
    {:else if !hosts.length}
      <p class="empty">No hay fiestas en tu Wi‑Fi. Si es a distancia, escribe arriba el código que te pasaron.</p>
    {:else}
      <div class="pt-hosts">
        {#each hosts as h}
          <button class="chip" class:on={pick?.url === h.url} onclick={() => { pick = h; results = []; search() }}><Icon name="party" size={15} /> {h.name}</button>
        {/each}
      </div>
    {/if}
    {#if pick}
      <div class="set-row pt-listen">
        <span class="set-name">Escuchar aquí lo mismo que suena en la fiesta de {pick.name}
          {#if together.on && together.url === pick.url}<small class="sub">Sincronizado{Math.abs(together.drift) > 1.8 ? ' · ajustando…' : ''}</small>{/if}
        </span>
        <button class="switch" role="switch" aria-checked={together.on && together.url === pick.url} aria-label="Escuchar juntos" onclick={listen}><i></i></button>
      </div>
      <div class="pt-form">
        <input class="field" bind:value={who} placeholder="Tu nombre" maxlength="24" />
        <div class="search-bar small"><Icon name="search" size={18} /><input bind:value={q} oninput={search} placeholder={`Busca para la fiesta de ${pick.name}`} aria-label="Buscar canción" /></div>
      </div>
      {#if busy}<p class="empty"><span class="spin"></span></p>{/if}
      <ul class="pt-res">
        {#each results as t (t.id)}
          <li>
            <Cover src={t.cover} size={44} radius={6} />
            <span class="st-t"><b>{t.title}</b><span>{t.artist}</span></span>
            <button class="btn sm" class:primary={!added[t.id]} disabled={added[t.id]} onclick={() => add(t)}>{added[t.id] ? 'En la cola' : 'Agregar'}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
