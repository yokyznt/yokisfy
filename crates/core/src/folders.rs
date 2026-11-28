use crate::model::Track;
use md5::Digest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

const EXTS: [&str; 9] = ["mp3", "flac", "m4a", "ogg", "opus", "wav", "aac", "aiff", "wma"];

struct Cache {
    at: Option<Instant>,
    list: Vec<Track>,
    tags: HashMap<PathBuf, (SystemTime, Track)>,
}

static CACHE: std::sync::LazyLock<Mutex<Cache>> = std::sync::LazyLock::new(|| Mutex::new(Cache { at: None, list: Vec::new(), tags: HashMap::new() }));

pub fn roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let h = PathBuf::from(home);
        for sub in ["Music", "Música", "OneDrive/Music", "OneDrive/Música", "OneDrive/Documentos/Música"] {
            let p = h.join(sub);
            if p.is_dir() && !out.iter().any(|o: &PathBuf| same(o, &p)) {
                out.push(p);
            }
        }
    }
    out
}

fn same(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 8 || out.len() > 20000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            walk(&p, depth + 1, out);
        } else if p.extension().map(|x| EXTS.contains(&x.to_string_lossy().to_lowercase().as_str())).unwrap_or(false) {
            out.push(p);
        }
    }
}

fn read(p: &Path) -> Track {
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::tag::Accessor;
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let (mut title, mut artist, mut album, mut no, mut dur, mut genre) = (stem.clone(), String::new(), String::new(), None, 0.0, None);
    if let Ok(f) = lofty::read_from_path(p) {
        dur = f.properties().duration().as_secs_f64();
        if let Some(t) = f.primary_tag().or_else(|| f.first_tag()) {
            if let Some(v) = t.title().filter(|v| !v.trim().is_empty()) {
                title = v.into_owned();
            }
            artist = t.artist().map(|v| v.into_owned()).unwrap_or_default();
            album = t.album().map(|v| v.into_owned()).unwrap_or_default();
            no = t.track();
            genre = t.genre().map(|v| v.into_owned());
        }
    }
    if artist.is_empty() {
        if let Some((a, b)) = stem.split_once(" - ") {
            artist = a.trim().to_string();
            if title == stem {
                title = b.trim().to_string();
            }
        }
    }
    let path = p.to_string_lossy().into_owned();
    let id = format!("file:{}", md5::Md5::digest(path.to_lowercase().as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>());
    Track {
        id,
        source: "local".into(),
        title,
        artist: if artist.is_empty() { "Desconocido".into() } else { artist },
        album,
        album_id: None,
        track_no: no,
        duration: dur,
        cover: None,
        stream_url: String::new(),
        formats: vec![],
        license: None,
        downloadable: false,
        local_path: Some(path),
        genre,
        live: false,
    }
}

pub fn tracks() -> Vec<Track> {
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if c.at.map(|a| a.elapsed() < Duration::from_secs(90)).unwrap_or(false) {
        return c.list.clone();
    }
    let mut files = Vec::new();
    for r in roots() {
        walk(&r, 0, &mut files);
    }
    let mut list = Vec::with_capacity(files.len());
    let mut tags = HashMap::with_capacity(files.len());
    for f in files {
        let m = std::fs::metadata(&f).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        let t = match c.tags.get(&f) {
            Some((mt, t)) if *mt == m => t.clone(),
            _ => read(&f),
        };
        list.push(t.clone());
        tags.insert(f, (m, t));
    }
    c.tags = tags;
    c.list = list.clone();
    c.at = Some(Instant::now());
    list
}
