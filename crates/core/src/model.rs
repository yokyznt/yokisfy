use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub kind: String,
    pub label: String,
    pub url: String,
    pub size: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub source: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_id: Option<String>,
    pub track_no: Option<u32>,
    pub duration: f64,
    pub cover: Option<String>,
    pub stream_url: String,
    pub formats: Vec<Format>,
    pub license: Option<String>,
    #[serde(default)]
    pub downloadable: bool,
    #[serde(default)]
    pub local_path: Option<String>,
}

impl Track {
    pub fn best_quality(&self) -> Option<&Format> {
        self.formats.iter().find(|f| f.kind != "wav")
    }

    pub fn stream_for(&self, high: bool) -> String {
        if high {
            if let Some(f) = self.best_quality() {
                return f.url.clone();
            }
        }
        self.stream_url.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub source: String,
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    pub year: Option<String>,
    pub license: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub title: String,
    pub albums: Vec<Album>,
    pub tracks: Vec<Track>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("red: {0}")]
    Net(#[from] reqwest::Error),
    #[error("base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Msg(String),
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
