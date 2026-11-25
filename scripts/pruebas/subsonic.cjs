const { chromium } = require('playwright')
const crypto = require('crypto')
const dgram = require('dgram')

;(async () => {
  const b = await chromium.connectOverCDP('http://127.0.0.1:9333')
  const page = b.contexts()[0].pages()[0]
  const inv = (cmd, args = {}) => page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args])
  await inv('set_setting', { key: 'share_library', value: 'on' })
  await page.waitForTimeout(800)
  const info = await inv('server_info')
  console.log('info', JSON.stringify(info))
  const salt = crypto.randomBytes(6).toString('hex')
  const tok = crypto.createHash('md5').update(info.password + salt).digest('hex')
  const auth = `u=${info.user}&t=${tok}&s=${salt}&v=1.16.1&c=test`
  const api = async (m, extra = '', f = 'json') => {
    const r = await fetch(`${info.url}/rest/${m}?${auth}&f=${f}${extra}`)
    return f === 'json' ? (await r.json())['subsonic-response'] : await r.text()
  }
  console.log('ping', (await api('ping')).status)
  const bad = await (await fetch(`${info.url}/rest/ping.view?u=yokisfy&p=mal&f=json`)).json()
  console.log('bad auth', bad['subsonic-response'].error?.code)
  const all = await api('search3', '&query=&songCount=500')
  const songs = all.searchResult3.song
  console.log('songs', songs.length, 'albums', all.searchResult3.album.length, 'artists', all.searchResult3.artist.length)
  const list = await api('getAlbumList2', '&type=newest&size=5')
  console.log('albumList2', list.albumList2.album.map((a) => `${a.artist} - ${a.name} (${a.songCount})`).join(' | '))
  const al = await api('getAlbum', `&id=${list.albumList2.album[0].id}`)
  console.log('getAlbum songs', al.album.song.length, al.album.song[0].title, al.album.song[0].suffix)
  const ar = await api('getArtists')
  console.log('artists index', ar.artists.index.length)
  const s0 = al.album.song[0]
  const r = await fetch(`${info.url}/rest/stream?${auth}&id=${s0.id}`, { headers: { Range: 'bytes=1000-1999' } })
  const buf = Buffer.from(await r.arrayBuffer())
  console.log('stream range', r.status, r.headers.get('content-range'), r.headers.get('content-type'), buf.length)
  const full = await fetch(`${info.url}/rest/stream?${auth}&id=${s0.id}`)
  console.log('stream full', full.status, full.headers.get('content-length'), 'size field', s0.size)
  const c = await fetch(`${info.url}/rest/getCoverArt?${auth}&id=${s0.coverArt}`, { redirect: 'manual' })
  console.log('cover', c.status, c.headers.get('content-type'), (await c.arrayBuffer()).byteLength)
  const pls = await api('getPlaylists')
  console.log('playlists', pls.playlists.playlist.map((p) => `${p.name}:${p.songCount}`).join(', '))
  if (pls.playlists.playlist[0]) {
    const p = await api('getPlaylist', `&id=${pls.playlists.playlist[0].id}`)
    console.log('playlist entries', p.playlist.entry.length)
    const remote = p.playlist.entry.find((e) => !songs.some((s) => s.id === e.id))
    if (remote) {
      const rr = await fetch(`${info.url}/rest/stream?${auth}&id=${remote.id}`, { redirect: 'manual' })
      console.log('remote stream', rr.status, (rr.headers.get('location') || '').slice(0, 60))
    }
  }
  const xml = await api('getAlbumList2', '&type=random&size=2', 'xml')
  console.log('xml', xml.slice(0, 200))
  const sock = dgram.createSocket('udp4')
  sock.bind(() => {
    sock.setBroadcast(true)
    sock.send('YOKISFY?', 45131, '255.255.255.255')
  })
  const reply = await new Promise((res) => {
    sock.on('message', (m, from) => res(`${from.address} ${m}`))
    setTimeout(() => res('sin respuesta'), 3000)
  })
  sock.close()
  console.log('discovery', reply.replace(/"password":"[^"]+"/, '"password":"***"'))
  process.exit(0)
})().catch((e) => {
  console.error('FAIL', e.message)
  process.exit(1)
})
