<script>
  import { outputMenu } from '../lib/actions.js'
  import { getVersion } from '@tauri-apps/api/app'
  import { checkUpdate } from '../lib/update.js'
  import Icon from '../components/Icon.svelte'
  import { api } from '../lib/api.js'
  import { ui, setTheme, toast, THEMES, look, setLook, resetLook, ACCENTS, BACKGROUNDS, FONTS } from '../lib/state.svelte.js'
  import { open } from '@tauri-apps/plugin-dialog'
  import { openUrl } from '@tauri-apps/plugin-opener'

  let jid = $state(ui.settings.jamendo_client_id || '')
  let saved = $state('')
  let timer

  async function save(key, value) {
    await api.setSetting(key, value)
    ui.settings = await api.settings()
    saved = 'Guardado'
    clearTimeout(timer)
    timer = setTimeout(() => (saved = ''), 1600)
  }

  function saveJid() {
    clearTimeout(timer)
    timer = setTimeout(async () => {
      await save('jamendo_client_id', jid.trim())
      ui.homeCache = null
    }, 800)
  }

  async function pickDir() {
    const d = await open({ directory: true, defaultPath: ui.settings.download_dir_effective })
    if (d) {
      await save('download_dir', d)
      toast('Carpeta de descargas cambiada')
    }
  }

  let hq = $derived((ui.settings.stream_quality || 'alta') !== 'normal')
  let fmt = $derived(ui.settings.download_format || 'ask')
  let auto = $derived(ui.settings.autoplay !== 'off')
  let xf = $derived(Number(ui.settings.crossfade ?? 5))
  let share = $derived(ui.settings.share_library !== 'off')
  let autosync = $derived(ui.settings.autosync !== 'off')
  let lastSync = $derived.by(() => {
    try {
      return ui.settings.last_sync ? JSON.parse(ui.settings.last_sync) : null
    } catch {
      return null
    }
  })
  let syncing = $state(false)
  let pcCode = $state('')
  function ago(s) {
    const m = Math.round((Date.now() / 1000 - s) / 60)
    return m < 1 ? 'hace un momento' : m < 60 ? `hace ${m} min` : `hace ${Math.round(m / 60)} h`
  }
  async function syncNow() {
    syncing = true
    try {
      const r = await api.pcSync()
      const n = (r.missing?.length || 0) + (r.theirs || 0)
      toast(`Listo con ${r.name}: ${r.received + r.sent} cambios${n ? `, ${n} descargas en camino` : ''}`)
      ui.settings.last_sync = JSON.stringify({ name: r.name, at: Math.floor(Date.now() / 1000) })
    } catch (e) {
      toast(String(e))
    }
    syncing = false
  }
  let srv = $state({ on: false })
  let srvT
  async function loadSrv() {
    srv = await api.serverInfo()
    clearTimeout(srvT)
    if (srv.on && !srv.public && !ui.settings.mobile && ui.settings.internet !== 'off') srvT = setTimeout(loadSrv, 3000)
  }
  loadSrv()
  $effect(() => () => clearTimeout(srvT))
  async function toggleShare() {
    await save('share_library', share ? 'off' : 'on')
    await loadSrv()
  }
  function copy(v) {
    navigator.clipboard.writeText(v)
    toast('Copiado')
  }
  let version = $state('')
  getVersion().then((v) => (version = v))
  function report() {
    const body = `**Qué pasó:**\n\n**Qué esperabas:**\n\n---\nYokisfy ${version} · ${ui.settings.mobile ? 'Android' : 'Windows'} · ${navigator.userAgent}`
    openUrl(`https://github.com/yokyznt/yokisfy/issues/new?body=${encodeURIComponent(body)}`)
  }
</script>

<div class="page narrow">
  <div class="sec-head">
    <h1 class="h1">Ajustes</h1>
    <span class="saved mono" class:show={saved}>{saved}</span>
  </div>

  <section class="set">
    <h2 class="label">Fuentes</h2>
    {#each [['Audius', true], ['Internet Archive', true], ['Radios del mundo', true], ...(ui.settings.jamendo_available ? [['Jamendo', true]] : [])] as [n, on]}
      <div class="set-row">
        <span class="set-name">{n}</span>
        <span class="pill" class:ok={on}>{on ? 'Activa' : 'No disponible'}</span>
      </div>
    {/each}
    <div class="set-row col">
      <span class="set-name">Desvanecer entre canciones · <span class="mono">{xf ? `${xf} s` : 'apagado'}</span></span>
      <input type="range" class="range" min="0" max="12" step="1" value={xf} style={`--p:${(xf / 12) * 100}%`} oninput={(e) => (ui.settings.crossfade = e.currentTarget.value)} onchange={(e) => save('crossfade', e.currentTarget.value)} aria-label="Segundos de desvanecimiento" />
    </div>
    {#if !ui.settings.mobile}
      <div class="set-row">
        <span class="set-name">Mostrar en Discord lo que escucho<small class="set-net">En tu perfil sale «Escuchando …» con la portada</small></span>
        <button class="switch" role="switch" aria-checked={ui.settings.discord !== 'off'} aria-label="Mostrar en Discord" onclick={() => save('discord', ui.settings.discord === 'off' ? 'on' : 'off')}><i></i></button>
      </div>
      {#if ui.settings.discord !== 'off' && srv.discord === false}
        <div class="set-row">
          <span class="set-name">ID de la app de Discord</span>
          <input class="field mono" style="max-width:220px" value={ui.settings.discord_app_id || ''} placeholder="123456789012345678" onchange={async (e) => { await save('discord_app_id', e.currentTarget.value.trim()); loadSrv() }} aria-label="ID de la app de Discord" />
        </div>
      {/if}
    {/if}
    <div class="set-row">
      <span class="set-name">Volumen parejo entre canciones<small class="set-net">Que todas suenen igual de fuerte, sin brincos</small></span>
      <button class="switch" role="switch" aria-checked={ui.settings.normalize !== 'off'} aria-label="Volumen parejo" onclick={() => save('normalize', ui.settings.normalize === 'off' ? 'on' : 'off')}><i></i></button>
    </div>
    <div class="set-row">
      <span class="set-name">Radio automática al terminar la cola</span>
      <button class="switch" role="switch" aria-checked={auto} aria-label="Radio automática" onclick={() => save('autoplay', auto ? 'off' : 'on')}><i></i></button>
    </div>
    <details class="adv">
      <summary>Avanzado</summary>
      <label class="set-row col">
        <span class="set-name">Client ID propio de Jamendo (opcional)</span>
        <input class="field mono" bind:value={jid} oninput={saveJid} placeholder="Vacío = el de Yokisfy" spellcheck="false" autocomplete="off" />
        <button class="link" type="button" onclick={() => openUrl('https://devportal.jamendo.com/')}>devportal.jamendo.com</button>
      </label>
    </details>
  </section>

  {#if !ui.settings.mobile}
  <section class="set">
    <h2 class="label">Salida de audio</h2>
    <div class="set-row">
      <span class="set-name">Dónde suena la música: audífonos, tele, bocinas</span>
      <button class="btn ghost" onclick={outputMenu}><Icon name="output" size={16} /> Elegir</button>
    </div>
  </section>

  <section class="set">
    <h2 class="label">Atajos de teclado</h2>
    <div class="set-row">
      <span class="set-name">Controlar la música desde cualquier programa o juego</span>
      <button class="switch" role="switch" aria-checked={ui.settings.hotkeys !== 'off'} aria-label="Atajos globales" onclick={() => save('hotkeys', ui.settings.hotkeys === 'off' ? 'on' : 'off')}><i></i></button>
    </div>
    <ul class="keys">
      <li><span><kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Espacio</kbd></span>Pausar / reproducir</li>
      <li><span><kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>→</kbd></span>Siguiente</li>
      <li><span><kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>←</kbd></span>Anterior</li>
      <li><span><kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>↑</kbd> / <kbd>↓</kbd></span>Subir / bajar volumen</li>
      <li><span>Teclas multimedia</span>Funcionan aunque la app esté minimizada</li>
    </ul>
  </section>

  <section class="set">
    <h2 class="label">Tu cuenta Yokisfy</h2>
    <p class="set-help">Tu PC y tu celular se unen solos en el mismo Wi-Fi. Fuera de casa, escribe una vez en el celular el código de la PC. Me gusta, playlists y descargas pasan de uno al otro cada minuto.</p>
    <div class="set-row">
      <span class="set-name">Sincronizar con mis dispositivos{#if lastSync} · <span class="mono">con {lastSync.name}, {ago(lastSync.at)}</span>{/if}</span>
      <button class="switch" role="switch" aria-checked={autosync} aria-label="Sincronizar con mis dispositivos" onclick={() => save('autosync', autosync ? 'off' : 'on')}><i></i></button>
    </div>
    {#if !ui.settings.mobile && srv.on}
      <div class="set-row">
        <span class="set-name">Código para tu celular</span>
        <button class="btn ghost mono" onclick={() => copy(srv.code)} title="Copiar">{srv.code}</button>
      </div>
      <div class="set-row">
        <span class="set-name">Conectar también por internet, aunque no estén en el mismo Wi-Fi
          {#if ui.settings.internet !== 'off'}<small class="set-net" class:ok={srv.public}>{srv.public ? 'Listo: tu celular te encuentra desde cualquier lugar' : srv.tunnel || 'Conectando…'}</small>{/if}
        </span>
        <button class="switch" role="switch" aria-checked={ui.settings.internet !== 'off'} aria-label="Conectar por internet" onclick={() => save('internet', ui.settings.internet === 'off' ? 'on' : 'off')}><i></i></button>
      </div>
    {:else if ui.settings.mobile}
      <form class="set-row pc-code" onsubmit={async (e) => { e.preventDefault(); try { const s = await api.pcLink(pcCode); toast('Conectado con ' + s.name); pcCode = '' } catch (err) { toast(String(err)) } }}>
        <input bind:value={pcCode} placeholder="Código de tu PC (abcde-fghij)" aria-label="Código de tu PC" autocapitalize="off" spellcheck="false" />
        <button class="btn primary" disabled={pcCode.replace(/[^a-z0-9]/gi, '').length < 8}>Conectar</button>
      </form>
    {/if}
    <div class="set-row">
      <span class="set-name">Lo que descargues en uno se descarga también en el otro</span>
      <button class="btn ghost" onclick={syncNow} disabled={syncing}><Icon name="sync" size={16} /> {syncing ? 'Buscando…' : 'Sincronizar ahora'}</button>
    </div>
  </section>

  <section class="set">
    <h2 class="label">Escuchar en el celular</h2>
    <div class="set-row">
      <span class="set-name">Compartir mis descargas y la carpeta Música con el celular</span>
      <button class="switch" role="switch" aria-checked={share} aria-label="Compartir biblioteca" onclick={toggleShare}><i></i></button>
    </div>
    {#if srv.on}
      <p class="set-help">Yokisfy en el celular encuentra esta PC solo. Si usas otra app compatible con Subsonic (Symfonium, Tempo, DSub), usa estos datos:</p>
      {#each [['Servidor', srv.url], ['Usuario', srv.user], ['Contraseña', srv.password]] as [k, v]}
        <div class="set-row">
          <span class="set-name">{k}</span>
          <button class="kv mono" onclick={() => copy(v)} title="Copiar">{v}</button>
        </div>
      {/each}
    {/if}
  </section>

  {/if}

  <section class="set">
    <h2 class="label">Calidad</h2>
    <div class="set-row">
      <span class="set-name">Streaming en la mejor calidad disponible (FLAC)</span>
      <button class="switch" role="switch" aria-checked={hq} aria-label="Streaming en alta calidad" onclick={() => save('stream_quality', hq ? 'normal' : 'alta')}><i></i></button>
    </div>
    <div class="set-row col">
      <span class="set-name">Formato al descargar</span>
      <div class="seg">
        {#each [['ask', 'Preguntar'], ['lossless', 'Sin pérdida'], ['high', 'Alta'], ['normal', 'Ligera']] as [k, l]}
          <button class:on={({ flac: 'lossless', mp3: 'high', ogg: 'high' }[fmt] || fmt) === k} onclick={() => save('download_format', k)}>{l}</button>
        {/each}
      </div>
    </div>
    {#if !ui.settings.mobile}
    <div class="set-row col">
      <span class="set-name">Carpeta de descargas</span>
      <div class="dir">
        <span class="mono">{ui.settings.download_dir_effective}</span>
        <button class="btn ghost" onclick={pickDir}><Icon name="folder" size={16} /> Cambiar</button>
      </div>
    </div>
    {:else}
    <div class="set-row col">
      <span class="set-name">Carpeta de descargas</span>
      <span class="mono set-help">{ui.settings.download_dir_effective}</span>
    </div>
    {/if}
  </section>

  <section class="set">
    <h2 class="label">Apariencia</h2>
    <div class="themes" role="radiogroup" aria-label="Tema">
      {#each THEMES as [id, name, bg, tx, ac]}
        <button class="theme-card" class:on={ui.theme === id} role="radio" aria-checked={ui.theme === id} onclick={() => setTheme(id)}>
          <span class="theme-sw" style={`background:${bg}`}><i style={`background:${tx}`}></i><i style={`background:${ac}`}></i></span>
          <span>{name}</span>
        </button>
      {/each}
    </div>

    <div class="set-row">
      <span class="set-name">Poner de fondo el video de la canción cuando haya (oscurecido)</span>
      <button class="switch" role="switch" aria-checked={ui.settings.yt_video !== 'off'} aria-label="Video de fondo" onclick={() => save('yt_video', ui.settings.yt_video === 'off' ? 'on' : 'off')}><i></i></button>
    </div>

    <h3 class="set-sub">Color de acento</h3>
    <div class="swatches">
      <button class="sw sw-auto" class:on={!look.accent && !look.dynamic} onclick={() => { setLook('dynamic', null); setLook('accent', null) }} aria-label="El del tema" title="El del tema">Tema</button>
      {#each ACCENTS as c}
        <button class="sw" class:on={!look.dynamic && look.accent === c} style={`--c:${c}`} onclick={() => { setLook('dynamic', null); setLook('accent', c) }} aria-label={`Acento ${c}`}></button>
      {/each}
      <label class="sw sw-pick" class:on={!look.dynamic && look.accent && !ACCENTS.includes(look.accent)} title="Otro color" style={`--c:${look.accent || '#888888'}`}>
        <input type="color" value={look.accent || '#7a6cff'} oninput={(e) => { setLook('dynamic', null); setLook('accent', e.currentTarget.value) }} aria-label="Elegir cualquier color de acento" />
      </label>
    </div>
    <div class="set-row">
      <span class="set-name">Que el acento tome el color de la portada de cada canción</span>
      <button class="switch" role="switch" aria-checked={!!look.dynamic} aria-label="Acento de la portada" onclick={() => setLook('dynamic', look.dynamic ? null : true)}><i></i></button>
    </div>

    <h3 class="set-sub">Color de fondo</h3>
    <div class="swatches">
      <button class="sw sw-auto" class:on={!look.bg} onclick={() => setLook('bg', null)} aria-label="El del tema" title="El del tema">Tema</button>
      {#each BACKGROUNDS as c}
        <button class="sw" class:on={look.bg === c} style={`--c:${c}`} onclick={() => setLook('bg', c)} aria-label={`Fondo ${c}`}></button>
      {/each}
      <label class="sw sw-pick" class:on={look.bg && !BACKGROUNDS.includes(look.bg)} title="Otro color" style={`--c:${look.bg || '#222222'}`}>
        <input type="color" value={look.bg || '#101018'} oninput={(e) => setLook('bg', e.currentTarget.value)} aria-label="Elegir cualquier color de fondo" />
      </label>
    </div>

    <h3 class="set-sub">Letra</h3>
    <div class="pt-hosts">
      {#each Object.entries(FONTS) as [k, [name, css]]}
        <button class="chip" class:on={(look.font || 'geist') === k} style={`font-family:${css}`} onclick={() => setLook('font', k === 'geist' ? null : k)}>{name}</button>
      {/each}
    </div>

    <h3 class="set-sub">Tamaño de todo</h3>
    <div class="pt-hosts">
      {#each [[0.9, 'Chico'], [1, 'Normal'], [1.1, 'Grande'], [1.25, 'Muy grande']] as [v, name]}
        <button class="chip" class:on={(look.scale || 1) === v} onclick={() => setLook('scale', v === 1 ? null : v)}>{name}</button>
      {/each}
    </div>

    <div class="set-row">
      <span class="set-name">Volver a como venía</span>
      <button class="btn ghost" onclick={resetLook}>Restablecer</button>
    </div>
  </section>

  <section class="set">
    <h2 class="label">Acerca de</h2>
    <p class="about">Yokisfy {version} · software libre GPL-3.0. Descarga solo música con licencia libre o con permiso del artista (Jamendo, Audius, Internet Archive); lo demás se escucha desde YouTube Music sin descargarse. Cada descarga guarda su licencia dentro del archivo.</p>
    <div class="dir">
      <button class="btn ghost" onclick={() => checkUpdate(true)}>Buscar actualizaciones</button>
      <button class="link" onclick={() => openUrl('https://github.com/yokyznt/yokisfy/releases/latest')}>Página de descargas</button>
      <button class="link" onclick={report}>Reportar un problema</button>
    </div>
  </section>
</div>
