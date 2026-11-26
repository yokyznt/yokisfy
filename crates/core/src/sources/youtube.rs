use crate::{Error, Format, Result, Track};
use reqwest::Client;
use serde_json::Value;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
const BAD: [&str; 16] = ["live", "en vivo", "cover", "karaoke", "hour", "hora", "loop", "slowed", "reverb", "sped up", "nightcore", "8d", "instrumental", "remix", "acustico", "acoustic"];

#[derive(Clone, Debug)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub album: Option<String>,
}

pub fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            '&' => ' ',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn core_title(t: &str) -> String {
    let cut = t.find(['(', '[']).unwrap_or(t.len());
    let t = &t[..cut];
    let t = t.split(" - ").next().unwrap_or(t);
    norm(t)
}

pub fn overlap(want: &str, have: &str) -> f64 {
    let words: Vec<&str> = want.split(' ').filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return 0.0;
    }
    let have: Vec<&str> = have.split(' ').collect();
    words.iter().filter(|w| have.contains(w)).count() as f64 / words.len() as f64
}

fn parse_len(s: &str) -> f64 {
    s.split(':').fold(0.0, |a, p| a * 60.0 + p.trim().parse::<f64>().unwrap_or(0.0))
}

fn walk(v: &Value, out: &mut Vec<Video>) {
    match v {
        Value::Object(m) => {
            if let Some(r) = m.get("videoRenderer") {
                if let Some(id) = r["videoId"].as_str() {
                    let duration = r["lengthText"]["simpleText"].as_str().map(parse_len).unwrap_or(0.0);
                    if duration > 0.0 {
                        out.push(Video {
                            id: id.to_string(),
                            title: r["title"]["runs"][0]["text"].as_str().unwrap_or_default().to_string(),
                            channel: r["ownerText"]["runs"][0]["text"].as_str().unwrap_or_default().to_string(),
                            duration,
                            album: None,
                        });
                    }
                }
                return;
            }
            for x in m.values() {
                walk(x, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                walk(x, out);
            }
        }
        _ => {}
    }
}

pub async fn videos(client: &Client, q: &str) -> Result<Vec<Video>> {
    let url = format!("https://www.youtube.com/results?search_query={}&sp=EgIQAQ%3D%3D", urlencoding::encode(q));
    let html = client
        .get(url)
        .header("User-Agent", UA)
        .header("Accept-Language", "es-MX,es;q=0.9,en;q=0.8")
        .header("Cookie", "CONSENT=YES+1; SOCS=CAI")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let key = "var ytInitialData = ";
    let start = html.find(key).ok_or_else(|| Error::Msg("YouTube no respondió la búsqueda".into()))? + key.len();
    let end = html[start..].find(";</script>").map(|e| e + start).ok_or_else(|| Error::Msg("YouTube cambió su página".into()))?;
    let data: Value = serde_json::from_str(&html[start..end])?;
    let mut out = Vec::new();
    walk(&data, &mut out);
    Ok(out)
}

fn walk_music(v: &Value, out: &mut Vec<Video>) {
    match v {
        Value::Object(m) => {
            if let Some(r) = m.get("musicResponsiveListItemRenderer") {
                let id = r["playlistItemData"]["videoId"].as_str().or_else(|| r["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"].as_str());
                let cols: Vec<String> = r["flexColumns"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array().into_iter().flatten().filter_map(|x| x["text"].as_str()).collect::<String>())
                    .collect();
                if let (Some(id), Some(title), Some(meta)) = (id, cols.first(), cols.get(1)) {
                    let parts: Vec<&str> = meta.split(" • ").collect();
                    let duration = parts.last().filter(|p| p.contains(':')).map(|p| parse_len(p)).unwrap_or(0.0);
                    if duration > 0.0 && parts.len() >= 2 {
                        out.push(Video {
                            id: id.to_string(),
                            title: title.clone(),
                            channel: format!("{} - Topic", parts[0]),
                            duration,
                            album: (parts.len() >= 3).then(|| parts[parts.len() - 2].to_string()),
                        });
                    }
                }
                return;
            }
            for x in m.values() {
                walk_music(x, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                walk_music(x, out);
            }
        }
        _ => {}
    }
}

pub async fn music(client: &Client, q: &str) -> Result<Vec<Video>> {
    let body = serde_json::json!({
        "context": { "client": { "clientName": "WEB_REMIX", "clientVersion": "1.20251110.01.00", "hl": "es", "gl": "MX" } },
        "query": q,
        "params": "EgWKAQIIAWoKEAkQBRAKEAMQBA%3D%3D"
    });
    let mut last = Error::Msg("YouTube Music no respondió".into());
    for i in 0..3 {
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(1500 * i)).await;
        }
        let r = client
            .post("https://music.youtube.com/youtubei/v1/search?prettyPrint=false")
            .header("User-Agent", UA)
            .header("Origin", "https://music.youtube.com")
            .header("Referer", "https://music.youtube.com/")
            .json(&body)
            .send()
            .await;
        match r {
            Ok(r) if r.status().is_success() => {
                let v: Value = r.json().await?;
                let mut out = Vec::new();
                walk_music(&v, &mut out);
                return Ok(out);
            }
            Ok(r) => last = Error::Msg(format!("YouTube Music respondió {}", r.status())),
            Err(e) => last = e.into(),
        }
    }
    Err(last)
}

fn channel_artist(c: &str) -> String {
    c.trim_end_matches(" - Topic").trim_end_matches("VEVO").trim().to_string()
}

pub fn to_track(v: &Video, alts: &[Video], title: Option<&str>, artist: Option<&str>, album: Option<&str>) -> Track {
    let mut formats = vec![Format { kind: "yt".into(), label: "YouTube".into(), url: v.id.clone(), size: None }];
    formats.extend(alts.iter().filter(|a| a.id != v.id).map(|a| Format { kind: "yt".into(), label: "YouTube".into(), url: a.id.clone(), size: None }));
    Track {
        id: format!("yt:{}", v.id),
        source: "youtube".into(),
        title: title.map(String::from).unwrap_or_else(|| v.title.clone()),
        artist: artist.map(String::from).unwrap_or_else(|| channel_artist(&v.channel)),
        album: album.map(String::from).or_else(|| v.album.clone()).unwrap_or_else(|| "YouTube".into()),
        album_id: None,
        track_no: None,
        duration: v.duration,
        cover: Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", v.id)),
        stream_url: v.id.clone(),
        formats,
        license: None,
        downloadable: false,
        local_path: None,
        genre: None,
        live: false,
    }
}

pub async fn search(client: &Client, q: &str) -> Result<Vec<Track>> {
    let v = match music(client, q).await {
        Ok(v) if !v.is_empty() => v,
        _ => videos(client, q).await?,
    };
    Ok(v.iter().filter(|v| v.duration >= 40.0 && v.duration <= 1200.0).take(12).map(|v| to_track(v, &[], None, None, None)).collect())
}

fn score(v: &Video, title: &str, artists: &[String], duration: f64) -> f64 {
    let vt = norm(&v.title);
    let vc = norm(&v.channel);
    let want = core_title(title);
    let o = overlap(&want, &vt);
    if o < 0.6 {
        return -10.0;
    }
    let mut s = o * 4.0;
    let a0 = artists.first().map(|a| norm(a)).unwrap_or_default();
    if !a0.is_empty() && (vt.contains(&a0) || vc.contains(&a0)) {
        s += 3.0;
    } else if artists.iter().skip(1).any(|a| { let a = norm(a); !a.is_empty() && (vt.contains(&a) || vc.contains(&a)) }) {
        s += 1.5;
    } else {
        s -= 3.0;
    }
    if v.channel.ends_with(" - Topic") {
        s += 3.0;
    } else if vc.ends_with("vevo") || (!a0.is_empty() && vc == a0) {
        s += 2.0;
    }
    if vt.contains("official audio") || vt.contains("audio oficial") {
        s += 1.0;
    }
    let wt = norm(title);
    for b in BAD {
        if vt.split(' ').collect::<Vec<_>>().windows(b.split(' ').count()).any(|w| w.join(" ") == b) && !wt.contains(b) {
            s -= 3.0;
        }
    }
    if duration > 0.0 {
        let d = (v.duration - duration).abs();
        if d <= 3.0 {
            s += 3.0;
        } else if d <= 12.0 {
            s += 1.0;
        } else if d > 40.0 {
            s -= 4.0;
        }
    }
    s
}

pub async fn find(client: &Client, title: &str, artists: &[String], duration: f64, album: Option<&str>) -> Result<Option<Track>> {
    let q = format!("{} {}", artists.first().map(String::as_str).unwrap_or(""), core_title(title));
    let rank = |vids: Vec<Video>| {
        let mut r: Vec<(f64, Video)> = vids.into_iter().take(15).map(|v| (score(&v, title, artists, duration), v)).filter(|(s, _)| *s >= 3.0).collect();
        r.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        r
    };
    let mut ranked = match music(client, q.trim()).await {
        Ok(v) => rank(v),
        Err(_) => vec![],
    };
    if ranked.first().map(|r| r.0 < 6.0).unwrap_or(true) {
        if let Ok(v) = videos(client, q.trim()).await {
            ranked.extend(rank(v));
            ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        }
    }
    let Some((_, best)) = ranked.first().cloned() else { return Ok(None) };
    let alts: Vec<Video> = ranked.iter().skip(1).take(3).map(|(_, v)| v.clone()).collect();
    let mut t = to_track(&best, &alts, Some(title), Some(&artists.join(", ")), album);
    if duration > 0.0 {
        t.duration = duration;
    }
    Ok(Some(t))
}
