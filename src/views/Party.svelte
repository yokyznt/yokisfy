<script>
  import Icon from '../components/Icon.svelte'
  import Cover from '../components/Cover.svelte'
  import { api } from '../lib/api.js'
  import { ui, toast } from '../lib/state.svelte.js'
  import QRCode from 'qrcode'

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
  let timer

  async function refreshHost() {
    if (ui.settings.mobile) return
    info = await api.serverInfo()
    if (info?.on) {
      const url = info.url.replace(/\/$/, '') + '/fiesta'
      info.party = url
      qr = await QRCode.toString(url, { type: 'svg', margin: 1, color: { dark: '#000000', light: '#ffffff' } })
    }
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
  <p class="sub">Tus amigos en el mismo Wi‑Fi le meten canciones a tu cola, desde Yokisfy o desde el navegador de su cel.</p>

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
            <h2 class="h2">Que escaneen esto</h2>
            <p>Con la cámara del cel abren la fiesta, buscan y agregan. No necesitan instalar nada.</p>
            <p class="mono pt-url">{info.party}</p>
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
    {#if hosts === null}
      <p class="empty"><span class="spin"></span> Buscando fiestas en tu Wi‑Fi…</p>
    {:else if !hosts.length}
      <p class="empty">No hay fiestas abiertas en tu red ahorita.</p>
    {:else}
      <div class="pt-hosts">
        {#each hosts as h}
          <button class="chip" class:on={pick?.url === h.url} onclick={() => { pick = h; results = []; search() }}><Icon name="party" size={15} /> {h.name}</button>
        {/each}
      </div>
    {/if}
    {#if pick}
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
