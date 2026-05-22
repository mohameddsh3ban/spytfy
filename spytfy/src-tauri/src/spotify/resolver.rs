use futures::StreamExt;
use rspotify::model::{AlbumId, FullTrack, PlaylistId, SimplifiedTrack, TrackId};
use rspotify::prelude::*;

use super::auth::SpotifyClient;
use super::parser::parse_spotify_url;
use super::types::*;

fn full_track_to_domain(t: &FullTrack) -> SpotifyTrack {
    let cover_url = t
        .album
        .images
        .iter()
        .max_by_key(|img| img.width.unwrap_or(0))
        .map(|img| img.url.clone());

    SpotifyTrack {
        id: t.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
        name: t.name.clone(),
        artists: t.artists.iter().map(|a| a.name.clone()).collect(),
        album: t.album.name.clone(),
        album_id: t
            .album
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default(),
        track_number: t.track_number as u16,
        disc_number: t.disc_number as u16,
        duration_ms: t.duration.num_milliseconds() as u64,
        isrc: t.external_ids.get("isrc").cloned(),
        cover_url,
        release_date: Some(t.album.release_date.clone().unwrap_or_default()),
    }
}

fn simplified_track_to_domain(
    t: &SimplifiedTrack,
    album_name: &str,
    album_id: &str,
    cover_url: &Option<String>,
    release_date: &str,
) -> SpotifyTrack {
    SpotifyTrack {
        id: t.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
        name: t.name.clone(),
        artists: t.artists.iter().map(|a| a.name.clone()).collect(),
        album: album_name.to_string(),
        album_id: album_id.to_string(),
        track_number: t.track_number as u16,
        disc_number: t.disc_number as u16,
        duration_ms: t.duration.num_milliseconds() as u64,
        isrc: None,
        cover_url: cover_url.clone(),
        release_date: Some(release_date.to_string()),
    }
}

async fn resolve_track(
    spotify: &rspotify::ClientCredsSpotify,
    id: &str,
) -> Result<ResolvedInput, String> {
    let track_id = TrackId::from_id(id).map_err(|e| format!("Invalid track ID: {e}"))?;

    // Get the access token for direct debugging
    let token = spotify.token.lock().await.expect("token lock poisoned");
    let access_token = token
        .as_ref()
        .map(|t| t.access_token.clone())
        .ok_or("No access token available")?;
    drop(token);

    // Direct API call to see the full error
    let url = format!("https://api.spotify.com/v1/tracks/{id}");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Spotify API error {status}: {body}"));
    }

    // If direct call works, use rspotify for proper deserialization
    let track = spotify
        .track(track_id, None)
        .await
        .map_err(|e| format!("Failed to fetch track: {e}"))?;
    Ok(ResolvedInput::Track(full_track_to_domain(&track)))
}

async fn resolve_album(
    spotify: &rspotify::ClientCredsSpotify,
    id: &str,
) -> Result<ResolvedInput, String> {
    let album_id = AlbumId::from_id(id).map_err(|e| format!("Invalid album ID: {e}"))?;
    let album = spotify
        .album(album_id.clone(), None)
        .await
        .map_err(|e| format!("Failed to fetch album: {e}"))?;

    let cover_url = album
        .images
        .iter()
        .max_by_key(|img| img.width.unwrap_or(0))
        .map(|img| img.url.clone());

    let release_date = album.release_date.clone();
    let album_id_str = album_id.to_string();

    // Collect all tracks via stream (handles pagination automatically)
    let mut tracks: Vec<SpotifyTrack> = Vec::new();
    let mut stream = spotify.album_track(album_id, None);
    while let Some(item) = stream.next().await {
        match item {
            Ok(t) => tracks.push(simplified_track_to_domain(
                &t,
                &album.name,
                &album_id_str,
                &cover_url,
                &release_date,
            )),
            Err(e) => return Err(format!("Failed to fetch album track: {e}")),
        }
    }

    Ok(ResolvedInput::Album(SpotifyAlbum {
        id: album_id_str,
        name: album.name,
        artists: album.artists.iter().map(|a| a.name.clone()).collect(),
        tracks,
        cover_url,
        release_date,
    }))
}

async fn resolve_playlist(
    spotify: &rspotify::ClientCredsSpotify,
    id: &str,
) -> Result<ResolvedInput, String> {
    let playlist_id =
        PlaylistId::from_id(id).map_err(|e| format!("Invalid playlist ID: {e}"))?;

    let playlist = spotify
        .playlist(playlist_id.clone(), None, None)
        .await
        .map_err(|e| format!("Failed to fetch playlist: {e}"))?;

    let cover_url = playlist
        .images
        .iter()
        .max_by_key(|img| img.width.unwrap_or(0))
        .map(|img| img.url.clone());

    // Collect all tracks via stream (handles pagination automatically)
    let mut tracks: Vec<SpotifyTrack> = Vec::new();
    let mut stream = spotify.playlist_items(playlist_id, None, None);
    while let Some(item) = stream.next().await {
        match item {
            Ok(playlist_item) => {
                if let Some(rspotify::model::PlayableItem::Track(t)) = playlist_item.track {
                    tracks.push(full_track_to_domain(&t));
                }
            }
            Err(e) => return Err(format!("Failed to fetch playlist track: {e}")),
        }
    }

    Ok(ResolvedInput::Playlist(SpotifyPlaylist {
        id: id.to_string(),
        name: playlist.name,
        owner: playlist
            .owner
            .display_name
            .unwrap_or_else(|| playlist.owner.id.to_string()),
        tracks,
        cover_url,
    }))
}

#[tauri::command]
pub async fn resolve_url(
    client: tauri::State<'_, SpotifyClient>,
    url: String,
) -> Result<ResolvedInput, String> {
    let parsed = parse_spotify_url(&url).ok_or("Invalid Spotify URL")?;

    // Try API first if credentials exist
    let guard = client.read().await;
    if let Some(spotify) = guard.as_ref() {
        if spotify.request_token().await.is_ok() {
            let api_result = match parsed.kind {
                SpotifyUrlKind::Track => resolve_track(spotify, &parsed.id).await,
                SpotifyUrlKind::Album => resolve_album(spotify, &parsed.id).await,
                SpotifyUrlKind::Playlist => resolve_playlist(spotify, &parsed.id).await,
            };
            if api_result.is_ok() {
                return api_result;
            }
        }
    }
    drop(guard);

    // Fallback: scrape public Spotify page (no auth needed)
    super::scraper::resolve_url_scraping(&url).await
}
