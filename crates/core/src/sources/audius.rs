use crate::model::*;
use reqwest::Client;
use serde_json::Value;

const API: &str = "https://api.audius.co/v1";
const APP: &str = "Yokisfy";

async fn get(client: &Client, path: &str, params: &[(&str, &str)]) -> Result<Vec<Value>> {
    let mut q: Vec<(&str, &str)> = vec![("app_name", APP)];
    q.extend_from_slice(params);
    let v: Value = client.get(format!("{API}/{path}")).query(&q).send().await?.error_for_status()?.json().await?;
    Ok(match &v["data"] {
        Value::Array(a) => a.clone(),
        Value::Object(_) => vec![v["data"].clone()],
        _ => vec![],
    })
}

fn s(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn art(v: &Value) -> Option<String> {
    ["480x480", "1000x1000", "150x150"].iter().map(|k| s(&v[*k])).find(|x| !x.is_empty())
}

pub fn track(v: &Value) -> Option<Track> {
    let id = s(&v["id"]);
    if id.is_empty() || v["is_streamable"].as_bool() == Some(false) || v["is_stream_gated"].as_bool() == Some(true) || v["access"]["stream"].as_bool() == Some(false) || v["is_delete"].as_bool() == Some(true) {
        return None;
    }
    let stream = format!("{API}/tracks/{id}/stream?app_name={APP}");
    let downloadable = v["is_downloadable"].as_bool().unwrap_or(false) && v["is_download_gated"].as_bool() != Some(true) && v["access"]["download"].as_bool() != Some(false);
    let mut formats = Vec::new();
    if downloadable {
        let orig = s(&v["orig_filename"]).to_ascii_lowercase();
        let ext = orig.rsplit('.').next().unwrap_or("").to_string();
        let original = (v["is_original_available"].as_bool() == Some(true) && ["flac", "wav", "aiff", "mp3", "ogg", "m4a"].contains(&ext.as_str()))
            .then(|| Format { kind: ext.clone(), label: format!("Original {}", ext.to_uppercase()), url: format!("{API}/tracks/{id}/download?app_name={APP}&original=true"), size: None });
        let mp3 = Format { kind: "mp3".into(), label: "MP3 320".into(), url: stream.clone(), size: None };
        match original {
            Some(o) if o.kind == "flac" => formats.extend([o, mp3]),
            Some(o) if o.kind == "mp3" => formats.push(o),
            Some(o) => formats.extend([mp3, o]),
            None => formats.push(mp3),
        }
    }
    let user = &v["user"];
    let album = s(&v["album_backlink"]["playlist_name"]);
    Some(Track {
        id: format!("au:{id}"),
        source: "audius".into(),
        title: s(&v["title"]),
        artist: [s(&user["name"]), s(&user["handle"])].into_iter().find(|x| !x.is_empty()).unwrap_or_default(),
        album,
        album_id: None,
        track_no: None,
        duration: v["duration"].as_f64().unwrap_or(0.0),
        cover: art(&v["artwork"]),
        stream_url: stream,
        formats,
        license: Some(s(&v["license"])).filter(|x| !x.is_empty() && x != "All rights reserved"),
        downloadable,
        local_path: None,
        genre: Some(s(&v["genre"])).filter(|x| !x.is_empty()),
        live: false,
    })
}

fn playlist(v: &Value) -> Option<Album> {
    let id = s(&v["id"]);
    if id.is_empty() || v["track_count"].as_u64() == Some(0) || v["is_stream_gated"].as_bool() == Some(true) {
        return None;
    }
    Some(Album {
        id: format!("au:{id}"),
        source: "audius".into(),
        title: s(&v["playlist_name"]),
        artist: s(&v["user"]["name"]),
        cover: art(&v["artwork"]),
        year: Some(s(&v["created_at"]).chars().take(4).collect()).filter(|x: &String| !x.is_empty()),
        license: None,
    })
}

pub async fn search(client: &Client, query: &str) -> Result<SearchResults> {
    let tp = [("query", query), ("limit", "20")];
    let pp = [("query", query), ("limit", "10")];
    let (t, p) = tokio::join!(get(client, "tracks/search", &tp), get(client, "playlists/search", &pp));
    Ok(SearchResults { tracks: t?.iter().filter_map(track).collect(), albums: p.unwrap_or_default().iter().filter_map(playlist).collect(), stations: vec![], warnings: vec![] })
}

pub async fn trending(client: &Client, genre: Option<&str>) -> Result<Vec<Track>> {
    let mut p = vec![("limit", "20"), ("time", "week")];
    if let Some(g) = genre {
        p.push(("genre", g));
    }
    Ok(get(client, "tracks/trending", &p).await?.iter().filter_map(track).collect())
}

pub async fn trending_playlists(client: &Client) -> Result<Vec<Album>> {
    Ok(get(client, "playlists/trending", &[("limit", "12"), ("time", "week")]).await?.iter().filter_map(playlist).take(12).collect())
}

pub async fn album(client: &Client, id: &str) -> Result<AlbumDetail> {
    let (pa, ta) = (format!("playlists/{id}"), format!("playlists/{id}/tracks"));
    let (p, t) = tokio::join!(get(client, &pa, &[]), get(client, &ta, &[]));
    let album = p?.iter().find_map(playlist).ok_or_else(|| Error::Msg("Playlist no encontrada".into()))?;
    let mut tracks: Vec<Track> = t?.iter().filter_map(track).collect();
    for (i, x) in tracks.iter_mut().enumerate() {
        x.track_no = Some(i as u32 + 1);
        x.album = album.title.clone();
        x.album_id = Some(album.id.clone());
        if x.cover.is_none() {
            x.cover = album.cover.clone();
        }
    }
    Ok(AlbumDetail { album, tracks })
}

pub async fn by_artist(client: &Client, name: &str) -> Result<Vec<Track>> {
    let n = name.to_lowercase();
    Ok(get(client, "tracks/search", &[("query", name), ("limit", "10")]).await?.iter().filter_map(track).filter(|t| t.artist.to_lowercase() == n).collect())
}
