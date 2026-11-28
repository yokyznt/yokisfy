pub mod audio;
pub mod discover;
pub mod dsp;
pub mod folders;
pub mod foryou;
mod keys;
pub mod download;
pub mod library;
pub mod lyrics;
pub mod colors;
pub mod model;
pub mod net;
pub mod pc;
pub mod player;
pub mod server;
pub mod sources;
pub mod spectrum;
pub mod tv;

use library::Library;
pub use model::*;
use player::{Event, Listener, Player};
use reqwest::Client;
use sources::{archive, audius, jamendo, radio, spotify, youtube, ytmusic};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Core {
    pub client: Client,
    pub lib: Arc<Library>,
    pub player: Player,
    pub discover: discover::Discover,
    pub dir: PathBuf,
}

impl Core {
    pub fn new(data_dir: &Path, listener: Listener) -> Result<Self> {
        let lib = Arc::new(Library::open(&data_dir.join("yokisfy.db"))?);
        let l2 = lib.clone();
        let wrapped: Listener = Arc::new(move |e: Event| {
            if let Event::Started(t) = &e {
                let _ = l2.record_play(t);
            }
            listener(e)
        });
        let player = Player::new(wrapped);
        let client = Client::builder().user_agent(net::USER_AGENT).build()?;
        let core = Core { client, lib, player, discover: Default::default(), dir: data_dir.to_path_buf() };
        core.apply_settings();
        Ok(core)
    }

    pub fn apply_settings(&self) {
        let high = self.lib.setting("stream_quality").map(|q| q != "normal").unwrap_or(true);
        self.player.high_quality.store(high, Ordering::Relaxed);
        crate::dsp::set_normalize(self.lib.setting("normalize").as_deref() != Some("off"));
        crate::dsp::set(self.lib.setting("eq").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default());
        let xf = self.lib.setting("crossfade").and_then(|v| v.parse::<f64>().ok()).unwrap_or(5.0).clamp(0.0, 12.0);
        self.player.crossfade_ms.store((xf * 1000.0) as u32, Ordering::Relaxed);
        if let Some(v) = self.lib.setting("volume").and_then(|v| v.parse::<f32>().ok()) {
            self.player.volume(v);
        }
    }

    fn jamendo_id(&self) -> Option<String> {
        self.lib
            .setting("jamendo_client_id")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| option_env!("YOKISFY_JAMENDO_ID").map(String::from))
            .or_else(|| Some(keys::JAMENDO_CLIENT_ID.to_string()))
            .filter(|s| !s.is_empty())
    }

    pub fn has_jamendo(&self) -> bool {
        self.jamendo_id().is_some()
    }

    pub fn download_dir(&self) -> PathBuf {
        self.lib
            .setting("download_dir")
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(PathBuf::from)
                    .unwrap_or_default()
                    .join("Music")
                    .join("Yokisfy")
            })
    }

    pub async fn quick_search(&self, q: &str) -> serde_json::Value {
        let nq: Vec<String> = q.to_lowercase().split_whitespace().map(String::from).collect();
        let mut seen = std::collections::HashSet::new();
        let local: Vec<Track> = self
            .lib
            .recent(400)
            .unwrap_or_default()
            .into_iter()
            .filter(|t| {
                let hay = format!("{} {}", t.title, t.artist).to_lowercase();
                !nq.is_empty() && nq.iter().all(|w| hay.contains(w.as_str()))
            })
            .filter(|t| seen.insert(t.id.clone()))
            .take(3)
            .collect();
        let (tracks, artists) = ytmusic::quick(&self.client, q).await.unwrap_or_default();
        let mut all = local.clone();
        all.extend(self.lib.resolve(tracks).into_iter().filter(|t| seen.insert(t.id.clone())));
        all.truncate(8);
        serde_json::json!({ "q": q, "tracks": all, "local": local.len(), "artists": artists })
    }

    pub async fn search(&self, q: &str) -> SearchResults {
        let jid = self.jamendo_id();
        let (ia, jm, au, rd, yt, fr) = tokio::join!(
            archive::search(&self.client, q),
            async {
                match &jid {
                    Some(id) => Some(jamendo::search(&self.client, id, q).await),
                    None => None,
                }
            },
            audius::search(&self.client, q),
            radio::search(&self.client, q),
            ytmusic::search(&self.client, q),
            sources::free::search(&self.client, q)
        );
        let mut out = SearchResults::default();
        let mut album_lists = Vec::new();
        let mut track_lists = Vec::new();
        match jm {
            Some(Ok(r)) => {
                track_lists.push(r.tracks);
                album_lists.push(r.albums);
            }
            Some(Err(e)) => out.warnings.push(e.to_string()),
            None => {}
        }
        match au {
            Ok(r) => {
                track_lists.push(r.tracks);
                album_lists.push(r.albums);
            }
            Err(e) => out.warnings.push(format!("Audius: {e}")),
        }
        match ia {
            Ok(r) => {
                track_lists.push(r.tracks);
                album_lists.push(r.albums);
            }
            Err(e) => out.warnings.push(format!("Internet Archive: {e}")),
        }
        track_lists.push(fr);
        let free = interleave(track_lists);
        let free_albums = interleave(album_lists);
        match yt {
            Ok((t, a, ar)) => {
                out.tracks = t;
                out.albums = a;
                out.artists = ar;
            }
            Err(e) => out.warnings.push(format!("YouTube Music: {e}")),
        }
        out.tracks.extend(free);
        out.albums.extend(free_albums);
        out.stations = rd.unwrap_or_default();
        out.tracks = self.lib.resolve(out.tracks);
        out
    }

    pub async fn artist(&self, id: &str) -> Result<ytmusic::ArtistDetail> {
        let mut d = if id.starts_with("UC") && !id.contains(' ') { ytmusic::artist(&self.client, id).await? } else { ytmusic::artist_by_name(&self.client, id).await? };
        d.top = self.lib.resolve(d.top);
        Ok(d)
    }

    pub async fn stations(&self, q: &str) -> Result<Vec<Track>> {
        radio::search(&self.client, q).await
    }

    pub async fn album(&self, id: &str) -> Result<AlbumDetail> {
        let mut d = if let Some(ia) = id.strip_prefix("ia:") {
            archive::album(&self.client, ia).await?
        } else if let Some(jm) = id.strip_prefix("jm:") {
            let cid = self.jamendo_id().ok_or_else(|| Error::Msg("Jamendo no está disponible en esta versión".into()))?;
            jamendo::album_detail(&self.client, &cid, jm).await?
        } else if let Some(au) = id.strip_prefix("au:") {
            audius::album(&self.client, au).await?
        } else if let Some(yt) = id.strip_prefix("ytm:") {
            ytmusic::album(&self.client, yt).await?
        } else if let Some(k) = id.strip_prefix("mix:") {
            return self.mix(k).await;
        } else if let Some(pl) = id.strip_prefix("ytpl:") {
            ytmusic::playlist_album(&self.client, pl).await?
        } else {
            return Err(Error::Msg("Fuente desconocida".into()));
        };
        d.tracks = self.lib.resolve(d.tracks);
        Ok(d)
    }

    pub async fn home(&self) -> Vec<Section> {
        let (personal, mx) = tokio::join!(self.personal(), self.mixes());
        if !personal.is_empty() {
            let mut out = personal;
            if !mx.is_empty() {
                let at = out.len().min(1);
                out.insert(at, Section { title: "Hecho para ti".into(), albums: mx, tracks: vec![], artists: vec![] });
            }
            return out;
        }
        let jid = self.jamendo_id();
        let country = locale_country();
        let (yh, live, net, jm_tracks, jm_albums, au_tracks, au_lists, radios) = tokio::join!(
            ytmusic::home(&self.client),
            archive::browse(&self.client, "etree", 14),
            archive::browse(&self.client, "netlabels", 14),
            async {
                match &jid {
                    Some(id) => jamendo::popular(&self.client, id).await.ok(),
                    None => None,
                }
            },
            async {
                match &jid {
                    Some(id) => jamendo::new_albums(&self.client, id).await.ok(),
                    None => None,
                }
            },
            audius::trending(&self.client, None),
            audius::trending_playlists(&self.client),
            radio::popular(&self.client, country.as_deref())
        );
        let mut s = Vec::new();
        let sec = |title: &str, albums: Vec<Album>, tracks: Vec<Track>| Section { title: title.into(), albums, tracks, artists: vec![] };
        let mx = self.mixes().await;
        if !mx.is_empty() {
            s.push(sec("Hecho para ti", mx, vec![]));
        }
        if let Ok(h) = yh {
            if !h.trending.is_empty() {
                s.push(sec("Lo más escuchado ahora", vec![], self.lib.resolve(h.trending)));
            }
            if !h.artists.is_empty() {
                s.push(Section { title: "Artistas del momento".into(), albums: vec![], tracks: vec![], artists: h.artists });
            }
            if !h.releases.is_empty() {
                s.push(sec("Estrenos", h.releases, vec![]));
            }
            if !h.top.is_empty() {
                s.push(sec("Listas de éxitos", h.top, vec![]));
            }
            if !h.playlists.is_empty() {
                s.push(sec("Playlists para ti", h.playlists, vec![]));
            }
        }
        if let Ok(t) = au_tracks.map(|t| t.into_iter().take(20).collect::<Vec<_>>()).map(|t| self.lib.resolve(t)) {
            if !t.is_empty() {
                s.push(sec("Música libre en tendencia", vec![], t));
            }
        }
        if let Some(t) = jm_tracks.filter(|t| !t.is_empty()) {
            s.push(sec("Lo más escuchado en Jamendo", vec![], self.lib.resolve(t)));
        }
        if let Some(a) = au_lists.ok().filter(|a| !a.is_empty()) {
            s.push(sec("Playlists libres populares", a, vec![]));
        }
        if let Some(a) = jm_albums.filter(|a| !a.is_empty()) {
            s.push(sec("Álbumes en tendencia", a, vec![]));
        }
        if let Some(r) = radios.ok().filter(|r| !r.is_empty()) {
            s.push(sec("Radios en vivo", vec![], r.into_iter().take(12).collect()));
        }
        if let Ok(a) = net {
            s.push(sec("Netlabels", a, vec![]));
        }
        if let Ok(a) = live {
            s.push(sec("Conciertos en vivo", a, vec![]));
        }
        s
    }

    pub async fn pc_library(&self) -> Result<(pc::PcServer, Vec<Track>)> {
        let s = self.pc_find().await?;
        let t = pc::library(&self.client, &s).await?;
        Ok((s, self.lib.resolve(t)))
    }

    pub async fn pc_find(&self) -> Result<pc::PcServer> {
        let saved: Option<pc::PcServer> = self.lib.setting("pc_server").and_then(|s| serde_json::from_str(&s).ok());
        let me = server::host_name();
        if let Some(s) = saved.as_ref().filter(|s| s.name != me) {
            if pc::call(&self.client, s, "now", &serde_json::Value::Null).await.is_ok() {
                return Ok(s.clone());
            }
        }
        let code = self.lib.setting("pc_code").filter(|c| !c.is_empty()).or_else(|| saved.as_ref().map(|s| s.password.clone()).filter(|p| !p.is_empty()));
        if let Some(code) = &code {
            if let Some(mut s) = self.pc_by_code(code).await {
                if let Some(old) = saved.as_ref().filter(|o| !o.name.is_empty()) {
                    s.name = old.name.clone();
                    let _ = self.lib.set_setting("pc_server", &serde_json::to_string(&s)?);
                }
                return Ok(s);
            }
        }
        let found = tokio::task::spawn_blocking(|| pc::discover(std::time::Duration::from_millis(1800))).await.unwrap_or_default();
        for s in found.into_iter().filter(|s| s.name != me) {
            if pc::call(&self.client, &s, "now", &serde_json::Value::Null).await.is_ok() {
                let _ = self.lib.set_setting("pc_server", &serde_json::to_string(&s)?);
                let _ = self.lib.set_setting("pc_code", &s.password);
                return Ok(s);
            }
        }
        Err(Error::Msg("No encontré tu PC. Abre Yokisfy en la PC (con internet) y escribe aquí el código que aparece en sus Ajustes.".into()))
    }

    pub async fn pc_by_code(&self, code: &str) -> Option<pc::PcServer> {
        let code = pc::clean_code(code);
        if self.lib.setting("server_password").as_deref() == Some(code.as_str()) {
            return None;
        }
        let url = pc::lookup(&self.client, &code).await?;
        let s = pc::PcServer { name: "Tu PC".into(), url, user: server::USER.into(), password: code, party: false };
        pc::call(&self.client, &s, "now", &serde_json::Value::Null).await.ok()?;
        let _ = self.lib.set_setting("pc_server", &serde_json::to_string(&s).ok()?);
        Some(s)
    }

    pub async fn pc_link(&self, code: &str) -> Result<pc::PcServer> {
        let code = pc::clean_code(code);
        self.lib.set_setting("pc_code", &code)?;
        let found = tokio::task::spawn_blocking(|| pc::discover(std::time::Duration::from_millis(1500))).await.unwrap_or_default();
        for s in found.into_iter().filter(|s| s.password == code) {
            if pc::call(&self.client, &s, "now", &serde_json::Value::Null).await.is_ok() {
                let _ = self.lib.set_setting("pc_server", &serde_json::to_string(&s)?);
                return Ok(s);
            }
        }
        self.pc_by_code(&code).await.ok_or_else(|| Error::Msg("Ese código no respondió. Revisa que Yokisfy esté abierto en la PC y que el código sea el mismo.".into()))
    }

    pub async fn pc_call(&self, action: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let s = self.pc_find().await?;
        pc::call(&self.client, &s, action, &body).await
    }

    pub async fn pc_sync(&self) -> Result<serde_json::Value> {
        let s = self.pc_find().await?;
        let remote = pc::call(&self.client, &s, "state", &serde_json::Value::Null).await?;
        let liked: Vec<Track> = serde_json::from_value(remote["liked"].clone()).unwrap_or_default();
        let lists: Vec<(String, Vec<Track>)> = remote["playlists"]
            .as_array()
            .map(|a| a.iter().filter_map(|p| Some((p["name"].as_str()?.to_string(), serde_json::from_value(p["tracks"].clone()).ok()?))).collect())
            .unwrap_or_default();
        let got = self.lib.merge_sync(&liked, &lists)?;
        let theirs: Vec<Track> = serde_json::from_value(remote["downloads"].clone()).unwrap_or_default();
        let missing = self.missing(theirs);
        let mine: Vec<serde_json::Value> = self
            .lib
            .playlists()?
            .into_iter()
            .filter_map(|p| self.lib.playlist(p.id).ok())
            .map(|d| serde_json::json!({ "name": d.playlist.name, "tracks": d.tracks }))
            .collect();
        let sent = pc::call(&self.client, &s, "merge", &serde_json::json!({ "liked": self.lib.liked()?, "playlists": mine, "downloads": self.lib.downloads()? })).await?;
        Ok(serde_json::json!({ "name": s.name, "received": got, "sent": sent["added"].as_u64().unwrap_or(0), "missing": missing, "theirs": sent["downloading"].as_u64().unwrap_or(0) }))
    }

    pub fn missing(&self, theirs: Vec<Track>) -> Vec<Track> {
        let have: std::collections::HashSet<String> = self.lib.downloads().unwrap_or_default().into_iter().map(|t| t.id).collect();
        let mut seen = std::collections::HashSet::new();
        theirs.into_iter().filter(|t| t.downloadable && !have.contains(&t.id) && seen.insert(t.id.clone())).collect()
    }

    pub async fn party_hosts(&self) -> Vec<pc::PcServer> {
        let mut seen = std::collections::HashSet::new();
        tokio::task::spawn_blocking(|| pc::discover(std::time::Duration::from_millis(1500))).await.unwrap_or_default().into_iter().filter(|s| s.party && seen.insert(s.url.clone())).collect()
    }

    pub async fn party_by_code(&self, code: &str) -> Result<pc::PcServer> {
        let code = pc::clean_code(code);
        let url = pc::lookup(&self.client, &format!("fiesta{code}")).await.ok_or_else(|| Error::Msg("No encontré esa fiesta. Revisa el código y que la fiesta siga abierta.".into()))?;
        let s = pc::PcServer { name: String::new(), url: url.clone(), user: String::new(), password: String::new(), party: true };
        let now = pc::call(&self.client, &s, "party_now", &serde_json::Value::Null).await.map_err(|_| Error::Msg("La fiesta no respondió. Puede que el anfitrión la haya cerrado.".into()))?;
        if now.get("on").and_then(|v| v.as_bool()) != Some(true) {
            return Err(Error::Msg("Esa fiesta está cerrada.".into()));
        }
        Ok(pc::PcServer { name: now.get("name").and_then(|v| v.as_str()).unwrap_or("Fiesta").to_string(), ..s })
    }

    pub async fn party_call(&self, url: &str, action: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let s = pc::PcServer { name: String::new(), url: url.to_string(), user: String::new(), password: String::new(), party: true };
        pc::call(&self.client, &s, action, &body).await
    }

    pub async fn radio_for(&self, seed: &Track, exclude: &[String]) -> Vec<Track> {
        let mut pool: Vec<Track> = Vec::new();
        if seed.source == "youtube" {
            let mut seen: std::collections::HashSet<String> = exclude.iter().cloned().collect();
            seen.insert(seed.id.clone());
            let out: Vec<Track> = ytmusic::radio(&self.client, &seed.stream_url).await.unwrap_or_default().into_iter().filter(|t| seen.insert(t.id.clone())).take(25).collect();
            if !out.is_empty() {
                return self.lib.resolve(out);
            }
        }
        if seed.live {
            if let Some(g) = &seed.genre {
                pool = radio::search(&self.client, g).await.unwrap_or_default();
            }
        } else {
            let jid = self.jamendo_id();
            let similar = self.discover.similar_artists(&self.client, &seed.artist);
            let direct = async {
                match seed.source.as_str() {
                    "audius" => audius::trending(&self.client, seed.genre.as_deref()).await.unwrap_or_default(),
                    "jamendo" => match &jid {
                        Some(cid) => jamendo::similar(&self.client, cid, seed.id.trim_start_matches("jm:")).await.unwrap_or_default(),
                        None => vec![],
                    },
                    _ => vec![],
                }
            };
            let (names, mut direct) = tokio::join!(similar, direct);
            let picks: Vec<String> = names.into_iter().take(6).collect();
            let found = futures_join(picks.iter().map(|n| audius::by_artist(&self.client, n))).await;
            for v in found.into_iter().flatten() {
                pool.extend(v.into_iter().take(3));
            }
            pool.append(&mut direct);
            if pool.len() < 8 {
                if let (Some(cid), Some(g)) = (&jid, &seed.genre) {
                    pool.extend(jamendo::by_tag(&self.client, cid, &g.to_lowercase()).await.unwrap_or_default());
                }
            }
            if pool.len() < 8 {
                pool.extend(audius::trending(&self.client, seed.genre.as_deref()).await.unwrap_or_default());
            }
            if pool.len() < 8 && seed.source == "archive" {
                if let Ok(r) = archive::search(&self.client, &seed.artist).await {
                    pool.extend(r.tracks);
                }
            }
        }
        let mut seen: std::collections::HashSet<String> = exclude.iter().cloned().collect();
        seen.insert(seed.id.clone());
        let mut out: Vec<Track> = pool.into_iter().filter(|t| seen.insert(t.id.clone())).collect();
        use rand::seq::SliceRandom;
        out.shuffle(&mut rand::thread_rng());
        out.truncate(20);
        self.lib.resolve(out)
    }

    pub async fn import_spotify(&self, link: &str, on: &(dyn Fn(usize, usize) + Send + Sync)) -> Result<ImportResult> {
        if spotify::parse_link(link).is_none() {
            if let Some(pid) = ytmusic::playlist_id(link) {
                on(0, 0);
                let (name, _, tracks) = ytmusic::playlist(&self.client, &pid).await?;
                let id = self.lib.create_playlist(&name)?;
                self.lib.add_to_playlist(id, &tracks)?;
                return Ok(ImportResult { playlist_id: id, name, total: tracks.len(), youtube: tracks.len(), ..Default::default() });
            }
            return Err(Error::Msg("Pega un link de Spotify (playlist, álbum o canción) o de una playlist de YouTube / YouTube Music".into()));
        }
        let list = spotify::import(&self.client, link).await?;
        let total = list.tracks.len();
        let done = std::sync::atomic::AtomicUsize::new(0);
        on(0, total);
        let found: Vec<Option<Track>> = futures_util::stream::iter(list.tracks.iter().cloned())
            .map(|w| {
                let client = self.client.clone();
                async move { match_one(&client, &w).await }
            })
            .buffered(4)
            .inspect(|_| on(done.fetch_add(1, Ordering::Relaxed) + 1, total))
            .collect()
            .await;
        let mut res = ImportResult { name: list.name.clone(), total, ..Default::default() };
        let mut tracks = Vec::new();
        for (w, t) in list.tracks.iter().zip(found) {
            match t {
                Some(mut t) => {
                    if t.cover.is_none() {
                        t.cover = list.cover.clone();
                    }
                    if t.source == "youtube" { res.youtube += 1 } else { res.free += 1 }
                    tracks.push(t);
                }
                None => res.missing.push(format!("{} — {}", w.title, w.artists)),
            }
        }
        let id = self.lib.create_playlist(&list.name)?;
        self.lib.add_to_playlist(id, &tracks)?;
        res.playlist_id = id;
        Ok(res)
    }

    pub async fn download(&self, track: Track, kind: Option<String>, on: &(dyn Fn(download::Progress) + Send + Sync)) -> Result<Track> {
        if !track.downloadable {
            return Err(Error::Msg("El artista solo permite escuchar esta canción en streaming".into()));
        }
        let fmt = match kind.as_deref() {
            Some(k) => track.formats.iter().find(|f| f.label == k || f.kind == k),
            None => track.best_quality(),
        }
        .cloned()
        .ok_or_else(|| Error::Msg("Esta canción no permite descarga".into()))?;
        let path = download::download(&self.client, &track, &fmt, &self.download_dir(), on).await?;
        let p = path.to_string_lossy().into_owned();
        self.lib.set_local(&track, &p)?;
        {
            let (c, d, t2) = (self.client.clone(), self.dir.join("lyrics"), track.clone());
            tokio::spawn(async move {
                let _ = lyrics::find_cached(&c, &d, &t2).await;
            });
        }
        let mut t = track;
        t.local_path = Some(p);
        self.player.update(t.clone());
        Ok(t)
    }

    pub async fn palette(&self, url: &str) -> Result<Vec<String>> {
        colors::palette(&self.client, url).await
    }

    pub async fn lyrics(&self, t: &Track) -> Result<Option<lyrics::Lyrics>> {
        lyrics::find_cached(&self.client, &self.dir.join("lyrics"), t).await
    }

    pub async fn free_versions(&self, tracks: Vec<Track>, on: &(dyn Fn(usize, usize) + Send + Sync)) -> Vec<Option<Track>> {
        let total = tracks.len();
        let done = std::sync::atomic::AtomicUsize::new(0);
        let jid = self.jamendo_id();
        on(0, total);
        let out: Vec<Option<Track>> = futures_util::stream::iter(tracks)
            .map(|t| {
                let client = self.client.clone();
                let jid = jid.clone();
                async move {
                    if t.downloadable {
                        return Some(t);
                    }
                    let w = spotify::Wanted { title: t.title.clone(), artist: t.artist.split(", ").next().unwrap_or(&t.artist).to_string(), artists: t.artist.clone(), duration: t.duration };
                    free_match(&client, jid.as_deref(), &w).await
                }
            })
            .buffered(3)
            .inspect(|_| on(done.fetch_add(1, Ordering::Relaxed) + 1, total))
            .collect()
            .await;
        out.into_iter().map(|t| t.map(|t| self.lib.resolve(vec![t]).remove(0))).collect()
    }

    pub async fn format_sizes(&self, urls: Vec<String>) -> Vec<Option<u64>> {
        futures_util::stream::iter(urls)
            .map(|u| {
                let client = self.client.clone();
                async move {
                    let head = client.head(&u).header("Referer", crate::net::referer(&u).unwrap_or("")).timeout(std::time::Duration::from_secs(6)).send().await.ok().filter(|r| r.status().is_success());
                    if let Some(n) = head.and_then(|r| r.content_length()).filter(|n| *n > 50_000) {
                        return Some(n);
                    }
                    let r = client.get(&u).header("Range", "bytes=0-0").header("Referer", crate::net::referer(&u).unwrap_or("")).timeout(std::time::Duration::from_secs(6)).send().await.ok()?;
                    r.headers()
                        .get("content-range")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.rsplit('/').next())
                        .and_then(|v| v.parse().ok())
                        .filter(|n: &u64| *n > 50_000)
                }
            })
            .buffered(12)
            .collect()
            .await
    }

    pub async fn download_as(&self, original: Track, free: Track, kind: Option<String>, on: &(dyn Fn(download::Progress) + Send + Sync)) -> Result<Track> {
        let got = if free.local_path.is_some() { free } else { self.download(free, kind, on).await? };
        if original.id != got.id {
            if let Some(p) = &got.local_path {
                self.lib.set_local(&original, p)?;
                {
                    let (c, d, t2) = (self.client.clone(), self.dir.join("lyrics"), original.clone());
                    tokio::spawn(async move {
                        let _ = lyrics::find_cached(&c, &d, &t2).await;
                    });
                }
                let mut o = original;
                o.local_path = Some(p.clone());
                self.player.update(o.clone());
                return Ok(o);
            }
        }
        Ok(got)
    }

    pub fn delete_download(&self, id: &str) -> Result<()> {
        if let Some(p) = self.lib.remove_local(id)? {
            let _ = std::fs::remove_file(&p);
        }
        Ok(())
    }

    pub fn play(&self, tracks: Vec<Track>, start: usize) {
        self.player.play(self.lib.resolve(tracks), start)
    }
}

#[derive(serde::Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub playlist_id: i64,
    pub name: String,
    pub total: usize,
    pub free: usize,
    pub youtube: usize,
    pub missing: Vec<String>,
}

async fn match_one(client: &Client, w: &spotify::Wanted) -> Option<Track> {
    let artists: Vec<String> = w.artists.split(", ").map(String::from).filter(|a| !a.is_empty()).collect();
    let want = youtube::core_title(&w.title);
    let q = format!("{} {}", w.artist, want);
    let au = audius::search(client, &q).await.map(|r| r.tracks).unwrap_or_default();
    let a0 = youtube::norm(&w.artist);
    let full = youtube::norm(&w.title);
    let mut legal: Vec<Track> = au
        .into_iter()
        .filter(|t| {
            let ta = youtube::norm(&t.artist);
            youtube::overlap(&want, &youtube::norm(&t.title)) >= 0.8
                && youtube::overlap(&youtube::core_title(&t.title), &full) >= 0.6
                && !a0.is_empty()
                && (ta.contains(&a0) || (!ta.is_empty() && a0.contains(&ta)))
                && (w.duration <= 0.0 || t.duration <= 0.0 || (t.duration - w.duration).abs() <= 8.0)
        })
        .collect();
    legal.sort_by_key(|t| !t.downloadable);
    if let Some(t) = legal.into_iter().next() {
        return Some(t);
    }
    for _ in 0..2 {
        match youtube::find(client, &w.title, &artists, w.duration, None).await {
            Ok(t) => return t,
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(700)).await,
        }
    }
    None
}

async fn free_match(client: &Client, jid: Option<&str>, w: &spotify::Wanted) -> Option<Track> {
    let want = youtube::core_title(&w.title);
    let q = format!("{} {}", w.artist, want);
    let (jm, au, ia) = tokio::join!(
        async {
            match jid {
                Some(id) => jamendo::search(client, id, &q).await.map(|r| r.tracks).unwrap_or_default(),
                None => vec![],
            }
        },
        async { audius::search(client, &q).await.map(|r| r.tracks).unwrap_or_default() },
        async { archive::search(client, &q).await.map(|r| r.tracks).unwrap_or_default() }
    );
    let a0 = youtube::norm(&w.artist);
    let full = youtube::norm(&w.title);
    let mut ok: Vec<Track> = jm
        .into_iter()
        .chain(au)
        .chain(ia)
        .filter(|t| {
            let ta = youtube::norm(&t.artist);
            t.downloadable
                && youtube::overlap(&want, &youtube::norm(&t.title)) >= 0.8
                && youtube::overlap(&youtube::core_title(&t.title), &full) >= 0.6
                && !a0.is_empty()
                && (ta.contains(&a0) || (!ta.is_empty() && a0.contains(&ta)))
                && (w.duration <= 0.0 || t.duration <= 0.0 || (t.duration - w.duration).abs() <= 8.0)
        })
        .collect();
    ok.sort_by_key(|t| !t.formats.iter().any(|f| f.kind == "flac"));
    ok.into_iter().next()
}

fn interleave<T>(lists: Vec<Vec<T>>) -> Vec<T> {
    let mut its: Vec<_> = lists.into_iter().map(|l| l.into_iter()).collect();
    let mut out = Vec::new();
    loop {
        let mut any = false;
        for it in its.iter_mut() {
            if let Some(x) = it.next() {
                out.push(x);
                any = true;
            }
        }
        if !any {
            return out;
        }
    }
}

async fn futures_join<F: std::future::Future>(futs: impl Iterator<Item = F>) -> Vec<F::Output> {
    let handles: Vec<_> = futs.collect();
    let mut out = Vec::with_capacity(handles.len());
    let pinned: Vec<_> = handles.into_iter().map(Box::pin).collect();
    for f in pinned {
        out.push(f.await);
    }
    out
}

pub fn locale_country() -> Option<String> {
    let l = std::env::var("YOKISFY_COUNTRY").ok().or_else(sys_locale)?;
    let c = l.split(['-', '_']).nth(1)?.chars().take(2).collect::<String>().to_uppercase();
    (c.len() == 2).then_some(c)
}

#[cfg(windows)]
fn sys_locale() -> Option<String> {
    extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    (n > 1).then(|| String::from_utf16_lossy(&buf[..(n - 1) as usize]))
}

#[cfg(not(windows))]
fn sys_locale() -> Option<String> {
    std::env::var("LANG").ok()
}
