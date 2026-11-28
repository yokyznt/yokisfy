use crate::model::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

pub struct Library {
    db: Mutex<Connection>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub cover: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub playlist: Playlist,
    pub tracks: Vec<Track>,
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Library {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let c = Connection::open(path)?;
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS tracks(id TEXT PRIMARY KEY, json TEXT NOT NULL, local_path TEXT, liked_at INTEGER, downloaded_at INTEGER);
             CREATE TABLE IF NOT EXISTS playlists(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS playlist_items(playlist_id INTEGER NOT NULL, pos INTEGER NOT NULL, track_id TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS history(track_id TEXT NOT NULL, played_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS idx_items ON playlist_items(playlist_id, pos);
             CREATE INDEX IF NOT EXISTS idx_hist ON history(played_at);",
        )?;
        Ok(Self { db: Mutex::new(c) })
    }

    fn upsert(c: &Connection, t: &Track) -> Result<()> {
        let mut t = t.clone();
        t.local_path = None;
        c.execute(
            "INSERT INTO tracks(id, json) VALUES(?1, ?2) ON CONFLICT(id) DO UPDATE SET json = excluded.json",
            params![t.id, serde_json::to_string(&t)?],
        )?;
        Ok(())
    }

    fn row_track(json: String, local: Option<String>) -> Option<Track> {
        let mut t: Track = serde_json::from_str(&json).ok()?;
        t.local_path = local.filter(|p| Path::new(p).exists());
        Some(t)
    }

    pub fn setting(&self, key: &str) -> Option<String> {
        let c = self.db.lock().unwrap();
        c.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn settings(&self) -> Result<serde_json::Map<String, serde_json::Value>> {
        let c = self.db.lock().unwrap();
        let mut st = c.prepare("SELECT key, value FROM settings")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        Ok(rows.flatten().map(|(k, v)| (k, serde_json::Value::String(v))).collect())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let c = self.db.lock().unwrap();
        c.execute("INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value])?;
        Ok(())
    }

    pub fn resolve(&self, tracks: Vec<Track>) -> Vec<Track> {
        let c = self.db.lock().unwrap();
        tracks
            .into_iter()
            .map(|mut t| {
                if let Ok(Some(p)) = c.query_row("SELECT local_path FROM tracks WHERE id = ?1", [&t.id], |r| r.get::<_, Option<String>>(0)).optional() {
                    t.local_path = p.filter(|p| Path::new(p).exists());
                }
                t
            })
            .collect()
    }

    pub fn liked_ids(&self) -> Result<Vec<String>> {
        let c = self.db.lock().unwrap();
        let mut st = c.prepare("SELECT id FROM tracks WHERE liked_at IS NOT NULL")?;
        let ids = st.query_map([], |r| r.get(0))?.flatten().collect();
        Ok(ids)
    }

    pub fn toggle_like(&self, t: &Track) -> Result<bool> {
        let c = self.db.lock().unwrap();
        Self::upsert(&c, t)?;
        let liked: Option<i64> = c.query_row("SELECT liked_at FROM tracks WHERE id = ?1", [&t.id], |r| r.get(0))?;
        let now_liked = liked.is_none();
        c.execute("UPDATE tracks SET liked_at = ?2 WHERE id = ?1", params![t.id, now_liked.then(now)])?;
        Ok(now_liked)
    }

    fn list(&self, sql: &str, p: impl rusqlite::Params) -> Result<Vec<Track>> {
        let c = self.db.lock().unwrap();
        let mut st = c.prepare(sql)?;
        let rows = st.query_map(p, |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))?;
        Ok(rows.flatten().filter_map(|(j, l)| Self::row_track(j, l)).collect())
    }

    pub fn liked(&self) -> Result<Vec<Track>> {
        self.list("SELECT json, local_path FROM tracks WHERE liked_at IS NOT NULL ORDER BY liked_at DESC", [])
    }

    pub fn downloads(&self) -> Result<Vec<Track>> {
        let v = self.list("SELECT json, local_path FROM tracks WHERE local_path IS NOT NULL ORDER BY downloaded_at DESC", [])?;
        Ok(v.into_iter().filter(|t| t.local_path.is_some()).collect())
    }

    pub fn recent(&self, limit: i64) -> Result<Vec<Track>> {
        self.list(
            "SELECT t.json, t.local_path FROM (SELECT track_id, MAX(played_at) AS last FROM history GROUP BY track_id ORDER BY last DESC LIMIT ?1) h JOIN tracks t ON t.id = h.track_id ORDER BY h.last DESC",
            [limit],
        )
    }

    pub fn plays(&self, since: i64) -> Result<Vec<(Track, i64)>> {
        let c = self.db.lock().unwrap();
        let mut st = c.prepare("SELECT t.json, h.played_at FROM history h JOIN tracks t ON t.id = h.track_id WHERE h.played_at >= ?1 ORDER BY h.played_at")?;
        let rows = st.query_map([since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        Ok(rows.flatten().filter_map(|(j, at)| serde_json::from_str::<Track>(&j).ok().map(|t| (t, at))).collect())
    }

    pub fn merge_sync(&self, liked: &[Track], lists: &[(String, Vec<Track>)]) -> Result<usize> {
        let ok = |t: &Track| !t.live && t.source != "local" && t.source != "pc" && (!t.stream_url.is_empty() || t.source == "youtube");
        let have: std::collections::HashSet<String> = self.liked_ids()?.into_iter().collect();
        let mut n = 0;
        for t in liked.iter().filter(|t| ok(t)) {
            if !have.contains(&t.id) {
                self.toggle_like(t)?;
                n += 1;
            }
        }
        let existing = self.playlists()?;
        for (name, ts) in lists {
            let ts: Vec<Track> = ts.iter().filter(|t| ok(t)).cloned().collect();
            if ts.is_empty() {
                continue;
            }
            let id = match existing.iter().find(|p| p.name == *name) {
                Some(p) => p.id,
                None => self.create_playlist(name)?,
            };
            let cur: std::collections::HashSet<String> = self.playlist(id)?.tracks.into_iter().map(|t| t.id).collect();
            let add: Vec<Track> = ts.into_iter().filter(|t| !cur.contains(&t.id)).collect();
            n += add.len();
            if !add.is_empty() {
                self.add_to_playlist(id, &add)?;
            }
        }
        Ok(n)
    }

    pub fn record_play(&self, t: &Track) -> Result<()> {
        let c = self.db.lock().unwrap();
        Self::upsert(&c, t)?;
        c.execute("INSERT INTO history(track_id, played_at) VALUES(?1, ?2)", params![t.id, now()])?;
        Ok(())
    }

    pub fn set_local(&self, t: &Track, path: &str) -> Result<()> {
        let c = self.db.lock().unwrap();
        Self::upsert(&c, t)?;
        c.execute("UPDATE tracks SET local_path = ?2, downloaded_at = ?3 WHERE id = ?1", params![t.id, path, now()])?;
        Ok(())
    }

    pub fn remove_local(&self, id: &str) -> Result<Option<String>> {
        let c = self.db.lock().unwrap();
        let p: Option<String> = c.query_row("SELECT local_path FROM tracks WHERE id = ?1", [id], |r| r.get(0)).optional()?.flatten();
        c.execute("UPDATE tracks SET local_path = NULL, downloaded_at = NULL WHERE id = ?1", [id])?;
        Ok(p)
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let c = self.db.lock().unwrap();
        let mut st = c.prepare(
            "SELECT p.id, p.name, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id = p.id),
                    (SELECT t.json FROM playlist_items i JOIN tracks t ON t.id = i.track_id WHERE i.playlist_id = p.id ORDER BY i.pos LIMIT 1)
             FROM playlists p ORDER BY p.created_at DESC",
        )?;
        let rows = st.query_map([], |r| {
            let json: Option<String> = r.get(3)?;
            Ok(Playlist {
                id: r.get(0)?,
                name: r.get(1)?,
                count: r.get(2)?,
                cover: json.and_then(|j| serde_json::from_str::<Track>(&j).ok()).and_then(|t| t.cover),
            })
        })?;
        Ok(rows.flatten().collect())
    }

    pub fn create_playlist(&self, name: &str) -> Result<i64> {
        let c = self.db.lock().unwrap();
        c.execute("INSERT INTO playlists(name, created_at) VALUES(?1, ?2)", params![name.trim(), now()])?;
        Ok(c.last_insert_rowid())
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> Result<()> {
        let c = self.db.lock().unwrap();
        c.execute("UPDATE playlists SET name = ?2 WHERE id = ?1", params![id, name.trim()])?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        let c = self.db.lock().unwrap();
        c.execute("DELETE FROM playlist_items WHERE playlist_id = ?1", [id])?;
        c.execute("DELETE FROM playlists WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn add_to_playlist(&self, id: i64, tracks: &[Track]) -> Result<()> {
        let c = self.db.lock().unwrap();
        let mut pos: i64 = c.query_row("SELECT COALESCE(MAX(pos), -1) + 1 FROM playlist_items WHERE playlist_id = ?1", [id], |r| r.get(0))?;
        for t in tracks {
            Self::upsert(&c, t)?;
            c.execute("INSERT INTO playlist_items(playlist_id, pos, track_id) VALUES(?1, ?2, ?3)", params![id, pos, t.id])?;
            pos += 1;
        }
        Ok(())
    }

    pub fn remove_from_playlist(&self, id: i64, index: i64) -> Result<()> {
        let c = self.db.lock().unwrap();
        let pos: Option<i64> = c
            .query_row("SELECT pos FROM playlist_items WHERE playlist_id = ?1 ORDER BY pos LIMIT 1 OFFSET ?2", params![id, index], |r| r.get(0))
            .optional()?;
        if let Some(p) = pos {
            c.execute("DELETE FROM playlist_items WHERE playlist_id = ?1 AND pos = ?2", params![id, p])?;
        }
        Ok(())
    }

    pub fn playlist(&self, id: i64) -> Result<PlaylistDetail> {
        let tracks = self.list(
            "SELECT t.json, t.local_path FROM playlist_items i JOIN tracks t ON t.id = i.track_id WHERE i.playlist_id = ?1 ORDER BY i.pos",
            [id],
        )?;
        let pl = self.playlists()?.into_iter().find(|p| p.id == id).ok_or_else(|| Error::Msg("Playlist no encontrada".into()))?;
        Ok(PlaylistDetail { playlist: pl, tracks })
    }
}
