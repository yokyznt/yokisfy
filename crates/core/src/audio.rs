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

pub struct Stream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    buf: Vec<f32>,
    idx: usize,
    channels: u16,
    rate: u32,
    total: Option<Duration>,
    meter: [f32; 6],
    dsp: crate::dsp::Chain,
    an: crate::spectrum::Analyzer,
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
        let mut s = Stream {
            track_id: track.id,
            channels: track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2),
            rate: track.codec_params.sample_rate.unwrap_or(44100),
            format,
            decoder,
            buf: Vec::new(),
            meter: [0.0; 6],
            an: crate::spectrum::Analyzer::new(),
            dsp: Default::default(),
            idx: 0,
            total,
        };
        if !s.refill() {
            return Err("no se pudo decodificar el audio".into());
        }
        Ok(s)
    }

    fn refill(&mut self) -> bool {
        let mut errors = 0;
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(_) => return false,
            };
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
                    self.buf.clear();
                    self.buf.extend_from_slice(sb.samples());
                    self.idx = 0;
                    self.channels = spec.channels.count() as u16;
                    self.rate = spec.rate;
                    self.dsp.process(&mut self.buf, self.channels as usize, self.rate);
                    return true;
                }
                Err(SymError::DecodeError(_)) if errors < 8 => errors += 1,
                Err(_) => return false,
            }
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
        Some(self.buf.len())
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
        let res = self
            .format
            .seek(SeekMode::Accurate, SeekTo::Time { time: Time::from(target.as_secs_f64()), track_id: Some(self.track_id) })
            .map_err(err)?;
        self.decoder.reset();
        self.buf.clear();
        self.idx = 0;
        let mut skip = res.required_ts.saturating_sub(res.actual_ts) as usize * self.channels.max(1) as usize;
        while self.refill() {
            if skip < self.buf.len() {
                self.idx = skip;
                break;
            }
            skip -= self.buf.len();
        }
        Ok(())
    }
}
