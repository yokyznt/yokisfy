import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ui, toast } from './state.svelte.js'

export async function checkUpdate(manual = false) {
  try {
    const info = await invoke('latest_release')
    if (!info) {
      if (manual) toast('Ya tienes la versión más nueva')
      return
    }
    ui.update = { ...info, state: 'ready', progress: 0 }
  } catch {
    if (manual) toast('No pude revisar si hay versión nueva')
  }
}

export async function installUpdate() {
  const u = ui.update
  if (!u) return
  if (ui.settings.mobile) {
    await openUrl(u.apk || u.page)
    return
  }
  try {
    const { check } = await import('@tauri-apps/plugin-updater')
    const up = await check()
    if (!up) {
      await openUrl(u.page)
      return
    }
    let total = 0
    let got = 0
    ui.update = { ...u, state: 'downloading', progress: 0 }
    await up.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data.contentLength || 0
      else if (e.event === 'Progress') {
        got += e.data.chunkLength
        if (total) ui.update = { ...ui.update, progress: got / total }
      } else if (e.event === 'Finished') ui.update = { ...ui.update, state: 'installing', progress: 1 }
    })
  } catch (e) {
    ui.update = { ...u, state: 'ready' }
    toast(`No se pudo actualizar: ${e}`)
  }
}
