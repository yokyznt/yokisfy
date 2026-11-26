use crate::{Album, AlbumDetail, Error, Format, Result, Track};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
const SONGS: &str = "EgWKAQIIAWoMEA4QChADEAQQCRAF";
const ALBUMS: &str = "EgWKAQIYAWoMEA4QChADEAQQCRAF";
const ARTISTS: &str = "EgWKAQIgAWoMEA4QChADEAQQCRAF";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub cover: Option<String>,
    pub subtitle: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub artist: Artist,
    pub top: Vec<Track>,
    pub albums: Vec<Album>,
    pub singles: Vec<Album>,
    pub related: Vec<Artist>,
}

pub async fn call(client: &Client, endpoint: &str, body: Value) -> Result<Value> {
    let mut b = json!({ "context": { "client": { "clientName": "WEB_REMIX", "clientVersion": "1.20251110.01.00", "hl": "es", "gl": "MX" } } });
    if let (Some(m), Value::Object(extra)) = (b.as_object_mut(), body) {
        m.extend(extra);
    }
    let mut last = Error::Msg("YouTube Music no respondió".into());
    for i in 0..3u64 {
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(1200 * i)).await;
        }
        let r = client
            .post(format!("https://music.youtube.com/youtubei/v1/{endpoint}?prettyPrint=false"))
            .header("User-Agent", UA)
            .header("Origin", "https://music.youtube.com")
            .header("Referer", "https://music.youtube.com/")
            .json(&b)
            .send()
            .await;
        match r {
            Ok(r) if r.status().is_success() => return Ok(r.json().await?),
            Ok(r) => last = Error::Msg(format!("YouTube Music respondió {}", r.status())),
            Err(e) => last = e.into(),
        }
    }
    Err(last)
}

fn text(v: &Value) -> String {
    v["runs"].as_array().into_iter().flatten().filter_map(|r| r["text"].as_str()).collect::<String>()
}

fn big(url: &str) -> String {
    match url.rfind('=') {
        Some(i) if url.contains("googleusercontent.com") || url.contains("ggpht.com") => format!("{}=w544-h544-l90-rj", &url[..i]),
        _ => url.to_string(),
    }
}

fn thumb(v: &Value) -> Option<String> {
    v["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
        .as_array()
        .and_then(|a| a.last())
        .and_then(|t| t["url"].as_str())
        .map(big)
}

use crate::sources::youtube::norm;

fn plays(cols: &[String]) -> f64 {
    for c in cols.iter().skip(1) {
        for p in c.split(" • ") {
            let p = p.replace('\u{a0}', " ");
            let low = p.to_lowercase();
            if !(low.contains("reproduc") || low.contains("plays") || low.contains("visualiz") || low.contains("views")) {
                continue;
            }
            let mut it = p.split_whitespace();
            let Some(n) = it.next().and_then(|n| n.replace(',', ".").parse::<f64>().ok()) else { continue };
            let mult = match it.next().unwrap_or("") {
                "K" | "k" | "mil" => 1e3,
                "M" => 1e6,
                "B" | "MM" => 1e9,
                _ => 1.0,
            };
            return n * mult;
        }
    }
    0.0
}

fn parse_len(s: &str) -> f64 {
    s.split(':').fold(0.0, |a, p| a * 60.0 + p.trim().parse::<f64>().unwrap_or(0.0))
}

fn browse_id(r: &Value) -> Option<&str> {
    r["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
}

fn runs(r: &Value) -> Vec<&Value> {
    r["flexColumns"].as_array().into_iter().flatten().flat_map(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array().into_iter().flatten()).collect()
}

fn cols(r: &Value) -> Vec<String> {
    r["flexColumns"].as_array().into_iter().flatten().map(|c| text(&c["musicResponsiveListItemFlexColumnRenderer"]["text"])).collect()
}

fn song(r: &Value, album: Option<(&str, &str, Option<&str>)>) -> Option<Track> {
    let vid = r["playlistItemData"]["videoId"]
        .as_str()
        .or_else(|| r["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"].as_str())?;
    let c = cols(r);
    let title = c.first()?.clone();
    let mut artists = Vec::new();
    let mut album_name = album.map(|a| a.1.to_string());
    let mut album_id = album.map(|a| format!("ytm:{}", a.0));
    for x in runs(r).into_iter().skip(1) {
        let Some(id) = x["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str() else { continue };
        let t = x["text"].as_str().unwrap_or_default().to_string();
        if id.starts_with("MPRE") {
            album_name.get_or_insert(t);
            album_id.get_or_insert(format!("ytm:{id}"));
        } else if id.starts_with("UC") {
            artists.push(t);
        }
    }
    if artists.is_empty() {
        let meta = c.get(1).cloned().unwrap_or_default();
        let parts: Vec<&str> = meta.split(" • ").collect();
        let first = if parts.len() > 1 && matches!(parts[0], "Canción" | "Video" | "Song") { parts.get(1) } else { parts.first() };
        if let Some(a) = first.filter(|a| !a.contains(':')) {
            artists.push(a.to_string());
        }
    }
    let mut duration = r["fixedColumns"][0]["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"][0]["text"].as_str().map(parse_len).unwrap_or(0.0);
    if duration == 0.0 {
        duration = c.iter().flat_map(|s| s.split(" • ")).filter(|p| p.contains(':') && p.chars().all(|c| c.is_ascii_digit() || c == ':')).map(parse_len).last().unwrap_or(0.0);
    }
    let cover = thumb(&r["thumbnail"]).or_else(|| album.and_then(|a| a.2.map(String::from))).or_else(|| Some(format!("https://i.ytimg.com/vi/{vid}/hqdefault.jpg")));
    let track_no = r["index"]["runs"][0]["text"].as_str().and_then(|s| s.parse().ok());
    if album.is_some() && r["musicItemRendererDisplayPolicy"].as_str() == Some("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") {
        return None;
    }
    Some(Track {
        id: format!("yt:{vid}"),
        source: "youtube".into(),
        title,
        artist: if artists.is_empty() { album.map(|a| a.1.to_string()).unwrap_or_default() } else { artists.join(", ") },
        album: album_name.unwrap_or_else(|| "YouTube Music".into()),
        album_id,
        track_no,
        duration,
        cover,
        stream_url: vid.to_string(),
        formats: vec![Format { kind: "yt".into(), label: "YouTube".into(), url: vid.to_string(), size: None }],
        license: Some("Se escucha desde YouTube Music; no se descarga".into()),
        downloadable: false,
        local_path: None,
        genre: None,
        live: false,
    })
}

fn album_item(r: &Value) -> Option<Album> {
    let id = browse_id(r).filter(|id| id.starts_with("MPRE"))?;
    let c = cols(r);
    let meta = c.get(1).cloned().unwrap_or_default();
    let parts: Vec<&str> = meta.split(" • ").collect();
    let year = parts.iter().rev().find(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit())).map(|s| s.to_string());
    let artist = runs(r).into_iter().filter(|x| x["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str().map(|b| b.starts_with("UC")).unwrap_or(false)).filter_map(|x| x["text"].as_str()).collect::<Vec<_>>().join(", ");
    Some(Album {
        id: format!("ytm:{id}"),
        source: "youtube".into(),
        title: c.first()?.clone(),
        artist: if artist.is_empty() { parts.get(1).map(|s| s.to_string()).unwrap_or_default() } else { artist },
        cover: thumb(&r["thumbnail"]),
        year,
        license: None,
    })
}

fn two_row_album(r: &Value, artist: &str) -> Option<Album> {
    let id = browse_id(r).filter(|id| id.starts_with("MPRE"))?;
    let sub = text(&r["subtitle"]);
    Some(Album {
        id: format!("ytm:{id}"),
        source: "youtube".into(),
        title: text(&r["title"]),
        artist: artist.to_string(),
        cover: thumb(&r["thumbnailRenderer"]),
        year: sub.split(" • ").find(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit())).map(String::from),
        license: None,
    })
}

fn artist_item(r: &Value) -> Option<Artist> {
    let id = browse_id(r).filter(|id| id.starts_with("UC"))?;
    let c = cols(r);
    Some(Artist {
        id: id.to_string(),
        name: c.first()?.clone(),
        cover: thumb(&r["thumbnail"]),
        subtitle: c.get(1).map(|s| s.trim_start_matches("Artista • ").to_string()),
    })
}

fn list_items(v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            if let Some(r) = m.get("musicResponsiveListItemRenderer") {
                out.push(r.clone());
                return;
            }
            for x in m.values() {
                list_items(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| list_items(x, out)),
        _ => {}
    }
}

async fn search_kind(client: &Client, q: &str, params: &str) -> Result<Vec<Value>> {
    let v = call(client, "search", json!({ "query": q, "params": params })).await?;
    let mut out = Vec::new();
    list_items(&v["contents"], &mut out);
    Ok(out)
}

pub async fn search(client: &Client, q: &str) -> Result<(Vec<Track>, Vec<Album>, Vec<Artist>)> {
    let (s, a, r) = tokio::join!(search_kind(client, q, SONGS), search_kind(client, q, ALBUMS), search_kind(client, q, ARTISTS));
    if let (Err(e), Err(_), Err(_)) = (&s, &a, &r) {
        return Err(Error::Msg(e.to_string()));
    }
    let nq = norm(q);
    let words: Vec<String> = nq.split(' ').filter(|w| !w.is_empty()).map(String::from).collect();
    let mut ranked: Vec<(bool, f64, usize, Track)> = s
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let t = song(r, None)?;
            let hay = norm(&format!("{} {} {}", t.title, t.artist, t.album));
            let hit = words.iter().all(|w| hay.split(' ').any(|h| h == w || (w.len() > 3 && h.starts_with(w.as_str()))));
            Some((hit, plays(&cols(r)), i, t))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)).then(a.2.cmp(&b.2)));
    let mut tracks: Vec<Track> = ranked.into_iter().map(|x| x.3).collect();
    let albums = a.unwrap_or_default().iter().filter_map(album_item).collect();
    let artists: Vec<Artist> = r.unwrap_or_default().iter().filter_map(artist_item).take(8).collect();
    if let Some(top) = artists.first().filter(|a| norm(&a.name) == nq) {
        if let Ok(d) = artist(client, &top.id).await {
            let seen: std::collections::HashSet<String> = d.top.iter().map(|t| t.id.clone()).collect();
            tracks.retain(|t| !seen.contains(&t.id));
            tracks.splice(0..0, d.top);
        }
    }
    Ok((tracks, albums, artists))
}

pub async fn album(client: &Client, id: &str) -> Result<AlbumDetail> {
    let v = call(client, "browse", json!({ "browseId": id })).await?;
    let tabs = &v["contents"]["twoColumnBrowseResultsRenderer"];
    let h = &tabs["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"][0]["musicResponsiveHeaderRenderer"];
    let title = text(&h["title"]);
    if title.is_empty() {
        return Err(Error::Msg("YouTube Music no devolvió el álbum".into()));
    }
    let artist = text(&h["straplineTextOne"]);
    let sub = text(&h["subtitle"]);
    let cover = thumb(&h["thumbnail"]);
    let album = Album {
        id: format!("ytm:{id}"),
        source: "youtube".into(),
        title: title.clone(),
        artist: artist.clone(),
        cover: cover.clone(),
        year: sub.split(" • ").find(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit())).map(String::from),
        license: None,
    };
    let mut items = Vec::new();
    list_items(&tabs["secondaryContents"], &mut items);
    let tracks = items.iter().filter_map(|r| song(r, Some((id, &title, cover.as_deref())))).map(|mut t| {
        if t.artist.is_empty() {
            t.artist = artist.clone();
        }
        t
    }).collect();
    Ok(AlbumDetail { album, tracks })
}

pub async fn artist(client: &Client, id: &str) -> Result<ArtistDetail> {
    let v = call(client, "browse", json!({ "browseId": id })).await?;
    let h = &v["header"]["musicImmersiveHeaderRenderer"];
    let h = if h.is_null() { &v["header"]["musicVisualHeaderRenderer"] } else { h };
    let name = text(&h["title"]);
    if name.is_empty() {
        return Err(Error::Msg("YouTube Music no devolvió el artista".into()));
    }
    let mut d = ArtistDetail {
        artist: Artist { id: id.to_string(), name: name.clone(), cover: thumb(&h["thumbnail"]), subtitle: Some(text(&h["subscriptionButton"]["subscribeButtonRenderer"]["subscriberCountText"])).filter(|s| !s.is_empty()) },
        top: vec![],
        albums: vec![],
        singles: vec![],
        related: vec![],
    };
    let secs = &v["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"];
    for s in secs.as_array().into_iter().flatten() {
        if let Some(shelf) = s.get("musicShelfRenderer") {
            d.top = shelf["contents"].as_array().into_iter().flatten().filter_map(|x| song(&x["musicResponsiveListItemRenderer"], None)).collect();
        } else if let Some(c) = s.get("musicCarouselShelfRenderer") {
            let t = text(&c["header"]["musicCarouselShelfBasicHeaderRenderer"]["title"]).to_lowercase();
            let items = c["contents"].as_array().cloned().unwrap_or_default();
            if t.starts_with("álbum") || t.starts_with("album") {
                d.albums = items.iter().filter_map(|x| two_row_album(&x["musicTwoRowItemRenderer"], &name)).collect();
            } else if t.starts_with("single") || t.starts_with("sencillo") {
                d.singles = items.iter().filter_map(|x| two_row_album(&x["musicTwoRowItemRenderer"], &name)).collect();
            } else if t.contains("gustar") || t.contains("fans") || t.contains("similar") {
                d.related = items
                    .iter()
                    .filter_map(|x| {
                        let r = &x["musicTwoRowItemRenderer"];
                        let id = browse_id(r).filter(|i| i.starts_with("UC"))?;
                        Some(Artist { id: id.to_string(), name: text(&r["title"]), cover: thumb(&r["thumbnailRenderer"]), subtitle: Some(text(&r["subtitle"])).filter(|s| !s.is_empty()) })
                    })
                    .collect();
            }
        }
    }
    Ok(d)
}

pub async fn artist_by_name(client: &Client, name: &str) -> Result<ArtistDetail> {
    let first = name.split(", ").next().unwrap_or(name).split(" y ").next().unwrap_or(name).trim();
    let found = search_kind(client, first, ARTISTS).await?;
    let list: Vec<Artist> = found.iter().filter_map(artist_item).collect();
    let want = norm(first);
    let pick = list.iter().find(|a| norm(&a.name) == want).or(list.first()).ok_or_else(|| Error::Msg(format!("No encontré al artista {first}")))?;
    artist(client, &pick.id).await
}

fn find_token(v: &Value) -> Option<String> {
    match v {
        Value::Object(m) => {
            if let Some(t) = m.get("continuationItemRenderer").and_then(|c| c["continuationEndpoint"]["continuationCommand"]["token"].as_str()) {
                return Some(t.to_string());
            }
            m.values().find_map(find_token)
        }
        Value::Array(a) => a.iter().find_map(find_token),
        _ => None,
    }
}

pub fn playlist_id(link: &str) -> Option<String> {
    let s = link.trim();
    let id = s.split("list=").nth(1)?.split(['&', '#', ' ']).next()?.to_string();
    (id.len() >= 10 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then_some(id)
}

pub async fn playlist(client: &Client, id: &str) -> Result<(String, Option<String>, Vec<Track>)> {
    let bid = if id.starts_with("VL") { id.to_string() } else { format!("VL{id}") };
    let v = call(client, "browse", json!({ "browseId": bid })).await?;
    let tc = &v["contents"]["twoColumnBrowseResultsRenderer"];
    let h = &tc["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"][0]["musicResponsiveHeaderRenderer"];
    let name = Some(text(&h["title"])).filter(|s| !s.is_empty()).unwrap_or_else(|| "Playlist de YouTube Music".into());
    let cover = thumb(&h["thumbnail"]);
    let mut items = Vec::new();
    list_items(&tc["secondaryContents"], &mut items);
    let mut token = find_token(&tc["secondaryContents"]);
    let mut pages = 0;
    while let Some(t) = token.take() {
        pages += 1;
        if pages > 50 {
            break;
        }
        let c = call(client, "browse", json!({ "continuation": t })).await?;
        let acts = &c["onResponseReceivedActions"];
        list_items(acts, &mut items);
        token = find_token(acts);
    }
    let tracks: Vec<Track> = items.iter().filter_map(|r| song(r, None)).collect();
    if tracks.is_empty() {
        return Err(Error::Msg("La playlist de YouTube Music está vacía o es privada".into()));
    }
    Ok((name, cover, tracks))
}
