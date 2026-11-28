use yokisfy_core::dsp::Chain;

fn rms(v: &[f32]) -> f32 {
    (v.iter().map(|s| s * s).sum::<f32>() / v.len() as f32).sqrt()
}

fn run(level: f32) -> (f32, f32) {
    let mut c = Chain::default();
    let rate = 44100;
    let mut first = 0.0;
    let mut last = 0.0;
    for k in 0..400 {
        let mut buf: Vec<f32> = (0..2048).flat_map(|i| {
            let t = (k * 1024 + i) as f32 / rate as f32;
            let s = level * (2.0 * std::f32::consts::PI * 220.0 * t).sin() * (1.0 + 0.3 * (t * 0.7).sin());
            [s, s]
        }).take(2048).collect();
        c.process(&mut buf, 2, rate);
        if k == 2 {
            first = rms(&buf);
        }
        last = rms(&buf);
    }
    (first, last)
}

#[test]
fn canciones_suenan_parejo() {
    let quiet = run(0.12);
    let loud = run(0.7);
    let mid = run(0.28);
    println!("quiet {quiet:?} mid {mid:?} loud {loud:?}");
    for (_, l) in [quiet, loud, mid] {
        assert!((l - 0.2).abs() < 0.04, "{l}");
    }
}
