const { chromium } = require('playwright')
;(async () => {
  const b = await chromium.connectOverCDP('http://127.0.0.1:9333')
  const page = b.contexts()[0].pages()[0]
  const inv = (cmd, args = {}) => page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args])
  await inv('volume', { v: 0 })
  const d = await inv('album', { id: "ia:jrad2026-09-24.mbox.flac" }).catch(() => null)
  let album = d
  if (!album) {
    const home = await inv('home')
    for (const a of home.find((s) => s.title.includes('vivo')).albums) {
      const x = await inv('album', { id: a.id })
      if (x.tracks[0]?.downloadable && x.tracks[0].formats.some((f) => f.kind === 'flac')) { album = x; break }
    }
  }
  console.log(album.album.id, album.tracks[1].formats.map(f => f.label + ' ' + Math.round((f.size||0)/1e6) + 'MB'))
  const t0 = Date.now()
  await inv('play', { tracks: album.tracks, start: 1 })
  await page.waitForFunction(() => document.querySelector('.pb-controls .play-btn')?.getAttribute('aria-label') === 'Pausar' && !document.querySelector('.pb-controls .spin'), null, { timeout: 60000 })
  console.log('start ms', Date.now() - t0)
  await page.waitForTimeout(5000)
  console.log('tag', await page.textContent('.pb-right .tag'), await page.$$eval('.pb-seek .time', (e) => e.map((x) => x.textContent)))
  await inv('seek', { secs: 200 })
  await page.waitForTimeout(3000)
  console.log('after seek', await page.$$eval('.pb-seek .time', (e) => e.map((x) => x.textContent)), await page.evaluate(() => document.querySelector('.pb-error')?.textContent || 'no error'))
  await page.screenshot({ path: process.env.OUT + '/27-flac.png' })
  await inv('toggle')
  process.exit(0)
})().catch(e => { console.error('FAIL', e.message.split('\n')[0]); process.exit(1) })
