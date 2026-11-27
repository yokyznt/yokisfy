#[cfg(windows)]
mod route;
#[cfg(desktop)]
mod presence;
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
fn boost(core: C<'_>, db: f32) -> R<f32> {
    yokisfy_core::dsp::set_boost(db);
    core.lib.set_setting("boost", &db.to_string())?;
    Ok(db)
}

#[tauri::command]
fn get_boost(core: C<'_>) -> f32 {
    let db = core.lib.setting("boost").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
    yokisfy_core::dsp::set_boost(db);
    db
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

static LAST_STATUS: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);
static LAST_AT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

fn now_position(status: &Option<serde_json::Value>) -> f64 {
    let Some(s) = status else { return 0.0 };
    let p = s.get("position").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let playing = s.get("playing").and_then(|v| v.as_bool()).unwrap_or(false) && !s.get("loading").and_then(|v| v.as_bool()).unwrap_or(false);
    let el = LAST_AT.lock().ok().and_then(|g| *g).map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
    let d = s.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let v = if playing { p + el } else { p };
    if d > 0.0 { v.min(d) } else { v }
}
static LAST_QUEUE: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

#[tauri::command]
fn snapshot() -> serde_json::Value {
    json!({ "player": LAST_STATUS.lock().ok().and_then(|g| g.clone()), "queue": LAST_QUEUE.lock().ok().and_then(|g| g.clone()) })
}

#[cfg(desktop)]
fn window_builder<'a>(app: &'a AppHandle, label: &'a str) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title("Yokisfy")
        .inner_size(1280.0, 820.0)
        .min_inner_size(360.0, 600.0)
        .decorations(false)
        .shadow(true)
        .initialization_script_for_all_frames(include_str!("../gen/android/app/src/main/assets/yk-yt.js"))
}

#[cfg(desktop)]
fn open_window(app: &AppHandle) {
    if let Some(m) = app.get_webview_window("main") {
        if !m.is_visible().unwrap_or(true) {
            let _ = m.show();
            let _ = m.set_focus();
            return;
        }
    }
    let n = (2..100).find(|i| app.get_webview_window(&format!("w{i}")).is_none()).unwrap_or(99);
    let label = format!("w{n}");
    let mut b = window_builder(app, &label);
    if let Some(src) = app.webview_windows().into_values().find(|w| w.is_focused().unwrap_or(false)).or_else(|| app.get_webview_window("main")) {
        if let (Ok(p), Ok(s)) = (src.outer_position(), src.scale_factor()) {
            let l = p.to_logical::<f64>(s);
            b = b.position(l.x + 40.0, l.y + 40.0);
        }
    }
    let _ = b.build();
}

#[tauri::command]
async fn new_window(app: AppHandle) {
    #[cfg(desktop)]
    open_window(&app);
    #[cfg(mobile)]
    let _ = app;
}

fn saved_output(core: &Core) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(&core.lib.setting("audio_out")?).ok()?;
    Some((v["id"].as_str()?.to_string(), v["name"].as_str()?.to_string()))
}

#[cfg(windows)]
fn route_youtube(app: &AppHandle) -> usize {
    let core = app.state::<Arc<Core>>();
    match saved_output(&core) {
        Some((id, _)) => route::route(Some(id)),
        None => 0,
    }
}

#[tauri::command]
fn system_outputs(app: AppHandle) {
    #[cfg(mobile)]
    {
        use tauri_plugin_yk_media::MediaExt;
        app.media().outputs();
    }
    #[cfg(desktop)]
    let _ = app;
}

#[tauri::command]
async fn audio_outputs(core: C<'_>) -> R<serde_json::Value> {
    let cur = saved_output(&core).map(|o| o.0);
    #[cfg(windows)]
    let list: Vec<serde_json::Value> = tauri::async_runtime::spawn_blocking(route::outputs).await.unwrap_or_default().into_iter().map(|o| json!({ "id": o.id, "name": o.name, "default": o.default })).collect();
    #[cfg(not(windows))]
    let list: Vec<serde_json::Value> = yokisfy_core::player::output_devices().into_iter().map(|n| json!({ "id": n, "name": n, "default": false })).collect();
    Ok(json!({ "current": cur, "list": list }))
}

#[tauri::command]
async fn set_audio_output(core: C<'_>, id: Option<String>, name: Option<String>) -> R<usize> {
    match (&id, &name) {
        (Some(i), Some(n)) => core.lib.set_setting("audio_out", &json!({ "id": i, "name": n }).to_string())?,
        _ => core.lib.set_setting("audio_out", "")?,
    }
    core.player.set_device(name.clone());
    #[cfg(windows)]
    {
        let n = tauri::async_runtime::spawn_blocking(move || route::route(id)).await.unwrap_or(0);
        return Ok(n);
    }
    #[cfg(not(windows))]
    Ok(0)
}

#[cfg(desktop)]
fn hotkeys(app: &tauri::App) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
    let ca = Some(Modifiers::CONTROL | Modifiers::ALT);
    let keys = [
        (Shortcut::new(ca, Code::Space), "toggle"),
        (Shortcut::new(ca, Code::ArrowRight), "next"),
        (Shortcut::new(ca, Code::ArrowLeft), "prev"),
        (Shortcut::new(ca, Code::ArrowUp), "up"),
        (Shortcut::new(ca, Code::ArrowDown), "down"),
        (Shortcut::new(None, Code::MediaPlayPause), "toggle"),
        (Shortcut::new(None, Code::MediaTrackNext), "next"),
        (Shortcut::new(None, Code::MediaTrackPrevious), "prev"),
    ];
    let map: Vec<(u32, &'static str)> = keys.iter().map(|(s, a)| (s.id(), *a)).collect();
    let _ = app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, sc, ev| {
                if ev.state() != ShortcutState::Pressed {
                    return;
                }
                let core = app.state::<Arc<Core>>();
                if core.lib.setting("hotkeys").as_deref() == Some("off") {
                    return;
                }
                let Some((_, a)) = map.iter().find(|(id, _)| *id == sc.id()) else { return };
                let vol = LAST_STATUS.lock().ok().and_then(|g| g.as_ref().and_then(|s| s.get("volume").and_then(|v| v.as_f64()))).unwrap_or(0.8);
                match *a {
                    "toggle" => core.player.toggle(),
                    "next" => core.player.next(),
                    "prev" => core.player.prev(),
                    "up" | "down" => {
                        let v = (vol + if *a == "up" { 0.05 } else { -0.05 }).clamp(0.0, 1.0);
                        core.player.volume(v as f32);
                        let _ = core.lib.set_setting("volume", &v.to_string());
                    }
                    _ => {}
                }
            })
            .build(),
    );
    let gs = app.global_shortcut();
    for (s, _) in keys {
        let _ = gs.register(s);
    }
}

#[tauri::command]
fn stats(core: C<'_>, days: i64, offset: i64) -> serde_json::Value {
    core.stats(days, offset)
}

#[tauri::command]
async fn pc_call(core: C<'_>, action: String, body: Option<serde_json::Value>) -> R<serde_json::Value> {
    core.pc_call(&action, body.unwrap_or_default()).await
}

#[tauri::command]
async fn pc_link(core: C<'_>, code: String) -> R<yokisfy_core::pc::PcServer> {
    core.pc_link(&code).await
}

static TUNNEL: std::sync::Mutex<Option<Arc<std::sync::atomic::AtomicBool>>> = std::sync::Mutex::new(None);
fn discord_state() -> &'static str {
    #[cfg(desktop)]
    return presence::state();
    #[cfg(not(desktop))]
    ""
}

fn discord_ok() -> bool {
    #[cfg(desktop)]
    return presence::available();
    #[cfg(not(desktop))]
    false
}

static DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn party_code(core: &Core) -> String {
    match core.lib.setting("party_code").filter(|c| c.len() == 6) {
        Some(c) => c,
        None => {
            use rand::Rng;
            const A: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
            let mut r = rand::thread_rng();
            let c: String = (0..6).map(|_| A[r.gen_range(0..A.len())] as char).collect();
            let _ = core.lib.set_setting("party_code", &c);
            c
        }
    }
}

#[tauri::command]
async fn image_data(core: C<'_>, url: String) -> R<String> {
    use base64::Engine;
    let r = core.client.get(&url).send().await.map_err(|e| yokisfy_core::Error::Msg(e.to_string()))?;
    let mime = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("image/jpeg").to_string();
    let b = r.bytes().await.map_err(|e| yokisfy_core::Error::Msg(e.to_string()))?;
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&b)))
}

#[tauri::command]
fn save_image(app: AppHandle, name: String, data: String) -> R<String> {
    use base64::Engine;
    let raw = data.split_once(',').map(|x| x.1).unwrap_or(&data);
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw).map_err(|e| yokisfy_core::Error::Msg(e.to_string()))?;
    let dir = app.path().picture_dir().or_else(|_| app.path().download_dir()).map_err(|e| yokisfy_core::Error::Msg(e.to_string()))?.join("Yokisfy");
    std::fs::create_dir_all(&dir).map_err(|e| yokisfy_core::Error::Msg(e.to_string()))?;
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == ' ' { c } else { '_' }).collect();
    let p = dir.join(format!("{safe}.png"));
    std::fs::write(&p, bytes).map_err(|e| yokisfy_core::Error::Msg(e.to_string()))?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer").arg("/select,").arg(&p).creation_flags(0x0800_0000).spawn();
    }
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn party_join(core: C<'_>, code: String) -> R<yokisfy_core::pc::PcServer> {
    core.party_by_code(&code).await
}

fn tunnel(core: &Core, port: Option<u16>) {
    if let Ok(mut g) = TUNNEL.lock() {
        if let Some(old) = g.take() {
            old.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let Some(port) = port else { return };
        if !cfg!(desktop) || core.lib.setting("internet").as_deref() == Some("off") {
            return;
        }
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        *g = Some(stop.clone());
        let pass = server_password(core);
        let lib = core.lib.clone();
        let codes: Arc<dyn Fn() -> Vec<String> + Send + Sync> = Arc::new(move || {
            let mut v = vec![pass.clone()];
            if lib.setting("party").as_deref() == Some("on") {
                if let Some(c) = lib.setting("party_code").filter(|c| !c.is_empty()) {
                    v.push(format!("fiesta{c}"));
                }
            }
            v
        });
        let dir = DATA_DIR.get().cloned().unwrap_or_default().join("bin");
        tauri::async_runtime::spawn(yokisfy_core::pc::tunnel(port, codes, core.client.clone(), stop, dir));
    }
}

#[tauri::command]
async fn pc_sync(app: AppHandle, core: C<'_>) -> R<serde_json::Value> {
    let r = core.pc_sync().await?;
    fetch_missing(&app, serde_json::from_value(r["missing"].clone()).unwrap_or_default());
    Ok(r)
}

static FETCHING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn fetch_missing(app: &AppHandle, tracks: Vec<Track>) -> usize {
    let tracks: Vec<Track> = {
        let Ok(mut g) = FETCHING.lock() else { return 0 };
        let mut out = Vec::new();
        for t in tracks {
            if !g.contains(&t.id) {
                g.push(t.id.clone());
                out.push(t);
            }
        }
        out
    };
    let n = tracks.len();
    if n == 0 {
        return 0;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let core = app.state::<Arc<Core>>().inner().clone();
        for t in tracks {
            let h = app.clone();
            let id = t.id.clone();
            let _ = core.download(t, None, &move |p: Progress| {
                let _ = h.emit("download", p);
            }).await;
            if let Ok(mut g) = FETCHING.lock() {
                g.retain(|x| x != &id);
            }
            let _ = app.emit("synced", 1);
        }
    });
    n
}

async fn autosync(app: AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    loop {
        let core = app.state::<Arc<Core>>().inner().clone();
        if core.lib.setting("autosync").as_deref() != Some("off") {
            if let Ok(r) = core.pc_sync().await {
                let n = fetch_missing(&app, serde_json::from_value(r["missing"].clone()).unwrap_or_default());
                if r["received"].as_u64().unwrap_or(0) > 0 || n > 0 {
                    let _ = app.emit("synced", r["received"].as_u64().unwrap_or(0) as usize + n);
                }
                let _ = core.lib.set_setting("last_sync", &json!({ "name": r["name"], "at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }).to_string());
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

#[tauri::command]
async fn party_hosts(core: C<'_>) -> R<Vec<yokisfy_core::pc::PcServer>> {
    Ok(core.party_hosts().await)
}

#[tauri::command]
async fn party_call(core: C<'_>, url: String, action: String, body: Option<serde_json::Value>) -> R<serde_json::Value> {
    core.party_call(&url, &action, body.unwrap_or_default()).await
}

#[tauri::command]
async fn open_mini(app: AppHandle) {
    #[cfg(desktop)]
    {
        if let Some(w) = app.get_webview_window("mini") {
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
        let mut b = WebviewWindowBuilder::new(&app, "mini", WebviewUrl::App("index.html?mini=1".into()))
            .title("Yokisfy mini")
            .inner_size(360.0, 128.0)
            .min_inner_size(300.0, 110.0)
            .decorations(false)
            .shadow(true)
            .always_on_top(true)
            .resizable(true)
            .skip_taskbar(false);
        if let Ok(Some(m)) = app.primary_monitor() {
            let s = m.scale_factor();
            let sz = m.size().to_logical::<f64>(s);
            let p = m.position().to_logical::<f64>(s);
            b = b.position(p.x + sz.width - 380.0, p.y + sz.height - 190.0);
        }
        let _ = b.build();
    }
    #[cfg(mobile)]
    let _ = app;
}

#[tauri::command]
async fn format_sizes(core: C<'_>, urls: Vec<String>) -> R<Vec<Option<u64>>> {
    Ok(core.format_sizes(urls).await)
}

#[tauri::command]
fn levels() -> [f32; 3] {
    yokisfy_core::audio::levels()
}

#[tauri::command]
fn spectrum(core: C<'_>, external: bool) -> Vec<f32> {
    if external {
        yokisfy_core::spectrum::system(saved_output(&core).map(|o| o.1))
    } else {
        yokisfy_core::spectrum::read()
    }
}

#[tauri::command]
async fn palette(core: C<'_>, url: String) -> R<Vec<String>> {
    core.palette(&url).await
}

#[tauri::command]
async fn lyrics(core: C<'_>, track: Track) -> R<Option<yokisfy_core::lyrics::Lyrics>> {
    core.lyrics(&track).await
}

#[tauri::command]
async fn free_versions(app: AppHandle, core: C<'_>, tracks: Vec<Track>) -> R<Vec<Option<Track>>> {
    let h = app.clone();
    Ok(core
        .free_versions(tracks, &move |done, total| {
            let _ = h.emit("scan", json!({ "done": done, "total": total }));
        })
        .await)
}

#[tauri::command]
async fn download_as(app: AppHandle, core: C<'_>, original: Track, free: Track, kind: Option<String>) -> R<Track> {
    let h = app.clone();
    core.download_as(original, free, kind, &move |p: Progress| {
        let _ = h.emit("download", p);
    })
    .await
}

#[tauri::command]
async fn latest_release(app: AppHandle, core: C<'_>) -> R<Option<serde_json::Value>> {
    let v: serde_json::Value = core
        .client
        .get("https://api.github.com/repos/yokyznt/yokisfy/releases/latest")
        .header("User-Agent", "Yokisfy")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(yokisfy_core::Error::from)?
        .json()
        .await
        .map_err(yokisfy_core::Error::from)?;
    let Some(tag) = v["tag_name"].as_str() else { return Ok(None) };
    let latest = tag.trim_start_matches('v').to_string();
    let parse = |s: &str| s.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    if parse(&latest) <= parse(&app.package_info().version.to_string()) {
        return Ok(None);
    }
    let apk = v["assets"].as_array().into_iter().flatten().find(|a| a["name"].as_str().map(|n| n.ends_with(".apk") && !n.contains("x86")).unwrap_or(false)).and_then(|a| a["browser_download_url"].as_str()).map(String::from);
    Ok(Some(json!({ "version": latest, "notes": v["body"], "apk": apk, "page": v["html_url"] })))
}

#[tauri::command]
async fn artist(core: C<'_>, id: String) -> R<yokisfy_core::sources::ytmusic::ArtistDetail> {
    core.artist(&id).await
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
async fn set_setting(app: AppHandle, core: C<'_>, key: String, value: String) -> R<()> {
    core.lib.set_setting(&key, &value)?;
    core.apply_settings();
    #[cfg(desktop)]
    if key == "discord" || key == "discord_app_id" {
        presence::set_app_id(core.lib.setting("discord_app_id"));
        presence::set_enabled(core.lib.setting("discord").as_deref() != Some("off"));
    }
    if key == "share_library" || key == "autosync" {
        apply_server(&app).await;
    }
    if key == "internet" {
        let port = app.state::<Srv>().lock().await.as_ref().map(|h| h.port);
        tunnel(&core, port);
    }
    if key == "party" {
        server::PARTY.store(value == "on", std::sync::atomic::Ordering::Relaxed);
        if value == "on" {
            let code = party_code(&core);
            apply_server(&app).await;
            if let Some(url) = yokisfy_core::pc::public_url() {
                let c = core.client.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = yokisfy_core::pc::announce(&c, &format!("fiesta{code}"), &url).await;
                });
            }
        }
    }
    Ok(())
}

fn remote(app: AppHandle) -> server::Remote {
    Arc::new(move |action: String, arg: serde_json::Value| {
        let app = app.clone();
        Box::pin(async move {
            let core = app.state::<Arc<Core>>().inner().clone();
            let status = LAST_STATUS.lock().ok().and_then(|g| g.clone());
            let s = |k: &str| arg.get(k).and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
            match action.as_str() {
                "now" => json!({ "player": status, "queue": LAST_QUEUE.lock().ok().and_then(|g| g.clone()) }),
                "cmd" => {
                    let v = arg.get("v").and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0.0);
                    match s("c").as_str() {
                        "toggle" => core.player.toggle(),
                        "play" => core.player.set_playing(true),
                        "pause" => core.player.set_playing(false),
                        "next" => core.player.next(),
                        "prev" => core.player.prev(),
                        "seek" => core.player.seek(v),
                        "volume" => {
                            core.player.volume(v as f32);
                            let _ = core.lib.set_setting("volume", &v.to_string());
                        }
                        "jump" => core.player.jump(v as usize),
                        _ => return json!({ "ok": false }),
                    }
                    let _ = app.emit("remote", s("c"));
                    json!({ "ok": true })
                }
                "state" => {
                    let lists: Vec<serde_json::Value> = core
                        .lib
                        .playlists()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|p| core.lib.playlist(p.id).ok())
                        .map(|d| json!({ "name": d.playlist.name, "tracks": d.tracks }))
                        .collect();
                    json!({ "liked": core.lib.liked().unwrap_or_default(), "playlists": lists, "downloads": core.lib.downloads().unwrap_or_default() })
                }
                "merge" => {
                    let liked: Vec<Track> = serde_json::from_value(arg.get("liked").cloned().unwrap_or_default()).unwrap_or_default();
                    let lists: Vec<(String, Vec<Track>)> = arg
                        .get("playlists")
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|p| Some((p.get("name")?.as_str()?.to_string(), serde_json::from_value(p.get("tracks")?.clone()).ok()?))).collect())
                        .unwrap_or_default();
                    let n = core.lib.merge_sync(&liked, &lists).unwrap_or(0);
                    if n > 0 {
                        let _ = app.emit("synced", n);
                    }
                    let theirs: Vec<Track> = serde_json::from_value(arg.get("downloads").cloned().unwrap_or_default()).unwrap_or_default();
                    let d = if core.lib.setting("autosync").as_deref() != Some("off") { fetch_missing(&app, core.missing(theirs)) } else { 0 };
                    json!({ "added": n, "downloading": d })
                }
                "party_now" => {
                    let on = server::PARTY.load(std::sync::atomic::Ordering::Relaxed);
                    let track = status.as_ref().and_then(|s| s.get("track").cloned()).filter(|t| !t.is_null());
                    let src = track.as_ref().map(|t| {
                        let id = t.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                        if t.get("source").and_then(|v| v.as_str()) == Some("youtube") {
                            json!({ "yt": t.get("stream_url").and_then(|v| v.as_str()).filter(|u| !u.contains('/')).map(String::from).unwrap_or_else(|| id.trim_start_matches("yt:").to_string()) })
                        } else {
                            json!({ "audio": format!("/party/audio?id={}", urlencoding::encode(id)) })
                        }
                    });
                    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
                    let g = |k: &str| status.as_ref().and_then(|s| s.get(k).cloned()).unwrap_or(serde_json::Value::Null);
                    json!({ "on": on, "name": server::host_name(), "track": track, "src": src, "position": now_position(&status), "playing": g("playing"), "loading": g("loading"), "duration": g("duration"), "ts": ms })
                }
                "party_track" => {
                    let id = s("id");
                    let mut all: Vec<serde_json::Value> = status.as_ref().and_then(|s| s.get("track").cloned()).into_iter().collect();
                    if let Some(q) = LAST_QUEUE.lock().ok().and_then(|g| g.clone()) {
                        all.extend(q.get("tracks").and_then(|v| v.as_array()).cloned().unwrap_or_default());
                    }
                    all.into_iter().find(|t| t.get("id").and_then(|v| v.as_str()) == Some(id.as_str())).unwrap_or(serde_json::Value::Null)
                }
                "party_search" => {
                    let r = core.search(&s("q")).await;
                    let t: Vec<Track> = r.tracks.into_iter().filter(|t| !t.live).take(25).collect();
                    json!({ "tracks": t })
                }
                "party_add" => {
                    let Some(t) = arg.get("track").cloned().and_then(|v| serde_json::from_value::<Track>(v).ok()) else { return json!({ "ok": false }) };
                    let from = s("from");
                    let from = if from.trim().is_empty() { "Alguien".to_string() } else { from.chars().take(24).collect() };
                    let idle = status.as_ref().and_then(|s| s.get("track")).map(|t| t.is_null()).unwrap_or(true);
                    if idle {
                        core.play(vec![t.clone()], 0);
                    } else {
                        core.player.enqueue(core.lib.resolve(vec![t.clone()]));
                    }
                    let _ = app.emit("party-add", json!({ "from": from, "title": t.title, "artist": t.artist }));
                    json!({ "ok": true })
                }
                _ => json!({ "error": "acción desconocida" }),
            }
        }) as futures_util::future::BoxFuture<'static, serde_json::Value>
    })
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

async fn apply_server(app: &AppHandle) {
    let core = app.state::<Arc<Core>>().inner().clone();
    let srv = app.state::<Srv>();
    server::PARTY.store(core.lib.setting("party").as_deref() == Some("on"), std::sync::atomic::Ordering::Relaxed);
    let want = cfg!(desktop) && (core.lib.setting("share_library").as_deref() != Some("off") || core.lib.setting("party").as_deref() == Some("on") || core.lib.setting("autosync").as_deref() != Some("off"));
    let mut g = srv.lock().await;
    if !want {
        *g = None;
        tunnel(&core, None);
        return;
    }
    if g.is_none() {
        *g = server::start(core.lib.clone(), server_password(&core), Some(remote(app.clone()))).await.ok();
        tunnel(&core, g.as_ref().map(|h| h.port));
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
        Some(h) => {
            let p = server_password(&core);
            json!({ "on": true, "url": format!("http://{}:{}", server::lan_ip().unwrap_or_else(|| "localhost".into()), h.port), "user": server::USER, "password": p, "code": format!("{}-{}", &p[..p.len() / 2], &p[p.len() / 2..]), "internet": core.lib.setting("internet").as_deref() != Some("off"), "name": server::host_name(), "public": yokisfy_core::pc::public_url(), "tunnel": yokisfy_core::pc::tunnel_state(), "party_code": core.lib.setting("party_code"), "discord": cfg!(desktop) && discord_ok(), "discord_state": discord_state() })
        }
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
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = if std::env::var("YOKISFY_DEV_POS").is_ok() {
        builder
    } else {
        builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { open_window(&app) });
        }))
    };
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    let builder = builder
        .on_window_event(|w, ev| match ev {
            tauri::WindowEvent::CloseRequested { api, .. } if w.label() == "main" => {
                let others = w.app_handle().webview_windows().values().any(|x| x.label() != "main");
                if others {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
            tauri::WindowEvent::Destroyed if w.label() != "main" => {
                let app = w.app_handle();
                let main_hidden = app.get_webview_window("main").map(|m| !m.is_visible().unwrap_or(true)).unwrap_or(true);
                let others = app.webview_windows().values().any(|x| x.label() != "main" && x.label() != w.label());
                if main_hidden && !others {
                    app.exit(0);
                }
            }
            _ => {}
        });
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_yk_media::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let _ = DATA_DIR.set(dir.clone());
            let h = app.handle().clone();
            let core = Core::new(
                &dir,
                Arc::new(move |e| {
                    if let Event::Status(s) = &e {
                        push_media(&h, s);
                        #[cfg(desktop)]
                        presence::update(s);
                    }
                    let _ = match e {
                        Event::Status(s) => {
                            if let Ok(mut g) = LAST_STATUS.lock() {
                                *g = serde_json::to_value(&s).ok();
                            }
                            if let Ok(mut g) = LAST_AT.lock() {
                                *g = Some(std::time::Instant::now());
                            }
                            h.emit("player", s)
                        }
                        Event::Queue(q, i) => {
                            let v = json!({ "tracks": q, "index": i });
                            if let Ok(mut g) = LAST_QUEUE.lock() {
                                *g = Some(v.clone());
                            }
                            h.emit("queue", v)
                        }
                        Event::Started(t) => {
                            #[cfg(windows)]
                            if t.source == "youtube" {
                                let h2 = h.clone();
                                std::thread::spawn(move || {
                                    for ms in [1500, 2500, 4000, 8000] {
                                        std::thread::sleep(std::time::Duration::from_millis(ms));
                                        route_youtube(&h2);
                                    }
                                });
                            }
                            h.emit("started", t)
                        }
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
            #[cfg(desktop)]
            {
                presence::set_app_id(core.lib.setting("discord_app_id"));
                presence::set_enabled(core.lib.setting("discord").as_deref() != Some("off"));
            }
            app.manage(core.clone());
            app.manage(Srv::default());
            let h3 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = &core;
                apply_server(&h3).await;
            });
            tauri::async_runtime::spawn(autosync(app.handle().clone()));
            #[cfg(desktop)]
            hotkeys(app);
            if let Some(o) = saved_output(&app.state::<Arc<Core>>()) {
                app.state::<Arc<Core>>().player.set_device(Some(o.1));
            }
            #[cfg(desktop)]
            {
                let mut w = window_builder(app.handle(), "main");
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
            recent, downloads, download, format_sizes, delete_download, settings, set_setting, server_info, pc_library,
            import_spotify, pc_link, party_join, image_data, save_image, snapshot, new_window, stats, pc_call, pc_sync, party_hosts, party_call, open_mini, audio_outputs, set_audio_output, system_outputs, spectrum, boost, get_boost, artist, latest_release, lyrics, palette, levels, free_versions, download_as, ext_progress, ext_ended, ext_failed
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Yokisfy");
}
