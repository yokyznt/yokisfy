use std::sync::{Arc, Mutex};
use std::time::Duration;
use yokisfy_core::player::{Event, Status};
use yokisfy_core::Core;

#[tokio::main]
async fn main() {
    let dir = std::env::temp_dir().join("yokisfy-probe");
    let last: Arc<Mutex<Option<Status>>> = Arc::new(Mutex::new(None));
    let l2 = last.clone();
    let core = Core::new(
        &dir,
        Arc::new(move |e| {
            if let Event::Status(s) = e {
                *l2.lock().unwrap() = Some(s);
            }
        }),
    )
    .unwrap();
    core.lib.set_setting("download_dir", dir.join("dl").to_str().unwrap()).unwrap();
    let t0 = std::time::Instant::now();
    let r = core.search("jazz").await;
    println!("search {:?}: {} tracks, {} albums, warnings {:?}", t0.elapsed(), r.tracks.len(), r.albums.len(), r.warnings);
    for t in r.tracks.iter().take(3) {
        println!("  {} — {} [{}] {:.0}s {:?}", t.title, t.artist, t.formats.iter().map(|f| f.label.as_str()).collect::<Vec<_>>().join(","), t.duration, t.license);
    }
    let home = core.home().await;
    for s in &home {
        println!("home: {} ({} albums)", s.title, s.albums.len());
    }
    let d = core.album("ia:gd1977-05-08.111493.mtx.seamons.sbeok.flac16").await.unwrap();
    println!("album {} — {} tracks, first {} {:?}", d.album.title, d.tracks.len(), d.tracks[0].title, d.tracks[0].formats.iter().map(|f| &f.label).collect::<Vec<_>>());
    let mode = std::env::args().nth(1).unwrap_or_default();
    core.player.volume(0.0);
    core.player.high_quality.store(mode == "flac", std::sync::atomic::Ordering::Relaxed);
    core.play(d.tracks.clone(), 0);
    for i in 0..16 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(s) = last.lock().unwrap().clone() {
            if i % 3 == 0 {
                println!("t+{:.1}s loading={} playing={} pos={:.2} dur={:.0} err={:?}", (i + 1) as f32 / 2.0, s.loading, s.playing, s.position, s.duration, s.error);
            }
        }
    }
    core.player.seek(120.0);
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let s = last.lock().unwrap().clone().unwrap();
    println!("after seek 120: pos={:.2} err={:?}", s.position, s.error);
    core.player.next();
    tokio::time::sleep(Duration::from_millis(4000)).await;
    let s = last.lock().unwrap().clone().unwrap();
    println!("after next: idx={:?} {} pos={:.2} err={:?}", s.index, s.track.map(|t| t.title).unwrap_or_default(), s.position, s.error);
    let d2 = core.album("ia:DWK123").await.unwrap();
    println!("DWK123 downloadable={} formats={:?}", d2.tracks[0].downloadable, d2.tracks[0].formats.iter().map(|f| &f.label).collect::<Vec<_>>());
    println!("GD downloadable={}", d.tracks[0].downloadable);
    let small = d2.tracks.iter().min_by_key(|t| t.formats.iter().find(|f| f.kind == "ogg").and_then(|f| f.size).unwrap_or(u64::MAX)).unwrap().clone();
    let dl = core
        .download(small.clone(), Some("MP3 VBR".into()), &|p| {
            if p.state != "downloading" {
                println!("dl {} {} {:?}", p.state, p.received, p.error)
            }
        })
        .await
        .unwrap();
    let path = dl.local_path.unwrap();
    use lofty::prelude::*;
    let f = lofty::read_from_path(&path).unwrap();
    let tag = f.primary_tag().unwrap();
    println!("tags: {:?} / {:?} / {:?} / pics {} / {:?}", tag.title(), tag.artist(), tag.album(), tag.pictures().len(), tag.comment());
    println!("path {path}");
    println!("recent {}", core.lib.recent(10).unwrap().len());
}
