package dev.yokyznt.yokisfy.media

import android.app.Activity
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

        fun emit(action: String, position: Double? = null) {
            val o = JSObject()
            o.put("action", action)
            if (position != null) o.put("position", position)
            channel?.send(o)
        }
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
        channel = invoke.parseArgs(ListenArgs::class.java).channel
        invoke.resolve()
    }

    @Command
    fun update(invoke: Invoke) {
        val a = invoke.parseArgs(UpdateArgs::class.java)
        playing = a.active && a.playing
        MediaService.push(activity.applicationContext, a)
        invoke.resolve()
    }
}
