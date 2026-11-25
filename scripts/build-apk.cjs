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
const android = path.join(root, 'src-tauri', 'gen', 'android')
const target = process.env.YOKISFY_ANDROID_TARGET || 'aarch64'
const abi = { aarch64: ['arm64-v8a', 'aarch64-linux-android', 'Arm64'], x86_64: ['x86_64', 'x86_64-linux-android', 'X86_64'] }[target]
const jni = path.join(android, 'app', 'src', 'main', 'jniLibs', abi[0])
fs.mkdirSync(jni, { recursive: true })
const prebuilt = path.join(env.NDK_HOME, 'toolchains', 'llvm', 'prebuilt')
fs.copyFileSync(path.join(prebuilt, fs.readdirSync(prebuilt)[0], 'sysroot', 'usr', 'lib', abi[1], 'libc++_shared.so'), path.join(jni, 'libc++_shared.so'))
const started = Date.now()
let out = path.join(android, 'app', 'build', 'outputs', 'apk', 'universal', 'release')
try {
  execSync(`npx tauri android build --apk --target ${target}`, { cwd: root, stdio: 'inherit', env })
} catch (e) {
  const so = path.join(root, 'target', abi[1], 'release', 'libyokisfy_lib.so')
  if (!fs.existsSync(so) || fs.statSync(so).mtimeMs < started) throw e
  fs.rmSync(path.join(jni, 'libyokisfy_lib.so'), { force: true })
  fs.copyFileSync(so, path.join(jni, 'libyokisfy_lib.so'))
  const gradle = process.platform === 'win32' ? 'gradlew.bat' : './gradlew'
  execSync(`${gradle} assemble${abi[2]}Release -x rustBuild${abi[2]}Release --console=plain`, { cwd: android, stdio: 'inherit', env, shell: true })
  out = path.join(android, 'app', 'build', 'outputs', 'apk', abi[2].toLowerCase(), 'release')
}
const apk = fs.readdirSync(out).find((f) => f.endsWith('.apk'))
const dest = path.join(process.env.YOKISFY_APK_DIR || 'D:/Trabajos/APKs/Yokisfy-APK')
fs.mkdirSync(dest, { recursive: true })
const name = target === 'aarch64' ? `Yokisfy-${conf.version}.apk` : `Yokisfy-${conf.version}-${target}.apk`
fs.copyFileSync(path.join(out, apk), path.join(dest, name))
console.log(path.join(dest, name))
