package dev.yokyznt.yokisfy.media

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.media.app.NotificationCompat.MediaStyle
import java.net.URL
import kotlin.concurrent.thread

class MediaService : Service() {
    companion object {
        private const val CHANNEL = "yokisfy_playback"
        private const val ID = 7
        @Volatile var state: UpdateArgs? = null
        @Volatile var instance: MediaService? = null

        fun push(ctx: Context, a: UpdateArgs) {
            state = a
            val inst = instance
            if (inst != null) {
                inst.refresh()
            } else if (a.active) {
                ContextCompat.startForegroundService(ctx, Intent(ctx, MediaService::class.java))
            }
        }
    }

    private lateinit var session: MediaSessionCompat
    private var art: Bitmap? = null
    private var artUrl: String? = null
    private var wake: PowerManager.WakeLock? = null
    private var wifi: WifiManager.WifiLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        instance = this
        val nm = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26) {
            nm.createNotificationChannel(NotificationChannel(CHANNEL, "Reproducción", NotificationManager.IMPORTANCE_LOW))
        }
        session = MediaSessionCompat(this, "Yokisfy")
        session.setCallback(object : MediaSessionCompat.Callback() {
            override fun onPlay() = MediaPlugin.emit("play")
            override fun onPause() = MediaPlugin.emit("pause")
            override fun onStop() = MediaPlugin.emit("pause")
            override fun onSkipToNext() = MediaPlugin.emit("next")
            override fun onSkipToPrevious() = MediaPlugin.emit("prev")
            override fun onSeekTo(pos: Long) = MediaPlugin.emit("seek", pos / 1000.0)
        })
        session.isActive = true
        wake = getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "yokisfy:play")
        wifi = (applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager).createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, "yokisfy:play")
    }

    fun cover(): Bitmap? = art

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val s = state
        if (intent?.hasExtra("action") == true && (MediaPlugin.channel == null || s == null || !s.active)) {
            YkWidget.render(this, null)
            stopSelf()
            instance = null
            return START_NOT_STICKY
        }
        intent?.getStringExtra("action")?.let { MediaPlugin.emit(it) }
        refresh()
        return START_NOT_STICKY
    }

    private fun action(name: String, code: Int): PendingIntent {
        val i = Intent(this, MediaService::class.java).putExtra("action", name)
        return PendingIntent.getService(this, code, i, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
    }

    private fun locks(on: Boolean) {
        try {
            if (on) {
                if (wake?.isHeld == false) wake?.acquire(6 * 60 * 60 * 1000L)
                if (wifi?.isHeld == false) wifi?.acquire()
            } else {
                if (wake?.isHeld == true) wake?.release()
                if (wifi?.isHeld == true) wifi?.release()
            }
        } catch (_: Exception) {
        }
    }

    fun refresh() {
        val s = state ?: return
        if (!s.active) {
            YkWidget.render(this, null)
            locks(false)
            session.isActive = false
            if (Build.VERSION.SDK_INT >= 24) stopForeground(STOP_FOREGROUND_REMOVE) else @Suppress("DEPRECATION") stopForeground(true)
            stopSelf()
            instance = null
            return
        }
        if (s.cover != artUrl) loadArt(s.cover)
        session.setMetadata(
            MediaMetadataCompat.Builder()
                .putString(MediaMetadataCompat.METADATA_KEY_TITLE, s.title)
                .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, s.artist)
                .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, if (s.duration > 0) (s.duration * 1000).toLong() else -1L)
                .putBitmap(MediaMetadataCompat.METADATA_KEY_ALBUM_ART, art)
                .build()
        )
        val actions = PlaybackStateCompat.ACTION_PLAY or PlaybackStateCompat.ACTION_PAUSE or PlaybackStateCompat.ACTION_PLAY_PAUSE or
            PlaybackStateCompat.ACTION_SKIP_TO_NEXT or PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
            (if (s.duration > 0) PlaybackStateCompat.ACTION_SEEK_TO else 0L)
        session.setPlaybackState(
            PlaybackStateCompat.Builder()
                .setActions(actions)
                .setState(if (s.playing) PlaybackStateCompat.STATE_PLAYING else PlaybackStateCompat.STATE_PAUSED, (s.position * 1000).toLong(), if (s.playing) 1f else 0f)
                .build()
        )
        val open = packageManager.getLaunchIntentForPackage(packageName)?.let {
            PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        }
        val n = NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(applicationInfo.icon)
            .setContentTitle(s.title)
            .setContentText(s.artist)
            .setLargeIcon(art)
            .setContentIntent(open)
            .setOnlyAlertOnce(true)
            .setShowWhen(false)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .addAction(android.R.drawable.ic_media_previous, "Anterior", action("prev", 1))
            .addAction(if (s.playing) android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play, if (s.playing) "Pausar" else "Reproducir", action(if (s.playing) "pause" else "play", 2))
            .addAction(android.R.drawable.ic_media_next, "Siguiente", action("next", 3))
            .setStyle(MediaStyle().setMediaSession(session.sessionToken).setShowActionsInCompactView(0, 1, 2))
            .build()
        if (Build.VERSION.SDK_INT >= 29) startForeground(ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK) else startForeground(ID, n)
        locks(s.playing)
        YkWidget.render(this, art)
    }

    private fun loadArt(url: String?) {
        artUrl = url
        art = null
        if (url.isNullOrEmpty() || !url.startsWith("http")) return
        thread {
            val bmp = try {
                URL(url).openStream().use { BitmapFactory.decodeStream(it) }?.let { b ->
                    val m = maxOf(b.width, b.height)
                    if (m > 400) Bitmap.createScaledBitmap(b, b.width * 400 / m, b.height * 400 / m, true) else b
                }
            } catch (_: Exception) {
                null
            }
            if (artUrl == url && bmp != null) {
                art = bmp
                refresh()
            }
        }
    }

    override fun onDestroy() {
        locks(false)
        session.release()
        instance = null
        super.onDestroy()
    }
}
