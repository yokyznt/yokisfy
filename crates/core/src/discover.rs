use crate::model::*;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

const UA: &str = concat!("Yokisfy/", env!("CARGO_PKG_VERSION"), " ( https://github.com/yokyznt/yokisfy )");
const ALGO: &str = "session_based_days_7500_session_300_contribution_5_threshold_10_limit_100_filter_True_skip_30";

#[derive(Default)]
pub struct Discover {
    cache: Mutex<HashMap<String, Vec<String>>>,
}

impl Discover {
    pub async fn similar_artists(&self, client: &Client, artist: &str) -> Vec<String> {
        let key = artist.trim().to_lowercase();
        if key.is_empty() {
            return vec![];
        }
        if let Some(v) = self.cache.lock().unwrap().get(&key) {
            return v.clone();
        }
        let r = fetch(client, artist).await.unwrap_or_default();
        self.cache.lock().unwrap().insert(key, r.clone());
        r
    }
}

async fn fetch(client: &Client, artist: &str) -> Result<Vec<String>> {
    let q = format!("artist:\"{}\"", artist.replace('"', ""));
    let mb: Value = client.get("https://musicbrainz.org/ws/2/artist/").header("User-Agent", UA).query(&[("query", q.as_str()), ("fmt", "json"), ("limit", "1")]).send().await?.error_for_status()?.json().await?;
    let a = &mb["artists"][0];
    if a["score"].as_i64().unwrap_or(0) < 90 {
        return Ok(vec![]);
    }
    let Some(mbid) = a["id"].as_str() else { return Ok(vec![]) };
    let lb: Value = client
        .get("https://labs.api.listenbrainz.org/similar-artists/json")
        .header("User-Agent", UA)
        .query(&[("artist_mbids", mbid), ("algorithm", ALGO)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(lb.as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str().map(String::from)).take(12).collect()).unwrap_or_default())
}
