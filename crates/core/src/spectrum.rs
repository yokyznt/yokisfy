use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const BANDS: usize = 8;
const FREQS: [f32; BANDS] = [60.0, 150.0, 400.0, 1000.0, 2500.0, 5000.0, 9000.0, 14000.0];

pub static SPEC: [AtomicU32; BANDS + 1] = [const { AtomicU32::new(0) }; BANDS + 1];
pub static SPEC_AT: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn read() -> Vec<f32> {
    if now_ms().saturating_sub(SPEC_AT.load(Ordering::Relaxed)) > 400 {
        return vec![0.0; BANDS + 1];
    }
    SPEC.iter().map(|a| f32::from_bits(a.load(Ordering::Relaxed))).collect()
}

#[derive(Clone, Copy, Default)]
struct Bp {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Bp {
    fn new(f: f32, fs: f32) -> Bp {
        let f = f.min(fs * 0.45);
        let w = 2.0 * std::f32::consts::PI * f / fs;
        let q = 1.4;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Bp { b0: alpha / a0, b2: -alpha / a0, a1: -2.0 * w.cos() / a0, a2: (1.0 - alpha) / a0, ..Default::default() }
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

pub struct Analyzer {
    rate: u32,
    bp: [Bp; BANDS],
    acc: [f32; BANDS + 1],
    n: u32,
    skip: u32,
}

impl Analyzer {
    pub fn new() -> Analyzer {
        Analyzer { rate: 0, bp: [Bp::default(); BANDS], acc: [0.0; BANDS + 1], n: 0, skip: 0 }
    }

    pub fn feed(&mut self, mono: f32, rate: u32) {
        if rate != self.rate {
            self.rate = rate;
            let fs = rate.max(8000) as f32;
            for (i, f) in FREQS.iter().enumerate() {
                self.bp[i] = Bp::new(*f, fs);
            }
        }
        self.skip += 1;
        if self.skip < 2 {
            return;
        }
        self.skip = 0;
        self.acc[BANDS] += mono * mono;
        for i in 0..BANDS {
            let y = self.bp[i].run(mono);
            self.acc[i] += y * y;
        }
        self.n += 1;
        if self.n >= (self.rate / 2 / 30).max(256) {
            let n = self.n as f32;
            for i in 0..=BANDS {
                let gain = if i < BANDS { 3.0 + i as f32 * 0.9 } else { 1.0 };
                let v = ((self.acc[i] / n).sqrt() * gain).min(2.0);
                SPEC[i].store(v.to_bits(), Ordering::Relaxed);
                self.acc[i] = 0.0;
            }
            SPEC_AT.store(now_ms(), Ordering::Relaxed);
            self.n = 0;
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
mod loop_back {
    use super::Analyzer;
    use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;

    static RUNNING: AtomicBool = AtomicBool::new(false);
    static LAST: AtomicU64 = AtomicU64::new(0);
    static DEVICE: Mutex<Option<String>> = Mutex::new(None);

    pub fn ensure(name: Option<String>) {
        LAST.store(super::now_ms(), Ordering::Relaxed);
        let changed = DEVICE.lock().map(|mut d| {
            let c = *d != name;
            *d = name.clone();
            c
        }).unwrap_or(false);
        if RUNNING.load(Ordering::Relaxed) && !changed {
            return;
        }
        if changed && RUNNING.load(Ordering::Relaxed) {
            LAST.store(0, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(600));
            LAST.store(super::now_ms(), Ordering::Relaxed);
        }
        RUNNING.store(true, Ordering::Relaxed);
        std::thread::spawn(move || {
            let _ = run(name);
            RUNNING.store(false, Ordering::Relaxed);
        });
    }

    fn run(name: Option<String>) -> Option<()> {
        let host = rodio::cpal::default_host();
        let dev = name
            .and_then(|n| host.output_devices().ok()?.find(|d| d.name().map(|x| x == n).unwrap_or(false)))
            .or_else(|| host.default_output_device())?;
        let cfg = dev.default_output_config().ok()?;
        let ch = cfg.channels().max(1) as usize;
        let rate = cfg.sample_rate().0;
        let mut an = Analyzer::new();
        let stream = match cfg.sample_format() {
            rodio::cpal::SampleFormat::F32 => dev
                .build_input_stream(
                    &cfg.config(),
                    move |data: &[f32], _: &_| {
                        for f in data.chunks(ch) {
                            an.feed(f.iter().sum::<f32>() / ch as f32, rate);
                        }
                    },
                    |_| {},
                    None,
                )
                .ok()?,
            rodio::cpal::SampleFormat::I16 => dev
                .build_input_stream(
                    &cfg.config(),
                    move |data: &[i16], _: &_| {
                        for f in data.chunks(ch) {
                            an.feed(f.iter().map(|x| *x as f32 / 32768.0).sum::<f32>() / ch as f32, rate);
                        }
                    },
                    |_| {},
                    None,
                )
                .ok()?,
            _ => return None,
        };
        stream.play().ok()?;
        while super::now_ms().saturating_sub(LAST.load(Ordering::Relaxed)) < 3000 {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        Some(())
    }
}

pub fn system(name: Option<String>) -> Vec<f32> {
    #[cfg(windows)]
    {
        loop_back::ensure(name);
        read()
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        vec![0.0; BANDS + 1]
    }
}
