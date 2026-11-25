use crate::{Error, Result};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Wanted {
    pub title: String,
    pub artist: String,
    pub artists: String,
    pub duration: f64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub name: String,
    pub cover: Option<String>,
    pub tracks: Vec<Wanted>,
}

pub fn parse_link(link: &str) -> Option<(String, String)> {
    let s = link.trim();
    for kind in ["playlist", "album", "track"] {
        let id = s
            .split(&format!("{kind}/"))
            .nth(1)
            .or_else(|| s.split(&format!("spotify:{kind}:")).nth(1))
            .map(|r| r.split(['?', '/', '&', '#', ' ']).next().unwrap_or("").to_string())
            .filter(|id| id.len() >= 10 && id.chars().all(|c| c.is_ascii_alphanumeric()));
        if let Some(id) = id {
            return Some((kind.to_string(), id));
        }
    }
    None
}

fn find_cover(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if s.starts_with("https://") && (s.contains("scdn.co/image") || s.contains("spotifycdn.com/image")) => Some(s.clone()),
        Value::Object(m) => m.values().find_map(find_cover),
        Value::Array(a) => a.iter().find_map(find_cover),
        _ => None,
    }
}

fn split_artists(s: &str) -> Vec<String> {
    s.replace('\u{a0}', " ").split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect()
}

pub async fn import(client: &Client, link: &str) -> Result<Imported> {
    let (kind, id) = parse_link(link).ok_or_else(|| Error::Msg("Ese link no es de una playlist, álbum o canción de Spotify".into()))?;
    let html = client
        .get(format!("https://open.spotify.com/embed/{kind}/{id}"))
        .header("User-Agent", UA)
        .header("Accept-Language", "es-MX,es;q=0.9,en;q=0.8")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let tag = "<script id=\"__NEXT_DATA__\" type=\"application/json\">";
    let start = html.find(tag).ok_or_else(|| Error::Msg("Spotify no devolvió la lista (¿es privada?)".into()))? + tag.len();
    let end = html[start..].find("</script>").map(|e| e + start).ok_or_else(|| Error::Msg("Spotify cambió su página".into()))?;
    let data: Value = serde_json::from_str(&html[start..end])?;
    let e = &data["props"]["pageProps"]["state"]["data"]["entity"];
    if e.is_null() {
        return Err(Error::Msg("Spotify no devolvió la lista (¿es privada?)".into()));
    }
    let name = e["name"].as_str().or(e["title"].as_str()).unwrap_or("Playlist de Spotify").to_string();
    let cover = find_cover(&e["coverArt"]).or_else(|| find_cover(&e["visualIdentity"])).or_else(|| find_cover(e));
    let mut tracks = Vec::new();
    if kind == "track" {
        let names: Vec<String> = e["artists"].as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect()).unwrap_or_default();
        tracks.push(Wanted {
            title: name.clone(),
            artist: names.first().cloned().unwrap_or_default(),
            artists: names.join(", "),
            duration: e["duration"].as_f64().unwrap_or(0.0) / 1000.0,
        });
    } else {
        for t in e["trackList"].as_array().into_iter().flatten() {
            let Some(title) = t["title"].as_str() else { continue };
            let names = split_artists(t["subtitle"].as_str().unwrap_or(""));
            tracks.push(Wanted {
                title: title.to_string(),
                artist: names.first().cloned().unwrap_or_default(),
                artists: names.join(", "),
                duration: t["duration"].as_f64().unwrap_or(0.0) / 1000.0,
            });
        }
    }
    if tracks.is_empty() {
        return Err(Error::Msg("La lista de Spotify está vacía o es privada".into()));
    }
    Ok(Imported { name, cover, tracks })
}
