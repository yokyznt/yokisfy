use crate::{Result, Track};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub words: Vec<(f64, String)>,
    pub exact: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub synced: Vec<(f64, String)>,
    pub lines: Vec<Line>,
    pub plain: Option<String>,
    pub instrumental: bool,
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        v[1..v.len() - 1].replace("''", "'")
    } else if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        serde_json::from_str::<String>(v).unwrap_or_else(|_| v[1..v.len() - 1].to_string())
    } else {
        v.to_string()
    }
}

fn parse_file(f: &str) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut in_lines = false;
    for raw in f.lines() {
        if raw.starts_with("lines:") {
            in_lines = true;
            continue;
        }
        if !in_lines {
            continue;
        }
        if !raw.starts_with(' ') && !raw.starts_with('-') {
            break;
        }
        let t = raw.trim_start();
        let indent = raw.len() - t.len();
        if indent == 0 {
            if let Some(v) = t.strip_prefix("- text:") {
                lines.push(Line { text: unquote(v), exact: true, ..Default::default() });
            }
            continue;
        }
        let Some(l) = lines.last_mut() else { continue };
        if indent == 2 {
            if let Some(v) = t.strip_prefix("start_ms:") {
                l.start = v.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
            } else if let Some(v) = t.strip_prefix("end_ms:") {
                l.end = v.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
            } else if let Some(v) = t.strip_prefix("- text:") {
                l.words.push((0.0, unquote(v)));
            }
        } else if let Some(v) = t.strip_prefix("start_ms:") {
            if let Some(w) = l.words.last_mut() {
                w.0 = v.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
            }
        }
    }
    lines.retain(|l| !l.text.trim().is_empty() || !l.words.is_empty());
    lines
}

fn estimate(synced: &[(f64, String)], total: f64) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, (start, text)) in synced.iter().enumerate() {
        let next = synced.get(i + 1).map(|n| n.0).unwrap_or(if total > *start { total } else { start + 5.0 });
        let span = (next - start).min(1.0 + text.chars().count() as f64 * 0.09).max(0.4);
        let parts: Vec<String> = text.split_inclusive(' ').map(String::from).collect();
        let chars: f64 = parts.iter().map(|p| p.trim().chars().count().max(1) as f64).sum::<f64>().max(1.0);
        let mut acc = 0.0;
        let words = parts
            .iter()
            .map(|p| {
                let w = (start + span * acc / chars, p.clone());
                acc += p.trim().chars().count().max(1) as f64;
                w
            })
            .collect();
        out.push(Line { start: *start, end: start + span, text: text.clone(), words, exact: false });
    }
    out
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
    let total = v["duration"].as_f64().unwrap_or(0.0);
    let mut lines = v["lyricsfile"].as_str().map(parse_file).unwrap_or_default();
    if lines.is_empty() || lines.iter().all(|l| l.words.is_empty()) {
        lines = estimate(&synced, total);
    } else {
        for l in lines.iter_mut() {
            if l.words.is_empty() {
                l.words = vec![(l.start, l.text.clone())];
            }
            if l.end <= l.start {
                l.end = l.start + 3.0;
            }
        }
    }
    Lyrics {
        synced,
        lines,
        plain: v["plainLyrics"].as_str().filter(|s| !s.trim().is_empty()).map(String::from),
        instrumental: v["instrumental"].as_bool().unwrap_or(false),
    }
}

fn cache_path(dir: &std::path::Path, t: &Track) -> std::path::PathBuf {
    use md5::Digest;
    let h = md5::Md5::digest(t.id.as_bytes());
    dir.join(format!("{}.json", h.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

pub fn cached(dir: &std::path::Path, t: &Track) -> Option<Lyrics> {
    serde_json::from_slice(&std::fs::read(cache_path(dir, t)).ok()?).ok()
}

pub fn store(dir: &std::path::Path, t: &Track, l: &Lyrics) {
    if l.lines.is_empty() && l.plain.is_none() && !l.instrumental {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    if let Ok(b) = serde_json::to_vec(l) {
        let _ = std::fs::write(cache_path(dir, t), b);
    }
}

pub async fn find_cached(client: &Client, dir: &std::path::Path, t: &Track) -> Result<Option<Lyrics>> {
    if let Some(l) = cached(dir, t) {
        return Ok(Some(l));
    }
    let r = find(client, t).await?;
    if let Some(l) = &r {
        store(dir, t, l);
    }
    Ok(r)
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
