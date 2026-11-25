use crate::model::*;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

const FILTER: &str = "(mediatype:audio OR mediatype:etree) AND (collection:etree OR collection:netlabels OR licenseurl:*creativecommons.org* OR licenseurl:*publicdomain*) AND NOT collection:oldtimeradio AND NOT collection:librivoxaudio AND NOT collection:podcasts AND NOT collection:audio_bookspoetry AND NOT collection:radioprograms";

fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Value::Array(a) => a.iter().find_map(text),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn clean_query(q: &str) -> String {
    q.chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '\'' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn cover(identifier: &str) -> String {
    format!("https://archive.org/services/img/{identifier}")
}

fn album_from_doc(d: &Value) -> Option<Album> {
    let id = text(&d["identifier"])?;
    let year = text(&d["year"]).or_else(|| text(&d["date"]).map(|s| s.chars().take(4).collect()));
    Some(Album {
        cover: Some(cover(&id)),
        title: text(&d["title"]).unwrap_or_else(|| id.clone()),
        artist: text(&d["creator"]).unwrap_or_else(|| "Artista desconocido".into()),
        year,
        license: text(&d["licenseurl"]),
        source: "archive".into(),
        id: format!("ia:{id}"),
    })
}

pub async fn albums(client: &Client, query: &str, rows: u32, sort: &str) -> Result<Vec<Album>> {
    let q = clean_query(query);
    let q = if q.is_empty() { FILTER.to_string() } else { format!("({q}) AND {FILTER}") };
    let rows = rows.to_string();
    let res: Value = client
        .get("https://archive.org/advancedsearch.php")
        .query(&[
            ("q", q.as_str()),
            ("fl[]", "identifier"),
            ("fl[]", "title"),
            ("fl[]", "creator"),
            ("fl[]", "licenseurl"),
            ("fl[]", "year"),
            ("fl[]", "date"),
            ("rows", rows.as_str()),
            ("sort[]", sort),
            ("output", "json"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res["response"]["docs"].as_array().map(|d| d.iter().filter_map(album_from_doc).collect()).unwrap_or_default())
}

pub async fn browse(client: &Client, collection: &str, rows: u32) -> Result<Vec<Album>> {
    let q = format!("collection:{collection} AND {FILTER}");
    let rows = rows.to_string();
    let res: Value = client
        .get("https://archive.org/advancedsearch.php")
        .query(&[
            ("q", q.as_str()),
            ("fl[]", "identifier"),
            ("fl[]", "title"),
            ("fl[]", "creator"),
            ("fl[]", "licenseurl"),
            ("fl[]", "year"),
            ("fl[]", "date"),
            ("rows", rows.as_str()),
            ("sort[]", "week desc"),
            ("output", "json"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res["response"]["docs"].as_array().map(|d| d.iter().filter_map(album_from_doc).collect()).unwrap_or_default())
}

fn format_info(f: &str) -> Option<(&'static str, &'static str, u8)> {
    Some(match f {
        "24bit Flac" => ("flac", "FLAC 24-bit", 0),
        "Flac" => ("flac", "FLAC", 1),
        "VBR MP3" => ("mp3", "MP3 VBR", 3),
        "320Kbps MP3" => ("mp3", "MP3 320", 2),
        "256Kbps MP3" => ("mp3", "MP3 256", 4),
        "MP3" => ("mp3", "MP3", 5),
        "Ogg Vorbis" => ("ogg", "OGG", 6),
        "Opus" => ("opus", "Opus", 7),
        "128Kbps MP3" => ("mp3", "MP3 128", 8),
        "64Kbps MP3" => ("mp3", "MP3 64", 9),
        "WAVE" => ("wav", "WAV", 10),
        _ => return None,
    })
}

fn parse_length(s: &str) -> f64 {
    if s.contains(':') {
        s.split(':').fold(0.0, |acc, p| acc * 60.0 + p.trim().parse::<f64>().unwrap_or(0.0))
    } else {
        s.trim().parse().unwrap_or(0.0)
    }
}

fn parse_track_no(s: &str) -> Option<u32> {
    s.split('/').next()?.trim().parse().ok()
}

fn file_url(id: &str, name: &str) -> String {
    let path = name.split('/').map(|p| urlencoding::encode(p).into_owned()).collect::<Vec<_>>().join("/");
    format!("https://archive.org/download/{id}/{path}")
}

fn stem(name: &str) -> String {
    let base = name.rsplit('/').next().unwrap_or(name);
    base.rsplit_once('.').map(|(a, _)| a).unwrap_or(base).replace('_', " ")
}

pub async fn album(client: &Client, identifier: &str) -> Result<AlbumDetail> {
    let d: Value = client
        .get(format!("https://archive.org/metadata/{identifier}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let m = &d["metadata"];
    if m.is_null() {
        return Err(Error::Msg("Álbum no encontrado".into()));
    }
    let in_col = |name: &str| match &m["collection"] {
        Value::Array(a) => a.iter().any(|c| c.as_str() == Some(name)),
        Value::String(s) => s == name,
        _ => false,
    };
    let is_etree = in_col("etree");
    let downloadable = !in_col("stream_only");
    let license = text(&m["licenseurl"]).or_else(|| is_etree.then(|| "Live Music Archive · grabación permitida por el artista".to_string()));
    let year = text(&m["year"]).or_else(|| text(&m["date"]).map(|s| s.chars().take(4).collect()));
    let album = Album {
        id: format!("ia:{identifier}"),
        source: "archive".into(),
        title: text(&m["title"]).unwrap_or_else(|| identifier.to_string()),
        artist: text(&m["creator"]).unwrap_or_else(|| "Artista desconocido".into()),
        cover: Some(cover(identifier)),
        year,
        license: license.clone(),
    };
    let files = d["files"].as_array().cloned().unwrap_or_default();
    let audio: Vec<&Value> = files
        .iter()
        .filter(|f| f["format"].as_str().and_then(format_info).is_some() && f["private"].as_str() != Some("true"))
        .collect();
    let mut groups: Vec<(String, Vec<&Value>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for f in audio.iter().filter(|f| f["source"].as_str() == Some("original")) {
        let name = f["name"].as_str().unwrap_or_default().to_string();
        index.insert(name.clone(), groups.len());
        groups.push((name, vec![*f]));
    }
    for f in audio.iter().filter(|f| f["source"].as_str() != Some("original")) {
        let orig = f["original"].as_str().unwrap_or_default();
        if let Some(&i) = index.get(orig) {
            groups[i].1.push(*f);
        } else {
            let key = stem(f["name"].as_str().unwrap_or_default());
            if let Some(&i) = index.get(&key) {
                groups[i].1.push(*f);
            } else {
                index.insert(key.clone(), groups.len());
                groups.push((key, vec![*f]));
            }
        }
    }
    let mut tracks: Vec<Track> = groups
        .into_iter()
        .filter_map(|(name, fs)| {
            let mut formats: Vec<(u8, Format)> = fs
                .iter()
                .filter_map(|f| {
                    let (kind, label, rank) = format_info(f["format"].as_str()?)?;
                    let fname = f["name"].as_str()?;
                    Some((
                        rank,
                        Format {
                            kind: kind.into(),
                            label: label.into(),
                            url: file_url(identifier, fname),
                            size: f["size"].as_str().and_then(|s| s.parse().ok()),
                        },
                    ))
                })
                .collect();
            formats.sort_by_key(|(r, _)| *r);
            formats.dedup_by(|a, b| a.1.label == b.1.label);
            let formats: Vec<Format> = formats.into_iter().map(|(_, f)| f).collect();
            let stream = formats
                .iter()
                .find(|f| f.kind == "mp3" || f.kind == "ogg")
                .or_else(|| formats.iter().find(|f| f.kind != "wav"))
                .or(formats.first())?
                .url
                .clone();
            let title = fs.iter().find_map(|f| text(&f["title"])).unwrap_or_else(|| stem(&name));
            let duration = fs.iter().filter_map(|f| f["length"].as_str()).map(parse_length).find(|l| *l > 0.0).unwrap_or(0.0);
            Some(Track {
                id: format!("ia:{identifier}/{name}"),
                source: "archive".into(),
                title,
                artist: fs.iter().find_map(|f| text(&f["creator"]).or_else(|| text(&f["artist"]))).unwrap_or_else(|| album.artist.clone()),
                album: album.title.clone(),
                album_id: Some(album.id.clone()),
                track_no: fs.iter().find_map(|f| f["track"].as_str().and_then(parse_track_no)),
                duration,
                cover: album.cover.clone(),
                stream_url: stream,
                formats,
                license: license.clone(),
                downloadable,
                local_path: None,
                genre: None,
                live: false,
            })
        })
        .collect();
    let mut nums: Vec<u32> = tracks.iter().filter_map(|t| t.track_no).collect();
    nums.sort_unstable();
    nums.dedup();
    if nums.len() == tracks.len() {
        tracks.sort_by_key(|t| t.track_no);
    } else {
        tracks.sort_by(|a, b| a.id.cmp(&b.id));
        for (i, t) in tracks.iter_mut().enumerate() {
            t.track_no = Some(i as u32 + 1);
        }
    }
    Ok(AlbumDetail { album, tracks })
}

pub async fn search(client: &Client, query: &str) -> Result<SearchResults> {
    let words: Vec<String> = clean_query(query).to_lowercase().split_whitespace().map(String::from).collect();
    let mut albums = albums(client, query, 18, "downloads desc").await?;
    if albums.is_empty() && words.len() > 1 {
        let strong: Vec<&str> = words.iter().map(|w| w.as_str()).filter(|w| w.chars().count() > 2 && !w.chars().all(|c| c.is_ascii_digit())).collect();
        if !strong.is_empty() {
            albums = self::albums(client, &strong.join(" "), 18, "downloads desc").await?;
        }
    }
    let futs = albums.iter().take(4).map(|a| async move {
        tokio::time::timeout(std::time::Duration::from_secs(6), album(client, a.id.trim_start_matches("ia:")))
            .await
            .map_err(|_| Error::Msg("tiempo agotado".into()))
            .and_then(|r| r)
    });
    let details = futures_util::future::join_all(futs).await;
    let mut matched = Vec::new();
    let mut fallback = Vec::new();
    for d in details.into_iter().flatten() {
        for (i, t) in d.tracks.into_iter().enumerate() {
            let hay = format!("{} {}", t.title, t.artist).to_lowercase();
            if !words.is_empty() && words.iter().all(|w| hay.contains(w)) {
                matched.push(t);
            } else if i < 2 {
                fallback.push(t);
            }
        }
    }
    matched.truncate(20);
    if matched.len() < 8 {
        matched.extend(fallback.into_iter().take(8 - matched.len()));
    }
    Ok(SearchResults { tracks: matched, albums, stations: vec![], warnings: vec![] })
}
