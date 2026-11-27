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
    #[serde(default)]
    pub party: bool,
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
        party: v["party"].as_bool().unwrap_or(false),
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

pub async fn call(client: &Client, s: &PcServer, action: &str, body: &Value) -> Result<Value> {
    let r = client
        .post(format!("{}/yk/{action}?p={}", s.url, urlencoding::encode(&s.password)))
        .body(body.to_string())
        .timeout(Duration::from_secs(if action == "party_search" { 25 } else { 8 }))
        .send()
        .await?
        .error_for_status()?;
    Ok(r.json().await?)
}

pub fn topic(code: &str) -> String {
    use md5::Digest;
    let clean: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    let h = md5::Md5::digest(format!("yokisfy:{clean}").as_bytes());
    format!("yokisfy-{}", h.iter().take(10).map(|b| format!("{b:02x}")).collect::<String>())
}

pub fn clean_code(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
}

pub async fn announce(client: &Client, code: &str, url: &str) -> Result<()> {
    client.post(format!("https://ntfy.sh/{}", topic(code))).body(url.to_string()).send().await?.error_for_status()?;
    Ok(())
}

pub async fn lookup(client: &Client, code: &str) -> Option<String> {
    let body = client.get(format!("https://ntfy.sh/{}/json?poll=1&since=24h", topic(code))).send().await.ok()?.text().await.ok()?;
    body.lines().rev().filter_map(|l| serde_json::from_str::<Value>(l).ok()).find_map(|v| v["message"].as_str().filter(|m| m.starts_with("http")).map(String::from))
}

pub static PUBLIC: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub static TUNNEL_STATE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

fn set_state(s: &str, url: Option<String>) {
    if let Ok(mut g) = TUNNEL_STATE.lock() {
        *g = s.to_string();
    }
    if let Ok(mut g) = PUBLIC.lock() {
        *g = url;
    }
}

pub fn public_url() -> Option<String> {
    PUBLIC.lock().ok().and_then(|g| g.clone())
}

pub fn tunnel_state() -> String {
    TUNNEL_STATE.lock().map(|g| g.clone()).unwrap_or_default()
}

async fn announce_all(client: &Client, codes: &[String], url: &str) {
    for c in codes {
        for _ in 0..3 {
            if announce(client, c, url).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}

async fn healthy(client: &Client, url: &str) -> bool {
    client.get(format!("{url}/fiesta")).timeout(Duration::from_secs(12)).send().await.map(|r| r.status().is_success()).unwrap_or(false)
}

#[cfg(windows)]
async fn cloudflared_bin(client: &Client, dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let p = dir.join("cloudflared.exe");
    if p.metadata().map(|m| m.len() > 10_000_000).unwrap_or(false) {
        return Some(p);
    }
    let _ = std::fs::create_dir_all(dir);
    set_state("Preparando la conexión por internet (descarga única de 55 MB)…", None);
    let part = dir.join("cloudflared.part");
    let r = client.get("https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe").timeout(Duration::from_secs(900)).send().await.ok()?.error_for_status().ok()?;
    let mut f = tokio::fs::File::create(&part).await.ok()?;
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut st = r.bytes_stream();
    while let Some(chunk) = st.next().await {
        f.write_all(&chunk.ok()?).await.ok()?;
    }
    f.flush().await.ok()?;
    drop(f);
    if part.metadata().map(|m| m.len() < 10_000_000).unwrap_or(true) {
        return None;
    }
    std::fs::rename(&part, &p).ok()?;
    Some(p)
}

#[cfg(not(windows))]
async fn cloudflared_bin(_client: &Client, _dir: &std::path::Path) -> Option<std::path::PathBuf> {
    None
}

async fn run_cloudflared(bin: &std::path::Path, port: u16, client: &Client, codes: &(dyn Fn() -> Vec<String> + Send + Sync), stop: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> bool {
    use tokio::io::AsyncBufReadExt;
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(["tunnel", "--no-autoupdate", "--protocol", "http2", "--url", &format!("http://127.0.0.1:{port}")]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let Ok(mut child) = cmd.spawn() else { return false };
    let Some(err) = child.stderr.take() else { return false };
    let mut lines = tokio::io::BufReader::new(err).lines();
    let found = tokio::time::timeout(Duration::from_secs(40), async {
        while let Ok(Some(l)) = lines.next_line().await {
            if let Some(i) = l.find("https://") {
                let u: String = l[i..].chars().take_while(|c| !c.is_whitespace() && *c != '|').collect();
                if u.contains(".trycloudflare.com") {
                    return Some(u);
                }
            }
        }
        None
    })
    .await
    .ok()
    .flatten();
    let Some(url) = found else { return false };
    tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
    let mut ok = false;
    for _ in 0..15 {
        if healthy(client, &url).await {
            ok = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    if !ok {
        return false;
    }
    set_state("ok", Some(url.clone()));
    announce_all(client, &codes(), &url).await;
    let mut tick = 0u32;
    let mut fails = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = child.kill().await;
            return true;
        }
        if child.try_wait().ok().flatten().is_some() {
            return false;
        }
        tick += 1;
        if tick % 12 == 0 {
            if healthy(client, &url).await {
                fails = 0;
            } else {
                fails += 1;
                if fails >= 3 {
                    let _ = child.kill().await;
                    return false;
                }
            }
        }
        if tick % 240 == 0 {
            announce_all(client, &codes(), &url).await;
        }
    }
}

pub async fn tunnel(port: u16, codes: std::sync::Arc<dyn Fn() -> Vec<String> + Send + Sync>, client: Client, stop: std::sync::Arc<std::sync::atomic::AtomicBool>, dir: std::path::PathBuf) {
    let mut fails = 0u32;
    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        set_state("Conectando por internet…", None);
        let bin = cloudflared_bin(&client, &dir).await;
        let mut done = false;
        if let Some(b) = bin.as_ref() {
            done = run_cloudflared(b, port, &client, codes.as_ref(), &stop).await;
        }
        if done || stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        fails += 1;
        if bin.is_none() || fails % 3 == 0 {
            if let Ok(c) = bore_cli::client::Client::new("127.0.0.1", port, "bore.pub", 0, None).await {
                let url = format!("http://bore.pub:{}", c.remote_port());
                set_state("ok", Some(url.clone()));
                announce_all(&client, &codes(), &url).await;
                let st2 = stop.clone();
                tokio::select! {
                    _ = c.listen() => {}
                    _ = async move { while !st2.load(std::sync::atomic::Ordering::Relaxed) { tokio::time::sleep(Duration::from_secs(1)).await } } => {}
                }
            }
        }
        set_state("Sin conexión por internet; reintentando…", None);
        for _ in 0..10 {
            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    set_state("", None);
}
