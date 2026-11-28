use rodio::source::SeekError;
use rodio::Source;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub static LEVELS: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
pub static LEVELS_AT: AtomicU64 = AtomicU64::new(0);

pub fn levels() -> [f32; 3] {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    if now.saturating_sub(LEVELS_AT.load(Ordering::Relaxed)) > 400 {
        return [0.0; 3];
    }
    [0, 1, 2].map(|i| f32::from_bits(LEVELS[i].load(Ordering::Relaxed)))
}

struct Sized<R> {
    inner: R,
    len: Option<u64>,
    seekable: bool,
}

impl<R: Read> Read for Sized<R> {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(b)
    }
}

impl<R: Seek> Seek for Sized<R> {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(p)
    }
}

impl<R: Read + Seek + Send + Sync> MediaSource for Sized<R> {
    fn is_seekable(&self) -> bool {
        self.seekable
    }
    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

type Chunk = (Vec<f32>, u16, u32);

struct Dec {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    channels: u16,
}

impl Dec {
    fn next(&mut self) -> Option<Chunk> {
        let mut errors = 0;
        loop {
            let packet = self.format.next_packet().ok()?;
            if packet.track_id() != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    if decoded.frames() == 0 {
                        continue;
                    }
                    let mut sb = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                    sb.copy_interleaved_ref(decoded);
                    self.channels = spec.channels.count() as u16;
                    return Some((sb.samples().to_vec(), self.channels, spec.rate));
                }
                Err(SymError::DecodeError(_)) if errors < 8 => errors += 1,
                Err(_) => return None,
            }
        }
    }

    fn seek(&mut self, target: Duration) -> Result<Option<Chunk>, String> {
        let res = self
            .format
            .seek(SeekMode::Accurate, SeekTo::Time { time: Time::from(target.as_secs_f64()), track_id: Some(self.track_id) })
            .map_err(|e| e.to_string())?;
        self.decoder.reset();
        let mut skip = res.required_ts.saturating_sub(res.actual_ts) as usize * self.channels.max(1) as usize;
        while let Some(mut c) = self.next() {
            if skip < c.0.len() {
                c.0.drain(..skip);
                return Ok(Some(c));
            }
            skip -= c.0.len();
        }
        Ok(None)
    }
}

#[derive(Default)]
struct Queue {
    chunks: std::collections::VecDeque<Chunk>,
    samples: usize,
    cap: usize,
    ended: bool,
    stop: bool,
    seek: Option<Duration>,
    seek_res: Option<Result<(), String>>,
}

struct Ahead {
    q: std::sync::Mutex<Queue>,
    cv: std::sync::Condvar,
}

fn run_ahead(sh: std::sync::Arc<Ahead>, mut dec: Dec) {
    let mut g = sh.q.lock().unwrap();
    loop {
        if g.stop {
            return;
        }
        if let Some(pos) = g.seek.take() {
            drop(g);
            let r = dec.seek(pos);
            g = sh.q.lock().unwrap();
            g.chunks.clear();
            g.samples = 0;
            g.ended = false;
            match r {
                Ok(Some(c)) => {
                    g.samples += c.0.len();
                    g.chunks.push_back(c);
                    g.seek_res = Some(Ok(()));
                }
                Ok(None) => {
                    g.ended = true;
                    g.seek_res = Some(Ok(()));
                }
                Err(e) => g.seek_res = Some(Err(e)),
            }
            sh.cv.notify_all();
            continue;
        }
        if g.ended || g.samples >= g.cap {
            g = sh.cv.wait_timeout(g, Duration::from_millis(250)).unwrap().0;
            continue;
        }
        drop(g);
        let c = dec.next();
        g = sh.q.lock().unwrap();
        if g.seek.is_some() {
            continue;
        }
        match c {
            Some(c) => {
                g.cap = (c.2 as usize * c.1.max(1) as usize * 10).max(240_000);
                g.samples += c.0.len();
                g.chunks.push_back(c);
            }
            None => g.ended = true,
        }
        sh.cv.notify_all();
    }
}

pub struct Stream {
    ahead: std::sync::Arc<Ahead>,
    buf: Vec<f32>,
    idx: usize,
    channels: u16,
    rate: u32,
    total: Option<Duration>,
    meter: [f32; 6],
    dsp: crate::dsp::Chain,
    an: crate::spectrum::Analyzer,
}

impl Drop for Stream {
    fn drop(&mut self) {
        if let Ok(mut g) = self.ahead.q.lock() {
            g.stop = true;
        }
        self.ahead.cv.notify_all();
    }
}

fn err(e: impl std::fmt::Display) -> SeekError {
    SeekError::Other(Box::new(std::io::Error::other(e.to_string())))
}

impl Stream {
    pub fn open<R: Read + Seek + Send + Sync + 'static>(reader: R, len: Option<u64>, ext: &str, seekable: bool) -> Result<Self, String> {
        let mss = MediaSourceStream::new(Box::new(Sized { inner: reader, len, seekable }), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        if !ext.is_empty() {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions { enable_gapless: true, ..Default::default() }, &MetadataOptions::default())
            .map_err(|e| format!("formato no reconocido: {e}"))?;
        let format = probed.format;
        let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).ok_or("sin pista de audio")?;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("códec no soportado: {e}"))?;
        let total = track
            .codec_params
            .time_base
            .zip(track.codec_params.n_frames)
            .map(|(tb, n)| tb.calc_time(n))
            .map(|t: Time| Duration::from_secs_f64(t.seconds as f64 + t.frac));
        let mut dec = Dec { track_id: track.id, channels: track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2), format, decoder };
        let first = dec.next().ok_or("no se pudo decodificar el audio")?;
        let ahead = std::sync::Arc::new(Ahead { q: std::sync::Mutex::new(Queue { cap: 192_000, ..Default::default() }), cv: std::sync::Condvar::new() });
        let sh = ahead.clone();
        std::thread::Builder::new().name("yk-decode".into()).spawn(move || run_ahead(sh, dec)).map_err(|e| e.to_string())?;
        let mut s = Stream { ahead, channels: first.1, rate: first.2, buf: Vec::new(), meter: [0.0; 6], an: crate::spectrum::Analyzer::new(), dsp: Default::default(), idx: 0, total };
        s.load(first);
        {
            let need = s.rate as usize * s.channels.max(1) as usize * 2;
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            let mut g = s.ahead.q.lock().unwrap();
            while g.samples < need && !g.ended {
                let now = std::time::Instant::now();
                if now >= deadline {
                    break;
                }
                g = s.ahead.cv.wait_timeout(g, deadline - now).unwrap().0;
            }
        }
        Ok(s)
    }

    fn load(&mut self, c: Chunk) {
        self.buf = c.0;
        self.channels = c.1;
        self.rate = c.2;
        self.idx = 0;
        self.dsp.process(&mut self.buf, self.channels as usize, self.rate);
    }

    fn refill(&mut self) -> bool {
        let mut g = self.ahead.q.lock().unwrap();
        loop {
            if let Some(c) = g.chunks.pop_front() {
                g.samples -= c.0.len();
                drop(g);
                self.ahead.cv.notify_all();
                self.load(c);
                return true;
            }
            if g.ended || g.stop {
                return false;
            }
            g = self.ahead.cv.wait_timeout(g, Duration::from_millis(500)).unwrap().0;
        }
    }
}

impl Iterator for Stream {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.idx >= self.buf.len() && !self.refill() {
            return None;
        }
        let s = self.buf[self.idx];
        if self.channels <= 1 || self.idx % self.channels as usize == 0 {
            self.an.feed(s, self.rate);
        }
        self.idx += 1;
        let m = &mut self.meter;
        m[0] += 0.02 * (s - m[0]);
        let hi = s - m[1];
        m[1] = s;
        m[2] += s * s;
        m[3] += m[0] * m[0];
        m[4] += hi * hi;
        m[5] += 1.0;
        if m[5] >= 2048.0 {
            let n = m[5];
            let vals = [(m[2] / n).sqrt(), (m[3] / n).sqrt() * 2.2, (m[4] / n).sqrt() * 1.4];
            for (i, v) in vals.iter().enumerate() {
                LEVELS[i].store(v.min(1.5).to_bits(), Ordering::Relaxed);
            }
            LEVELS_AT.store(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0), Ordering::Relaxed);
            m[2] = 0.0;
            m[3] = 0.0;
            m[4] = 0.0;
            m[5] = 0.0;
        }
        Some(s)
    }
}

impl Source for Stream {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buf.len().saturating_sub(self.idx).max(1))
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let mut target = pos;
        if let Some(t) = self.total {
            target = target.min(t.saturating_sub(Duration::from_millis(50)));
        }
        let res = {
            let mut g = self.ahead.q.lock().unwrap();
            g.seek = Some(target);
            g.seek_res = None;
            g.chunks.clear();
            g.samples = 0;
            self.ahead.cv.notify_all();
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            loop {
                if let Some(r) = g.seek_res.take() {
                    break r;
                }
                let now = std::time::Instant::now();
                if now >= deadline {
                    break Err("la fuente no respondió a tiempo".to_string());
                }
                g = self.ahead.cv.wait_timeout(g, deadline - now).unwrap().0;
            }
        };
        res.map_err(err)?;
        self.buf.clear();
        self.idx = 0;
        self.refill();
        Ok(())
    }
}
