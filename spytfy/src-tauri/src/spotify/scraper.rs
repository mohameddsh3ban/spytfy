use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::parser::parse_spotify_url;
use super::types::*;

async fn fetch_page_html(url: &str) -> Result<String, String> {
    let client = Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("Page request failed: {e}"))?;

    resp.text()
        .await
        .map_err(|e| format!("Failed to read page: {e}"))
}

fn extract_embedded_json(html: &str) -> Option<Value> {
    // Look for <script id="__NEXT_DATA__" ...>JSON</script>
    let marker = "__NEXT_DATA__";
    if let Some(idx) = html.find(marker) {
        let rest = &html[idx..];
        if let Some(start) = rest.find('>') {
            let json_start = idx + start + 1;
            if let Some(end) = html[json_start..].find("</script>") {
                let json_str = &html[json_start..json_start + end];
                if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                    return Some(val);
                }
            }
        }
    }

    // Look for Spotify resource JSON embedded in script tags
    // Pattern: {"type":"track","name":"...","duration_ms":...}
    if let Some(idx) = html.find("\"duration_ms\"") {
        // Walk backwards to find the start of this JSON object
        let search_start = if idx > 2000 { idx - 2000 } else { 0 };
        let chunk = &html[search_start..std::cmp::min(idx + 500, html.len())];
        // Find a complete JSON object containing duration_ms
        for brace_start in chunk.rmatch_indices('{').map(|(i, _)| i) {
            let remaining = &chunk[brace_start..];
            if let Ok(val) = serde_json::from_str::<Value>(remaining) {
                if val.get("duration_ms").is_some() {
                    return Some(val);
                }
            }
            // Try with a larger slice
            let end_slice = &html[search_start + brace_start..std::cmp::min(search_start + brace_start + 5000, html.len())];
            if let Ok(val) = serde_json::from_str::<Value>(end_slice) {
                if val.get("duration_ms").is_some() {
                    return Some(val);
                }
            }
        }
    }

    None
}

fn track_from_json(json: &Value, track_id: &str) -> Option<SpotifyTrack> {
    let dur = json.get("duration_ms")?.as_u64()?;

    let name = json.get("name")?.as_str()?.to_string();

    let artists: Vec<String> = json.get("artists")
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter().filter_map(|a| a.get("name")?.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let album_obj = json.get("album");
    let album = album_obj
        .and_then(|a| a.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let album_id = album_obj
        .and_then(|a| a.get("id"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();

    let cover_url = album_obj
        .and_then(|a| a.get("images"))
        .and_then(|imgs| imgs.as_array())
        .and_then(|arr| arr.first())
        .and_then(|img| img.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from);

    let release_date = album_obj
        .and_then(|a| a.get("release_date"))
        .and_then(|d| d.as_str())
        .map(String::from);

    let track_number = json.get("track_number").and_then(|n| n.as_u64()).unwrap_or(1) as u16;
    let disc_number = json.get("disc_number").and_then(|n| n.as_u64()).unwrap_or(1) as u16;
    let isrc = json.get("external_ids")
        .and_then(|e| e.get("isrc"))
        .and_then(|i| i.as_str())
        .map(String::from);

    Some(SpotifyTrack {
        id: json.get("id").and_then(|i| i.as_str()).unwrap_or(track_id).to_string(),
        name,
        artists,
        album,
        album_id,
        track_number,
        disc_number,
        duration_ms: dur,
        isrc,
        cover_url,
        release_date,
    })
}

struct TrackPageData {
    title: String,
    description: String,
    image: String,
    duration_ms: u64,
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
}

fn parse_og_tags(html: &str) -> Result<TrackPageData, String> {
    let get_content = |search: &str| -> Option<String> {
        let idx = html.find(search)?;
        let content_start = idx + search.len();
        let end = html[content_start..].find('"')?;
        Some(decode_html_entities(&html[content_start..content_start + end]))
    };

    let title = get_content("property=\"og:title\" content=\"")
        .or_else(|| get_content("name=\"og:title\" content=\""))
        .unwrap_or_default();

    let description = get_content("property=\"og:description\" content=\"")
        .or_else(|| get_content("name=\"description\" content=\""))
        .unwrap_or_default();

    let image = get_content("property=\"og:image\" content=\"")
        .unwrap_or_default();

    // Try multiple duration sources
    let duration_ms = get_content("property=\"music:duration\" content=\"")
        .and_then(|d| {
            let val = d.parse::<u64>().ok()?;
            // If under 1000, it's in seconds; otherwise milliseconds
            Some(if val < 1000 { val * 1000 } else { val })
        })
        // Try ISO 8601 duration from JSON-LD (e.g., "PT3M42S")
        .or_else(|| parse_iso_duration(html))
        // Try "X min Y sec" from description
        .or_else(|| parse_duration_from_text(&description))
        .unwrap_or(0);

    if title.is_empty() {
        return Err("Could not extract track info from Spotify page".to_string());
    }

    Ok(TrackPageData {
        title,
        description,
        image,
        duration_ms,
    })
}

fn parse_iso_duration(html: &str) -> Option<u64> {
    // Try "duration":"PTxMxS" in JSON-LD
    if let Some(ms) = parse_pt_duration(html, "\"duration\":\"PT") {
        return Some(ms);
    }

    // Try "duration_ms":123456 in embedded JSON
    if let Some(idx) = html.find("\"duration_ms\":") {
        let start = idx + "\"duration_ms\":".len();
        let rest = &html[start..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        if let Ok(ms) = rest[..end].parse::<u64>() {
            if ms > 0 {
                return Some(ms);
            }
        }
    }

    // Try "duration":123456 (milliseconds as number)
    if let Some(idx) = html.find("\"duration\":") {
        let start = idx + "\"duration\":".len();
        let rest = html[start..].trim_start();
        if rest.starts_with('"') {
            // String value like "PT3M42S"
            return parse_pt_duration(html, "\"duration\":\"PT");
        } else {
            // Numeric value
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            if let Ok(val) = rest[..end].parse::<u64>() {
                if val > 0 {
                    return Some(if val > 1_000_000 { val / 1000 } else { val });
                }
            }
        }
    }

    None
}

fn parse_pt_duration(html: &str, marker: &str) -> Option<u64> {
    let idx = html.find(marker)?;
    let start = idx + marker.len();
    let rest = &html[start..std::cmp::min(start + 20, html.len())];
    let end = rest.find('"')?;
    let iso = &rest[..end];

    let mut minutes: u64 = 0;
    let mut seconds: u64 = 0;

    if let Some(m_pos) = iso.find('M') {
        minutes = iso[..m_pos].parse().ok()?;
        if let Some(s_str) = iso.get(m_pos + 1..) {
            seconds = s_str.trim_end_matches('S').parse().unwrap_or(0);
        }
    } else {
        seconds = iso.trim_end_matches('S').parse().ok()?;
    }

    Some((minutes * 60 + seconds) * 1000)
}

fn parse_duration_from_text(desc: &str) -> Option<u64> {
    // Look for patterns like "3:42" or "3 min 42 sec"
    let re_colon = regex::Regex::new(r"(\d+):(\d{2})").ok()?;
    if let Some(caps) = re_colon.captures(desc) {
        let min: u64 = caps.get(1)?.as_str().parse().ok()?;
        let sec: u64 = caps.get(2)?.as_str().parse().ok()?;
        return Some((min * 60 + sec) * 1000);
    }
    None
}

fn parse_artist_title(og_title: &str) -> (String, String) {
    let cleaned = og_title
        .trim_end_matches(" | Spotify")
        .trim_end_matches(" - Spotify")
        .trim();

    (String::new(), cleaned.to_string())
}

fn parse_description(desc: &str) -> (Vec<String>, String, Option<String>) {
    // Spotify og:description format for tracks:
    // "Artist1, Artist2 · Album Name · Song · Year"
    // or "Listen to Title on Spotify. Artist · Album · Year"
    let cleaned = desc
        .trim_start_matches("Listen to ")
        .split(" on Spotify")
        .last()
        .unwrap_or(desc)
        .trim_start_matches('.')
        .trim();

    let parts: Vec<&str> = cleaned.split(" · ").collect();

    match parts.len() {
        // "Artist(s) · Album · Song · Year"
        4.. => {
            let artists: Vec<String> = parts[0].split(", ").map(|s| s.trim().to_string()).collect();
            let album = parts[1].trim().to_string();
            let year = parts.last().and_then(|y| {
                if y.trim().len() == 4 && y.trim().parse::<u32>().is_ok() {
                    Some(y.trim().to_string())
                } else {
                    None
                }
            });
            (artists, album, year)
        }
        // "Artist(s) · Album · Year"
        3 => {
            let artists: Vec<String> = parts[0].split(", ").map(|s| s.trim().to_string()).collect();
            let album = parts[1].trim().to_string();
            let year = parts[2].trim().parse::<u32>().ok().map(|_| parts[2].trim().to_string());
            (artists, album, year)
        }
        // "Artist(s) · Album"
        2 => {
            let artists: Vec<String> = parts[0].split(", ").map(|s| s.trim().to_string()).collect();
            let album = parts[1].trim().to_string();
            (artists, album, None)
        }
        _ => (vec![], String::new(), None),
    }
}

#[tauri::command]
pub async fn debug_scrape(url: String) -> Result<String, String> {
    let parsed = parse_spotify_url(&url).ok_or("Invalid Spotify URL")?;
    let kind_str = match parsed.kind {
        SpotifyUrlKind::Track => "track",
        SpotifyUrlKind::Album => "album",
        SpotifyUrlKind::Playlist => "playlist",
    };
    let full_url = format!("https://open.spotify.com/{kind_str}/{}", parsed.id);
    let html = fetch_page_html(&full_url).await?;

    let mut debug = String::new();
    debug.push_str(&format!("HTML length: {} chars\n\n", html.len()));

    // Check what we find
    if let Some(json) = extract_embedded_json(&html) {
        debug.push_str(&format!("Found embedded JSON:\n{}\n", serde_json::to_string_pretty(&json).unwrap_or_default()));
    } else {
        debug.push_str("No embedded JSON found.\n");
        // Show if duration_ms exists anywhere
        if html.contains("duration_ms") {
            let idx = html.find("duration_ms").unwrap();
            let start = if idx > 100 { idx - 100 } else { 0 };
            let end = std::cmp::min(idx + 200, html.len());
            debug.push_str(&format!("Found 'duration_ms' at pos {idx}:\n...{}...\n", &html[start..end]));
        } else {
            debug.push_str("'duration_ms' not found in HTML.\n");
        }
    }

    // Show OG tags
    let og_title = extract_content(&html, "property=\"og:title\" content=\"");
    let og_desc = extract_content(&html, "property=\"og:description\" content=\"");
    debug.push_str(&format!("\nog:title = {:?}\nog:description = {:?}\n", og_title, og_desc));

    Ok(debug)
}

fn extract_content(html: &str, search: &str) -> Option<String> {
    let idx = html.find(search)?;
    let start = idx + search.len();
    let end = html[start..].find('"')?;
    Some(decode_html_entities(&html[start..start + end]))
}

#[tauri::command]
pub async fn resolve_from_json(json_str: String) -> Result<ResolvedInput, String> {
    let json: Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Invalid JSON: {e}"))?;

    let id = json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");

    if let Some(track) = track_from_json(&json, id) {
        return Ok(ResolvedInput::Track(track));
    }

    Err("Could not parse track data from JSON".to_string())
}

pub async fn resolve_url_scraping(url: &str) -> Result<ResolvedInput, String> {
    let parsed = parse_spotify_url(url).ok_or("Invalid Spotify URL")?;
    let kind_str = match parsed.kind {
        SpotifyUrlKind::Track => "track",
        SpotifyUrlKind::Album => "album",
        SpotifyUrlKind::Playlist => "playlist",
    };
    let full_url = format!("https://open.spotify.com/{kind_str}/{}", parsed.id);

    let html = fetch_page_html(&full_url).await?;

    // Try extracting full track data from embedded JSON (best quality)
    if parsed.kind == SpotifyUrlKind::Track {
        if let Some(json) = extract_embedded_json(&html) {
            if let Some(track) = track_from_json(&json, &parsed.id) {
                return Ok(ResolvedInput::Track(track));
            }
            // JSON might be nested in __NEXT_DATA__; search deeper
            if let Some(props) = json.pointer("/props/pageProps") {
                if let Some(track) = track_from_json(props, &parsed.id) {
                    return Ok(ResolvedInput::Track(track));
                }
            }
        }
    }

    // Fallback to OG tag parsing
    let page = parse_og_tags(&html)?;

    let cover_url = if !page.image.is_empty() {
        Some(page.image.clone())
    } else {
        None
    };

    let (_artist_from_title, title) = parse_artist_title(&page.title);
    let (artists, album, year) = parse_description(&page.description);

    match parsed.kind {
        SpotifyUrlKind::Track => {
            Ok(ResolvedInput::Track(SpotifyTrack {
                id: parsed.id.clone(),
                name: title,
                artists: if artists.is_empty() { vec!["Unknown".to_string()] } else { artists },
                album,
                album_id: String::new(),
                track_number: 1,
                disc_number: 1,
                duration_ms: page.duration_ms,
                isrc: None,
                cover_url,
                release_date: year,
            }))
        }
        SpotifyUrlKind::Album | SpotifyUrlKind::Playlist => {
            let name = page.title.trim_end_matches(" | Spotify").to_string();
            if parsed.kind == SpotifyUrlKind::Album {
                Ok(ResolvedInput::Album(SpotifyAlbum {
                    id: parsed.id,
                    name,
                    artists: if artists.is_empty() { vec!["Unknown".to_string()] } else { artists },
                    tracks: vec![],
                    cover_url,
                    release_date: year.unwrap_or_default(),
                }))
            } else {
                Ok(ResolvedInput::Playlist(SpotifyPlaylist {
                    id: parsed.id,
                    name,
                    owner: artists.first().cloned().unwrap_or_default(),
                    tracks: vec![],
                    cover_url,
                }))
            }
        }
    }
}
