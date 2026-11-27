use discord_rich_presence::activity::{Activity, ActivityType, Assets, Button, StatusDisplayType, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BUILT_IN: &str = match option_env!("YOKISFY_DISCORD_ID") {
    Some(v) => v,
    None => "1576007810498568382",
};
const ICON: &str = "https://raw.githubusercontent.com/yokyznt/yokisfy/main/src-tauri/icons/128x128.png";
static STATE: Mutex<&'static str> = Mutex::new("");

fn set_state(s: &'static str) {
    if let Ok(mut g) = STATE.lock() {
        *g = s;
    }
}

pub fn state() -> &'static str {
    STATE.lock().map(|g| *g).unwrap_or("")
}
static APP_ID: Mutex<String> = Mutex::new(String::new());

pub fn set_app_id(id: Option<String>) {
    let v = id.map(|s| s.trim().to_string()).filter(|s| s.len() >= 15 && s.chars().all(|c| c.is_ascii_digit())).unwrap_or_else(|| BUILT_IN.to_string());
    if let Ok(mut g) = APP_ID.lock() {
        if *g != v {
            *g = v;
            if let Ok(mut l) = LAST.lock() {
                *l = None;
            }
        }
    }
}

fn app_id() -> String {
    APP_ID.lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn available() -> bool {
    !app_id().is_empty()
}

#[derive(Clone, PartialEq)]
struct Info {
    id: String,
    title: String,
    artist: String,
    cover: Option<String>,
    start: i64,
    end: Option<i64>,
}

static TX: OnceLock<Mutex<Sender<Option<Info>>>> = OnceLock::new();
static LAST: Mutex<Option<Info>> = Mutex::new(None);
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn show(c: &mut DiscordIpcClient, i: &Info) -> bool {
    let large = i.cover.as_deref().filter(|u| u.starts_with("https://") && u.len() < 256).unwrap_or(ICON);
    let mut ts = Timestamps::new().start(i.start);
    if let Some(e) = i.end {
        ts = ts.end(e);
    }
    let title: String = i.title.chars().take(120).collect();
    let artist: String = i.artist.chars().take(120).collect();
    let a = Activity::new()
        .activity_type(ActivityType::Listening)
        .status_display_type(StatusDisplayType::Details)
        .details(title.as_str())
        .state(artist.as_str())
        .assets(Assets::new().large_image(large).large_text("Yokisfy").small_image(ICON).small_text("Yokisfy"))
        .timestamps(ts)
        .buttons(vec![Button::new("Descargar Yokisfy", "https://github.com/yokyznt/yokisfy/releases/latest")]);
    c.set_activity(a).is_ok()
}

fn worker(rx: std::sync::mpsc::Receiver<Option<Info>>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut want: Option<Info> = None;
    let mut dirty = false;
    loop {
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(m) => {
                if m != want {
                    want = m;
                    dirty = true;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if !dirty && client.is_some() {
            continue;
        }
        if client.is_none() {
            if want.is_none() {
                dirty = false;
                continue;
            }
            let mut c = DiscordIpcClient::new(app_id());
            if c.connect().is_err() {
                set_state("no_discord");
                continue;
            }
            client = Some(c);
        }
        let Some(c) = client.as_mut() else { continue };
        let ok = match &want {
            Some(i) => show(c, i),
            None => c.clear_activity().is_ok(),
        };
        if ok {
            dirty = false;
            set_state(if want.is_some() { "showing" } else { "idle" });
        } else {
            set_state("error");
            let _ = c.close();
            client = None;
            dirty = true;
        }
    }
}

pub fn set_enabled(on: bool) {
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
    if !on {
        send(None);
    }
}

fn send(m: Option<Info>) {
    if !available() {
        return;
    }
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel();
        let _ = std::thread::Builder::new().name("yk-discord".into()).spawn(move || worker(rx));
        Mutex::new(tx)
    });
    if let Ok(t) = tx.lock() {
        let _ = t.send(m);
    }
}

pub fn update(s: &yokisfy_core::player::Status) {
    let info = match &s.track {
        Some(t) if s.playing && ON.load(std::sync::atomic::Ordering::Relaxed) => {
            let start = now_ms() - (s.position * 1000.0) as i64;
            let end = (!t.live && s.duration > 1.0).then(|| start + (s.duration * 1000.0) as i64);
            Some(Info { id: t.id.clone(), title: t.title.clone(), artist: t.artist.clone(), cover: t.cover.clone(), start, end })
        }
        _ => None,
    };
    let mut g = match LAST.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let same = match (&*g, &info) {
        (None, None) => true,
        (Some(a), Some(b)) => a.id == b.id && (a.start - b.start).abs() < 3000,
        _ => false,
    };
    if same {
        return;
    }
    *g = info.clone();
    drop(g);
    send(info);
}
