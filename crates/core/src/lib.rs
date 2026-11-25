pub mod audio;
pub mod discover;
mod keys;
pub mod download;
pub mod library;
pub mod model;
pub mod net;
pub mod player;
pub mod sources;

use library::Library;
pub use model::*;
use player::{Event, Listener, Player};
use reqwest::Client;
use sources::{archive, audius, jamendo, radio};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Core {
    pub client: Client,
    pub lib: Arc<Library>,
    pub player: Player,
    pub discover: discover::Discover,
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
        let core = Core { client, lib, player, discover: Default::default() };
        core.apply_settings();
        Ok(core)
    }

    pub fn apply_settings(&self) {
        let high = self.lib.setting("stream_quality").map(|q| q != "normal").unwrap_or(true);
        self.player.high_quality.store(high, Ordering::Relaxed);
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

    pub async fn search(&self, q: &str) -> SearchResults {
        let jid = self.jamendo_id();
        let (ia, jm, au, rd) = tokio::join!(
            archive::search(&self.client, q),
            async {
                match &jid {
                    Some(id) => Some(jamendo::search(&self.client, id, q).await),
                    None => None,
                }
            },
            audius::search(&self.client, q),
            radio::search(&self.client, q)
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
        out.tracks = interleave(track_lists);
        out.albums = interleave(album_lists);
        out.stations = rd.unwrap_or_default();
        out.tracks = self.lib.resolve(out.tracks);
        out
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
        } else {
            return Err(Error::Msg("Fuente desconocida".into()));
        };
        d.tracks = self.lib.resolve(d.tracks);
        Ok(d)
    }

    pub async fn home(&self) -> Vec<Section> {
        let jid = self.jamendo_id();
        let country = locale_country();
        let (live, net, jm_tracks, jm_albums, au_tracks, au_lists, radios) = tokio::join!(
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
        let sec = |title: &str, albums: Vec<Album>, tracks: Vec<Track>| Section { title: title.into(), albums, tracks };
        if let Ok(t) = au_tracks.map(|t| t.into_iter().take(20).collect::<Vec<_>>()).map(|t| self.lib.resolve(t)) {
            if !t.is_empty() {
                s.push(sec("En tendencia esta semana", vec![], t));
            }
        }
        if let Some(t) = jm_tracks.filter(|t| !t.is_empty()) {
            s.push(sec("Lo más escuchado en Jamendo", vec![], self.lib.resolve(t)));
        }
        if let Some(a) = au_lists.ok().filter(|a| !a.is_empty()) {
            s.push(sec("Playlists populares", a, vec![]));
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

    pub async fn radio_for(&self, seed: &Track, exclude: &[String]) -> Vec<Track> {
        let mut pool: Vec<Track> = Vec::new();
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
        let mut t = track;
        t.local_path = Some(p);
        self.player.update(t.clone());
        Ok(t)
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

fn locale_country() -> Option<String> {
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
