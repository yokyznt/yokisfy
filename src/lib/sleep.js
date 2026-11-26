import { api } from './api.js'
import { ui, player, toast } from './state.svelte.js'

let timer = null
let watchId = null

export function sleepItems() {
  const set = (min) => () => setSleep(min)
  return [
    { label: '15 minutos', icon: 'moon', action: set(15) },
    { label: '30 minutos', icon: 'moon', action: set(30) },
    { label: '45 minutos', icon: 'moon', action: set(45) },
    { label: '1 hora', icon: 'moon', action: set(60) },
    { label: 'Al terminar esta canción', icon: 'moon', action: set('track') },
    ui.sleep ? { label: 'Apagar temporizador', icon: 'close', action: () => setSleep(null) } : null
  ].filter(Boolean)
}

function stop() {
  api.setPlaying(false)
  player.playing = false
  ui.sleep = null
  toast('Temporizador: música en pausa')
}

export function setSleep(min) {
  clearTimeout(timer)
  clearInterval(watchId)
  timer = null
  watchId = null
  if (min === null) {
    ui.sleep = null
    toast('Temporizador apagado')
    return
  }
  if (min === 'track') {
    const id = player.track?.id
    ui.sleep = { until: null, label: 'al terminar la canción' }
    watchId = setInterval(() => {
      if (player.track?.id !== id) {
        clearInterval(watchId)
        stop()
      }
    }, 500)
  } else {
    const until = Date.now() + min * 60000
    ui.sleep = { until, label: `${min} min` }
    timer = setTimeout(stop, min * 60000)
  }
  toast(`La música se pausa ${ui.sleep.label === 'al terminar la canción' ? 'al terminar la canción' : `en ${ui.sleep.label}`}`)
}
