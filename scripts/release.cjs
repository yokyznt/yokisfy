const { execSync } = require('child_process')
const fs = require('fs')
const path = require('path')

const root = path.join(__dirname, '..')
const conf = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'))
const version = conf.version
const tag = `v${version}`
const repo = 'yokyznt/yokisfy'
const keyFile = process.env.YOKISFY_UPDATER_KEY || 'D:/Trabajos/APKs/Yokisfy-PC/llaves/updater.key'
const outPc = process.env.YOKISFY_PC_DIR || 'D:/Trabajos/APKs/Yokisfy-PC'
const outApk = process.env.YOKISFY_APK_DIR || 'D:/Trabajos/APKs/Yokisfy-APK'
const notesArg = process.argv.slice(2).join(' ').trim()
const skipBuild = process.env.YOKISFY_SKIP_BUILD === '1'

const run = (cmd, opts = {}) => execSync(cmd, { cwd: root, stdio: 'inherit', ...opts })

if (!skipBuild) {
  const env = { ...process.env, TAURI_SIGNING_PRIVATE_KEY: fs.readFileSync(keyFile, 'utf8').trim(), TAURI_SIGNING_PRIVATE_KEY_PASSWORD: '' }
  run('npx tauri build', { env })
  run('npm run android:apk')
}

const nsis = path.join(root, 'target', 'release', 'bundle', 'nsis')
const exeName = `Yokisfy_${version}_x64-setup.exe`
const exe = path.join(nsis, exeName)
const sig = fs.readFileSync(`${exe}.sig`, 'utf8').trim()
const apk = path.join(outApk, `Yokisfy-${version}.apk`)
fs.mkdirSync(outPc, { recursive: true })
fs.copyFileSync(exe, path.join(outPc, exeName))

const notes = notesArg || `Yokisfy ${version}`
const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: { 'windows-x86_64': { signature: sig, url: `https://github.com/${repo}/releases/download/${tag}/${exeName}` } }
}
const latestFile = path.join(nsis, 'latest.json')
fs.writeFileSync(latestFile, JSON.stringify(latest, null, 2))

const apkName = `Yokisfy-${version}.apk`
const body = `${notes}

**Windows:** baja \`${exeName}\` y ábrelo. Las siguientes versiones se instalan solas desde la app.
**Android:** baja \`${apkName}\` en el celular y ábrelo (permite instalar apps de esta fuente). La app avisa cuando hay versión nueva.`
const bodyFile = path.join(nsis, 'release-notes.md')
fs.writeFileSync(bodyFile, body)

const branch = execSync('git rev-parse --abbrev-ref HEAD', { cwd: root }).toString().trim()
const files = [exe, `${exe}.sig`, latestFile, apk].map((f) => `"${f}"`).join(' ')
let exists = true
try {
  execSync(`gh release view ${tag} --repo ${repo}`, { cwd: root, stdio: 'ignore' })
} catch {
  exists = false
}
if (exists) run(`gh release upload ${tag} ${files} --clobber --repo ${repo}`)
else run(`gh release create ${tag} ${files} --repo ${repo} --target ${branch} --title "Yokisfy ${version}" --notes-file "${bodyFile}" --latest`)
console.log(`https://github.com/${repo}/releases/tag/${tag}`)
