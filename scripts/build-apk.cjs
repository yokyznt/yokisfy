const { execSync } = require('child_process')
const fs = require('fs')
const path = require('path')
const os = require('os')

const root = path.join(__dirname, '..')
const conf = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'))
const sdk = process.env.ANDROID_HOME || path.join(process.env.LOCALAPPDATA || '', 'Android', 'Sdk')
const ndkDir = path.join(sdk, 'ndk')
const env = { ...process.env, ANDROID_HOME: sdk }
if (!env.NDK_HOME && fs.existsSync(ndkDir)) env.NDK_HOME = path.join(ndkDir, fs.readdirSync(ndkDir).sort().pop())
if (!env.JAVA_HOME) {
  const jdks = path.join(os.homedir(), '.jdks')
  if (fs.existsSync(jdks)) env.JAVA_HOME = path.join(jdks, fs.readdirSync(jdks).filter((d) => d.includes('21')).sort().pop())
}
execSync('npx tauri android build --apk --target aarch64', { cwd: root, stdio: 'inherit', env })
const out = path.join(root, 'src-tauri', 'gen', 'android', 'app', 'build', 'outputs', 'apk', 'universal', 'release')
const apk = fs.readdirSync(out).find((f) => f.endsWith('.apk'))
const dest = path.join(process.env.YOKISFY_APK_DIR || 'D:/Trabajos/APKs/Yokisfy-APK')
fs.mkdirSync(dest, { recursive: true })
const name = `Yokisfy-${conf.version}.apk`
fs.copyFileSync(path.join(out, apk), path.join(dest, name))
console.log(path.join(dest, name))
