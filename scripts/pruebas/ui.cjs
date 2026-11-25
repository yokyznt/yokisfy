const { chromium } = require('playwright')
const fs = require('fs')
const out = process.env.OUT
const step = process.argv[2] || 'all'

;(async () => {
  const b = await chromium.connectOverCDP('http://127.0.0.1:9333')
  const ctx = b.contexts()[0]
  const page = ctx.pages().find((p) => p.url().includes('tauri') || p.url().includes('localhost')) || ctx.pages()[0]
  const errors = []
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
  page.on('pageerror', (e) => errors.push(String(e)))
  const inv = (cmd, args = {}) => page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args])
  const shot = async (name) => {
    await page.screenshot({ path: `${out}/${name}.png` })
    console.log('shot', name)
  }
  const wait = (ms) => page.waitForTimeout(ms)
  const state = () => page.evaluate(() => ({ title: document.querySelector('.pb-title')?.textContent, time: [...document.querySelectorAll('.pb-seek .time')].map((e) => e.textContent), playing: document.querySelector('.pb-controls .play-btn')?.getAttribute('aria-label') }))

  console.log('url', page.url())
  if (step === 'all' || step === 'desk') {
    await inv('volume', { v: 0 })
    await page.click('.side-link:has-text("Inicio")')
    await page.waitForSelector('.card', { timeout: 30000 })
    await wait(2500)
    await shot('01-home')
    await page.click('.side-link:has-text("Buscar")')
    await shot('02-search-empty')
    await page.fill('.search-bar input', 'ambient')
    await page.waitForSelector('.tl-row', { timeout: 30000 })
    await wait(2500)
    await shot('03-search')
    await page.click('.card-hit >> nth=0')
    await page.waitForSelector('.hero-title', { timeout: 30000 })
    await wait(1500)
    await shot('04-album')
    await page.click('.actions .play-btn')
    await wait(6000)
    console.log('state', JSON.stringify(await state()))
    await shot('05-playing')
    await page.click('.tl-row >> nth=1 >> .icon-btn[aria-label="Más opciones"]')
    await wait(400)
    await shot('06-menu')
    await page.keyboard.press('Escape')
    await page.click('.pb-right .icon-btn[aria-label="Cola"]')
    await wait(600)
    await shot('07-queue')
    await page.click('.pb-right .icon-btn[aria-label="Cola"]')
    await page.click('.side-link:has-text("Ajustes")')
    await wait(500)
    await shot('08-settings')
  }
  if (step === 'all' || step === 'mobile') {
    const s = await ctx.newCDPSession(page)
    await s.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: true })
    await wait(500)
    await page.click('.tabbar button:has-text("Inicio")')
    await wait(1500)
    await shot('10-m-home')
    await page.click('.tabbar button:has-text("Biblioteca")')
    await wait(600)
    await shot('11-m-library')
    await page.click('.pb-cover')
    await wait(700)
    await shot('12-m-player')
    await page.click('.fp-head .icon-btn[aria-label="Cerrar"]')
    await s.send('Emulation.clearDeviceMetricsOverride')
  }
  console.log('errors', JSON.stringify(errors))
  await b.close().catch(() => {})
  process.exit(0)
})().catch((e) => {
  console.error('FAIL', e.message)
  process.exit(1)
})
