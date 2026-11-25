const { chromium } = require('playwright')
const out = process.env.OUT

;(async () => {
  const b = await chromium.connectOverCDP('http://127.0.0.1:9333')
  const ctx = b.contexts()[0]
  const page = ctx.pages()[0]
  const errors = []
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
  page.on('pageerror', (e) => errors.push(String(e)))
  const inv = (cmd, args = {}) => page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args])
  const shot = async (name) => {
    await page.screenshot({ path: `${out}/${name}.png` })
    console.log('shot', name)
  }
  const wait = (ms) => page.waitForTimeout(ms)
  await page.keyboard.press('Escape')
  await inv('volume', { v: 0 })
  const home = await inv('home')
  const live = home.find((s) => s.title.includes('vivo'))
  let flacAlbum = null
  for (const a of live.albums) {
    const d = await inv('album', { id: a.id })
    if (d.tracks.length && d.tracks[0].downloadable && d.tracks[0].formats.some((f) => f.kind === 'flac')) {
      flacAlbum = d
      break
    }
  }
  console.log('flac album', flacAlbum?.album.title, flacAlbum?.tracks[0].formats.map((f) => f.label))
  await page.evaluate((id) => {
    const btn = [...document.querySelectorAll('.side-link')].find((b) => b.textContent.includes('Inicio'))
    btn.click()
  })
  await page.evaluate(() => 0)
  await inv('set_setting', { key: 'stream_quality', value: 'alta' })
  await page.evaluate(async (id) => {}, 0)
  await page.goto(page.url())
  await wait(1500)
  await page.evaluate((id) => {
    window.__yk_go = id
  }, flacAlbum.album.id)
  await page.click('.side-link:has-text("Buscar")')
  await page.fill('.search-bar input', flacAlbum.album.title.replace(/[^\w ]/g, ' '))
  await page.waitForSelector('.card', { timeout: 30000 })
  await wait(1500)
  const cards = await page.$$eval('.card-title', (els) => els.map((e) => e.textContent))
  const idx = cards.findIndex((c) => c === flacAlbum.album.title)
  console.log('card idx', idx)
  if (idx >= 0) await page.click(`.card-hit >> nth=${idx}`)
  else await page.click('.card-hit >> nth=0')
  await page.waitForSelector('.hero-title', { timeout: 30000 })
  await page.click('.actions .play-btn')
  const t0 = Date.now()
  await page.waitForFunction(() => document.querySelector('.pb-controls .play-btn')?.getAttribute('aria-label') === 'Pausar' && !document.querySelector('.pb-controls .spin'), null, { timeout: 60000 })
  console.log('flac playback start ms', Date.now() - t0)
  await wait(4000)
  console.log('tag', await page.textContent('.pb-right .tag'), 'time', await page.$$eval('.pb-seek .time', (e) => e.map((x) => x.textContent)))
  await shot('20-flac-playing')
  await page.click('.tl-row >> nth=0 >> .like')
  await wait(400)
  await page.click('.actions .icon-btn[aria-label="Añadir a playlist"]')
  await wait(300)
  await page.click('.menu-item:has-text("Nueva playlist")')
  await wait(300)
  await shot('21-prompt')
  await page.click('.dlg .btn.primary')
  await wait(800)
  await page.click('.tl-row >> nth=1 >> .icon-btn[aria-label="Más opciones"]')
  await page.click('.menu-item:has-text("Descargar")')
  await wait(300)
  await shot('22-dl-menu')
  await page.click('.menu-item.mono >> nth=0')
  await page.click('.side-link:has-text("Descargas")')
  await wait(1200)
  await shot('23-downloading')
  await page.waitForFunction(() => !document.querySelector('.dl-list'), null, { timeout: 120000 })
  await wait(800)
  await shot('24-downloaded')
  await page.click('.side-pl >> nth=-1')
  await wait(1000)
  await shot('25-playlist')
  await page.click('.side-pl >> nth=0')
  await wait(1000)
  await shot('26-liked')
  const s = await inv('settings')
  console.log('dir', s.download_dir_effective)
  await page.click('.pb-controls .play-btn')
  console.log('errors', JSON.stringify(errors))
  process.exit(0)
})().catch((e) => {
  console.error('FAIL', e.message.split('\n')[0])
  process.exit(1)
})
