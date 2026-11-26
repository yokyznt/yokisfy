use crate::{Result, Track};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub synced: Vec<(f64, String)>,
    pub plain: Option<String>,
    pub instrumental: bool,
}

fn clean_title(t: &str) -> String {
    let cut = t.find(" (feat").or_else(|| t.find(" (con ")).or_else(|| t.find(" [")).or_else(|| t.find(" (from")).or_else(|| t.find(" (From")).unwrap_or(t.len());
    t[..cut].trim().to_string()
}

fn parse(v: &Value) -> Lyrics {
    let mut synced = Vec::new();
    for line in v["syncedLyrics"].as_str().unwrap_or("").lines() {
        let mut rest = line;
        let mut stamps = Vec::new();
        while let Some(s) = rest.strip_prefix('[') {
            let Some(end) = s.find(']') else { break };
            let tag = &s[..end];
            let mut p = tag.split(':');
            if let (Some(m), Some(sec)) = (p.next().and_then(|m| m.parse::<f64>().ok()), p.next().and_then(|s| s.parse::<f64>().ok())) {
                stamps.push(m * 60.0 + sec);
            }
            rest = &s[end + 1..];
        }
        for t in stamps {
            synced.push((t, rest.trim().to_string()));
        }
    }
    synced.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Lyrics {
        synced,
        plain: v["plainLyrics"].as_str().filter(|s| !s.trim().is_empty()).map(String::from),
        instrumental: v["instrumental"].as_bool().unwrap_or(false),
    }
}

pub async fn find(client: &Client, t: &Track) -> Result<Option<Lyrics>> {
    let title = clean_title(&t.title);
    let artist = t.artist.split(", ").next().unwrap_or(&t.artist).to_string();
    let mut q: Vec<(&str, String)> = vec![("artist_name", artist.clone()), ("track_name", title.clone())];
    if t.duration > 0.0 {
        q.push(("duration", format!("{}", t.duration.round() as i64)));
    }
    let r = client.get("https://lrclib.net/api/get").query(&q).header("User-Agent", "Yokisfy/0.3 (https://github.com/yokyznt/yokisfy)").send().await?;
    if r.status().is_success() {
        let v: Value = r.json().await?;
        let l = parse(&v);
        if !l.synced.is_empty() || l.plain.is_some() || l.instrumental {
            return Ok(Some(l));
        }
    }
    let r = client
        .get("https://lrclib.net/api/search")
        .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
        .header("User-Agent", "Yokisfy/0.3 (https://github.com/yokyznt/yokisfy)")
        .send()
        .await?;
    if !r.status().is_success() {
        return Ok(None);
    }
    let list: Vec<Value> = r.json().await.unwrap_or_default();
    let best = list
        .iter()
        .filter(|v| t.duration <= 0.0 || v["duration"].as_f64().map(|d| (d - t.duration).abs() <= 6.0).unwrap_or(false))
        .max_by_key(|v| v["syncedLyrics"].as_str().map(|_| 1).unwrap_or(0));
    Ok(best.map(parse).filter(|l| !l.synced.is_empty() || l.plain.is_some()))
}
