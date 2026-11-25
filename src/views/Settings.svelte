<script>
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
</script>

<div class="page narrow">
  <div class="sec-head">
    <h1 class="h1">Ajustes</h1>
    <span class="saved mono" class:show={saved}>{saved}</span>
  </div>

  <section class="set">
    <h2 class="label">Fuentes</h2>
    <label class="set-row col">
      <span class="set-name">Client ID de Jamendo</span>
      <input class="field mono" bind:value={jid} oninput={saveJid} placeholder="a1b2c3d4" spellcheck="false" autocomplete="off" />
      <button class="link" type="button" onclick={() => openUrl('https://devportal.jamendo.com/')}>Créalo gratis en devportal.jamendo.com</button>
    </label>
    <div class="set-row">
      <span class="set-name">Internet Archive</span>
      <span class="pill ok">Activo</span>
    </div>
  </section>

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
    <div class="set-row col">
      <span class="set-name">Carpeta de descargas</span>
      <div class="dir">
        <span class="mono">{ui.settings.download_dir_effective}</span>
        <button class="btn ghost" onclick={pickDir}><Icon name="folder" size={16} /> Cambiar</button>
      </div>
    </div>
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
    <p class="about">Yokisfy 0.1.0 · software libre GPL-3.0. Solo usa música con licencia libre o con permiso del artista: Jamendo (Creative Commons), Internet Archive (netlabels y Live Music Archive). Cada descarga guarda su licencia dentro del archivo.</p>
    <button class="link" onclick={() => openUrl('https://github.com/yokyznt/yokisfy')}>github.com/yokyznt/yokisfy</button>
  </section>
</div>
