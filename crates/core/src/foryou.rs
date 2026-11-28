use crate::model::*;
use crate::sources::ytmusic;
use crate::Core;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

static PERSONAL: std::sync::Mutex<Option<(String, i64, Vec<Section>)>> = std::sync::Mutex::new(None);
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

    pub fn taste(&self) -> Vec<(String, Track)> {
        let t0 = now();
        let plays = self.lib.plays(t0 - 21 * 86400).unwrap_or_default();
        let mut m: HashMap<String, (f64, Track, i64)> = HashMap::new();
        for (t, at) in plays {
            if t.live || t.artist.trim().is_empty() {
                continue;
            }
            let w = (-((t0 - at).max(0) as f64) / (2.5 * 86400.0)).exp();
            let e = m.entry(first_artist(&t.artist).to_lowercase()).or_insert((0.0, t.clone(), at));
            e.0 += w;
            if at >= e.2 {
                e.1 = t;
                e.2 = at;
            }
        }
        let mut v: Vec<(f64, Track)> = m.into_values().map(|(w, t, _)| (w, t)).collect();
        v.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        v.into_iter().take(3).map(|(_, t)| (first_artist(&t.artist), t)).collect()
    }

    pub async fn personal(&self) -> Vec<Section> {
        let taste = self.taste();
        if taste.is_empty() {
            return vec![];
        }
        let key = taste.iter().map(|(a, _)| a.as_str()).collect::<Vec<_>>().join("|");
        if let Some((k, at, s)) = PERSONAL.lock().ok().and_then(|g| g.clone()) {
            if k == key && now() - at < 1200 {
                return s;
            }
        }
        let details = futures_util::future::join_all(taste.iter().map(|(a, _)| ytmusic::artist_by_name(&self.client, a))).await;
        let radios = futures_util::future::join_all(taste.iter().map(|(_, t)| self.radio_for(t, &[]))).await;
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let sec = |title: String, albums: Vec<Album>, tracks: Vec<Track>, artists: Vec<ytmusic::Artist>| Section { title, albums, tracks, artists };
        for (i, ((name, _), (d, r))) in taste.iter().zip(details.into_iter().zip(radios)).enumerate() {
            let d = d.ok();
            let name = d.as_ref().map(|d| d.artist.name.clone()).unwrap_or_else(|| name.clone());
            let mine = first_artist(&name).to_lowercase();
            let mut from_radio: Vec<ytmusic::Artist> = Vec::new();
            let mut an = HashSet::new();
            for t in &r {
                let a = first_artist(&t.artist);
                let k = a.to_lowercase();
                if k != mine && !k.is_empty() && !taste.iter().any(|(x, _)| x.to_lowercase() == k) && an.insert(k) {
                    from_radio.push(ytmusic::Artist { id: a.clone(), name: a, cover: t.cover.clone(), subtitle: None });
                }
            }
            let radio: Vec<Track> = r.into_iter().filter(|t| seen.insert(t.id.clone())).take(20).collect();
            if !radio.is_empty() {
                out.push(sec(format!("Porque escuchas {name}"), vec![], self.lib.resolve(radio), vec![]));
            }
            if let Some(d) = d {
                if i == 0 {
                    let top: Vec<Track> = d.top.into_iter().filter(|t| seen.insert(t.id.clone())).take(12).collect();
                    if !top.is_empty() {
                        out.push(sec(format!("Lo mejor de {name}"), vec![], self.lib.resolve(top), vec![]));
                    }
                    let mut albums = d.albums;
                    albums.extend(d.singles);
                    if !albums.is_empty() {
                        out.push(sec(format!("Discografía de {name}"), albums.into_iter().take(14).collect(), vec![], vec![]));
                    }
                }
                let mut rel = d.related;
                if rel.is_empty() {
                    rel = std::mem::take(&mut from_radio);
                }
                let rel: Vec<ytmusic::Artist> = rel.into_iter().filter(|a| seen.insert(format!("ar:{}", a.name.to_lowercase()))).take(12).collect();
                if !rel.is_empty() {
                    out.push(sec(format!("Artistas parecidos a {name}"), vec![], vec![], rel));
                }
            } else {
                let rel: Vec<ytmusic::Artist> = from_radio.into_iter().filter(|a| seen.insert(format!("ar:{}", a.name.to_lowercase()))).take(12).collect();
                if !rel.is_empty() {
                    out.push(sec(format!("Artistas parecidos a {name}"), vec![], vec![], rel));
                }
            }
        }
        if let Ok(mut g) = PERSONAL.lock() {
            *g = Some((key, now(), out.clone()));
        }
        out
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
