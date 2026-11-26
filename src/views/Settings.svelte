<script>
  import { getVersion } from '@tauri-apps/api/app'
  import { checkUpdate } from '../lib/update.js'
  import Icon from '../components/Icon.svelte'
  import { api } from '../lib/api.js'
  import { ui, setTheme, toast } from '../lib/state.svelte.js'
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
  let share = $derived(ui.settings.share_library === 'on')
  let srv = $state({ on: false })
  async function loadSrv() {
    srv = await api.serverInfo()
  }
  loadSrv()
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
    <h2 class="label">Escuchar en el celular</h2>
    <div class="set-row">
      <span class="set-name">Compartir mis descargas y playlists en la red de casa</span>
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
        {#each [['ask', 'Preguntar'], ['flac', 'FLAC'], ['mp3', 'MP3'], ['ogg', 'OGG']] as [k, l]}
          <button class:on={fmt === k} onclick={() => save('download_format', k)}>{l}</button>
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
    <div class="set-row">
      <span class="set-name">Tema oscuro</span>
      <button class="switch" role="switch" aria-checked={ui.theme === 'dark'} aria-label="Tema oscuro" onclick={() => setTheme(ui.theme === 'dark' ? 'light' : 'dark')}><i></i></button>
    </div>
  </section>

  <section class="set">
    <h2 class="label">Acerca de</h2>
    <p class="about">Yokisfy {version} · software libre GPL-3.0. Descarga solo música con licencia libre o con permiso del artista (Jamendo, Audius, Internet Archive); lo demás se escucha desde YouTube Music sin descargarse. Cada descarga guarda su licencia dentro del archivo.</p>
    <div class="dir">
      <button class="btn ghost" onclick={() => checkUpdate(true)}>Buscar actualizaciones</button>
      <button class="link" onclick={() => openUrl('https://github.com/yokyznt/yokisfy/releases/latest')}>Página de descargas</button>
    </div>
  </section>
</div>
