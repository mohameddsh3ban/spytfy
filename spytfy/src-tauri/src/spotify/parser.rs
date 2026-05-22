use regex::Regex;
use std::sync::LazyLock;

use super::types::{SpotifyUrl, SpotifyUrlKind};

static SPOTIFY_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?(?:open\.)?spotify\.com/(track|album|playlist)/([a-zA-Z0-9]+)").unwrap()
});

static SPOTIFY_URI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^spotify:(track|album|playlist):([a-zA-Z0-9]+)$").unwrap()
});

pub fn parse_spotify_url(input: &str) -> Option<SpotifyUrl> {
    let input = input.trim();

    if let Some(caps) = SPOTIFY_URL_RE.captures(input) {
        return Some(SpotifyUrl {
            kind: match &caps[1] {
                "track" => SpotifyUrlKind::Track,
                "album" => SpotifyUrlKind::Album,
                "playlist" => SpotifyUrlKind::Playlist,
                _ => return None,
            },
            id: caps[2].to_string(),
        });
    }

    if let Some(caps) = SPOTIFY_URI_RE.captures(input) {
        return Some(SpotifyUrl {
            kind: match &caps[1] {
                "track" => SpotifyUrlKind::Track,
                "album" => SpotifyUrlKind::Album,
                "playlist" => SpotifyUrlKind::Playlist,
                _ => return None,
            },
            id: caps[2].to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_track_url() {
        let url = "https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT?si=abc123";
        let result = parse_spotify_url(url).unwrap();
        assert_eq!(result.kind, SpotifyUrlKind::Track);
        assert_eq!(result.id, "4cOdK2wGLETKBW3PvgPWqT");
    }

    #[test]
    fn parse_album_url() {
        let url = "https://open.spotify.com/album/2noRn2Aes5aoNVsU6iWThc";
        let result = parse_spotify_url(url).unwrap();
        assert_eq!(result.kind, SpotifyUrlKind::Album);
        assert_eq!(result.id, "2noRn2Aes5aoNVsU6iWThc");
    }

    #[test]
    fn parse_playlist_url() {
        let url = "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M";
        let result = parse_spotify_url(url).unwrap();
        assert_eq!(result.kind, SpotifyUrlKind::Playlist);
        assert_eq!(result.id, "37i9dQZF1DXcBWIGoYBM5M");
    }

    #[test]
    fn parse_spotify_uri() {
        let uri = "spotify:track:4cOdK2wGLETKBW3PvgPWqT";
        let result = parse_spotify_url(uri).unwrap();
        assert_eq!(result.kind, SpotifyUrlKind::Track);
        assert_eq!(result.id, "4cOdK2wGLETKBW3PvgPWqT");
    }

    #[test]
    fn parse_invalid_url() {
        assert!(parse_spotify_url("https://google.com").is_none());
        assert!(parse_spotify_url("not a url").is_none());
        assert!(parse_spotify_url("").is_none());
    }
}
