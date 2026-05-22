use super::browser;
use super::engine;
use super::html_parser;
use super::parser::{self, ParsedTrack};
use crate::spotify::types::{ResolvedInput, SpotifyPlaylist, SpotifyTrack};

#[tauri::command]
pub async fn process_screenshots(image_paths: Vec<String>) -> Result<Vec<ParsedTrack>, String> {
    let mut all_tracks: Vec<ParsedTrack> = Vec::new();

    for path in &image_paths {
        let text = engine::ocr_image(path).await?;
        let mut tracks = parser::parse_spotify_screenshot(&text);
        all_tracks.append(&mut tracks);
    }

    // Renumber tracks sequentially
    for (i, track) in all_tracks.iter_mut().enumerate() {
        track.track_number = (i + 1) as u16;
    }

    // Deduplicate
    all_tracks.dedup_by(|a, b| a.title == b.title && a.artist == b.artist);

    Ok(all_tracks)
}

#[tauri::command]
pub async fn debug_ocr(image_path: String) -> Result<String, String> {
    engine::ocr_image(&image_path).await
}

#[tauri::command]
pub async fn create_playlist_from_tracks(
    playlist_name: String,
    cover_url: Option<String>,
    tracks: Vec<ParsedTrack>,
) -> Result<ResolvedInput, String> {
    let spotify_tracks: Vec<SpotifyTrack> = tracks
        .iter()
        .map(|t| SpotifyTrack {
            id: format!("ocr-{}-{}", t.track_number, slug(&t.title)),
            name: t.title.clone(),
            artists: t.artist.split(", ").map(String::from).collect(),
            album: if t.album.is_empty() {
                playlist_name.clone()
            } else {
                t.album.clone()
            },
            album_id: String::new(),
            track_number: t.track_number,
            disc_number: 1,
            duration_ms: t.duration_ms,
            isrc: None,
            cover_url: t.cover_url.clone().or_else(|| cover_url.clone()),
            release_date: None,
        })
        .collect();

    Ok(ResolvedInput::Playlist(SpotifyPlaylist {
        id: format!("ocr-playlist-{}", slug(&playlist_name)),
        name: playlist_name,
        owner: String::new(),
        tracks: spotify_tracks,
        cover_url,
    }))
}

#[tauri::command]
pub async fn parse_text_tracklist(text: String) -> Result<Vec<ParsedTrack>, String> {
    let mut tracks: Vec<ParsedTrack> = Vec::new();
    let duration_re = regex::Regex::new(r"(\d{1,2}):(\d{2})\s*$").unwrap();

    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try common formats:
        // "Artist - Title"
        // "Title - Artist"
        // "Artist — Title" (em dash)
        // "1. Artist - Title"
        // "Artist - Title 3:42"

        let cleaned = line
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ');

        let (text_part, duration_ms) = if let Some(caps) = duration_re.captures(cleaned) {
            let min: u64 = caps[1].parse().unwrap_or(0);
            let sec: u64 = caps[2].parse().unwrap_or(0);
            (cleaned[..caps.get(0).unwrap().start()].trim(), (min * 60 + sec) * 1000)
        } else {
            (cleaned.trim(), 0u64)
        };

        if text_part.is_empty() {
            continue;
        }

        // Split by " - " or " — " or " – "
        let (artist, title) = if let Some(pos) = text_part.find(" - ") {
            (text_part[..pos].trim().to_string(), text_part[pos + 3..].trim().to_string())
        } else if let Some(pos) = text_part.find(" — ") {
            (text_part[..pos].trim().to_string(), text_part[pos + 4..].trim().to_string())
        } else if let Some(pos) = text_part.find(" – ") {
            (text_part[..pos].trim().to_string(), text_part[pos + 4..].trim().to_string())
        } else {
            // No separator — treat entire line as title
            (String::new(), text_part.to_string())
        };

        let is_title_empty = title.is_empty();
        tracks.push(ParsedTrack {
            track_number: (i + 1) as u16,
            title: if is_title_empty { artist.clone() } else { title },
            artist: if is_title_empty { String::new() } else { artist },
            album: String::new(),
            duration_ms,
            cover_url: None,
        });
    }

    // Renumber
    for (i, t) in tracks.iter_mut().enumerate() {
        t.track_number = (i + 1) as u16;
    }

    Ok(tracks)
}

#[tauri::command]
pub async fn scrape_playlist_tracks(app: tauri::AppHandle, url: String) -> Result<Vec<ParsedTrack>, String> {
    browser::scrape_playlist(&url, &app).await
}

#[tauri::command]
pub async fn parse_spotify_html(html: String) -> Result<Vec<ParsedTrack>, String> {
    let tracks = html_parser::parse_spotify_html(&html);
    if tracks.is_empty() {
        return Err("No tracks found in HTML. Make sure you copied the playlist tracklist HTML.".to_string());
    }
    Ok(tracks)
}

fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
