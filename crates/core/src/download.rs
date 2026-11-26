use crate::model::*;
use crate::net::USER_AGENT;
use futures_util::StreamExt;
use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::Tag;
use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::io::AsyncWriteExt;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub format: String,
    pub received: u64,
    pub total: Option<u64>,
    pub state: String,
    pub path: Option<String>,
    pub error: Option<String>,
}

pub fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { ' ' } else { c })
        .collect();
    let joined = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = joined.chars().take(90).collect();
    let out = cut.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    if out.is_empty() { "Sin título".into() } else { out }
}

pub fn target_path(dir: &Path, t: &Track, f: &Format) -> PathBuf {
    let artist = sanitize(&t.artist);
    let album = sanitize(if t.album.is_empty() { "Sencillos" } else { &t.album });
    let name = match t.track_no {
        Some(n) => format!("{:02} - {}", n, sanitize(&t.title)),
        None => sanitize(&t.title),
    };
    dir.join(artist).join(album).join(format!("{name}.{}", f.kind))
}

fn write_tags(path: &Path, t: &Track, cover: Option<Vec<u8>>) -> std::result::Result<(), String> {
    let mut file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    if file.primary_tag_mut().is_none() {
        let tt = file.primary_tag_type();
        file.insert_tag(Tag::new(tt));
    }
    let tag = file.primary_tag_mut().ok_or("sin etiqueta")?;
    tag.set_title(t.title.clone());
    tag.set_artist(t.artist.clone());
    if !t.album.is_empty() {
        tag.set_album(t.album.clone());
    }
    if let Some(n) = t.track_no {
        tag.set_track(n);
    }
    let source = match t.source.as_str() {
        "jamendo" => "Jamendo",
        "audius" => "Audius",
        "ccmixter" => "ccMixter",
        "commons" => "Wikimedia Commons",
        "pc" => "tu PC",
        "archive" => "Internet Archive",
        o => o,
    };
    let lic = t.license.clone().unwrap_or_default();
    tag.set_comment(format!("Descargado con Yokisfy desde {source}. Licencia: {lic}").trim().to_string());
    if !lic.is_empty() {
        tag.insert_text(ItemKey::CopyrightMessage, lic);
    }
    if let Some(bytes) = cover {
        let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { MimeType::Png } else { MimeType::Jpeg };
        tag.push_picture(Picture::new_unchecked(PictureType::CoverFront, Some(mime), None, bytes));
    }
    tag.save_to_path(path, WriteOptions::default()).map_err(|e| e.to_string())
}

pub async fn download(client: &Client, t: &Track, f: &Format, dir: &Path, on: &(dyn Fn(Progress) + Send + Sync)) -> Result<PathBuf> {
    let path = target_path(dir, t, f);
    let mut p = Progress {
        id: t.id.clone(),
        title: t.title.clone(),
        artist: t.artist.clone(),
        format: f.label.clone(),
        received: 0,
        total: f.size,
        state: "downloading".into(),
        path: None,
        error: None,
    };
    on(p.clone());
    let res: Result<PathBuf> = async {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part = path.with_extension(format!("{}.part", f.kind));
        let resp = client.get(&f.url).header("User-Agent", USER_AGENT).header("Referer", crate::net::referer(&f.url).unwrap_or("")).send().await?.error_for_status()?;
        p.total = resp.content_length().or(f.size);
        let mut out = tokio::fs::File::create(&part).await?;
        let mut stream = resp.bytes_stream();
        let mut last = Instant::now();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            out.write_all(&chunk).await?;
            p.received += chunk.len() as u64;
            if last.elapsed().as_millis() > 250 {
                on(p.clone());
                last = Instant::now();
            }
        }
        out.flush().await?;
        drop(out);
        tokio::fs::rename(&part, &path).await?;
        let cover = match &t.cover {
            Some(u) => match client.get(u).header("User-Agent", USER_AGENT).send().await {
                Ok(r) if r.status().is_success() => r.bytes().await.ok().map(|b| b.to_vec()),
                _ => None,
            },
            None => None,
        };
        let (pp, tt) = (path.clone(), t.clone());
        let _ = tokio::task::spawn_blocking(move || write_tags(&pp, &tt, cover)).await;
        Ok(path.clone())
    }
    .await;
    match &res {
        Ok(path) => {
            p.state = "done".into();
            p.path = Some(path.to_string_lossy().into_owned());
        }
        Err(e) => {
            p.state = "error".into();
            p.error = Some(e.to_string());
        }
    }
    on(p);
    res
}
