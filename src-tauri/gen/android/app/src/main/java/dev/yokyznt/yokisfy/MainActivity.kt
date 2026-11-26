package dev.yokyznt.yokisfy

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.WebView
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
