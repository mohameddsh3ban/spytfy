use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTrack {
    pub track_number: u16,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

pub fn parse_spotify_screenshot(text: &str) -> Vec<ParsedTrack> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let mut tracks: Vec<ParsedTrack> = Vec::new();

    let duration_re = Regex::new(r"(\d{1,2}):(\d{2})\s*$").unwrap();
    let track_num_re = Regex::new(r"^(\d{1,3})\s+").unwrap();
    let date_re = Regex::new(r"\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2},?\s*\d{4}\b").unwrap();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Look for lines starting with a track number
        if let Some(num_caps) = track_num_re.captures(line) {
            let track_num: u16 = num_caps[1].parse().unwrap_or(0);
            if track_num == 0 || track_num > 999 {
                i += 1;
                continue;
            }

            let after_num = &line[num_caps[0].len()..].trim();

            // Check if duration is on this line
            let (title_part, duration_ms) = if let Some(dur_caps) = duration_re.captures(after_num) {
                let min: u64 = dur_caps[1].parse().unwrap_or(0);
                let sec: u64 = dur_caps[2].parse().unwrap_or(0);
                let ms = (min * 60 + sec) * 1000;
                let before_dur = after_num[..dur_caps.get(0).unwrap().start()].trim();
                (before_dur.to_string(), ms)
            } else {
                (after_num.to_string(), 0)
            };

            // Remove date if present
            let title_part = date_re.replace_all(&title_part, "").trim().to_string();

            // Try to extract title, artist, album from the text
            // Spotify format in OCR typically gives: Title\nArtist\nAlbum or Title Artist Album
            let mut title = title_part.clone();
            let mut artist = String::new();
            let mut album = String::new();

            // Check next lines for artist and album (Spotify shows them on separate lines)
            if i + 1 < lines.len() && !track_num_re.is_match(lines[i + 1]) {
                let next = lines[i + 1];
                // If next line doesn't look like a duration or track number, it's likely the artist
                if !duration_re.is_match(next) {
                    artist = next.to_string();
                    i += 1;

                    // Check one more line for album
                    if i + 1 < lines.len() && !track_num_re.is_match(lines[i + 1]) {
                        let next2 = lines[i + 1];
                        if !duration_re.is_match(next2) {
                            // Could be album + date + duration on one line
                            let cleaned = date_re.replace_all(next2, "");
                            let cleaned = duration_re.replace_all(&cleaned, "");
                            let cleaned = cleaned.trim();
                            if !cleaned.is_empty() {
                                album = cleaned.to_string();
                            }
                            i += 1;
                        }
                    }
                }
            }

            // If we got title but no artist, try splitting title by common separators
            if artist.is_empty() && title.contains(" - ") {
                let parts: Vec<String> = title.splitn(2, " - ").map(|s| s.trim().to_string()).collect();
                if parts.len() == 2 {
                    title = parts[0].clone();
                    artist = parts[1].clone();
                }
            }

            // Clean up: remove trailing/leading noise
            title = clean_text(&title);
            artist = clean_text(&artist);
            album = clean_text(&album);

            if !title.is_empty() {
                tracks.push(ParsedTrack {
                    track_number: track_num,
                    title,
                    artist,
                    album,
                    duration_ms,
                    cover_url: None,
                });
            }
        }

        i += 1;
    }

    // Deduplicate by title+artist
    tracks.dedup_by(|a, b| a.title == b.title && a.artist == b.artist);

    tracks
}

fn clean_text(s: &str) -> String {
    s.trim()
        .trim_matches(|c: char| c == '.' || c == '|' || c == '#')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_spotify_screenshot() {
        let text = r#"
1   3azra2eel 2
    Abyusif
    3azra2eel 2         May 23, 2025    3:25
2   Stressed Out
    Twenty One Pilots
    Blurryface          May 23, 2025    3:22
3   Geb Felos
    Molotof, Marwan Pablo
    Geb Felos           May 23, 2025    3:03
"#;

        let tracks = parse_spotify_screenshot(text);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].title, "3azra2eel 2");
        assert_eq!(tracks[1].title, "Stressed Out");
        assert_eq!(tracks[1].artist, "Twenty One Pilots");
        assert_eq!(tracks[2].duration_ms, 183000);
    }
}
