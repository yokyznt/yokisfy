package dev.yokyznt.yokisfy.media

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.widget.RemoteViews

class YkWidget : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
        render(ctx, MediaService.instance?.cover())
    }

    companion object {
        private const val FLAGS = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE

        private fun svc(ctx: Context, name: String, code: Int): PendingIntent =
            PendingIntent.getService(ctx, code, Intent(ctx, MediaService::class.java).putExtra("action", name), FLAGS)

        fun render(ctx: Context, art: Bitmap?) {
            val mgr = AppWidgetManager.getInstance(ctx)
            val ids = mgr.getAppWidgetIds(ComponentName(ctx, YkWidget::class.java))
            if (ids.isEmpty()) return
            val s = MediaService.state
            val live = s != null && s.active && MediaPlugin.channel != null
            val v = RemoteViews(ctx.packageName, R.layout.yk_widget)
            v.setTextViewText(R.id.yk_title, if (live) s!!.title else "Yokisfy")
            v.setTextViewText(R.id.yk_artist, if (live) s!!.artist else "Toca para abrir")
            if (live && art != null) v.setImageViewBitmap(R.id.yk_art, art) else v.setImageViewResource(R.id.yk_art, R.drawable.yk_widget_art)
            v.setImageViewResource(R.id.yk_play, if (live && s!!.playing) R.drawable.yk_pause else R.drawable.yk_play)
            val open = ctx.packageManager.getLaunchIntentForPackage(ctx.packageName)?.let { PendingIntent.getActivity(ctx, 10, it, FLAGS) }
            v.setOnClickPendingIntent(R.id.yk_root, open)
            v.setOnClickPendingIntent(R.id.yk_art, open)
            if (live) {
                v.setOnClickPendingIntent(R.id.yk_prev, svc(ctx, "prev", 11))
                v.setOnClickPendingIntent(R.id.yk_play, svc(ctx, if (s!!.playing) "pause" else "play", 12))
                v.setOnClickPendingIntent(R.id.yk_next, svc(ctx, "next", 13))
            } else {
                v.setOnClickPendingIntent(R.id.yk_prev, open)
                v.setOnClickPendingIntent(R.id.yk_play, open)
                v.setOnClickPendingIntent(R.id.yk_next, open)
            }
            mgr.updateAppWidget(ids, v)
        }
    }
}
