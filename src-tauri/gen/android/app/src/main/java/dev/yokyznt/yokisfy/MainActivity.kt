package dev.yokyznt.yokisfy

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import androidx.activity.enableEdgeToEdge
import dev.yokyznt.yokisfy.media.MediaPlugin

class MainActivity : TauriActivity() {
  private var web: WebView? = null
  private val handler = Handler(Looper.getMainLooper())

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  private var insetJs = ""

  private fun pushInsets() {
    val w = web ?: return
    if (insetJs.isEmpty()) return
    w.evaluateJavascript(insetJs, null)
  }

  @Deprecated("Deprecated in Java")
  override fun onBackPressed() {
    val w = web
    if (w == null) {
      moveTaskToBack(true)
      return
    }
    w.evaluateJavascript("(function(){try{return window.__ykBack?String(window.__ykBack()):'false'}catch(e){return 'false'}})()") { r ->
      if (r == null || !r.contains("true")) moveTaskToBack(true)
    }
  }

  override fun onResume() {
    super.onResume()
    for (d in longArrayOf(100, 800)) handler.postDelayed({ pushInsets() }, d)
  }

  override fun onWebViewCreate(webView: WebView) {
    web = webView
    ViewCompat.setOnApplyWindowInsetsListener(webView) { v, insets ->
      val b = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      val d = resources.displayMetrics.density
      insetJs = "(function(){var s=document.documentElement.style;s.setProperty('--sat','${b.top / d}px');s.setProperty('--sab','${b.bottom / d}px');s.setProperty('--sal','${b.left / d}px');s.setProperty('--sar','${b.right / d}px')})()"
      for (t in longArrayOf(0, 400, 1200, 3000, 6000)) handler.postDelayed({ pushInsets() }, t)
      ViewCompat.onApplyWindowInsets(v, insets)
    }
    if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
      try {
        WebViewCompat.addDocumentStartJavaScript(webView, KEEP, setOf("https://www.youtube-nocookie.com", "https://www.youtube.com"))
      } catch (_: Exception) {
      }
    }
  }

  companion object {
    private const val KEEP = "(function(){try{" +
      "var d=Document.prototype;" +
      "Object.defineProperty(d,'hidden',{get:function(){return false},configurable:true});" +
      "Object.defineProperty(d,'visibilityState',{get:function(){return 'visible'},configurable:true});" +
      "Object.defineProperty(d,'webkitHidden',{get:function(){return false},configurable:true});" +
      "Object.defineProperty(d,'webkitVisibilityState',{get:function(){return 'visible'},configurable:true});" +
      "var s=function(e){e.stopImmediatePropagation()};" +
      "['visibilitychange','webkitvisibilitychange','freeze','pagehide'].forEach(function(n){window.addEventListener(n,s,true);document.addEventListener(n,s,true)});" +
      "}catch(e){}})();"
  }

  private fun wake() {
    if (!MediaPlugin.playing) return
    web?.onResume()
    web?.resumeTimers()
    web?.dispatchWindowVisibilityChanged(View.VISIBLE)
  }

  private fun keepAlive() {
    wake()
    for (d in longArrayOf(50, 250, 800, 2000)) handler.postDelayed({ wake() }, d)
  }

  override fun onPause() {
    super.onPause()
    keepAlive()
  }

  override fun onStop() {
    super.onStop()
    keepAlive()
  }
}
