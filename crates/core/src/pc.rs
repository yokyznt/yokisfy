use crate::model::*;
use crate::server::{DISCOVERY_PORT, USER};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PcServer {
    pub name: String,
    pub url: String,
    pub user: String,
    pub password: String,
}

pub fn discover(wait: Duration) -> Vec<PcServer> {
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else { return vec![] };
    let _ = sock.set_broadcast(true);
    let _ = sock.set_read_timeout(Some(Duration::from_millis(250)));
    let mut targets = vec!["255.255.255.255".to_string()];
    if let Some(ip) = crate::server::lan_ip() {
        if let Some((pre, _)) = ip.rsplit_once('.') {
            targets.push(format!("{pre}.255"));
        }
    }
    for t in &targets {
        let _ = sock.send_to(b"YOKISFY?", format!("{t}:{DISCOVERY_PORT}"));
    }
    let end = Instant::now() + wait;
    let mut out: Vec<PcServer> = Vec::new();
    let mut buf = [0u8; 1024];
    while Instant::now() < end {
        if let Ok((n, from)) = sock.recv_from(&mut buf) {
            if let Some(s) = parse(&buf[..n], from) {
                if !out.iter().any(|x| x.url == s.url) {
                    out.push(s);
                }
            }
        }
    }
    out
}

fn parse(b: &[u8], from: SocketAddr) -> Option<PcServer> {
    let v: Value = serde_json::from_slice(b).ok()?;
    (v["app"] == "yokisfy").then(|| PcServer {
        name: v["name"].as_str().unwrap_or("PC").to_string(),
        url: format!("http://{}:{}", from.ip(), v["port"].as_u64().unwrap_or(45130)),
        user: v["user"].as_str().unwrap_or(USER).to_string(),
        password: v["password"].as_str().unwrap_or_default().to_string(),
    })
}

fn auth(s: &PcServer) -> String {
    let p: String = s.password.bytes().map(|b| format!("{b:02x}")).collect();
    format!("u={}&p=enc:{p}&v=1.16.1&c=yokisfy-android&f=json", urlencoding::encode(&s.user))
}

pub async fn library(client: &Client, s: &PcServer) -> Result<Vec<Track>> {
    let a = auth(s);
    let v: Value = client
        .get(format!("{}/rest/search3?{a}&query=&songCount=5000&albumCount=0&artistCount=0", s.url))
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let r = &v["subsonic-response"];
    if r["status"] != "ok" {
        return Err(Error::Msg(r["error"]["message"].as_str().unwrap_or("la PC rechazó la conexión").to_string()));
    }
    let songs = r["searchResult3"]["song"].as_array().cloned().unwrap_or_default();
    Ok(songs
        .iter()
        .filter_map(|x| {
            let id = x["id"].as_str()?;
            let sfx = x["suffix"].as_str().unwrap_or("mp3").to_string();
            let stream = format!("{}/rest/stream?{a}&id={id}", s.url);
            Some(Track {
                id: format!("pc:{id}"),
                source: "pc".into(),
                title: x["title"].as_str().unwrap_or_default().to_string(),
                artist: x["artist"].as_str().unwrap_or_default().to_string(),
                album: x["album"].as_str().unwrap_or_default().to_string(),
                album_id: None,
                track_no: x["track"].as_u64().map(|n| n as u32),
                duration: x["duration"].as_f64().unwrap_or(0.0),
                cover: Some(format!("{}/rest/getCoverArt?{a}&id={}&size=300", s.url, x["coverArt"].as_str().unwrap_or(id))),
                stream_url: stream.clone(),
                formats: vec![Format { kind: sfx.clone(), label: sfx.to_uppercase(), url: format!("{}/rest/download?{a}&id={id}", s.url), size: x["size"].as_u64() }],
                license: None,
                downloadable: true,
                local_path: None,
                genre: x["genre"].as_str().map(String::from),
                live: false,
            })
        })
        .collect())
}
