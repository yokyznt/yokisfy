use crate::model::*;
use crate::sources::ytmusic;
use crate::Core;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

static MIXES: Mutex<Option<(i64, HashMap<String, Vec<Track>>)>> = Mutex::new(None);

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn day() -> i64 {
    (now() - 6 * 3600) / 86400
}

fn first_artist(a: &str) -> String {
    a.split([',', '&']).next().unwrap_or(a).split(" feat").next().unwrap_or(a).trim().to_string()
}

fn interleave(lists: Vec<Vec<Track>>, max: usize, seen: &mut HashSet<String>) -> Vec<Track> {
    let mut out = Vec::new();
    let mut its: Vec<std::vec::IntoIter<Track>> = lists.into_iter().map(|l| l.into_iter()).collect();
    loop {
        let mut any = false;
        for it in its.iter_mut() {
            if let Some(t) = it.next() {
                any = true;
                if seen.insert(t.id.clone()) {
                    out.push(t);
                }
            }
        }
        if !any || out.len() >= max {
            break;
        }
    }
    out.truncate(max);
    out
}

impl Core {
    fn top_tracks(&self, days: i64) -> Vec<(Track, usize)> {
        let plays = self.lib.plays(now() - days * 86400).unwrap_or_default();
        let mut m: HashMap<String, (Track, usize)> = HashMap::new();
        for (t, _) in plays {
            if t.live {
                continue;
            }
            m.entry(t.id.clone()).or_insert((t, 0)).1 += 1;
        }
        let mut v: Vec<(Track, usize)> = m.into_values().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    pub fn stats(&self, days: i64, offset: i64) -> Value {
        let since = if days > 0 { now() - days * 86400 } else { 0 };
        let plays = self.lib.plays(since).unwrap_or_default();
        let mut secs = 0.0;
        let mut tracks: HashMap<String, (Track, usize, f64)> = HashMap::new();
        let mut artists: HashMap<String, (usize, f64, Option<String>)> = HashMap::new();
        let mut hours = [0usize; 24];
        let mut days_map: HashMap<i64, f64> = HashMap::new();
        for (t, at) in &plays {
            let d = if t.duration > 0.0 { t.duration.min(900.0) } else { 180.0 };
            secs += d;
            let e = tracks.entry(t.id.clone()).or_insert((t.clone(), 0, 0.0));
            e.1 += 1;
            e.2 += d;
            let a = artists.entry(first_artist(&t.artist)).or_insert((0, 0.0, t.cover.clone()));
            a.0 += 1;
            a.1 += d;
            let local = at + offset;
            hours[((local % 86400) / 3600) as usize] += 1;
            *days_map.entry(local / 86400).or_default() += d;
        }
        let mut tv: Vec<(Track, usize, f64)> = tracks.into_values().collect();
        tv.sort_by(|a, b| b.1.cmp(&a.1));
        let mut av: Vec<(String, (usize, f64, Option<String>))> = artists.into_iter().collect();
        av.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
        let today = (now() + offset) / 86400;
        let span = if days > 0 { days.min(60) } else { 30 };
        let per_day: Vec<Value> = (0..span).rev().map(|i| json!({ "day": today - i, "minutes": (days_map.get(&(today - i)).copied().unwrap_or(0.0) / 60.0).round() })).collect();
        json!({
            "plays": plays.len(),
            "minutes": (secs / 60.0).round(),
            "uniqueTracks": tv.len(),
            "uniqueArtists": av.len(),
            "topTracks": tv.iter().take(15).map(|(t, n, s)| json!({ "track": t, "plays": n, "minutes": (s / 60.0).round() })).collect::<Vec<_>>(),
            "topArtists": av.iter().take(12).map(|(a, (n, s, c))| json!({ "name": a, "plays": n, "minutes": (s / 60.0).round(), "cover": c })).collect::<Vec<_>>(),
            "hours": hours,
            "perDay": per_day,
        })
    }

    pub async fn mixes(&self) -> Vec<Album> {
        let top = self.top_tracks(45);
        if top.len() < 3 {
            return vec![];
        }
        let names = {
            let mut seen = HashSet::new();
            top.iter().map(|(t, _)| first_artist(&t.artist)).filter(|a| seen.insert(a.clone())).take(3).collect::<Vec<_>>().join(", ")
        };
        let cover_at = |i: usize| top.get(i).and_then(|(t, _)| t.cover.clone());
        vec![
            Album { id: "mix:daily".into(), source: "mix".into(), title: "Tu mix de hoy".into(), artist: names, cover: cover_at(0), year: None, license: None },
            Album { id: "mix:discover".into(), source: "mix".into(), title: "Descubrimientos".into(), artist: "Música nueva parecida a la tuya".into(), cover: cover_at(1), year: None, license: None },
            Album { id: "mix:top".into(), source: "mix".into(), title: "Tus favoritas del mes".into(), artist: "Lo que más has puesto".into(), cover: cover_at(2), year: None, license: None },
            Album { id: "mix:sleep".into(), source: "mix".into(), title: "Para dormir".into(), artist: "Tranquilo y suave".into(), cover: None, year: None, license: None },
        ]
    }

    pub async fn mix(&self, kind: &str) -> Result<AlbumDetail> {
        let key = day();
        if let Some((d, m)) = MIXES.lock().ok().and_then(|g| g.clone()) {
            if d == key {
                if let Some(t) = m.get(kind) {
                    return Ok(self.mix_detail(kind, self.lib.resolve(t.clone())));
                }
            }
        }
        let top = self.top_tracks(45);
        let heard: HashSet<String> = self.top_tracks(3650).into_iter().map(|(t, _)| t.id).collect();
        let tracks: Vec<Track> = match kind {
            "top" => top.iter().take(40).map(|(t, _)| t.clone()).collect(),
            "daily" | "discover" => {
                let mut seeds: Vec<Track> = Vec::new();
                let mut arts = HashSet::new();
                for (t, _) in &top {
                    if arts.insert(first_artist(&t.artist)) {
                        seeds.push(t.clone());
                    }
                    if seeds.len() >= 4 {
                        break;
                    }
                }
                let ex: Vec<String> = if kind == "discover" { heard.iter().cloned().collect() } else { vec![] };
                let lists = futures_util::future::join_all(seeds.iter().map(|s| self.radio_for(s, &ex))).await;
                let mut seen = HashSet::new();
                if kind == "discover" {
                    let lists: Vec<Vec<Track>> = lists.into_iter().map(|l| l.into_iter().filter(|t| !heard.contains(&t.id)).collect()).collect();
                    interleave(lists, 40, &mut seen)
                } else {
                    let favs: Vec<Track> = top.iter().take(12).map(|(t, _)| t.clone()).collect();
                    let mut l = vec![favs];
                    l.extend(lists);
                    interleave(l, 40, &mut seen)
                }
            }
            "sleep" => {
                let mut out = Vec::new();
                let mut seen = HashSet::new();
                for q in ["música relajante para dormir", "piano para dormir", "lofi para dormir"] {
                    if let Ok((t, _, _)) = ytmusic::search(&self.client, q).await {
                        out.push(t.into_iter().filter(|t| t.duration == 0.0 || t.duration > 90.0).take(14).collect());
                    }
                }
                interleave(out, 36, &mut seen)
            }
            _ => return Err(crate::Error::Msg("Mix desconocido".into())),
        };
        if let Ok(mut g) = MIXES.lock() {
            let e = g.get_or_insert((key, HashMap::new()));
            if e.0 != key {
                *e = (key, HashMap::new());
            }
            e.1.insert(kind.to_string(), tracks.clone());
        }
        Ok(self.mix_detail(kind, self.lib.resolve(tracks)))
    }

    fn mix_detail(&self, kind: &str, tracks: Vec<Track>) -> AlbumDetail {
        let title = match kind {
            "daily" => "Tu mix de hoy",
            "discover" => "Descubrimientos",
            "top" => "Tus favoritas del mes",
            _ => "Para dormir",
        };
        AlbumDetail {
            album: Album { id: format!("mix:{kind}"), source: "mix".into(), title: title.into(), artist: "Hecho para ti · se renueva cada día".into(), cover: tracks.first().and_then(|t| t.cover.clone()), year: None, license: None },
            tracks,
        }
    }
}
