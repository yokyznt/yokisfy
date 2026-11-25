use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use yokisfy_core::download::Progress;
use yokisfy_core::library::{Playlist, PlaylistDetail};
use yokisfy_core::player::Event;
use yokisfy_core::{AlbumDetail, Core, Error, SearchResults, Section, Track};

type C<'a> = State<'a, Arc<Core>>;
type R<T> = Result<T, Error>;

#[tauri::command]
async fn search(core: C<'_>, q: String) -> R<SearchResults> {
    Ok(core.search(&q).await)
}

#[tauri::command]
async fn album(core: C<'_>, id: String) -> R<AlbumDetail> {
    core.album(&id).await
}

#[tauri::command]
async fn home(core: C<'_>) -> R<Vec<Section>> {
    Ok(core.home().await)
}

#[tauri::command]
async fn stations(core: C<'_>, q: String) -> R<Vec<Track>> {
    core.stations(&q).await
}

#[tauri::command]
async fn start_radio(core: C<'_>, track: Track) -> R<usize> {
    let more = core.radio_for(&track, &[]).await;
    let mut q = vec![track];
    q.extend(more);
    let n = q.len();
    core.play(q, 0);
    Ok(n - 1)
}

#[tauri::command]
fn play(core: C<'_>, tracks: Vec<Track>, start: usize) {
    core.play(tracks, start)
}

#[tauri::command]
fn toggle(core: C<'_>) {
    core.player.toggle()
}

#[tauri::command]
fn set_playing(core: C<'_>, playing: bool) {
    core.player.set_playing(playing)
}

#[tauri::command]
fn next(core: C<'_>) {
    core.player.next()
}

#[tauri::command]
fn prev(core: C<'_>) {
    core.player.prev()
}

#[tauri::command]
fn seek(core: C<'_>, secs: f64) {
    core.player.seek(secs)
}

#[tauri::command]
fn volume(core: C<'_>, v: f32) -> R<()> {
    core.player.volume(v);
    core.lib.set_setting("volume", &v.to_string())
}

#[tauri::command]
fn enqueue(core: C<'_>, tracks: Vec<Track>) {
    core.player.enqueue(core.lib.resolve(tracks))
}

#[tauri::command]
fn play_next(core: C<'_>, track: Track) {
    if let Some(t) = core.lib.resolve(vec![track]).pop() {
        core.player.play_next(t)
    }
}

#[tauri::command]
fn jump(core: C<'_>, index: usize) {
    core.player.jump(index)
}

#[tauri::command]
fn queue_remove(core: C<'_>, index: usize) {
    core.player.remove(index)
}

#[tauri::command]
fn shuffle(core: C<'_>, on: bool) {
    core.player.shuffle(on)
}

#[tauri::command]
fn repeat(core: C<'_>, mode: String) {
    core.player.repeat(mode)
}

#[tauri::command]
fn playlists(core: C<'_>) -> R<Vec<Playlist>> {
    core.lib.playlists()
}

#[tauri::command]
fn playlist(core: C<'_>, id: i64) -> R<PlaylistDetail> {
    core.lib.playlist(id)
}

#[tauri::command]
fn playlist_create(core: C<'_>, name: String, tracks: Vec<Track>) -> R<i64> {
    let id = core.lib.create_playlist(&name)?;
    core.lib.add_to_playlist(id, &tracks)?;
    Ok(id)
}

#[tauri::command]
fn playlist_rename(core: C<'_>, id: i64, name: String) -> R<()> {
    core.lib.rename_playlist(id, &name)
}

#[tauri::command]
fn playlist_delete(core: C<'_>, id: i64) -> R<()> {
    core.lib.delete_playlist(id)
}

#[tauri::command]
fn playlist_add(core: C<'_>, id: i64, tracks: Vec<Track>) -> R<()> {
    core.lib.add_to_playlist(id, &tracks)
}

#[tauri::command]
fn playlist_remove(core: C<'_>, id: i64, index: i64) -> R<()> {
    core.lib.remove_from_playlist(id, index)
}

#[tauri::command]
fn liked(core: C<'_>) -> R<Vec<Track>> {
    core.lib.liked()
}

#[tauri::command]
fn liked_ids(core: C<'_>) -> R<Vec<String>> {
    core.lib.liked_ids()
}

#[tauri::command]
fn toggle_like(core: C<'_>, track: Track) -> R<bool> {
    core.lib.toggle_like(&track)
}

#[tauri::command]
fn recent(core: C<'_>) -> R<Vec<Track>> {
    core.lib.recent(24)
}

#[tauri::command]
fn downloads(core: C<'_>) -> R<Vec<Track>> {
    core.lib.downloads()
}

#[tauri::command]
async fn download(app: AppHandle, core: C<'_>, track: Track, kind: Option<String>) -> R<Track> {
    let h = app.clone();
    core.download(track, kind, &move |p: Progress| {
        let _ = h.emit("download", p);
    })
    .await
}

#[tauri::command]
fn delete_download(core: C<'_>, id: String) -> R<()> {
    core.delete_download(&id)
}

#[tauri::command]
fn settings(core: C<'_>) -> R<serde_json::Value> {
    let mut m = core.lib.settings()?;
    m.insert("download_dir_effective".into(), json!(core.download_dir().to_string_lossy()));
    m.insert("jamendo_available".into(), json!(core.has_jamendo()));
    Ok(serde_json::Value::Object(m))
}

#[tauri::command]
fn set_setting(core: C<'_>, key: String, value: String) -> R<()> {
    core.lib.set_setting(&key, &value)?;
    core.apply_settings();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let h = app.handle().clone();
            let core = Core::new(
                &dir,
                Arc::new(move |e| {
                    let _ = match e {
                        Event::Status(s) => h.emit("player", s),
                        Event::Queue(q, i) => h.emit("queue", json!({ "tracks": q, "index": i })),
                        Event::Started(t) => h.emit("started", t),
                        Event::Ended(t, ids) => {
                            let h2 = h.clone();
                            tauri::async_runtime::spawn(async move {
                                let core = h2.state::<Arc<Core>>();
                                if core.lib.setting("autoplay").as_deref() == Some("off") {
                                    return;
                                }
                                let more = core.radio_for(&t, &ids).await;
                                if !more.is_empty() {
                                    let _ = h2.emit("autoradio", &t.artist);
                                    core.play(more, 0);
                                }
                            });
                            Ok(())
                        }
                    };
                }),
            )?;
            app.manage(Arc::new(core));
            let mut w = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Yokisfy")
                .inner_size(1280.0, 820.0)
                .min_inner_size(360.0, 600.0);
            if let Ok(pos) = std::env::var("YOKISFY_DEV_POS") {
                let p: Vec<f64> = pos.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                if p.len() >= 2 {
                    w = w.position(p[0], p[1]).focused(false);
                }
                if p.len() >= 4 {
                    w = w.inner_size(p[2], p[3]);
                }
            }
            w.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search, album, home, stations, start_radio, play, toggle, set_playing, next, prev, seek, volume, enqueue, play_next, jump, queue_remove, shuffle, repeat,
            playlists, playlist, playlist_create, playlist_rename, playlist_delete, playlist_add, playlist_remove, liked, liked_ids, toggle_like,
            recent, downloads, download, delete_download, settings, set_setting
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Yokisfy");
}
