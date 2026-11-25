use crate::model::*;
use reqwest::Client;
use serde_json::Value;

const API: &str = "https://api.jamendo.com/v3.0";

async fn get(client: &Client, path: &str, client_id: &str, params: &[(&str, &str)]) -> Result<Vec<Value>> {
    let mut q: Vec<(&str, &str)> = vec![("client_id", client_id), ("format", "json")];
    q.extend_from_slice(params);
    let mut out = Vec::new();
    for attempt in 0..5 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(200 + 150 * attempt)).await;
        }
        let v: Value = client.get(format!("{API}/{path}/")).query(&q).send().await?.error_for_status()?.json().await?;
        if v["headers"]["status"].as_str() != Some("success") {
            let msg = v["headers"]["error_message"].as_str().unwrap_or("error desconocido");
            return Err(Error::Msg(format!("Jamendo: {msg}")));
        }
        out = v["results"].as_array().cloned().unwrap_or_default();
        if !out.is_empty() {
            break;
        }
    }
    Ok(out)
}

fn s(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn track(v: &Value) -> Option<Track> {
    let id = s(&v["id"]);
    let stream = s(&v["audio"]);
    if id.is_empty() || stream.is_empty() {
        return None;
    }
    let mut formats = Vec::new();
    let dl = s(&v["audiodownload"]);
    let downloadable = v["audiodownload_allowed"].as_bool().unwrap_or(true) && !dl.is_empty();
    if downloadable {
        if dl.contains("/flac") {
            formats.push(Format { kind: "flac".into(), label: "FLAC".into(), url: dl.clone(), size: None });
            formats.push(Format { kind: "mp3".into(), label: "MP3 VBR".into(), url: dl.replace("/flac", "/mp32"), size: None });
            formats.push(Format { kind: "ogg".into(), label: "OGG".into(), url: dl.replace("/flac", "/ogg"), size: None });
        } else {
            formats.push(Format { kind: "mp3".into(), label: "MP3".into(), url: dl, size: None });
        }
    }
    let album_id = s(&v["album_id"]);
    let cover = [s(&v["album_image"]), s(&v["image"])].into_iter().find(|x| !x.is_empty());
    Some(Track {
        id: format!("jm:{id}"),
        source: "jamendo".into(),
        title: s(&v["name"]),
        artist: s(&v["artist_name"]),
        album: s(&v["album_name"]),
        album_id: (!album_id.is_empty()).then(|| format!("jm:{album_id}")),
        track_no: s(&v["position"]).parse().ok(),
        duration: v["duration"].as_f64().or_else(|| s(&v["duration"]).parse().ok()).unwrap_or(0.0),
        cover,
        stream_url: stream,
        formats,
        license: Some(s(&v["license_ccurl"])).filter(|x| !x.is_empty()),
        downloadable,
        local_path: None,
        genre: v["musicinfo"]["tags"]["genres"][0].as_str().map(String::from),
        live: false,
    })
}

fn album(v: &Value) -> Option<Album> {
    let id = s(&v["id"]);
    if id.is_empty() {
        return None;
    }
    Some(Album {
        id: format!("jm:{id}"),
        source: "jamendo".into(),
        title: s(&v["name"]),
        artist: s(&v["artist_name"]),
        cover: Some(s(&v["image"])).filter(|x| !x.is_empty()),
        year: Some(s(&v["releasedate"]).chars().take(4).collect()).filter(|x: &String| !x.is_empty()),
        license: None,
    })
}

const TRACK_PARAMS: [(&str, &str); 4] = [("include", "licenses musicinfo"), ("audioformat", "mp32"), ("audiodlformat", "flac"), ("imagesize", "300")];

pub async fn search(client: &Client, cid: &str, query: &str) -> Result<SearchResults> {
    let mut tp = TRACK_PARAMS.to_vec();
    tp.extend([("search", query), ("limit", "20"), ("order", "relevance")]);
    let ap = [("search", query), ("limit", "12"), ("imagesize", "300")];
    let (t, a) = tokio::join!(get(client, "tracks", cid, &tp), get(client, "albums", cid, &ap));
    Ok(SearchResults {
        tracks: t?.iter().filter_map(track).collect(),
        albums: a?.iter().filter_map(album).collect(),
        stations: vec![],
        warnings: vec![],
    })
}

pub async fn popular(client: &Client, cid: &str) -> Result<Vec<Track>> {
    let mut tp = TRACK_PARAMS.to_vec();
    tp.extend([("order", "popularity_week"), ("limit", "20")]);
    Ok(get(client, "tracks", cid, &tp).await?.iter().filter_map(track).collect())
}

pub async fn new_albums(client: &Client, cid: &str) -> Result<Vec<Album>> {
    Ok(get(client, "albums", cid, &[("order", "popularity_week"), ("limit", "12"), ("imagesize", "300")]).await?.iter().filter_map(album).collect())
}

pub async fn similar(client: &Client, cid: &str, id: &str) -> Result<Vec<Track>> {
    let mut tp = TRACK_PARAMS.to_vec();
    tp.extend([("id", id), ("limit", "20")]);
    Ok(get(client, "tracks/similar", cid, &tp).await?.iter().filter_map(track).collect())
}

pub async fn by_tag(client: &Client, cid: &str, tag: &str) -> Result<Vec<Track>> {
    let mut tp = TRACK_PARAMS.to_vec();
    tp.extend([("tags", tag), ("limit", "20"), ("order", "popularity_month")]);
    Ok(get(client, "tracks", cid, &tp).await?.iter().filter_map(track).collect())
}

pub async fn album_detail(client: &Client, cid: &str, id: &str) -> Result<AlbumDetail> {
    let mut tp = TRACK_PARAMS.to_vec();
    tp.extend([("album_id", id), ("limit", "100")]);
    let ap = [("id", id), ("imagesize", "300")];
    let (a, t) = tokio::join!(get(client, "albums", cid, &ap), get(client, "tracks", cid, &tp));
    let mut tracks: Vec<Track> = t?.iter().filter_map(track).collect();
    tracks.sort_by_key(|t| t.track_no.unwrap_or(u32::MAX));
    let mut album = a?.iter().find_map(album).ok_or_else(|| Error::Msg("Álbum no encontrado".into()))?;
    album.license = tracks.first().and_then(|t| t.license.clone());
    Ok(AlbumDetail { album, tracks })
}
