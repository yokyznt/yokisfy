use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

pub const FREQS: [f32; 10] = [31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub gains: [f32; 10],
    #[serde(default)]
    pub preamp: f32,
    #[serde(default)]
    pub karaoke: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { enabled: false, gains: [0.0; 10], preamp: 0.0, karaoke: false }
    }
}

static CFG: RwLock<Config> = RwLock::new(Config { enabled: false, gains: [0.0; 10], preamp: 0.0, karaoke: false });
static VER: AtomicU64 = AtomicU64::new(1);

pub fn set(c: Config) {
    if let Ok(mut g) = CFG.write() {
        *g = c;
    }
    VER.fetch_add(1, Ordering::Relaxed);
}

pub fn get() -> Config {
    CFG.read().map(|g| g.clone()).unwrap_or_default()
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    fn peak(f: f32, fs: f32, gain: f32, q: f32) -> Biquad {
        let a = 10f32.powf(gain / 40.0);
        let w = 2.0 * std::f32::consts::PI * f / fs;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q);
        let a0 = 1.0 + al / a;
        Biquad { b0: (1.0 + al * a) / a0, b1: (-2.0 * c) / a0, b2: (1.0 - al * a) / a0, a1: (-2.0 * c) / a0, a2: (1.0 - al / a) / a0 }
    }

    fn shelf(f: f32, fs: f32, gain: f32, high: bool) -> Biquad {
        let a = 10f32.powf(gain / 40.0);
        let w = 2.0 * std::f32::consts::PI * f / fs;
        let (s, c) = w.sin_cos();
        let al = s / 2.0 * std::f32::consts::SQRT_2;
        let sa = 2.0 * a.sqrt() * al;
        if high {
            let a0 = (a + 1.0) - (a - 1.0) * c + sa;
            Biquad {
                b0: a * ((a + 1.0) + (a - 1.0) * c + sa) / a0,
                b1: -2.0 * a * ((a - 1.0) + (a + 1.0) * c) / a0,
                b2: a * ((a + 1.0) + (a - 1.0) * c - sa) / a0,
                a1: 2.0 * ((a - 1.0) - (a + 1.0) * c) / a0,
                a2: ((a + 1.0) - (a - 1.0) * c - sa) / a0,
            }
        } else {
            let a0 = (a + 1.0) + (a - 1.0) * c + sa;
            Biquad {
                b0: a * ((a + 1.0) - (a - 1.0) * c + sa) / a0,
                b1: 2.0 * a * ((a - 1.0) - (a + 1.0) * c) / a0,
                b2: a * ((a + 1.0) - (a - 1.0) * c - sa) / a0,
                a1: -2.0 * ((a - 1.0) + (a + 1.0) * c) / a0,
                a2: ((a + 1.0) + (a - 1.0) * c - sa) / a0,
            }
        }
    }

    fn lowpass(f: f32, fs: f32) -> Biquad {
        let w = 2.0 * std::f32::consts::PI * f / fs;
        let (s, c) = w.sin_cos();
        let al = s / std::f32::consts::SQRT_2;
        let a0 = 1.0 + al;
        Biquad { b0: (1.0 - c) / 2.0 / a0, b1: (1.0 - c) / a0, b2: (1.0 - c) / 2.0 / a0, a1: -2.0 * c / a0, a2: (1.0 - al) / a0 }
    }

    #[inline]
    fn run(&self, x: f32, z: &mut [f32; 2]) -> f32 {
        let y = self.b0 * x + z[0];
        z[0] = self.b1 * x - self.a1 * y + z[1];
        z[1] = self.b2 * x - self.a2 * y;
        y
    }
}

#[derive(Default)]
pub struct Chain {
    ver: u64,
    rate: u32,
    ch: usize,
    on: bool,
    karaoke: bool,
    pre: f32,
    filters: Vec<Biquad>,
    state: Vec<[f32; 2]>,
    lp: Biquad,
    lp_state: [f32; 2],
}

impl Chain {
    fn rebuild(&mut self, rate: u32, ch: usize) {
        let c = get();
        self.ver = VER.load(Ordering::Relaxed);
        self.rate = rate;
        self.ch = ch;
        self.on = c.enabled;
        self.karaoke = c.karaoke && ch == 2;
        self.pre = if c.enabled { 10f32.powf(c.preamp.clamp(-12.0, 12.0) / 20.0) } else { 1.0 };
        self.filters.clear();
        let fs = rate as f32;
        if c.enabled {
            for (i, f) in FREQS.iter().enumerate() {
                let g = c.gains[i].clamp(-12.0, 12.0);
                if g.abs() < 0.05 || *f >= fs * 0.45 {
                    continue;
                }
                self.filters.push(match i {
                    0 => Biquad::shelf(*f * 1.6, fs, g, false),
                    9 => Biquad::shelf(*f * 0.7, fs, g, true),
                    _ => Biquad::peak(*f, fs, g, 1.05),
                });
            }
        }
        self.state = vec![[0.0; 2]; self.filters.len() * ch.max(1)];
        self.lp = Biquad::lowpass(170.0, fs);
        self.lp_state = [0.0; 2];
    }

    pub fn process(&mut self, buf: &mut [f32], ch: usize, rate: u32) {
        if self.ver != VER.load(Ordering::Relaxed) || self.rate != rate || self.ch != ch {
            self.rebuild(rate, ch);
        }
        if !self.on && !self.karaoke {
            return;
        }
        let ch = ch.max(1);
        if self.karaoke {
            for fr in buf.chunks_exact_mut(2) {
                let mid = (fr[0] + fr[1]) * 0.5;
                let side = (fr[0] - fr[1]) * 0.5;
                let bass = self.lp.run(mid, &mut self.lp_state);
                fr[0] = side * 1.4 + bass;
                fr[1] = -side * 1.4 + bass;
            }
        }
        if !self.filters.is_empty() {
            for fr in buf.chunks_exact_mut(ch) {
                for (c, s) in fr.iter_mut().enumerate() {
                    let mut x = *s;
                    for (k, f) in self.filters.iter().enumerate() {
                        x = f.run(x, &mut self.state[k * ch + c]);
                    }
                    *s = x;
                }
            }
        }
        let pre = self.pre;
        let boosting = self.on && (pre > 1.0 || self.filters.len() > 0);
        for s in buf.iter_mut() {
            let x = *s * pre;
            *s = if boosting { soft(x) } else { x };
        }
    }
}

#[inline]
fn soft(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.85 {
        x
    } else {
        x.signum() * (0.85 + 0.15 * ((a - 0.85) / 0.15).tanh())
    }
}
