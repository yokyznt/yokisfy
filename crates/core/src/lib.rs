pub mod audio;
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
use sources::{archive, jamendo};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Core {
    pub client: Client,
    pub lib: Arc<Library>,
    pub player: Player,
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
        let core = Core { client, lib, player };
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
        self.lib.setting("jamendo_client_id").map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
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
        let (ia, jm) = tokio::join!(archive::search(&self.client, q), async {
            match &jid {
                Some(id) => Some(jamendo::search(&self.client, id, q).await),
                None => None,
            }
        });
        let mut out = SearchResults::default();
        match jm {
            Some(Ok(r)) => {
                out.tracks.extend(r.tracks);
                out.albums.extend(r.albums);
            }
            Some(Err(e)) => out.warnings.push(e.to_string()),
            None => out.warnings.push("Jamendo desactivado: agrega tu client_id en Ajustes".into()),
        }
        match ia {
            Ok(r) => {
                out.tracks.extend(r.tracks);
                let mut merged = Vec::new();
                let (mut a, mut b) = (std::mem::take(&mut out.albums).into_iter(), r.albums.into_iter());
                loop {
                    match (a.next(), b.next()) {
                        (None, None) => break,
                        (x, y) => merged.extend(x.into_iter().chain(y)),
                    }
                }
                out.albums = merged;
            }
            Err(e) => out.warnings.push(format!("Internet Archive: {e}")),
        }
        out.tracks = self.lib.resolve(out.tracks);
        out
    }

    pub async fn album(&self, id: &str) -> Result<AlbumDetail> {
        let mut d = if let Some(ia) = id.strip_prefix("ia:") {
            archive::album(&self.client, ia).await?
        } else if let Some(jm) = id.strip_prefix("jm:") {
            let cid = self.jamendo_id().ok_or_else(|| Error::Msg("Agrega tu client_id de Jamendo en Ajustes".into()))?;
            jamendo::album_detail(&self.client, &cid, jm).await?
        } else {
            return Err(Error::Msg("Fuente desconocida".into()));
        };
        d.tracks = self.lib.resolve(d.tracks);
        Ok(d)
    }

    pub async fn home(&self) -> Vec<Section> {
        let jid = self.jamendo_id();
        let (live, net, jm_tracks, jm_albums) = tokio::join!(
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
            }
        );
        let mut s = Vec::new();
        if let Some(t) = jm_tracks.filter(|t| !t.is_empty()) {
            s.push(Section { title: "Lo más escuchado esta semana".into(), albums: vec![], tracks: self.lib.resolve(t) });
        }
        if let Some(a) = jm_albums.filter(|a| !a.is_empty()) {
            s.push(Section { title: "Álbumes en tendencia".into(), albums: a, tracks: vec![] });
        }
        if let Ok(a) = net {
            s.push(Section { title: "Netlabels".into(), albums: a, tracks: vec![] });
        }
        if let Ok(a) = live {
            s.push(Section { title: "Conciertos en vivo".into(), albums: a, tracks: vec![] });
        }
        s
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
