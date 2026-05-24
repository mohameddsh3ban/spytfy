use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::settings::Settings;
use crate::spotify::types::SpotifyTrack;

use super::cover;
use super::downloader;
use super::scorer;
use super::tagger;
use super::verifier;
use super::youtube;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub output_path: String,
    pub yt_url: String,
    pub yt_score: i32,
}

fn emit_state(app: &AppHandle, stage: &str) {
    let _ = app.emit(
        "download:state",
        serde_json::json!({ "stage": stage }),
    );
}

fn resolve_sidecar_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    // In dev mode, sidecars are next to Cargo.toml in src-tauri/binaries/
    // In production, they're in the resource directory
    let dev_path = std::env::current_dir()
        .unwrap_or_default()
        .join("binaries")
        .join(format!("{name}-x86_64-pc-windows-gnu.exe"));

    if dev_path.exists() {
        return Ok(dev_path);
    }

    // Try from the manifest dir (src-tauri/)
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!("{name}-x86_64-pc-windows-gnu.exe"));

    if manifest_path.exists() {
        return Ok(manifest_path);
    }

    // Try Tauri resource path for production builds
    app.path()
        .resolve(format!("binaries/{name}-x86_64-pc-windows-gnu.exe"), tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Sidecar {name} not found at {dev_path:?} or resource dir: {e}"))
}

#[tauri::command]
pub async fn download_track(
    app: AppHandle,
    track: SpotifyTrack,
) -> Result<DownloadResult, String> {
    let app_data_dir = crate::platform::data_dir(&app);
    let settings = {
        let pool = app.state::<sqlx::SqlitePool>();
        load_settings_for_pipeline(&pool).await
    };
    let output_root = PathBuf::from(&settings.output_root);
    let bitrate = settings.bitrate_kbps;
    let artist = track.artists.first().cloned().unwrap_or_default();

    // Step 1: Search YouTube (platform-gated)
    emit_state(&app, "searching");

    #[cfg(not(target_os = "android"))]
    let candidates = {
        let yt_dlp_path = resolve_sidecar_path(&app, "yt-dlp")?;
        let yt_dlp = yt_dlp_path.to_str().ok_or("Invalid yt-dlp path")?;
        youtube::search_youtube(yt_dlp, &artist, &track.name).await?
    };

    #[cfg(target_os = "android")]
    let candidates = super::android::search_youtube_android(&app, &artist, &track.name).await?;

    if candidates.is_empty() {
        return Err("No YouTube results found".to_string());
    }

    // Step 2: Score — pick best or fallback to first
    emit_state(&app, "matching");
    let matched = scorer::score_candidates(&artist, &track.name, track.duration_ms, &candidates)
        .unwrap_or_else(|| {
            let c = candidates[0].clone();
            scorer::ScoredMatch {
                url: format!("https://www.youtube.com/watch?v={}", c.id),
                score: 0,
                candidate: c,
            }
        });

    // Step 3: Download MP3 (platform-gated)
    emit_state(&app, "downloading");
    let output_path = downloader::build_output_path(&output_root, &track, &track.album, "{folder}/{number} - {artist} - {title}");

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    #[cfg(not(target_os = "android"))]
    {
        let yt_dlp_path = resolve_sidecar_path(&app, "yt-dlp")?;
        let yt_dlp = yt_dlp_path.to_str().ok_or("Invalid yt-dlp path")?;
        downloader::download_mp3(&app, yt_dlp, &matched.url, &output_path, bitrate, "", "").await?;
    }

    #[cfg(target_os = "android")]
    {
        let output_str = output_path.to_string_lossy().to_string();
        super::android::download_audio_android(&app, &matched.candidate.id, &output_str, bitrate as u32).await?;
    }

    // Step 4: Fetch cover art + tag (shared across platforms)
    emit_state(&app, "tagging");
    let cache_dir = cover::cover_cache_dir(&app_data_dir);
    let cache_key = if track.album_id.is_empty() { &track.id } else { &track.album_id };
    let cover_result = cover::fetch_cover(&cache_dir, cache_key, track.cover_url.as_deref()).await;

    match &cover_result {
        Ok(cr) => {
            tagger::tag_mp3(&output_path, &track, cr)?;
            emit_state(&app, "verifying");
            verifier::verify_mp3(&output_path, &cr.hash)?;
        }
        Err(_) => {
            use id3::{Tag, TagLike, Version};
            let mut tag = Tag::new();
            tag.set_title(&track.name);
            tag.set_artist(track.artists.join(", "));
            tag.set_album(&track.album);
            tag.set_track(track.track_number as u32);
            tag.write_to_path(&output_path, Version::Id3v24)
                .map_err(|e| format!("Tag write failed: {e}"))?;
        }
    }

    if settings.write_cover_jpg {
        if let Ok(cr) = &cover_result {
            if let Some(parent) = output_path.parent() {
                let cover_path = parent.join("cover.jpg");
                if !cover_path.exists() {
                    std::fs::write(&cover_path, &cr.bytes).ok();
                }
            }
        }
    }

    emit_state(&app, "done");

    Ok(DownloadResult {
        output_path: output_path.to_string_lossy().to_string(),
        yt_url: matched.url,
        yt_score: matched.score,
    })
}

async fn load_settings_for_pipeline(pool: &sqlx::SqlitePool) -> Settings {
    Settings {
        output_root: get_setting(pool, "output_root")
            .await
            .unwrap_or_else(|| {
                #[cfg(not(target_os = "android"))]
                {
                    dirs::audio_dir()
                        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
                        .join("Spytfy")
                        .to_string_lossy()
                        .to_string()
                }
                #[cfg(target_os = "android")]
                {
                    "/data/data/com.spytfy.app/files/Music/Spytfy".to_string()
                }
            }),
        concurrency: get_setting(pool, "concurrency")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        bitrate_kbps: get_setting(pool, "bitrate_kbps")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(320),
        overwrite_existing: get_setting(pool, "overwrite_existing")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        write_cover_jpg: get_setting(pool, "write_cover_jpg")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(true),
        naming_template: get_setting(pool, "naming_template")
            .await
            .unwrap_or_else(|| "{folder}/{number} - {artist} - {title}".to_string()),
    }
}

async fn get_setting(pool: &sqlx::SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}
