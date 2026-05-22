use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyTrack {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub album_id: String,
    pub track_number: u16,
    pub disc_number: u16,
    pub duration_ms: u64,
    pub isrc: Option<String>,
    pub cover_url: Option<String>,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyAlbum {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub tracks: Vec<SpotifyTrack>,
    pub cover_url: Option<String>,
    pub release_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub tracks: Vec<SpotifyTrack>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ResolvedInput {
    Track(SpotifyTrack),
    Album(SpotifyAlbum),
    Playlist(SpotifyPlaylist),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpotifyUrlKind {
    Track,
    Album,
    Playlist,
}

#[derive(Debug, Clone)]
pub struct SpotifyUrl {
    pub kind: SpotifyUrlKind,
    pub id: String,
}
