package dev.yokyznt.yokisfy.media

import android.app.Activity
import android.content.Intent
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import android.speech.RecognizerIntent
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ListenArgs {
    lateinit var channel: Channel
}

@InvokeArg
class UpdateArgs {
    var title: String = ""
    var artist: String = ""
    var cover: String? = null
    var playing: Boolean = false
    var position: Double = 0.0
    var duration: Double = 0.0
    var active: Boolean = false
}

@TauriPlugin
class MediaPlugin(private val activity: Activity) : Plugin(activity) {
    companion object {
        @Volatile
        var channel: Channel? = null

        @Volatile
        var playing: Boolean = false

        @Volatile
        var pending: MutableList<JSObject> = mutableListOf()

        fun send(o: JSObject) {
            val c = channel
            if (c == null) {
                synchronized(pending) { pending.add(o) }
                return
            }
            c.send(o)
        }

        fun emit(action: String, position: Double? = null) {
            val o = JSObject()
            o.put("action", action)
            if (position != null) o.put("position", position)
            channel?.send(o)
        }

        fun say(text: String) {
            if (text.isBlank()) return
            val o = JSObject()
            o.put("action", "voice")
            o.put("text", text)
            send(o)
        }
    }

    private var devices: AudioDeviceCallback? = null

    private fun bt(d: AudioDeviceInfo) = d.isSink && (d.type == AudioDeviceInfo.TYPE_BLUETOOTH_A2DP || (android.os.Build.VERSION.SDK_INT >= 31 && d.type == AudioDeviceInfo.TYPE_BLE_SPEAKER))

    override fun load(webView: WebView) {
        super.load(webView)
        val am = activity.getSystemService(AudioManager::class.java) ?: return
        val cb = object : AudioDeviceCallback() {
            override fun onAudioDevicesAdded(added: Array<out AudioDeviceInfo>) {
                for (d in added) if (bt(d)) tell(d, true)
            }

            override fun onAudioDevicesRemoved(removed: Array<out AudioDeviceInfo>) {
                for (d in removed) if (bt(d)) tell(d, false)
            }
        }
        devices = cb
        am.registerAudioDeviceCallback(cb, Handler(Looper.getMainLooper()))
    }

    private fun tell(d: AudioDeviceInfo, on: Boolean) {
        val o = JSObject()
        o.put("action", "bt")
        o.put("name", d.productName?.toString() ?: "Bluetooth")
        o.put("on", on)
        send(o)
    }

    @Command
    fun speech(invoke: Invoke) {
        val i = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE, "es-MX")
            .putExtra(RecognizerIntent.EXTRA_PROMPT, "¿Qué quieres escuchar?")
            .putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
        try {
            startActivityForResult(invoke, i, "speechDone")
        } catch (_: Exception) {
            invoke.reject("Tu teléfono no tiene reconocimiento de voz")
        }
    }

    @ActivityCallback
    fun speechDone(invoke: Invoke, result: ActivityResult) {
        val t = result.data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull() ?: ""
        val o = JSObject()
        o.put("text", t)
        invoke.resolve(o)
    }

    @Command
    fun outputs(invoke: Invoke) {
        val ok = try {
            if (android.os.Build.VERSION.SDK_INT >= 34) {
                android.media.MediaRouter2.getInstance(activity).showSystemOutputSwitcher()
            } else if (android.os.Build.VERSION.SDK_INT >= 31) {
                activity.sendBroadcast(android.content.Intent("com.android.systemui.action.LAUNCH_MEDIA_OUTPUT_DIALOG").setPackage("com.android.systemui").putExtra("package_name", activity.packageName))
                true
            } else {
                activity.startActivity(android.content.Intent("android.settings.panel.action.MEDIA_OUTPUT").putExtra("com.android.settings.panel.extra.package_name", activity.packageName))
                true
            }
        } catch (_: Exception) {
            false
        }
        if (!ok) {
            try {
                activity.startActivity(android.content.Intent(android.provider.Settings.ACTION_BLUETOOTH_SETTINGS))
            } catch (_: Exception) {
            }
        }
        invoke.resolve()
    }

    @Command
    fun listen(invoke: Invoke) {
        val c = invoke.parseArgs(ListenArgs::class.java).channel
        channel = c
        invoke.resolve()
        val left = synchronized(pending) { pending.toList().also { pending.clear() } }
        for (o in left) c.send(o)
    }

    @Command
    fun update(invoke: Invoke) {
        val a = invoke.parseArgs(UpdateArgs::class.java)
        playing = a.active && a.playing
        MediaService.push(activity.applicationContext, a)
        invoke.resolve()
    }
}
