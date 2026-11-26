package dev.yokyznt.yokisfy

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.WebView
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

  override fun onWebViewCreate(webView: WebView) {
    web = webView
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
