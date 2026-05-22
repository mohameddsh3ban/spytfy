use tauri::Emitter;
use regex::Regex;

use super::parser::ParsedTrack;

fn emit_log(app: &tauri::AppHandle, msg: &str, pct: u32) {
    let _ = app.emit("scrape:log", serde_json::json!({ "message": msg, "percent": pct }));
}

pub async fn scrape_playlist(url: &str, app: &tauri::AppHandle) -> Result<Vec<ParsedTrack>, String> {
    let parsed = crate::spotify::parser::parse_spotify_url(url)
        .ok_or("Invalid Spotify URL")?;
    let id = &parsed.id;
    let kind = match parsed.kind {
        crate::spotify::types::SpotifyUrlKind::Playlist => "playlist",
        crate::spotify::types::SpotifyUrlKind::Album => "album",
        _ => return Err("Only playlists and albums supported for auto-load".to_string()),
    };

    let embed_url = format!("https://open.spotify.com/embed/{kind}/{id}");
    emit_log(app, &format!("Fetching {embed_url}"), 10);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .get(&embed_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    emit_log(app, &format!("Response: {status}"), 30);

    if !status.is_success() {
        return Err(format!("Spotify returned {status}"));
    }

    let html = resp.text().await.map_err(|e| format!("Read failed: {e}"))?;
    emit_log(app, &format!("Got {} chars", html.len()), 40);

    // Strategy 1: Parse __NEXT_DATA__ JSON
    let mut tracks = parse_next_data(&html);
    if !tracks.is_empty() {
        emit_log(app, &format!("__NEXT_DATA__: found {} tracks", tracks.len()), 100);
        return Ok(tracks);
    }

    // Strategy 2: Parse track data from any embedded JSON
    tracks = parse_embedded_json(&html);
    if !tracks.is_empty() {
        emit_log(app, &format!("Embedded JSON: found {} tracks", tracks.len()), 100);
        return Ok(tracks);
    }

    // Strategy 3: Parse from HTML text patterns (track titles + artists + durations)
    emit_log(app, "Trying HTML text pattern matching...", 60);
    tracks = parse_html_text_patterns(&html);
    if !tracks.is_empty() {
        emit_log(app, &format!("Text patterns: found {} tracks", tracks.len()), 100);
        return Ok(tracks);
    }

    // Strategy 4: Parse from any link patterns
    emit_log(app, "Trying link extraction...", 80);
    tracks = parse_track_links(&html);
    if !tracks.is_empty() {
        emit_log(app, &format!("Links: found {} tracks", tracks.len()), 100);
        return Ok(tracks);
    }

    // Debug: show what strategies found
    let has_next_data = html.contains("__NEXT_DATA__");
    let has_track_links = html.contains("/track/");
    let has_artist_links = html.contains("/artist/");
    let has_durations = regex::Regex::new(r"\d{1,2}:\d{2}").unwrap().is_match(&html);

    let preview = if html.len() > 300 { &html[..300] } else { &html };
    emit_log(app, &format!(
        "DEBUG: __NEXT_DATA__={}, /track/={}, /artist/={}, durations={}, html_start: {}",
        has_next_data, has_track_links, has_artist_links, has_durations,
        preview.replace('\n', " ").replace('\r', "")
    ), 95);

    Err("Could not extract tracks. Use manual import.".to_string())
}

fn parse_next_data(html: &str) -> Vec<ParsedTrack> {
    let mut tracks = Vec::new();
    if let Some(idx) = html.find("__NEXT_DATA__") {
        if let Some(start) = html[idx..].find('>') {
            let json_start = idx + start + 1;
            if let Some(end) = html[json_start..].find("</script>") {
                let json_str = &html[json_start..json_start + end];
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    find_tracks_recursive(&val, &mut tracks);
                }
            }
        }
    }
    tracks
}

fn parse_embedded_json(html: &str) -> Vec<ParsedTrack> {
    let mut tracks = Vec::new();
    for chunk in html.split("<script") {
        if let Some(start) = chunk.find('>') {
            let content = &chunk[start + 1..];
            if let Some(end) = content.find("</script>") {
                let script = &content[..end].trim();
                if script.starts_with('{') || script.starts_with('[') {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(script) {
                        find_tracks_recursive(&val, &mut tracks);
                    }
                }
            }
        }
    }
    tracks
}

fn parse_html_text_patterns(html: &str) -> Vec<ParsedTrack> {
    let mut tracks = Vec::new();

    // Pattern: track titles in anchor tags pointing to /track/
    let track_re = Regex::new(r#"href="[^"]*?/track/([^"]+)"[^>]*>([^<]+)</a>"#).unwrap();
    let artist_re = Regex::new(r#"href="[^"]*?/artist/[^"]+?"[^>]*>([^<]+)</a>"#).unwrap();

    // Collect all track links
    let track_matches: Vec<(String, String)> = track_re
        .captures_iter(html)
        .map(|c| (c[1].to_string(), decode_entities(&c[2])))
        .collect();

    // Collect all artist links
    let artist_matches: Vec<String> = artist_re
        .captures_iter(html)
        .map(|c| decode_entities(&c[1]))
        .collect();

    // Duration pattern: M:SS or MM:SS
    let dur_re = Regex::new(r#"(\d{1,2}):(\d{2})"#).unwrap();
    let durations: Vec<u64> = dur_re
        .captures_iter(html)
        .filter_map(|c| {
            let m: u64 = c[1].parse().ok()?;
            let s: u64 = c[2].parse().ok()?;
            if m < 20 && s < 60 { Some((m * 60 + s) * 1000) } else { None }
        })
        .collect();

    if track_matches.is_empty() {
        return tracks;
    }

    // Try to pair tracks with artists (heuristic: artists follow tracks in HTML order)
    let mut artist_idx = 0;
    for (i, (_id, title)) in track_matches.iter().enumerate() {
        let artist = if artist_idx < artist_matches.len() {
            let a = artist_matches[artist_idx].clone();
            artist_idx += 1;
            // Check if next artist is also for this track (multiple artists)
            // Heuristic: skip if same artist appears consecutively
            a
        } else {
            String::new()
        };

        let duration_ms = if i < durations.len() { durations[i] } else { 0 };

        tracks.push(ParsedTrack {
            track_number: (i + 1) as u16,
            title: title.clone(),
            artist,
            album: String::new(),
            duration_ms,
            cover_url: None,
        });
    }

    tracks
}

fn parse_track_links(html: &str) -> Vec<ParsedTrack> {
    // Fallback: just find anything that looks like track data
    let re = Regex::new(r#"/track/([a-zA-Z0-9]+)"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut tracks = Vec::new();

    for caps in re.captures_iter(html) {
        let id = caps[1].to_string();
        if seen.insert(id.clone()) {
            tracks.push(ParsedTrack {
                track_number: (tracks.len() + 1) as u16,
                title: format!("Track {}", id),
                artist: String::new(),
                album: String::new(),
                duration_ms: 0,
                cover_url: None,
            });
        }
    }

    tracks
}

fn find_tracks_recursive(val: &serde_json::Value, tracks: &mut Vec<ParsedTrack>) {
    match val {
        serde_json::Value::Object(map) => {
            // Spotify embed format: { title, subtitle, duration, entityType: "track" }
            let is_track = map.get("entityType").and_then(|t| t.as_str()) == Some("track")
                || map.get("type").and_then(|t| t.as_str()) == Some("track");

            if is_track {
                let cover_url = extract_cover_url(map);

                if let Some(title) = map.get("title").and_then(|t| t.as_str()) {
                    let artist = map.get("subtitle")
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    let dur = map.get("duration")
                        .and_then(|d| d.as_u64())
                        .or_else(|| map.get("duration_ms").and_then(|d| d.as_u64()))
                        .unwrap_or(0);

                    if !title.is_empty() {
                        tracks.push(ParsedTrack {
                            track_number: (tracks.len() + 1) as u16,
                            title: title.to_string(),
                            artist: artist.to_string(),
                            album: String::new(),
                            duration_ms: dur,
                            cover_url,
                        });
                        return;
                    }
                }

                // Also try name + artists[] format (API style)
                if let (Some(name), Some(artists)) = (
                    map.get("name").and_then(|n| n.as_str()),
                    map.get("artists"),
                ) {
                    if let Some(arr) = artists.as_array() {
                        let names: Vec<String> = arr.iter()
                            .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(String::from))
                            .collect();
                        if !names.is_empty() {
                            let dur = map.get("duration_ms").and_then(|d| d.as_u64()).unwrap_or(0);
                            let album = map.get("album")
                                .and_then(|a| a.get("name"))
                                .and_then(|n| n.as_str())
                                .unwrap_or("").to_string();
                            tracks.push(ParsedTrack {
                                track_number: (tracks.len() + 1) as u16,
                                title: name.to_string(),
                                artist: names.join(", "),
                                album,
                                duration_ms: dur,
                                cover_url,
                            });
                            return;
                        }
                    }
                }
            }

            for v in map.values() { find_tracks_recursive(v, tracks); }
        }
        serde_json::Value::Array(arr) => {
            for v in arr { find_tracks_recursive(v, tracks); }
        }
        _ => {}
    }
}

fn extract_cover_url(map: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    // Try album.images[0].url (API format)
    if let Some(album) = map.get("album") {
        if let Some(images) = album.get("images").and_then(|i| i.as_array()) {
            if let Some(url) = images.iter()
                .filter_map(|img| {
                    let w = img.get("width").and_then(|w| w.as_u64()).unwrap_or(0);
                    let url = img.get("url").and_then(|u| u.as_str())?;
                    Some((w, url))
                })
                .max_by_key(|(w, _)| *w)
                .map(|(_, url)| url.to_string())
            {
                return Some(url);
            }
        }
    }
    // Try coverArt.sources[0].url (embed format)
    if let Some(cover_art) = map.get("coverArt") {
        if let Some(sources) = cover_art.get("sources").and_then(|s| s.as_array()) {
            if let Some(url) = sources.iter()
                .filter_map(|s| {
                    let w = s.get("width").and_then(|w| w.as_u64()).unwrap_or(0);
                    let url = s.get("url").and_then(|u| u.as_str())?;
                    Some((w, url))
                })
                .max_by_key(|(w, _)| *w)
                .map(|(_, url)| url.to_string())
            {
                return Some(url);
            }
        }
    }
    // Try image/imageUrl directly
    map.get("imageUrl").or(map.get("image"))
        .and_then(|v| v.as_str())
        .filter(|s| s.starts_with("http"))
        .map(String::from)
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
