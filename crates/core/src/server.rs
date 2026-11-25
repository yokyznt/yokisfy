use crate::library::Library;
use crate::model::*;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::any;
use axum::Router;
use md5::{Digest, Md5};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub const PORT: u16 = 45130;
pub const DISCOVERY_PORT: u16 = 45131;
pub const USER: &str = "yokisfy";
const API_VERSION: &str = "1.16.1";

struct Ctx {
    lib: Arc<Library>,
    password: String,
    name: String,
}

pub struct ServerHandle {
    pub port: u16,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    udp_stop: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(s) = self.stop.take() {
            let _ = s.send(());
        }
        self.udp_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn lan_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

pub fn host_name() -> String {
    std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "PC".into())
}

pub async fn start(lib: Arc<Library>, password: String) -> Result<ServerHandle> {
    let ctx = Arc::new(Ctx { lib, password: password.clone(), name: host_name() });
    let app = Router::new().route("/rest/{method}", any(handle)).route("/", any(|| async { "Yokisfy" })).with_state(ctx.clone());
    let listener = match tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], PORT))).await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?,
    };
    let port = listener.local_addr()?.port();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = rx.await;
        }).await;
    });
    let udp_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let us = udp_stop.clone();
    let name = ctx.name.clone();
    std::thread::Builder::new().name("yk-discovery".into()).spawn(move || discovery(port, password, name, us))?;
    Ok(ServerHandle { port, stop: Some(tx), udp_stop })
}

fn discovery(port: u16, password: String, name: String, stop: Arc<std::sync::atomic::AtomicBool>) {
    let Ok(sock) = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], DISCOVERY_PORT))) else { return };
    let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(500)));
    let mut buf = [0u8; 256];
    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok((n, from)) = sock.recv_from(&mut buf) {
            if buf[..n].starts_with(b"YOKISFY?") {
                let msg = json!({ "app": "yokisfy", "name": name, "port": port, "user": USER, "password": password, "version": env!("CARGO_PKG_VERSION") });
                let _ = sock.send_to(msg.to_string().as_bytes(), from);
            }
        }
    }
}

fn hid(prefix: &str, s: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.to_lowercase().hash(&mut h);
    format!("{prefix}{:x}", h.finish())
}

fn artist_id(t: &Track) -> String {
    hid("ar", &t.artist)
}

fn album_key(t: &Track) -> String {
    format!("{}\u{1}{}", t.artist, if t.album.is_empty() { "Sencillos" } else { &t.album })
}

fn album_id(t: &Track) -> String {
    hid("al", &album_key(t))
}

fn song_id(t: &Track) -> String {
    hid("so", &t.id)
}

fn suffix(t: &Track) -> String {
    t.local_path
        .as_deref()
        .and_then(|p| Path::new(p).extension())
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "mp3".into())
}

fn mime(s: &str) -> &'static str {
    match s {
        "flac" => "audio/flac",
        "ogg" | "oga" => "audio/ogg",
        "wav" => "audio/wav",
        "m4a" | "aac" => "audio/mp4",
        _ => "audio/mpeg",
    }
}

struct Index {
    songs: Vec<Track>,
    by_song: HashMap<String, usize>,
}

impl Index {
    fn build(lib: &Library) -> Index {
        let mut songs: Vec<Track> = lib.downloads().unwrap_or_default();
        let mut seen: std::collections::HashSet<String> = songs.iter().map(|t| t.id.clone()).collect();
        let mut extra = lib.liked().unwrap_or_default();
        for p in lib.playlists().unwrap_or_default() {
            if let Ok(d) = lib.playlist(p.id) {
                extra.extend(d.tracks);
            }
        }
        for t in lib.resolve(extra) {
            if !t.live && seen.insert(t.id.clone()) {
                songs.push(t);
            }
        }
        let by_song = songs.iter().enumerate().map(|(i, t)| (song_id(t), i)).collect();
        Index { songs, by_song }
    }

    fn local(&self) -> impl Iterator<Item = &Track> {
        self.songs.iter().filter(|t| t.local_path.as_deref().map(|p| Path::new(p).exists()).unwrap_or(false))
    }

    fn song(&self, id: &str) -> Option<&Track> {
        self.by_song.get(id).map(|i| &self.songs[*i])
    }

    fn albums(&self) -> BTreeMap<String, Vec<&Track>> {
        let mut m: BTreeMap<String, Vec<&Track>> = BTreeMap::new();
        for t in self.local() {
            m.entry(album_id(t)).or_default().push(t);
        }
        for v in m.values_mut() {
            v.sort_by_key(|t| t.track_no.unwrap_or(999));
        }
        m
    }
}

fn song_json(t: &Track) -> Value {
    let sfx = suffix(t);
    let size = t.local_path.as_deref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0);
    let mut v = json!({
        "id": song_id(t),
        "parent": album_id(t),
        "isDir": false,
        "title": t.title,
        "album": if t.album.is_empty() { "Sencillos" } else { &t.album },
        "artist": t.artist,
        "albumId": album_id(t),
        "artistId": artist_id(t),
        "coverArt": song_id(t),
        "duration": t.duration.round() as u64,
        "suffix": sfx,
        "contentType": mime(&sfx),
        "size": size,
        "type": "music",
        "isVideo": false,
        "path": format!("{}/{}/{}.{}", t.artist, t.album, t.title, sfx),
    });
    if let Some(n) = t.track_no {
        v["track"] = json!(n);
    }
    if let Some(g) = &t.genre {
        v["genre"] = json!(g);
    }
    v
}

fn album_json(id: &str, tracks: &[&Track], with_songs: bool) -> Value {
    let f = tracks[0];
    let mut v = json!({
        "id": id,
        "name": if f.album.is_empty() { "Sencillos" } else { &f.album },
        "title": if f.album.is_empty() { "Sencillos" } else { &f.album },
        "album": if f.album.is_empty() { "Sencillos" } else { &f.album },
        "artist": f.artist,
        "artistId": artist_id(f),
        "isDir": true,
        "coverArt": song_id(f),
        "songCount": tracks.len(),
        "duration": tracks.iter().map(|t| t.duration).sum::<f64>().round() as u64,
        "created": "2024-01-01T00:00:00Z",
    });
    if with_songs {
        v["song"] = Value::Array(tracks.iter().map(|t| song_json(t)).collect());
    }
    v
}

fn ok(payload: Value) -> Value {
    let mut m = Map::new();
    m.insert("status".into(), json!("ok"));
    m.insert("version".into(), json!(API_VERSION));
    m.insert("type".into(), json!("yokisfy"));
    m.insert("serverVersion".into(), json!(env!("CARGO_PKG_VERSION")));
    m.insert("openSubsonic".into(), json!(true));
    if let Value::Object(p) = payload {
        m.extend(p);
    }
    Value::Object(m)
}

fn fail(code: u32, msg: &str) -> Value {
    json!({ "status": "failed", "version": API_VERSION, "type": "yokisfy", "openSubsonic": true, "error": { "code": code, "message": msg } })
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn to_xml(name: &str, v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            out.push('<');
            out.push_str(name);
            if name == "subsonic-response" {
                out.push_str(" xmlns=\"http://subsonic.org/restapi\"");
            }
            let mut children = Vec::new();
            for (k, x) in m {
                match x {
                    Value::Object(_) | Value::Array(_) => children.push((k, x)),
                    Value::Null => {}
                    Value::String(s) => out.push_str(&format!(" {k}=\"{}\"", xml_escape(s))),
                    other => out.push_str(&format!(" {k}=\"{other}\"")),
                }
            }
            out.push('>');
            for (k, x) in children {
                match x {
                    Value::Array(a) => a.iter().for_each(|i| to_xml(k, i, out)),
                    _ => to_xml(k, x, out),
                }
            }
            out.push_str(&format!("</{name}>"));
        }
        Value::Array(a) => a.iter().for_each(|i| to_xml(name, i, out)),
        Value::String(s) => out.push_str(&format!("<{name}>{}</{name}>", xml_escape(s))),
        other => out.push_str(&format!("<{name}>{other}</{name}>")),
    }
}

fn respond(q: &HashMap<String, String>, body: Value) -> Response {
    let f = q.get("f").map(|s| s.as_str()).unwrap_or("xml");
    if f == "json" || f == "jsonp" {
        let txt = json!({ "subsonic-response": body }).to_string();
        if let (true, Some(cb)) = (f == "jsonp", q.get("callback")) {
            return ([(header::CONTENT_TYPE, "application/javascript")], format!("{cb}({txt})")).into_response();
        }
        return ([(header::CONTENT_TYPE, "application/json; charset=utf-8")], txt).into_response();
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    to_xml("subsonic-response", &body, &mut out);
    ([(header::CONTENT_TYPE, "text/xml; charset=utf-8")], out).into_response()
}

fn authed(ctx: &Ctx, q: &HashMap<String, String>) -> bool {
    if q.get("u").map(|u| u != USER).unwrap_or(true) {
        return false;
    }
    if let (Some(t), Some(s)) = (q.get("t"), q.get("s")) {
        let mut h = Md5::new();
        h.update(ctx.password.as_bytes());
        h.update(s.as_bytes());
        let d = h.finalize();
        let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
        return hex.eq_ignore_ascii_case(t);
    }
    if let Some(p) = q.get("p") {
        let plain = match p.strip_prefix("enc:") {
            Some(h) => (0..h.len() / 2).filter_map(|i| u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).ok()).map(|b| b as char).collect(),
            None => p.clone(),
        };
        return plain == ctx.password;
    }
    false
}

fn num(q: &HashMap<String, String>, k: &str, d: usize) -> usize {
    q.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

async fn handle(State(ctx): State<Arc<Ctx>>, req: Request) -> Response {
    let method = req.uri().path().rsplit('/').next().unwrap_or("").trim_end_matches(".view").to_string();
    let mut q: HashMap<String, String> = req.uri().query().map(|s| url_params(s)).unwrap_or_default();
    if req.method() == axum::http::Method::POST {
        let is_form = req.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|v| v.starts_with("application/x-www-form-urlencoded")).unwrap_or(false);
        if is_form {
            let (_, body) = req.into_parts();
            if let Ok(b) = axum::body::to_bytes(body, 1 << 20).await {
                q.extend(url_params(&String::from_utf8_lossy(&b)));
            }
            return dispatch(&ctx, &method, &q, None).await;
        }
    }
    if !authed(&ctx, &q) {
        return respond(&q, fail(40, "Usuario o contraseña incorrectos"));
    }
    dispatch(&ctx, &method, &q, Some(req)).await
}

fn url_params(s: &str) -> HashMap<String, String> {
    s.split('&')
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            let dec = |x: &str| urlencoding::decode(&x.replace('+', " ")).map(|c| c.into_owned()).ok();
            Some((dec(k)?, dec(v)?))
        })
        .collect()
}

async fn dispatch(ctx: &Arc<Ctx>, method: &str, q: &HashMap<String, String>, req: Option<Request>) -> Response {
    if req.is_none() && !authed(ctx, q) {
        return respond(q, fail(40, "Usuario o contraseña incorrectos"));
    }
    let lib = ctx.lib.clone();
    let idx = match tokio::task::spawn_blocking(move || Index::build(&lib)).await {
        Ok(i) => i,
        Err(_) => return respond(q, fail(0, "error interno")),
    };
    let id = q.get("id").cloned().unwrap_or_default();
    let body = match method {
        "ping" => ok(json!({})),
        "getLicense" => ok(json!({ "license": { "valid": true, "email": "yokisfy@localhost" } })),
        "getOpenSubsonicExtensions" => ok(json!({ "openSubsonicExtensions": [] })),
        "getMusicFolders" => ok(json!({ "musicFolders": { "musicFolder": [{ "id": 1, "name": format!("Yokisfy · {}", ctx.name) }] } })),
        "getUser" => ok(json!({ "user": { "username": USER, "scrobblingEnabled": true, "adminRole": false, "settingsRole": false, "downloadRole": true, "uploadRole": false, "playlistRole": false, "coverArtRole": false, "commentRole": false, "podcastRole": false, "streamRole": true, "jukeboxRole": false, "shareRole": false, "folder": [1] } })),
        "getArtists" | "getIndexes" => {
            let mut arts: BTreeMap<String, (String, String, std::collections::HashSet<String>)> = BTreeMap::new();
            for t in idx.local() {
                let e = arts.entry(t.artist.to_lowercase()).or_insert_with(|| (artist_id(t), t.artist.clone(), Default::default()));
                e.2.insert(album_id(t));
            }
            let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for (_, (aid, name, albums)) in arts {
                let letter = name.chars().next().map(|c| c.to_uppercase().to_string()).filter(|c| c.chars().all(char::is_alphabetic)).unwrap_or_else(|| "#".into());
                groups.entry(letter).or_default().push(json!({ "id": aid, "name": name, "albumCount": albums.len(), "coverArt": aid }));
            }
            let index: Vec<Value> = groups.into_iter().map(|(n, a)| json!({ "name": n, "artist": a })).collect();
            let key = if method == "getArtists" { "artists" } else { "indexes" };
            ok(json!({ key: { "ignoredArticles": "", "lastModified": 0, "index": index } }))
        }
        "getArtist" => {
            let albums = idx.albums();
            let mine: Vec<Value> = albums.iter().filter(|(_, v)| artist_id(v[0]) == id).map(|(k, v)| album_json(k, v, false)).collect();
            match albums.values().find(|v| artist_id(v[0]) == id) {
                Some(v) => ok(json!({ "artist": { "id": id, "name": v[0].artist, "coverArt": id, "albumCount": mine.len(), "album": mine } })),
                None => fail(70, "Artista no encontrado"),
            }
        }
        "getAlbum" | "getMusicDirectory" => match idx.albums().get(&id) {
            Some(v) if method == "getAlbum" => ok(json!({ "album": album_json(&id, v, true) })),
            Some(v) => ok(json!({ "directory": { "id": id, "name": v[0].album, "child": v.iter().map(|t| song_json(t)).collect::<Vec<_>>() } })),
            None => fail(70, "Álbum no encontrado"),
        },
        "getAlbumList" | "getAlbumList2" => {
            let albums = idx.albums();
            let mut list: Vec<(&String, &Vec<&Track>)> = albums.iter().collect();
            match q.get("type").map(|s| s.as_str()).unwrap_or("newest") {
                "random" => {
                    use rand::seq::SliceRandom;
                    list.shuffle(&mut rand::thread_rng());
                }
                "alphabeticalByArtist" => list.sort_by_key(|(_, v)| v[0].artist.to_lowercase()),
                "alphabeticalByName" => list.sort_by_key(|(_, v)| v[0].album.to_lowercase()),
                _ => {
                    let order: HashMap<String, usize> = idx.local().enumerate().map(|(i, t)| (album_id(t), i)).collect();
                    list.sort_by_key(|(k, _)| std::cmp::Reverse(order.get(*k).copied().unwrap_or(0)));
                }
            }
            let v: Vec<Value> = list.into_iter().skip(num(q, "offset", 0)).take(num(q, "size", 10).min(500)).map(|(k, v)| album_json(k, v, false)).collect();
            let key = if method == "getAlbumList2" { "albumList2" } else { "albumList" };
            ok(json!({ key: { "album": v } }))
        }
        "search2" | "search3" => {
            let needle = q.get("query").map(|s| s.trim_matches('"').to_lowercase()).unwrap_or_default();
            let hit = |s: &str| needle.is_empty() || s.to_lowercase().contains(&needle);
            let songs: Vec<Value> = idx.local().filter(|t| hit(&t.title) || hit(&t.artist) || hit(&t.album)).skip(num(q, "songOffset", 0)).take(num(q, "songCount", 20)).map(song_json).collect();
            let albums = idx.albums();
            let al: Vec<Value> = albums.iter().filter(|(_, v)| hit(&v[0].album) || hit(&v[0].artist)).skip(num(q, "albumOffset", 0)).take(num(q, "albumCount", 20)).map(|(k, v)| album_json(k, v, false)).collect();
            let mut ar: BTreeMap<String, Value> = BTreeMap::new();
            for v in albums.values().filter(|v| hit(&v[0].artist)) {
                ar.entry(artist_id(v[0])).or_insert_with(|| json!({ "id": artist_id(v[0]), "name": v[0].artist, "coverArt": artist_id(v[0]) }));
            }
            let ar: Vec<Value> = ar.into_values().skip(num(q, "artistOffset", 0)).take(num(q, "artistCount", 20)).collect();
            let key = if method == "search3" { "searchResult3" } else { "searchResult2" };
            ok(json!({ key: { "artist": ar, "album": al, "song": songs } }))
        }
        "getSong" => match idx.song(&id) {
            Some(t) => ok(json!({ "song": song_json(t) })),
            None => fail(70, "Canción no encontrada"),
        },
        "getRandomSongs" => {
            use rand::seq::SliceRandom;
            let mut v: Vec<&Track> = idx.local().collect();
            v.shuffle(&mut rand::thread_rng());
            ok(json!({ "randomSongs": { "song": v.into_iter().take(num(q, "size", 10)).map(song_json).collect::<Vec<_>>() } }))
        }
        "getStarred" | "getStarred2" => {
            let liked: std::collections::HashSet<String> = ctx.lib.liked().unwrap_or_default().into_iter().map(|t| t.id).collect();
            let songs: Vec<Value> = idx.songs.iter().filter(|t| liked.contains(&t.id) && t.source != "youtube").map(song_json).collect();
            let key = if method == "getStarred2" { "starred2" } else { "starred" };
            ok(json!({ key: { "song": songs, "album": [], "artist": [] } }))
        }
        "getPlaylists" => {
            let v: Vec<Value> = ctx.lib.playlists().unwrap_or_default().into_iter().map(|p| json!({ "id": format!("pl{}", p.id), "name": p.name, "songCount": p.count, "duration": 0, "owner": USER, "public": false, "created": "2024-01-01T00:00:00Z", "changed": "2024-01-01T00:00:00Z" })).collect();
            ok(json!({ "playlists": { "playlist": v } }))
        }
        "getPlaylist" => match id.strip_prefix("pl").and_then(|n| n.parse().ok()).and_then(|n| ctx.lib.playlist(n).ok()) {
            Some(d) => {
                let tracks = ctx.lib.resolve(d.tracks);
                let songs: Vec<Value> = tracks.iter().filter(|t| !t.live && t.source != "youtube").map(song_json).collect();
                ok(json!({ "playlist": { "id": id, "name": d.playlist.name, "songCount": songs.len(), "duration": tracks.iter().map(|t| t.duration).sum::<f64>().round() as u64, "owner": USER, "public": false, "created": "2024-01-01T00:00:00Z", "changed": "2024-01-01T00:00:00Z", "entry": songs } }))
            }
            None => fail(70, "Playlist no encontrada"),
        },
        "scrobble" => {
            if q.get("submission").map(|s| s != "false").unwrap_or(true) {
                if let Some(t) = idx.song(&id) {
                    let _ = ctx.lib.record_play(t);
                }
            }
            ok(json!({}))
        }
        "star" | "unstar" | "setRating" => ok(json!({})),
        "stream" | "download" => return media(&idx, &id, req).await,
        "getCoverArt" => return cover(&idx, &id).await,
        _ => fail(0, "Función no soportada por Yokisfy"),
    };
    respond(q, body)
}

fn find_track<'a>(idx: &'a Index, id: &str) -> Option<&'a Track> {
    if let Some(t) = idx.song(id) {
        return Some(t);
    }
    if id.starts_with("al") {
        return idx.albums().get(id).and_then(|v| v.first().copied()).and_then(|t| idx.song(&song_id(t)));
    }
    if id.starts_with("ar") {
        return idx.local().find(|t| artist_id(t) == id);
    }
    None
}

async fn media(idx: &Index, id: &str, req: Option<Request>) -> Response {
    let Some(t) = idx.song(id) else { return StatusCode::NOT_FOUND.into_response() };
    match t.local_path.as_deref().filter(|p| Path::new(p).exists()) {
        Some(p) => {
            let req = req.unwrap_or_else(|| Request::new(Body::empty()));
            let mime = mime(&suffix(t));
            match ServeFile::new_with_mime(p, &mime.parse().unwrap()).oneshot(req).await {
                Ok(r) => r.map(Body::new).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        None => Redirect::temporary(&t.stream_url).into_response(),
    }
}

async fn cover(idx: &Index, id: &str) -> Response {
    let Some(t) = find_track(idx, id) else { return StatusCode::NOT_FOUND.into_response() };
    if let Some(p) = t.local_path.clone().filter(|p| Path::new(p).exists()) {
        let pic = tokio::task::spawn_blocking(move || {
            use lofty::file::TaggedFileExt;
            let f = lofty::read_from_path(&p).ok()?;
            let tag = f.primary_tag().or_else(|| f.first_tag())?;
            let pic = tag.pictures().first()?;
            Some((pic.data().to_vec(), pic.mime_type().map(|m| m.as_str().to_string()).unwrap_or_else(|| "image/jpeg".into())))
        })
        .await
        .ok()
        .flatten();
        if let Some((data, m)) = pic {
            return ([(header::CONTENT_TYPE, m), (header::CACHE_CONTROL, "max-age=86400".into())], data).into_response();
        }
    }
    match &t.cover {
        Some(u) => Redirect::temporary(u).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
