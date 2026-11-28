use crate::model::*;
use reqwest::Client;
use serde_json::Value;

const HOSTS: [&str; 3] = ["https://de1.api.radio-browser.info", "https://nl1.api.radio-browser.info", "https://at1.api.radio-browser.info"];

fn s(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn station(v: &Value) -> Option<Track> {
    let id = s(&v["stationuuid"]);
    let url = s(&v["url_resolved"]);
    let codec = s(&v["codec"]).to_ascii_uppercase();
    if id.is_empty() || url.is_empty() || v["hls"].as_i64() == Some(1) || !(codec == "MP3" || codec == "AAC" || codec == "AAC+" || codec == "OGG") {
        return None;
    }
    let br = v["bitrate"].as_i64().filter(|b| *b > 0).map(|b| format!(" · {b} kbps")).unwrap_or_default();
    let place = [s(&v["country"]), s(&v["state"])].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(", ");
    let tags: Vec<String> = s(&v["tags"]).split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).take(3).collect();
    Some(Track {
        id: format!("rd:{id}"),
        source: "radio".into(),
        title: s(&v["name"]).chars().take(80).collect(),
        artist: if place.is_empty() { "Radio en vivo".into() } else { place },
        album: format!("{}{}", if tags.is_empty() { "Radio".into() } else { tags.join(", ") }, br),
        album_id: None,
        track_no: None,
        duration: 0.0,
        cover: Some(s(&v["favicon"])).filter(|x| x.starts_with("https://")),
        stream_url: url,
        formats: vec![],
        license: None,
        downloadable: false,
        local_path: None,
        genre: tags.first().cloned(),
        live: true,
    })
}

async fn get(client: &Client, path: &str, params: &[(&str, &str)]) -> Result<Vec<Track>> {
    let mut last = Error::Msg("radio sin respuesta".into());
    for h in HOSTS {
        let r = client.get(format!("{h}/json/{path}")).query(params).timeout(std::time::Duration::from_secs(8)).send().await.and_then(|r| r.error_for_status());
        match r {
            Ok(r) => {
                let v: Vec<Value> = r.json().await?;
                return Ok(v.iter().filter_map(station).collect());
            }
            Err(e) => last = e.into(),
        }
    }
    Err(last)
}

const BASE: [(&str, &str); 4] = [("hidebroken", "true"), ("order", "clickcount"), ("reverse", "true"), ("limit", "24")];

pub async fn search(client: &Client, query: &str) -> Result<Vec<Track>> {
    let mut p = BASE.to_vec();
    p.push(("name", query));
    let mut out = get(client, "stations/search", &p).await?;
    if out.len() < 6 {
        let mut p = BASE.to_vec();
        p.push(("tag", query));
        for t in get(client, "stations/search", &p).await.unwrap_or_default() {
            if !out.iter().any(|x| x.id == t.id) {
                out.push(t);
            }
        }
    }
    Ok(out)
}

pub async fn popular(client: &Client, country: Option<&str>) -> Result<Vec<Track>> {
    let mut p = BASE.to_vec();
    if let Some(c) = country {
        p.push(("countrycode", c));
    }
    get(client, "stations/search", &p).await
}
