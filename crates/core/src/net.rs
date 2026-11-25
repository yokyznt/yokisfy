use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Default)]
struct Inner {
    data: Vec<u8>,
    base: u64,
    live: bool,
    total: Option<u64>,
    done: bool,
    error: Option<String>,
}

struct Shared {
    inner: Mutex<Inner>,
    cv: Condvar,
    cancel: AtomicBool,
}

pub struct HttpReader {
    shared: Arc<Shared>,
    pos: u64,
}

pub const USER_AGENT: &str = concat!("Yokisfy/", env!("CARGO_PKG_VERSION"), " (+https://github.com/yokyznt/yokisfy)");

impl HttpReader {
    pub fn open(url: &str) -> io::Result<Self> {
        Self::open_with(url, false)
    }

    pub fn open_live(url: &str) -> io::Result<Self> {
        Self::open_with(url, true)
    }

    fn open_with(url: &str, live: bool) -> io::Result<Self> {
        let shared = Arc::new(Shared { inner: Mutex::new(Inner { live, ..Default::default() }), cv: Condvar::new(), cancel: AtomicBool::new(false) });
        let s = shared.clone();
        let url = url.to_string();
        std::thread::Builder::new().name("yk-http".into()).spawn(move || {
            let fail = |s: &Shared, e: String| {
                let mut g = s.inner.lock().unwrap();
                g.error = Some(e);
                g.done = true;
                s.cv.notify_all();
            };
            let client = match reqwest::blocking::Client::builder().user_agent(USER_AGENT).timeout(None).connect_timeout(Duration::from_secs(15)).build() {
                Ok(c) => c,
                Err(e) => return fail(&s, e.to_string()),
            };
            let mut resp = match client.get(&url).send().and_then(|r| r.error_for_status()) {
                Ok(r) => r,
                Err(e) => return fail(&s, e.to_string()),
            };
            {
                let mut g = s.inner.lock().unwrap();
                g.total = if g.live { None } else { resp.content_length() };
                if let Some(t) = g.total {
                    g.data.reserve(t as usize);
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
                        g.data.extend_from_slice(&buf[..n]);
                        s.cv.notify_all();
                    }
                    Err(e) => return fail(&s, e.to_string()),
                }
            }
            let mut g = s.inner.lock().unwrap();
            g.done = true;
            s.cv.notify_all();
        })?;
        Ok(Self { shared, pos: 0 })
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
        g.total.or(g.done.then_some(g.data.len() as u64))
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
            let len = g.base + g.data.len() as u64;
            if self.pos < len && self.pos >= g.base {
                let n = ((len - self.pos) as usize).min(out.len());
                let start = (self.pos - g.base) as usize;
                out[..n].copy_from_slice(&g.data[start..start + n]);
                self.pos += n as u64;
                return Ok(n);
            }
            if self.pos < g.base {
                return Err(io::Error::new(io::ErrorKind::Unsupported, "fuera del búfer en vivo"));
            }
            if g.done {
                return match &g.error {
                    Some(e) if self.pos < g.total.unwrap_or(0) => Err(io::Error::other(e.clone())),
                    _ => Ok(0),
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
