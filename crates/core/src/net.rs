use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const JUMP: u64 = 768 * 1024;

#[derive(Default)]
struct Inner {
    data: Vec<u8>,
    base: u64,
    live: bool,
    total: Option<u64>,
    done: bool,
    error: Option<String>,
    gen: u64,
    ranges: bool,
}

struct Shared {
    inner: Mutex<Inner>,
    cv: Condvar,
    cancel: AtomicBool,
    url: String,
    client: reqwest::blocking::Client,
}

pub struct HttpReader {
    shared: Arc<Shared>,
    pos: u64,
}

pub const USER_AGENT: &str = concat!("Yokisfy/", env!("CARGO_PKG_VERSION"), " (+https://github.com/yokyznt/yokisfy)");

fn fail(s: &Shared, gen: u64, e: String) {
    let mut g = s.inner.lock().unwrap();
    if g.gen == gen {
        g.error = Some(e);
        g.done = true;
        s.cv.notify_all();
    }
}

fn worker(s: Arc<Shared>, gen: u64, start: u64) {
    let mut req = s.client.get(&s.url);
    if start > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={start}-"));
    }
    let mut resp = match req.send().and_then(|r| r.error_for_status()) {
        Ok(r) => r,
        Err(e) => return fail(&s, gen, e.to_string()),
    };
    {
        let mut g = s.inner.lock().unwrap();
        if g.gen != gen {
            return;
        }
        let partial = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        if start > 0 && !partial {
            g.ranges = false;
            g.base = 0;
        }
        if start == 0 {
            g.ranges = resp.headers().get(reqwest::header::ACCEPT_RANGES).and_then(|v| v.to_str().ok()).map(|v| v.contains("bytes")).unwrap_or(false);
            g.total = if g.live { None } else { resp.content_length() };
        } else if partial && g.total.is_none() {
            g.total = resp.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()).and_then(|v| v.rsplit('/').next()).and_then(|v| v.parse().ok());
        }
        if let Some(t) = g.total {
            let need = (t.saturating_sub(g.base) as usize).min(256 * 1024 * 1024);
            g.data.reserve(need);
        }
    }
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if s.cancel.load(Ordering::Relaxed) {
            return;
        }
        match resp.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let mut g = s.inner.lock().unwrap();
                if g.gen != gen {
                    return;
                }
                g.data.extend_from_slice(&buf[..n]);
                s.cv.notify_all();
            }
            Err(e) => return fail(&s, gen, e.to_string()),
        }
    }
    let mut g = s.inner.lock().unwrap();
    if g.gen == gen {
        g.done = true;
        s.cv.notify_all();
    }
}

impl HttpReader {
    pub fn open(url: &str) -> io::Result<Self> {
        Self::open_with(url, false)
    }

    pub fn open_live(url: &str) -> io::Result<Self> {
        Self::open_with(url, true)
    }

    fn open_with(url: &str, live: bool) -> io::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(None)
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(io::Error::other)?;
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner { live, ..Default::default() }),
            cv: Condvar::new(),
            cancel: AtomicBool::new(false),
            url: url.to_string(),
            client,
        });
        let s = shared.clone();
        std::thread::Builder::new().name("yk-http".into()).spawn(move || worker(s, 0, 0))?;
        Ok(Self { shared, pos: 0 })
    }

    fn restart(&self, g: &mut Inner, at: u64) {
        g.gen += 1;
        g.data.clear();
        g.base = at;
        g.done = false;
        g.error = None;
        let s = self.shared.clone();
        let gen = g.gen;
        let _ = std::thread::Builder::new().name("yk-http".into()).spawn(move || worker(s, gen, at));
    }

    pub fn wait_for(&self, bytes: u64, timeout: Duration) -> io::Result<()> {
        let deadline = std::time::Instant::now() + timeout;
        let mut g = self.shared.inner.lock().unwrap();
        loop {
            if let Some(e) = &g.error {
                return Err(io::Error::other(e.clone()));
            }
            if g.done || g.base + g.data.len() as u64 >= bytes {
                return Ok(());
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "la fuente no respondió a tiempo"));
            }
            g = self.shared.cv.wait_timeout(g, deadline - now).unwrap().0;
        }
    }

    pub fn len(&self) -> Option<u64> {
        let g = self.shared.inner.lock().unwrap();
        if g.live {
            return None;
        }
        g.total.or((g.done && g.base == 0).then_some(g.data.len() as u64))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }
}

impl Drop for HttpReader {
    fn drop(&mut self) {
        self.shared.cancel.store(true, Ordering::Relaxed);
    }
}

impl Read for HttpReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let mut g = self.shared.inner.lock().unwrap();
        loop {
            if g.live && self.pos > g.base + 4 * 1024 * 1024 {
                let drop = (self.pos - g.base - 1024 * 1024) as usize;
                g.data.drain(..drop);
                g.base += drop as u64;
            }
            let end = g.base + g.data.len() as u64;
            if self.pos >= g.base && self.pos < end {
                let n = ((end - self.pos) as usize).min(out.len());
                let start = (self.pos - g.base) as usize;
                out[..n].copy_from_slice(&g.data[start..start + n]);
                self.pos += n as u64;
                return Ok(n);
            }
            if let Some(t) = g.total {
                if self.pos >= t {
                    return Ok(0);
                }
            }
            if g.live && self.pos < g.base {
                return Err(io::Error::new(io::ErrorKind::Unsupported, "fuera del búfer en vivo"));
            }
            let far = self.pos < g.base || self.pos > end + JUMP;
            if far && g.ranges && !g.live {
                let at = self.pos;
                self.restart(&mut g, at);
                continue;
            }
            if g.done {
                return match &g.error {
                    Some(e) => Err(io::Error::other(e.clone())),
                    None if self.pos < g.base => Err(io::Error::new(io::ErrorKind::Unsupported, "la fuente no permite saltar")),
                    None => Ok(0),
                };
            }
            if self.shared.cancel.load(Ordering::Relaxed) {
                return Ok(0);
            }
            g = self.shared.cv.wait_timeout(g, Duration::from_millis(500)).unwrap().0;
        }
    }
}

impl Seek for HttpReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let target = match from {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(d) => self.pos as i64 + d,
            SeekFrom::End(_) if self.shared.inner.lock().unwrap().live => return Err(io::Error::new(io::ErrorKind::Unsupported, "transmisión en vivo")),
            SeekFrom::End(d) => {
                let total = match self.len() {
                    Some(t) => t,
                    None => {
                        self.wait_for(u64::MAX, Duration::from_secs(600))?;
                        self.len().unwrap_or(0)
                    }
                };
                total as i64 + d
            }
        };
        if target < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "posición negativa"));
        }
        self.pos = target as u64;
        Ok(self.pos)
    }
}
