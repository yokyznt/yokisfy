use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use yokisfy_core::download::Progress;
use yokisfy_core::library::{Playlist, PlaylistDetail};
use yokisfy_core::player::Event;
use tauri_plugin_yk_media::MediaState;
use yokisfy_core::{server, AlbumDetail, Core, Error, SearchResults, Section, Track};

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
async fn import_spotify(app: AppHandle, core: C<'_>, link: String) -> R<yokisfy_core::ImportResult> {
    let h = app.clone();
    core.import_spotify(&link, &move |done, total| {
        let _ = h.emit("import", json!({ "done": done, "total": total }));
    })
    .await
}

#[tauri::command]
fn ext_progress(core: C<'_>, pos: f64, dur: f64) {
    core.player.ext_progress(pos, dur)
}

#[tauri::command]
fn ext_ended(core: C<'_>) {
    core.player.ext_ended()
}

#[tauri::command]
fn ext_failed(core: C<'_>, error: String) {
    core.player.ext_failed(error)
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
    m.insert("mobile".into(), json!(cfg!(mobile)));
    Ok(serde_json::Value::Object(m))
}

#[tauri::command]
async fn set_setting(core: C<'_>, srv: State<'_, Srv>, key: String, value: String) -> R<()> {
    core.lib.set_setting(&key, &value)?;
    core.apply_settings();
    if key == "share_library" {
        apply_server(&core, &srv).await;
    }
    Ok(())
}

type Srv = tokio::sync::Mutex<Option<server::ServerHandle>>;

fn server_password(core: &Core) -> String {
    match core.lib.setting("server_password").filter(|p| p.len() >= 8) {
        Some(p) => p,
        None => {
            use rand::Rng;
            let p: String = rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(10).map(char::from).collect::<String>().to_lowercase();
            let _ = core.lib.set_setting("server_password", &p);
            p
        }
    }
}

async fn apply_server(core: &Core, srv: &Srv) {
    let want = core.lib.setting("share_library").as_deref() == Some("on");
    let mut g = srv.lock().await;
    if !want {
        *g = None;
        return;
    }
    if g.is_none() {
        *g = server::start(core.lib.clone(), server_password(core)).await.ok();
    }
}

#[tauri::command]
async fn pc_library(core: C<'_>) -> R<serde_json::Value> {
    let (s, tracks) = core.pc_library().await?;
    Ok(json!({ "name": s.name, "tracks": tracks }))
}

#[tauri::command]
async fn server_info(core: C<'_>, srv: State<'_, Srv>) -> R<serde_json::Value> {
    let g = srv.lock().await;
    Ok(match g.as_ref() {
        Some(h) => json!({ "on": true, "url": format!("http://{}:{}", server::lan_ip().unwrap_or_else(|| "localhost".into()), h.port), "user": server::USER, "password": server_password(&core), "name": server::host_name() }),
        None => json!({ "on": false }),
    })
}

static LAST_MEDIA: std::sync::Mutex<Option<(MediaState, std::time::Instant)>> = std::sync::Mutex::new(None);

fn push_media(h: &AppHandle, s: &yokisfy_core::player::Status) {
    use tauri_plugin_yk_media::MediaExt;
    let st = match &s.track {
        Some(t) => MediaState { title: t.title.clone(), artist: t.artist.clone(), cover: t.cover.clone(), playing: s.playing, position: s.position, duration: if t.live { 0.0 } else { s.duration }, active: true },
        None => MediaState::default(),
    };
    {
        let mut g = LAST_MEDIA.lock().unwrap();
        let same = g.as_ref().map(|(p, at)| p.title == st.title && p.playing == st.playing && p.active == st.active && (p.position - st.position).abs() < 3.0 + at.elapsed().as_secs_f64()).unwrap_or(false);
        if same {
            return;
        }
        *g = Some((st.clone(), std::time::Instant::now()));
    }
    let h2 = h.clone();
    std::thread::spawn(move || h2.media().update(&st));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_yk_media::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let h = app.handle().clone();
            let core = Core::new(
                &dir,
                Arc::new(move |e| {
                    if let Event::Status(s) = &e {
                        push_media(&h, s);
                    }
                    let _ = match e {
                        Event::Status(s) => h.emit("player", s),
                        Event::Queue(q, i) => h.emit("queue", json!({ "tracks": q, "index": i })),
                        Event::Started(t) => h.emit("started", t),
                        Event::ExtSeek(p) => h.emit("ext-seek", p),
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
            let core = Arc::new(core);
            #[cfg(mobile)]
            if core.lib.setting("download_dir").map(|d| d.trim().is_empty()).unwrap_or(true) {
                let public = std::path::PathBuf::from("/storage/emulated/0/Music/Yokisfy");
                let dir = if std::fs::create_dir_all(&public).is_ok() && std::fs::write(public.join(".yk"), b"").is_ok() {
                    let _ = std::fs::remove_file(public.join(".yk"));
                    public
                } else {
                    dir.join("Music")
                };
                let _ = core.lib.set_setting("download_dir", &dir.to_string_lossy());
            }
            {
                use tauri_plugin_yk_media::MediaExt;
                let c2 = core.clone();
                app.media().listen(move |v| {
                    match v["action"].as_str().unwrap_or("") {
                        "play" => c2.player.set_playing(true),
                        "pause" => c2.player.set_playing(false),
                        "next" => c2.player.next(),
                        "prev" => c2.player.prev(),
                        "seek" => c2.player.seek(v["position"].as_f64().unwrap_or(0.0)),
                        _ => {}
                    }
                });
            }
            app.manage(core.clone());
            app.manage(Srv::default());
            let h3 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let srv = h3.state::<Srv>();
                apply_server(&core, &srv).await;
            });
            #[cfg(desktop)]
            {
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
            }
            #[cfg(mobile)]
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default()).build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search, album, home, stations, start_radio, play, toggle, set_playing, next, prev, seek, volume, enqueue, play_next, jump, queue_remove, shuffle, repeat,
            playlists, playlist, playlist_create, playlist_rename, playlist_delete, playlist_add, playlist_remove, liked, liked_ids, toggle_like,
            recent, downloads, download, delete_download, settings, set_setting, server_info, pc_library,
            import_spotify, ext_progress, ext_ended, ext_failed
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Yokisfy");
}
