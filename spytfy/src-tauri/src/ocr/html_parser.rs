use regex::Regex;
use super::parser::ParsedTrack;

pub fn parse_spotify_html(html: &str) -> Vec<ParsedTrack> {
    let mut tracks: Vec<ParsedTrack> = Vec::new();

    // Pattern 1: aria-label="Play {title} by {artist}" on play buttons
    let play_re = Regex::new(r#"aria-label="Play ([^"]+?) by ([^"]+?)""#).unwrap();

    // Pattern 2: track link href="/track/{id}"
    let track_link_re = Regex::new(r#"href="/track/([a-zA-Z0-9]+)""#).unwrap();

    // Pattern 3: Duration text (M:SS format) in the last column
    let duration_re = Regex::new(r#"Na06TEk5cCR4FwBd[^>]*>(\d{1,2}:\d{2})<"#).unwrap();

    // Pattern 4: Album link in gridcell 3
    let album_re = Regex::new(r#"href="/album/[^"]+?"[^>]*?>([^<]+?)</a>"#).unwrap();

    // Pattern 5: Cover art
    let cover_re = Regex::new(r#"src="(https://i\.scdn\.co/image/[^"]+)""#).unwrap();

    // Split by track rows
    let row_re = Regex::new(r#"role="row""#).unwrap();
    let row_positions: Vec<usize> = row_re.find_iter(html).map(|m| m.start()).collect();

    for (i, &start) in row_positions.iter().enumerate() {
        let end = if i + 1 < row_positions.len() {
            row_positions[i + 1]
        } else {
            html.len()
        };
        let row_html = &html[start..end];

        // Extract title + artist from play button aria-label
        let Some(play_caps) = play_re.captures(row_html) else {
            continue;
        };
        let title = decode_html_entities(&play_caps[1]);
        let artist = decode_html_entities(&play_caps[2]);

        // Extract duration
        let duration_ms = duration_re
            .captures(row_html)
            .map(|caps| {
                let parts: Vec<&str> = caps[1].split(':').collect();
                if parts.len() == 2 {
                    let min: u64 = parts[0].parse().unwrap_or(0);
                    let sec: u64 = parts[1].parse().unwrap_or(0);
                    (min * 60 + sec) * 1000
                } else {
                    0
                }
            })
            .unwrap_or(0);

        // Extract album name (first album link in the row after gridcell 3)
        let album = album_re
            .captures_iter(row_html)
            .last()
            .map(|caps| decode_html_entities(&caps[1]))
            .unwrap_or_default();

        tracks.push(ParsedTrack {
            track_number: (tracks.len() + 1) as u16,
            title,
            artist,
            album,
            duration_ms,
            cover_url: None,
        });
    }

    tracks
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spotify_playlist_html() {
        let html = r#"
        <div role="row" aria-rowindex="2">
            <button aria-label="Play 3azra2eel 2 by Abyusif"></button>
            <a href="/track/5Q7Qv0qkOEUv8yAr9BRVg4">
                <div>3azra2eel 2</div>
            </a>
            <a href="/album/xxx">3azra2eel 2</a>
            <div class="Na06TEk5cCR4FwBd">3:25</div>
        </div>
        <div role="row" aria-rowindex="3">
            <button aria-label="Play Stressed Out by Twenty One Pilots"></button>
            <a href="/track/3CRDbSIZ4r">
                <div>Stressed Out</div>
            </a>
            <a href="/album/yyy">Blurryface</a>
            <div class="Na06TEk5cCR4FwBd">3:22</div>
        </div>
        "#;

        let tracks = parse_spotify_html(html);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "3azra2eel 2");
        assert_eq!(tracks[0].artist, "Abyusif");
        assert_eq!(tracks[0].duration_ms, 205000);
        assert_eq!(tracks[1].title, "Stressed Out");
        assert_eq!(tracks[1].artist, "Twenty One Pilots");
    }
}
