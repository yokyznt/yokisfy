use crate::{Error, Result};
use reqwest::Client;

fn hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r { (g - b) / d + if g < b { 6.0 } else { 0.0 } } else if max == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    (h * 60.0, s, l)
}

pub async fn palette(client: &Client, url: &str) -> Result<Vec<String>> {
    let bytes = client.get(url).send().await?.error_for_status()?.bytes().await?;
    let img = image::load_from_memory(&bytes).map_err(|e| Error::Msg(e.to_string()))?;
    let small = img.resize_exact(40, 40, image::imageops::FilterType::Triangle).to_rgb8();
    let mut bins: Vec<(f32, f32, f32, f32)> = vec![(0.0, 0.0, 0.0, 0.0); 36];
    let mut avg = (0.0f32, 0.0f32, 0.0f32);
    for p in small.pixels() {
        let (r, g, b) = (p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
        avg.0 += r;
        avg.1 += g;
        avg.2 += b;
        let (h, s, l) = hsl(r, g, b);
        if s < 0.18 || !(0.12..=0.88).contains(&l) {
            continue;
        }
        let w = s * (1.0 - (l - 0.5).abs() * 1.6).max(0.1);
        let i = ((h / 10.0) as usize).min(35);
        bins[i].0 += r * w;
        bins[i].1 += g * w;
        bins[i].2 += b * w;
        bins[i].3 += w;
    }
    let n = small.pixels().len() as f32;
    let mut picks: Vec<(usize, f32)> = bins.iter().enumerate().filter(|(_, b)| b.3 > 0.0).map(|(i, b)| (i, b.3)).collect();
    picks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut chosen: Vec<usize> = Vec::new();
    for (i, _) in &picks {
        if chosen.iter().all(|c| { let d = (*c as i32 - *i as i32).abs(); d.min(36 - d) >= 3 }) {
            chosen.push(*i);
        }
        if chosen.len() == 3 {
            break;
        }
    }
    let hex = |r: f32, g: f32, b: f32| format!("#{:02x}{:02x}{:02x}", (r.clamp(0.0, 1.0) * 255.0) as u8, (g.clamp(0.0, 1.0) * 255.0) as u8, (b.clamp(0.0, 1.0) * 255.0) as u8);
    let mut out: Vec<String> = chosen.iter().map(|i| { let b = bins[*i]; hex(b.0 / b.3, b.1 / b.3, b.2 / b.3) }).collect();
    let base = hex(avg.0 / n, avg.1 / n, avg.2 / n);
    while out.len() < 3 {
        out.push(base.clone());
    }
    out.push(base);
    Ok(out)
}
