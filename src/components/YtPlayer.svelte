<script>
  import { listen } from '@tauri-apps/api/event'
  import { api } from '../lib/api.js'
  import { player, toast } from '../lib/state.svelte.js'

  let frame = $state()
  let ready = $state(false)
  let firstId = $state(null)
  let alt = 0
  let loadedId = null
  let lastState = -1
  let lastSent = 0
  let trackId = null
  let hiddenAt = 0
  let lastPos = 0
  let lastVid = null

  let active = $derived(!!player.track && player.track.source === 'youtube' && player.external)
  let ids = $derived(active ? (player.track.formats || []).filter((f) => f.kind === 'yt').map((f) => f.url) : [])
  let src = $derived(firstId ? `https://www.youtube-nocookie.com/embed/${firstId}?enablejsapi=1&autoplay=1&playsinline=1&rel=0&iv_load_policy=3&origin=${encodeURIComponent(location.origin)}` : '')

  $effect(() => {
    if (active && !firstId && ids[0]) {
      firstId = ids[0]
      loadedId = ids[0]
      trackId = player.track.id
    }
  })

  function cmd(func, args = []) {
    frame?.contentWindow?.postMessage(JSON.stringify({ event: 'command', func, args }), '*')
  }

  function hello() {
    frame?.contentWindow?.postMessage(JSON.stringify({ event: 'listening', id: 1, channel: 'widget' }), '*')
  }

  function load(id) {
    loadedId = id
    lastPos = 0
    lastState = -1
    cmd(player.playing ? 'loadVideoById' : 'cueVideoById', [{ videoId: id, startSeconds: 0 }])
    cmd('setVolume', [Math.round(player.volume * 100)])
  }

  $effect(() => {
    if (!active || !ready) return
    const id = player.track.id
    if (id !== trackId) {
      trackId = id
      alt = 0
      if (ids[0]) load(ids[0])
    }
  })

  $effect(() => {
    if (!ready) return
    if (!active) {
      cmd('pauseVideo')
      trackId = null
      loadedId = null
      return
    }
    if (player.playing) cmd('playVideo')
    else cmd('pauseVideo')
  })

  $effect(() => {
    if (ready) cmd('setVolume', [Math.round(player.volume * 100)])
  })

  $effect(() => {
    const vis = () => {
      if (document.visibilityState === 'hidden') hiddenAt = performance.now()
      else if (active && player.playing) cmd('playVideo')
    }
    document.addEventListener('visibilitychange', vis)
    return () => document.removeEventListener('visibilitychange', vis)
  })

  $effect(() => {
    const un = listen('ext-seek', (e) => {
      if (!active) return
      lastPos = e.payload
      cmd('seekTo', [e.payload, true])
    })
    return () => un.then((f) => f())
  })

  function onMessage(e) {
    if (!String(e.origin).includes('youtube')) return
    let d
    try {
      d = JSON.parse(e.data)
    } catch {
      return
    }
    if (d.event === 'onReady' || d.event === 'initialDelivery') {
      if (!ready) ready = true
      return
    }
    if (d.event === 'onError') {
      if (!active) return
      alt += 1
      if (ids[alt]) {
        load(ids[alt])
      } else {
        toast(`YouTube no deja reproducir «${player.track.title}» aquí; sigo con la siguiente`)
        api.extFailed('YouTube no permite este video fuera de su app')
      }
      return
    }
    if (d.event !== 'infoDelivery' || !d.info || !active) return
    const i = d.info
    if (typeof i.playerState === 'number' && i.playerState !== lastState) {
      lastState = i.playerState
      if (i.playerState === 0) api.extEnded()
      else if (i.playerState === 2 && player.playing) {
        if (performance.now() - hiddenAt < 4000) setTimeout(() => active && player.playing && cmd('playVideo'), 300)
        else api.setPlaying(false)
      } else if (i.playerState === 1 && !player.playing) api.setPlaying(true)
    }
    if (typeof i.currentTime === 'number') {
      if (i.currentTime < 0.5 && lastPos > 2 && loadedId === lastVid) return
      lastPos = i.currentTime
      lastVid = loadedId
      const now = performance.now()
      if (now - lastSent > 900) {
        lastSent = now
        api.extProgress(Math.max(0, i.currentTime), Math.max(0, i.duration || 0))
      }
    }
  }
</script>

<svelte:window onmessage={onMessage} />

{#if firstId}
  <div class="yt" aria-hidden="true">
    <iframe bind:this={frame} {src} title="Reproductor" tabindex="-1" allow="autoplay; encrypted-media" referrerpolicy="strict-origin-when-cross-origin" onload={hello}></iframe>
  </div>
{/if}
