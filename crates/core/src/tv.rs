use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Tv {
    pub name: String,
    pub control: String,
}

fn tag(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let i = xml.find(&open)? + open.len();
    let j = xml[i..].find(&format!("</{name}>"))? + i;
    Some(xml[i..j].trim().to_string())
}

fn join(base: &str, path: &str) -> String {
    if path.starts_with("http") {
        return path.to_string();
    }
    let root = base.find("://").and_then(|p| base[p + 3..].find('/').map(|q| &base[..p + 3 + q])).unwrap_or(base);
    if path.starts_with('/') { format!("{root}{path}") } else { format!("{root}/{path}") }
}

fn ssdp(wait: Duration) -> Vec<String> {
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else { return vec![] };
    let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));
    for st in ["urn:schemas-upnp-org:service:AVTransport:1", "urn:schemas-upnp-org:device:MediaRenderer:1"] {
        let msg = format!("M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {st}\r\n\r\n");
        let _ = sock.send_to(msg.as_bytes(), "239.255.255.250:1900");
    }
    let end = Instant::now() + wait;
    let mut out: Vec<String> = Vec::new();
    let mut buf = [0u8; 2048];
    while Instant::now() < end {
        if let Ok((n, _)) = sock.recv_from(&mut buf) {
            let s = String::from_utf8_lossy(&buf[..n]);
            if let Some(l) = s.lines().find(|l| l.to_ascii_lowercase().starts_with("location:")) {
                let u = l[9..].trim().to_string();
                if !out.contains(&u) {
                    out.push(u);
                }
            }
        }
    }
    out
}

pub async fn discover(client: &Client) -> Vec<Tv> {
    let locs = tokio::task::spawn_blocking(|| ssdp(Duration::from_millis(2600))).await.unwrap_or_default();
    let mut out: Vec<Tv> = Vec::new();
    for loc in locs {
        let Ok(r) = client.get(&loc).timeout(Duration::from_secs(4)).send().await else { continue };
        let Ok(xml) = r.text().await else { continue };
        let Some(si) = xml.find("urn:schemas-upnp-org:service:AVTransport:1") else { continue };
        let rest = &xml[si..];
        let Some(ctl) = tag(rest, "controlURL") else { continue };
        let name = tag(&xml, "friendlyName").unwrap_or_else(|| "Tele".into());
        let control = join(&loc, &ctl);
        if !out.iter().any(|t| t.control == control) {
            out.push(Tv { name: name.replace("&amp;", "&"), control });
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

async fn soap(client: &Client, control: &str, action: &str, args: &str) -> Result<String, String> {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"urn:schemas-upnp-org:service:AVTransport:1\"><InstanceID>0</InstanceID>{args}</u:{action}></s:Body></s:Envelope>"
    );
    let r = client
        .post(control)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPACTION", format!("\"urn:schemas-upnp-org:service:AVTransport:1#{action}\""))
        .body(body)
        .timeout(Duration::from_secs(6))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let ok = r.status().is_success();
    let t = r.text().await.unwrap_or_default();
    if ok { Ok(t) } else { Err(format!("La tele respondió con error a {action}")) }
}

pub async fn load(client: &Client, control: &str, url: &str, title: &str, artist: &str, cover: Option<&str>, mime: &str) -> Result<(), String> {
    let art = cover.map(|c| format!("<upnp:albumArtURI>{}</upnp:albumArtURI>", esc(c))).unwrap_or_default();
    let didl = format!(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\"><item id=\"0\" parentID=\"-1\" restricted=\"1\"><dc:title>{}</dc:title><upnp:artist>{}</upnp:artist>{art}<upnp:class>object.item.audioItem.musicTrack</upnp:class><res protocolInfo=\"http-get:*:{mime}:*\">{}</res></item></DIDL-Lite>",
        esc(title),
        esc(artist),
        esc(url)
    );
    let _ = soap(client, control, "Stop", "").await;
    soap(client, control, "SetAVTransportURI", &format!("<CurrentURI>{}</CurrentURI><CurrentURIMetaData>{}</CurrentURIMetaData>", esc(url), esc(&didl))).await?;
    play(client, control).await
}

pub async fn play(client: &Client, control: &str) -> Result<(), String> {
    soap(client, control, "Play", "<Speed>1</Speed>").await.map(|_| ())
}

pub async fn pause(client: &Client, control: &str) -> Result<(), String> {
    soap(client, control, "Pause", "").await.map(|_| ())
}

pub async fn stop(client: &Client, control: &str) -> Result<(), String> {
    soap(client, control, "Stop", "").await.map(|_| ())
}

pub async fn seek(client: &Client, control: &str, secs: f64) -> Result<(), String> {
    let s = secs.max(0.0) as u64;
    soap(client, control, "Seek", &format!("<Unit>REL_TIME</Unit><Target>{:02}:{:02}:{:02}</Target>", s / 3600, (s / 60) % 60, s % 60)).await.map(|_| ())
}

pub async fn state(client: &Client, control: &str) -> Option<String> {
    let t = soap(client, control, "GetTransportInfo", "").await.ok()?;
    tag(&t, "CurrentTransportState")
}
