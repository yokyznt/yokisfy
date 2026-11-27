(function () {
  if (!/youtube/.test(location.hostname) || window.__ykHook) return
  window.__ykHook = 1
  var want = 1
  var ctx, gain, comp, post, node, el
  function chain(v) {
    if (el === v && ctx) return
    try {
      ctx = ctx || new (window.AudioContext || window.webkitAudioContext)()
      node = ctx.createMediaElementSource(v)
      el = v
      gain = ctx.createGain()
      comp = ctx.createDynamicsCompressor()
      post = ctx.createGain()
      node.connect(gain)
      gain.connect(comp)
      comp.connect(post)
      post.connect(ctx.destination)
    } catch (e) {}
  }
  function apply() {
    var v = document.querySelector('video')
    if (!v) return
    if (want > 1.001) chain(v)
    if (!gain) return
    if (ctx.state === 'suspended') ctx.resume()
    var t = ctx.currentTime
    gain.gain.setTargetAtTime(want, t, 0.05)
    var on = want > 1.001
    comp.threshold.setTargetAtTime(on ? -22 : 0, t, 0.05)
    comp.knee.value = on ? 4 : 0
    comp.ratio.setTargetAtTime(on ? 14 : 1, t, 0.05)
    comp.attack.value = 0.002
    comp.release.value = 0.09
    post.gain.setTargetAtTime(on ? Math.sqrt(want) : 1, t, 0.05)
  }
  window.addEventListener('message', function (e) {
    var d = e.data
    if (typeof d === 'string' && d.indexOf('ykBoost') >= 0) {
      try {
        d = JSON.parse(d)
      } catch (x) {
        return
      }
    }
    if (d && typeof d.ykBoost === 'number') {
      want = Math.pow(10, Math.max(0, Math.min(20, d.ykBoost)) / 20)
      apply()
    }
  })
  setInterval(function () {
    if (want > 1.001) apply()
  }, 1500)
})()
