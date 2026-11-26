use yokisfy_core::sources::{audius, spotify, youtube, ytmusic};

fn client() -> reqwest::Client {
    reqwest::Client::builder().user_agent(yokisfy_core::net::USER_AGENT).build().unwrap()
}

#[tokio::test]
async fn ytmusic_busqueda_album_artista_radio() {
    let c = client();
    let (tracks, albums, artists) = ytmusic::search(&c, "bad bunny").await.expect("búsqueda");
    assert!(tracks.len() >= 5, "canciones: {}", tracks.len());
    assert!(!albums.is_empty(), "sin álbumes");
    assert!(!artists.is_empty(), "sin artistas");
    let a = ytmusic::artist(&c, &artists[0].id).await.expect("artista");
    assert!(!a.top.is_empty() && !a.albums.is_empty(), "artista vacío");
    let d = ytmusic::album(&c, albums[0].id.trim_start_matches("ytm:")).await.expect("álbum");
    assert!(d.tracks.len() >= 3, "álbum con {} pistas", d.tracks.len());
    let r = ytmusic::radio(&c, &tracks[0].stream_url).await.expect("radio");
    assert!(r.len() >= 10, "radio con {}", r.len());
}

#[tokio::test]
async fn ytmusic_inicio() {
    let h = ytmusic::home(&client()).await.expect("inicio");
    assert!(!h.trending.is_empty(), "sin tendencias");
    assert!(!h.releases.is_empty() || !h.top.is_empty(), "sin estrenos ni listas");
}

#[tokio::test]
async fn ytmusic_version_actual() {
    assert!(ytmusic::refresh_version(&client()).await.is_some(), "no se pudo leer la versión de YouTube Music");
}

#[tokio::test]
async fn spotify_importa() {
    let c = client();
    let l = spotify::import(&c, "https://open.spotify.com/playlist/37i9dQZF1DX4UtSsGT1Sbe").await.expect("spotify");
    assert!(l.tracks.len() > 100, "solo {} canciones (falló la paginación)", l.tracks.len());
    let artists: Vec<String> = l.tracks[0].artists.split(", ").map(String::from).collect();
    assert!(youtube::find(&c, &l.tracks[0].title, &artists, l.tracks[0].duration, None).await.expect("youtube").is_some(), "no encontró la canción en YouTube");
}

#[tokio::test]
async fn audius_responde() {
    let r = audius::search(&client(), "lofi").await.expect("audius");
    assert!(!r.tracks.is_empty(), "audius vacío");
}

#[tokio::test]
async fn letras_responden() {
    let c = client();
    let r = c.get("https://lrclib.net/api/get?artist_name=Bad%20Bunny&track_name=DtMF&duration=237").header("User-Agent", "Yokisfy").send().await.expect("lrclib");
    assert!(r.status().is_success(), "lrclib {}", r.status());
}
