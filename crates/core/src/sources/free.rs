use crate::model::*;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

fn s(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn secs(ps: &str) -> f64 {
    ps.split(':').fold(0.0, |a, p| a * 60.0 + p.trim().parse::<f64>().unwrap_or(0.0))
}

fn base(id: String, source: &str, title: String, artist: String, url: String, kind: &str, size: Option<u64>, duration: f64, license: Option<String>, cover: Option<String>) -> Track {
    Track {
        id,
        source: source.into(),
        title,
        artist,
        album: String::new(),
        album_id: None,
        track_no: None,
        duration,
        cover,
        stream_url: url.clone(),
        formats: vec![Format { kind: kind.into(), label: kind.to_uppercase(), url, size }],
        license,
        downloadable: true,
        local_path: None,
        genre: None,
        live: false,
    }
}

fn ccmixter_track(v: &Value) -> Option<Track> {
    let id = s(&v["upload_id"]);
    let files = v["files"].as_array()?;
    let f = files.iter().find(|f| s(&f["download_url"]).to_lowercase().ends_with(".mp3")).or_else(|| files.iter().find(|f| !s(&f["download_url"]).is_empty()))?;
    let url = s(&f["download_url"]);
    let kind = url.rsplit('.').next().unwrap_or("mp3").to_lowercase();
    if id.is_empty() || !["mp3", "flac", "ogg", "wav"].contains(&kind.as_str()) {
        return None;
    }
    let artist = [s(&v["user_real_name"]), s(&v["user_name"])].into_iter().find(|x| !x.is_empty()).unwrap_or_default();
    Some(base(format!("cc:{id}"), "ccmixter", s(&v["upload_name"]), artist, url, &kind, f["file_rawsize"].as_u64(), secs(&s(&f["file_format_info"]["ps"])), Some(s(&v["license_name"])).filter(|x| !x.is_empty()), None))
}

pub async fn ccmixter(client: &Client, q: &str) -> Result<Vec<Track>> {
    let v: Value = client
        .get("https://ccmixter.org/api/query")
        .query(&[("f", "json"), ("search", q), ("searchp", q), ("search_type", "all"), ("sort", "rank"), ("limit", "15")])
        .timeout(Duration::from_secs(12))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(v.as_array().map(|a| a.iter().filter_map(ccmixter_track).collect()).unwrap_or_default())
}

fn commons_track(v: &Value) -> Option<Track> {
    let id = s(&v["id"]);
    let url = s(&v["url"]);
    let kind = s(&v["filetype"]).to_lowercase();
    if id.is_empty() || url.is_empty() || !["mp3", "ogg", "oga", "flac", "wav"].contains(&kind.as_str()) {
        return None;
    }
    let duration = v["duration"].as_f64().unwrap_or(0.0) / 1000.0;
    if duration > 0.0 && duration < 45.0 {
        return None;
    }
    let lic = s(&v["license"]);
    let license = match lic.as_str() {
        "" => None,
        "pdm" | "cc0" => Some("Dominio público".to_string()),
        l => Some(format!("CC {} {}", l.to_uppercase(), s(&v["license_version"])).trim().to_string()),
    };
    let title = s(&v["title"]).trim_end_matches(".ogg").trim_end_matches(".flac").trim_end_matches(".mp3").replace('_', " ");
    Some(base(format!("wm:{id}"), "commons", title, s(&v["creator"]), url, &kind, v["filesize"].as_u64(), duration, license, None))
}

pub async fn commons(client: &Client, q: &str) -> Result<Vec<Track>> {
    let v: Value = client
        .get("https://api.openverse.org/v1/audio/")
        .query(&[("q", q), ("source", "wikimedia_audio"), ("page_size", "15")])
        .timeout(Duration::from_secs(12))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(v["results"].as_array().map(|a| a.iter().filter_map(commons_track).collect()).unwrap_or_default())
}

pub async fn search(client: &Client, q: &str) -> Vec<Track> {
    let (a, b) = tokio::join!(ccmixter(client, q), commons(client, q));
    let (a, b) = (a.unwrap_or_default(), b.unwrap_or_default());
    let mut out = Vec::new();
    let mut ia = a.into_iter();
    let mut ib = b.into_iter();
    loop {
        let (x, y) = (ia.next(), ib.next());
        if x.is_none() && y.is_none() {
            break;
        }
        out.extend(x);
        out.extend(y);
    }
    out
}
