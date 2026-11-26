use crate::model::Track;
use crate::net::HttpReader;
use rand::seq::SliceRandom;
use crate::audio::Stream;
use rodio::{OutputStream, Sink, Source};
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum Media {
    File(BufReader<File>),
    Http(HttpReader),
}

impl Read for Media {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Media::File(f) => f.read(b),
            Media::Http(h) => h.read(b),
        }
    }
}

impl Seek for Media {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        match self {
            Media::File(f) => f.seek(p),
            Media::Http(h) => h.seek(p),
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub track: Option<Track>,
    pub index: Option<usize>,
    pub queue_len: usize,
    pub playing: bool,
    pub loading: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: String,
    pub error: Option<String>,
    pub external: bool,
}

pub enum Event {
    Status(Status),
    Queue(Vec<Track>, Option<usize>),
    Started(Track),
    Ended(Track, Vec<String>),
    ExtSeek(f64),
}

pub type Listener = Arc<dyn Fn(Event) + Send + Sync>;

enum Cmd {
    Play(Vec<Track>, usize),
    Toggle,
    SetPlaying(bool),
    Next,
    Prev,
    Seek(f64),
    Volume(f32),
    Enqueue(Vec<Track>),
    PlayNext(Track),
    Jump(usize),
    Remove(usize),
    Shuffle(bool),
    Repeat(String),
    Update(Track),
    Ready(u64, Result<Stream, String>),
    ExtProgress(f64, f64),
    ExtEnded,
    ExtFailed(String),
}

#[derive(Clone)]
pub struct Player {
    tx: Sender<Cmd>,
    pub high_quality: Arc<AtomicBool>,
    pub crossfade_ms: Arc<AtomicU32>,
}

struct Fade {
    sink: Sink,
    at: Instant,
    secs: f64,
    from: f32,
}

fn hint(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".flac") || path.contains("/flac") {
        "flac"
    } else if path.ends_with(".ogg") || path.contains("/ogg") {
        "ogg"
    } else if path.ends_with(".wav") {
        "wav"
    } else if path.ends_with(".mp3") || path.contains("mp3") {
        "mp3"
    } else {
        ""
    }
}

pub fn open_media(track: &Track, high: bool) -> Result<Stream, String> {
    let (media, h, len) = match track.local_path.as_ref().filter(|p| std::path::Path::new(p).exists()) {
        Some(p) => {
            let f = File::open(p).map_err(|e| e.to_string())?;
            let len = f.metadata().ok().map(|m| m.len());
            (Media::File(BufReader::new(f)), hint(p), len)
        }
        None if track.live => {
            let r = HttpReader::open_live(&track.stream_url).map_err(|e| e.to_string())?;
            r.wait_for(48 * 1024, Duration::from_secs(20)).map_err(|e| e.to_string())?;
            return Stream::open(Media::Http(r), None, "", false);
        }
        None => {
            let open = |url: &str| -> Result<HttpReader, String> {
                let r = HttpReader::open(url).map_err(|e| e.to_string())?;
                r.wait_for(384 * 1024, Duration::from_secs(25)).map_err(|e| e.to_string())?;
                Ok(r)
            };
            let mut url = track.stream_for(high);
            let r = match open(&url) {
                Ok(r) => r,
                Err(e) if url != track.stream_url => {
                    let _ = e;
                    url = track.stream_url.clone();
                    open(&url)?
                }
                Err(e) => return Err(e),
            };
            let len = r.len();
            (Media::Http(r), hint(&url), len)
        }
    };
    Stream::open(media, len, h, true)
}

struct Engine {
    queue: Vec<Track>,
    order: Vec<usize>,
    pos: usize,
    shuffle: bool,
    repeat: String,
    volume: f32,
    want_play: bool,
    loading: bool,
    loaded: bool,
    gen: u64,
    duration: f64,
    error: Option<String>,
    fails: usize,
    external: bool,
    ext_pos: f64,
    ext_at: Option<std::time::Instant>,
    pending_xf: f64,
    fade_in: Option<(Instant, f64)>,
}

impl Engine {
    fn current(&self) -> Option<usize> {
        self.order.get(self.pos).copied()
    }

    fn rebuild_order(&mut self, keep: Option<usize>) {
        let n = self.queue.len();
        if self.shuffle {
            let mut rest: Vec<usize> = (0..n).filter(|i| Some(*i) != keep).collect();
            rest.shuffle(&mut rand::thread_rng());
            self.order = keep.into_iter().chain(rest).collect();
            self.pos = 0;
        } else {
            self.order = (0..n).collect();
            self.pos = keep.unwrap_or(0);
        }
    }
}

impl Player {
    pub fn new(listener: Listener) -> Self {
        let (tx, rx) = channel::<Cmd>();
        let high = Arc::new(AtomicBool::new(true));
        let xfade = Arc::new(AtomicU32::new(5000));
        let me = Player { tx: tx.clone(), high_quality: high.clone(), crossfade_ms: xfade.clone() };
        std::thread::Builder::new()
            .name("yk-audio".into())
            .spawn(move || {
                #[cfg(target_os = "android")]
                {
                    if oboe::DefaultStreamValues::get_frames_per_burst() <= 0 {
                        oboe::DefaultStreamValues::set_frames_per_burst(192);
                    }
                    if oboe::DefaultStreamValues::get_sample_rate() <= 0 {
                        oboe::DefaultStreamValues::set_sample_rate(48000);
                    }
                }
                let out = OutputStream::try_default();
                let (_stream, handle) = match out {
                    Ok(o) => o,
                    Err(e) => {
                        listener(Event::Status(Status { error: Some(format!("sin salida de audio: {e}")), ..Default::default() }));
                        return;
                    }
                };
                let mut sink = Sink::try_new(&handle).expect("sink");
                let mut e = Engine {
                    queue: vec![],
                    order: vec![],
                    pos: 0,
                    shuffle: false,
                    repeat: "off".into(),
                    volume: 0.8,
                    want_play: false,
                    loading: false,
                    loaded: false,
                    gen: 0,
                    duration: 0.0,
                    error: None,
                    fails: 0,
                    external: false,
                    ext_pos: 0.0,
                    ext_at: None,
                    pending_xf: 0.0,
                    fade_in: None,
                };
                let mut fading: Vec<Fade> = Vec::new();
                let xf_secs = || xfade.load(Ordering::Relaxed) as f64 / 1000.0;
                let short = || xf_secs().min(1.5);
                let status = |e: &Engine, sink: &Sink| -> Status {
                    let idx = e.current();
                    Status {
                        track: idx.and_then(|i| e.queue.get(i).cloned()),
                        index: idx,
                        queue_len: e.queue.len(),
                        playing: e.want_play,
                        loading: e.loading,
                        position: if e.external { e.ext_pos + if e.want_play { e.ext_at.map(|a| a.elapsed().as_secs_f64().min(2.0)).unwrap_or(0.0) } else { 0.0 } } else if e.loaded { sink.get_pos().as_secs_f64() } else { 0.0 },
                        duration: e.duration,
                        volume: e.volume,
                        shuffle: e.shuffle,
                        repeat: e.repeat.clone(),
                        error: e.error.clone(),
                        external: e.external,
                    }
                };
                let load = |e: &mut Engine, sink: &mut Sink, fading: &mut Vec<Fade>, xf: f64| {
                    e.gen += 1;
                    let can = xf > 0.05 && e.loaded && !e.loading && !e.external && e.want_play && !sink.empty() && !sink.is_paused();
                    let mut swapped = false;
                    if can {
                        if let Ok(ns) = Sink::try_new(&handle) {
                            let old = std::mem::replace(sink, ns);
                            fading.push(Fade { sink: old, at: Instant::now(), secs: xf, from: e.volume * e.volume });
                            swapped = true;
                        }
                    }
                    if !swapped {
                        sink.clear();
                    }
                    e.pending_xf = if swapped { xf } else { 0.0 };
                    e.fade_in = None;
                    e.loaded = false;
                    e.error = None;
                    let Some(i) = e.current() else {
                        e.loading = false;
                        e.want_play = false;
                        return;
                    };
                    let track = e.queue[i].clone();
                    e.duration = track.duration;
                    e.external = track.source == "youtube" && track.local_path.is_none();
                    e.ext_pos = 0.0;
                    e.ext_at = None;
                    if e.external {
                        e.loading = false;
                        e.loaded = true;
                        e.fails = 0;
                        listener(Event::Started(track));
                        return;
                    }
                    e.loading = true;
                    let gen = e.gen;
                    let tx = tx.clone();
                    let hq = high.load(Ordering::Relaxed);
                    std::thread::spawn(move || {
                        let _ = tx.send(Cmd::Ready(gen, open_media(&track, hq)));
                    });
                };
                let advance = |e: &mut Engine, sink: &mut Sink, fading: &mut Vec<Fade>, auto: bool, xf: f64| {
                    if e.queue.is_empty() {
                        return;
                    }
                    if auto && e.repeat == "one" {
                    } else if e.pos + 1 < e.order.len() {
                        e.pos += 1;
                    } else if e.repeat == "all" {
                        if e.shuffle {
                            e.rebuild_order(None);
                        }
                        e.pos = 0;
                    } else {
                        e.want_play = false;
                        sink.clear();
                        e.loaded = false;
                        if auto {
                            if let Some(t) = e.current().and_then(|i| e.queue.get(i)).cloned() {
                                let ids = e.queue.iter().rev().take(200).map(|t| t.id.clone()).collect();
                                listener(Event::Ended(t, ids));
                            }
                        }
                        return;
                    }
                    load(e, sink, fading, xf);
                };
                let mut last_emit = Instant::now();
                loop {
                    let mut dirty = false;
                    let mut queue_dirty = false;
                    let wait = if fading.is_empty() && e.fade_in.is_none() { 200 } else { 25 };
                    match rx.recv_timeout(Duration::from_millis(wait)) {
                        Ok(cmd) => {
                            dirty = true;
                            match cmd {
                                Cmd::Play(tracks, start) => {
                                    e.queue = tracks;
                                    let start = start.min(e.queue.len().saturating_sub(1));
                                    e.rebuild_order(Some(start));
                                    e.want_play = true;
                                    e.fails = 0;
                                    queue_dirty = true;
                                    load(&mut e, &mut sink, &mut fading, short());
                                }
                                Cmd::Toggle => {
                                    if e.external {
                                        if let Some(a) = e.ext_at.take() {
                                            if e.want_play {
                                                e.ext_pos += a.elapsed().as_secs_f64().min(2.0);
                                            }
                                        }
                                    }
                                    e.want_play = !e.want_play;
                                    if e.want_play {
                                        if e.loaded {
                                            sink.play();
                                        } else if !e.loading {
                                            load(&mut e, &mut sink, &mut fading, 0.0);
                                        }
                                    } else {
                                        sink.pause();
                                        for f in fading.drain(..) {
                                            f.sink.stop();
                                        }
                                    }
                                }
                                Cmd::SetPlaying(p) => {
                                    if e.external {
                                        if let Some(a) = e.ext_at.take() {
                                            if e.want_play {
                                                e.ext_pos += a.elapsed().as_secs_f64().min(2.0);
                                            }
                                        }
                                    }
                                    e.want_play = p;
                                    if p {
                                        sink.play()
                                    } else {
                                        sink.pause();
                                        for f in fading.drain(..) {
                                            f.sink.stop();
                                        }
                                    }
                                }
                                Cmd::Next => {
                                    e.want_play = true;
                                    advance(&mut e, &mut sink, &mut fading, false, short());
                                }
                                Cmd::Prev => {
                                    if e.external && e.ext_pos > 3.0 {
                                        e.ext_pos = 0.0;
                                        e.ext_at = None;
                                        listener(Event::ExtSeek(0.0));
                                    } else if !e.external && e.loaded && sink.get_pos().as_secs_f64() > 3.0 {
                                        let _ = sink.try_seek(Duration::ZERO);
                                    } else if e.pos > 0 {
                                        e.pos -= 1;
                                        e.want_play = true;
                                        load(&mut e, &mut sink, &mut fading, short());
                                    } else if e.loaded {
                                        let _ = sink.try_seek(Duration::ZERO);
                                    }
                                }
                                Cmd::Seek(s) => {
                                    if e.external {
                                        e.ext_pos = s.max(0.0);
                                        e.ext_at = None;
                                        listener(Event::ExtSeek(e.ext_pos));
                                    } else if e.loaded {
                                        if let Err(err) = sink.try_seek(Duration::from_secs_f64(s.max(0.0))) {
                                            e.error = Some(format!("no se pudo adelantar: {err}"));
                                        }
                                    }
                                }
                                Cmd::Volume(v) => {
                                    e.volume = v.clamp(0.0, 1.0);
                                    if e.fade_in.is_none() {
                                        sink.set_volume(e.volume * e.volume);
                                    }
                                }
                                Cmd::Enqueue(ts) => {
                                    let start = e.queue.len();
                                    e.queue.extend(ts);
                                    e.order.extend(start..e.queue.len());
                                    queue_dirty = true;
                                }
                                Cmd::PlayNext(t) => {
                                    let at = e.current().map(|i| i + 1).unwrap_or(0);
                                    e.queue.insert(at, t);
                                    for o in e.order.iter_mut() {
                                        if *o >= at {
                                            *o += 1;
                                        }
                                    }
                                    e.order.insert((e.pos + 1).min(e.order.len()), at);
                                    queue_dirty = true;
                                }
                                Cmd::Jump(i) => {
                                    if let Some(p) = e.order.iter().position(|o| *o == i) {
                                        e.pos = p;
                                        e.want_play = true;
                                        load(&mut e, &mut sink, &mut fading, short());
                                    }
                                }
                                Cmd::Remove(i) => {
                                    if i < e.queue.len() && Some(i) != e.current() {
                                        e.queue.remove(i);
                                        let cur = e.current();
                                        e.order.retain(|o| *o != i);
                                        for o in e.order.iter_mut() {
                                            if *o > i {
                                                *o -= 1;
                                            }
                                        }
                                        if let Some(c) = cur {
                                            let c = if c > i { c - 1 } else { c };
                                            e.pos = e.order.iter().position(|o| *o == c).unwrap_or(0);
                                        }
                                        queue_dirty = true;
                                    }
                                }
                                Cmd::Shuffle(s) => {
                                    e.shuffle = s;
                                    let cur = e.current();
                                    e.rebuild_order(cur);
                                    queue_dirty = true;
                                }
                                Cmd::Repeat(r) => e.repeat = r,
                                Cmd::Update(t) => {
                                    for q in e.queue.iter_mut().filter(|q| q.id == t.id) {
                                        *q = t.clone();
                                    }
                                }
                                Cmd::ExtProgress(p, d) => {
                                    if e.external {
                                        e.ext_pos = p;
                                        e.ext_at = Some(std::time::Instant::now());
                                        if d > 0.0 {
                                            e.duration = d;
                                        }
                                    }
                                    dirty = false;
                                }
                                Cmd::ExtEnded => {
                                    if e.external {
                                        advance(&mut e, &mut sink, &mut fading, true, 0.0);
                                    }
                                }
                                Cmd::ExtFailed(err) => {
                                    if e.external {
                                        e.error = Some(err);
                                        e.fails += 1;
                                        if e.fails < 3 && e.pos + 1 < e.order.len() {
                                            let msg = e.error.clone();
                                            advance(&mut e, &mut sink, &mut fading, false, 0.0);
                                            e.error = msg;
                                        } else {
                                            e.want_play = false;
                                        }
                                    }
                                }
                                Cmd::Ready(gen, res) => {
                                    if gen == e.gen {
                                        e.loading = false;
                                        match res {
                                            Ok(dec) => {
                                                if e.duration <= 0.0 {
                                                    e.duration = dec.total_duration().map(|d| d.as_secs_f64()).unwrap_or(0.0);
                                                }
                                                if e.pending_xf > 0.0 {
                                                    sink.set_volume(0.0);
                                                    e.fade_in = Some((Instant::now(), e.pending_xf));
                                                } else {
                                                    sink.set_volume(e.volume * e.volume);
                                                }
                                                e.pending_xf = 0.0;
                                                sink.append(dec);
                                                if e.want_play { sink.play() } else { sink.pause() }
                                                e.loaded = true;
                                                e.fails = 0;
                                                if let Some(t) = e.current().and_then(|i| e.queue.get(i)) {
                                                    listener(Event::Started(t.clone()));
                                                }
                                            }
                                            Err(err) => {
                                                e.error = Some(err);
                                                e.fails += 1;
                                                if e.fails < 3 && e.pos + 1 < e.order.len() {
                                                    let msg = e.error.clone();
                                                    advance(&mut e, &mut sink, &mut fading, false, 0.0);
                                                    e.error = msg;
                                                } else {
                                                    e.want_play = false;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    if e.loaded && !e.loading && !e.external && e.want_play && sink.empty() {
                        advance(&mut e, &mut sink, &mut fading, true, 0.0);
                        dirty = true;
                    }
                    let cf = xf_secs();
                    let has_next = e.repeat != "off" || e.pos + 1 < e.order.len();
                    if cf > 0.05 && has_next && e.loaded && !e.loading && !e.external && e.want_play && !sink.empty() && e.duration > cf * 3.0 {
                        let left = e.duration - sink.get_pos().as_secs_f64();
                        if left <= cf {
                            advance(&mut e, &mut sink, &mut fading, true, left.max(0.8));
                            dirty = true;
                        }
                    }
                    fading.retain(|f| {
                        let t = f.at.elapsed().as_secs_f64() / f.secs.max(0.05);
                        if t >= 1.0 || f.sink.empty() {
                            f.sink.stop();
                            false
                        } else {
                            f.sink.set_volume(f.from * ((t * std::f64::consts::FRAC_PI_2).cos() as f32));
                            true
                        }
                    });
                    if let Some((at, secs)) = e.fade_in {
                        let t = at.elapsed().as_secs_f64() / secs.max(0.05);
                        let full = e.volume * e.volume;
                        if t >= 1.0 || !e.want_play {
                            sink.set_volume(full);
                            e.fade_in = None;
                        } else {
                            sink.set_volume(full * ((t * std::f64::consts::FRAC_PI_2).sin() as f32));
                        }
                    }
                    if queue_dirty {
                        let ordered: Vec<Track> = e.queue.clone();
                        listener(Event::Queue(ordered, e.current()));
                    }
                    if dirty || (e.want_play && last_emit.elapsed() >= Duration::from_millis(500)) {
                        listener(Event::Status(status(&e, &sink)));
                        last_emit = Instant::now();
                    }
                }
            })
            .expect("hilo de audio");
        me
    }

    fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }

    pub fn play(&self, tracks: Vec<Track>, start: usize) {
        self.send(Cmd::Play(tracks, start))
    }
    pub fn toggle(&self) {
        self.send(Cmd::Toggle)
    }
    pub fn set_playing(&self, p: bool) {
        self.send(Cmd::SetPlaying(p))
    }
    pub fn next(&self) {
        self.send(Cmd::Next)
    }
    pub fn prev(&self) {
        self.send(Cmd::Prev)
    }
    pub fn seek(&self, secs: f64) {
        self.send(Cmd::Seek(secs))
    }
    pub fn volume(&self, v: f32) {
        self.send(Cmd::Volume(v))
    }
    pub fn enqueue(&self, t: Vec<Track>) {
        self.send(Cmd::Enqueue(t))
    }
    pub fn play_next(&self, t: Track) {
        self.send(Cmd::PlayNext(t))
    }
    pub fn jump(&self, i: usize) {
        self.send(Cmd::Jump(i))
    }
    pub fn remove(&self, i: usize) {
        self.send(Cmd::Remove(i))
    }
    pub fn shuffle(&self, s: bool) {
        self.send(Cmd::Shuffle(s))
    }
    pub fn repeat(&self, r: String) {
        self.send(Cmd::Repeat(r))
    }
    pub fn ext_progress(&self, pos: f64, dur: f64) {
        self.send(Cmd::ExtProgress(pos, dur))
    }
    pub fn ext_ended(&self) {
        self.send(Cmd::ExtEnded)
    }
    pub fn ext_failed(&self, err: String) {
        self.send(Cmd::ExtFailed(err))
    }
    pub fn update(&self, t: Track) {
        self.send(Cmd::Update(t))
    }
}
