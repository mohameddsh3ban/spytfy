use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::manager::{get_bitrate, get_naming_template, get_output_root, get_write_cover_jpg, JobInfo, QueueManager};
use crate::download::{cover, downloader, scorer, tagger, verifier, youtube};

pub struct WorkerPool {
    pub shutdown: CancellationToken,
    handle: JoinHandle<()>,
}

impl WorkerPool {
    pub fn spawn(
        rx: mpsc::Receiver<(JobInfo, crate::spotify::types::SpotifyTrack)>,
        mgr: QueueManager,
        app: AppHandle,
        concurrency: usize,
    ) -> Self {
        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();

        let handle = tokio::spawn(async move {
            supervisor(rx, mgr, app, concurrency, shutdown_clone).await;
        });

        Self { shutdown, handle }
    }

    pub async fn shutdown(self) {
        self.shutdown.cancel();
        let _ = self.handle.await;
    }
}

async fn supervisor(
    mut rx: mpsc::Receiver<(JobInfo, crate::spotify::types::SpotifyTrack)>,
    mgr: QueueManager,
    app: AppHandle,
    concurrency: usize,
    shutdown: CancellationToken,
) {
    let semaphore = Arc::new(Semaphore::new(concurrency));

    loop {
        let item = tokio::select! {
            item = rx.recv() => item,
            _ = shutdown.cancelled() => break,
        };

        let Some((job, track)) = item else {
            break;
        };

        // Check if batch is still active before processing
        let batch_active: bool = sqlx::query_scalar::<_, String>(
            "SELECT state FROM batches WHERE id = ?"
        )
        .bind(&job.batch_id)
        .fetch_optional(&mgr.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s == "active")
        .unwrap_or(false);

        if !batch_active {
            // Batch paused or cancelled between send and receive — reset job to queued
            mgr.update_job_state(&job.id, "queued", None).await;
            continue;
        }

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => break,
        };

        // Stagger job dispatch to avoid YouTube rate limiting
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let mgr_c = mgr.clone();
        let app_c = app.clone();

        tokio::spawn(async move {
            let result = process_job(&mgr_c, &app_c, &job.id, &job.batch_id, &track).await;

            if let Err(err) = result {
                eprintln!("[SPYTFY] Job {} ({}) FAILED: {}", job.title, job.artist, err);
                mgr_c.update_job_state(&job.id, "failed", Some(&err)).await;
                let _ = app_c.emit("job:state", serde_json::json!({
                    "jobId": job.id, "batchId": job.batch_id, "state": "failed", "error": err,
                }));
            } else {
                let state: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id = ?")
                    .bind(&job.id)
                    .fetch_one(&mgr_c.pool)
                    .await
                    .unwrap_or_default();
                eprintln!("[SPYTFY] Job {} ({}) → {}", job.title, job.artist, state);
            }

            mgr_c.check_batch_complete(&app_c, &job.batch_id).await;
            mgr_c.push_one_queued_job().await;
            drop(permit);
        });
    }
}

fn resolve_sidecar(name: &str) -> Result<PathBuf, String> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!("{name}-x86_64-pc-windows-gnu.exe"));

    if manifest_path.exists() {
        return Ok(manifest_path);
    }

    let cwd_path = std::env::current_dir()
        .unwrap_or_default()
        .join("binaries")
        .join(format!("{name}-x86_64-pc-windows-gnu.exe"));

    if cwd_path.exists() {
        return Ok(cwd_path);
    }

    Err(format!("Sidecar {name} not found"))
}

async fn process_job(
    mgr: &QueueManager,
    app: &AppHandle,
    job_id: &str,
    batch_id: &str,
    track: &crate::spotify::types::SpotifyTrack,
) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    let yt_dlp = resolve_sidecar("yt-dlp")?;
    #[cfg(not(target_os = "android"))]
    let yt_dlp_str = yt_dlp.to_str().ok_or("Invalid yt-dlp path")?;

    let app_data_dir = crate::platform::data_dir(&app);
    let output_root = get_output_root(&mgr.pool).await;
    let bitrate = get_bitrate(&mgr.pool).await;
    let naming_template = get_naming_template(&mgr.pool).await;
    let write_cover_jpg = get_write_cover_jpg(&mgr.pool).await;

    let batch_name: String = sqlx::query_scalar("SELECT name FROM batches WHERE id = ?")
        .bind(batch_id)
        .fetch_optional(&mgr.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let emit_state = |state: &str| {
        let _ = app.emit("job:state", serde_json::json!({
            "jobId": job_id, "batchId": batch_id, "state": state,
        }));
    };

    let output_path = downloader::build_output_path(&PathBuf::from(&output_root), track, &batch_name, &naming_template);
    let artist = track.artists.first().cloned().unwrap_or_default();

    // Check if user pre-selected a YouTube URL (from pick_candidate)
    let pre_selected_url: Option<String> = sqlx::query_scalar::<_, Option<String>>("SELECT yt_url FROM jobs WHERE id = ?")
        .bind(job_id)
        .fetch_optional(&mgr.pool)
        .await
        .ok()
        .flatten()
        .flatten()
        .filter(|u| !u.is_empty() && u.starts_with("http"));

    let mut matched_yt_url = String::new();
    let mut download_err = String::new();
    let mut scored = Vec::new();

    if let Some(ref url) = pre_selected_url {
        emit_state("downloading");
        mgr.update_job_state(job_id, "downloading", None).await;
        mgr.update_job_output(job_id, url, &output_path.to_string_lossy()).await;

        #[cfg(not(target_os = "android"))]
        match downloader::download_mp3(app, yt_dlp_str, url, &output_path, bitrate, job_id, batch_id).await {
            Ok(()) => { matched_yt_url = url.to_string(); }
            Err(e) => { download_err = e; }
        }

        #[cfg(target_os = "android")]
        {
            let video_id = url.split("v=").last().unwrap_or("").split('&').next().unwrap_or("");
            match crate::download::android::download_audio_android(app, video_id, &output_path.to_string_lossy(), bitrate as u32).await {
                Ok(_) => { matched_yt_url = url.to_string(); }
                Err(e) => { download_err = e; }
            }
        }
    } else {
        emit_state("resolving");
        mgr.update_job_state(job_id, "resolving", None).await;

        #[cfg(not(target_os = "android"))]
        let candidates = youtube::search_youtube(yt_dlp_str, &artist, &track.name).await?;

        #[cfg(target_os = "android")]
        let candidates = crate::download::android::search_youtube_android(app, &artist, &track.name).await?;

        if candidates.is_empty() {
            return Err("No YouTube results found".to_string());
        }

        scored = scorer::score_all_candidates(&artist, &track.name, track.duration_ms, &candidates);

        emit_state("downloading");
        mgr.update_job_state(job_id, "downloading", None).await;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        for (i, matched) in scored.iter().take(3).enumerate() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            mgr.update_job_output(job_id, &matched.url, &output_path.to_string_lossy()).await;

            #[cfg(not(target_os = "android"))]
            let dl_result = downloader::download_mp3(app, yt_dlp_str, &matched.url, &output_path, bitrate, job_id, batch_id).await;

            #[cfg(target_os = "android")]
            let dl_result = crate::download::android::download_audio_android(app, &matched.candidate.id, &output_path.to_string_lossy(), bitrate as u32).await.map(|_| ());

            match dl_result {
                Ok(()) => {
                    matched_yt_url = matched.url.clone();
                    download_err.clear();
                    break;
                }
                Err(e) => {
                    eprintln!("[SPYTFY] Download failed for {}: {e}, trying next candidate", matched.url);
                    download_err = e;
                }
            }
        }
    }

    if !download_err.is_empty() && !output_path.exists() {
        // Store candidates for manual review
        let candidate_data: Vec<serde_json::Value> = scored.iter().take(5).map(|s| {
            serde_json::json!({
                "url": s.url,
                "title": s.candidate.title,
                "uploader": s.candidate.uploader,
                "durationSecs": s.candidate.duration_secs,
                "score": s.score,
            })
        }).collect();
        let candidates_json = serde_json::to_string(&candidate_data).unwrap_or_default();
        let _ = sqlx::query("UPDATE jobs SET candidates_json = ? WHERE id = ?")
            .bind(&candidates_json)
            .bind(job_id)
            .execute(&mgr.pool)
            .await;

        mgr.update_job_state(job_id, "needs_review", Some(&download_err)).await;
        let _ = app.emit("job:state", serde_json::json!({
            "jobId": job_id, "batchId": batch_id, "state": "needs_review", "error": download_err,
        }));
        return Ok(());
    }

    // Step 4: Resolve real cover art — priority: Spotify API > YouTube thumbnail > playlist cover
    emit_state("tagging");
    mgr.update_job_state(job_id, "tagging", None).await;

    let cache_dir = cover::cover_cache_dir(&app_data_dir);
    let cache_key = if track.album_id.is_empty() { &track.id } else { &track.album_id };

    // Try Spotify API first, then YouTube thumbnail
    let resolved_cover = resolve_real_cover(app, &artist, &track.name, track.cover_url.as_deref()).await
        .or_else(|| yt_thumbnail_url(&matched_yt_url));

    let cover_url_ref = resolved_cover.as_deref().or(track.cover_url.as_deref());

    if let Some(url) = &resolved_cover {
        mgr.update_job_cover(job_id, url).await;
        let _ = app.emit("job:cover", serde_json::json!({
            "jobId": job_id, "batchId": batch_id, "coverUrl": url,
        }));
        // Invalidate stale cache so the real cover gets fetched
        let stale = cache_dir.join(format!("{cache_key}.jpg"));
        let _ = std::fs::remove_file(&stale);
    }

    let cover_result = cover::fetch_cover(&cache_dir, cache_key, cover_url_ref).await;

    match &cover_result {
        Ok(cr) => {
            tagger::tag_mp3(&output_path, track, cr)?;

            if write_cover_jpg {
                if let Some(parent) = output_path.parent() {
                    let cover_path = parent.join("cover.jpg");
                    if !cover_path.exists() {
                        std::fs::write(&cover_path, &cr.bytes).ok();
                    }
                }
            }

            // Step 5: Verify
            emit_state("verifying");
            mgr.update_job_state(job_id, "verifying", None).await;

            match verifier::verify_mp3(&output_path, &cr.hash)? {
                verifier::VerifyResult::Ok => {
                    emit_state("done");
                    mgr.update_job_state(job_id, "done", None).await;
                }
                verifier::VerifyResult::Warning(msg) => {
                    eprintln!("[SPYTFY] Job {job_id} done with warning: {msg}");
                    let _ = app.emit("job:state", serde_json::json!({
                        "jobId": job_id, "batchId": batch_id, "state": "done_warning", "error": msg,
                    }));
                    mgr.update_job_state(job_id, "done_warning", Some(&msg)).await;
                }
            }
        }
        Err(_) => {
            tag_mp3_no_cover(&output_path, track)?;
            let msg = "no cover art available";
            let _ = app.emit("job:state", serde_json::json!({
                "jobId": job_id, "batchId": batch_id, "state": "done_warning", "error": msg,
            }));
            mgr.update_job_state(job_id, "done_warning", Some(msg)).await;
        }
    }

    #[cfg(target_os = "android")]
    {
        if output_path.exists() {
            let display_name = output_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "track.mp3".to_string());
            if let Err(e) = crate::download::android::register_in_media_store(
                app, &output_path.to_string_lossy(), &display_name, &batch_name,
            ) {
                eprintln!("[SPYTFY] MediaStore registration failed: {e}");
            }
        }
    }

    Ok(())
}

fn yt_thumbnail_url(yt_url: &str) -> Option<String> {
    let id = if let Some(pos) = yt_url.find("v=") {
        let rest = &yt_url[pos + 2..];
        rest.split('&').next().unwrap_or(rest)
    } else if let Some(pos) = yt_url.find("youtu.be/") {
        let rest = &yt_url[pos + 9..];
        rest.split('?').next().unwrap_or(rest)
    } else {
        return None;
    };
    if id.is_empty() {
        return None;
    }
    Some(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg"))
}

async fn resolve_real_cover(app: &AppHandle, artist: &str, title: &str, current_url: Option<&str>) -> Option<String> {
    use rspotify::model::SearchType;
    use rspotify::prelude::*;

    let client = app.try_state::<crate::spotify::auth::SpotifyClient>()?;
    let guard = client.read().await;
    let spotify = guard.as_ref()?;

    if spotify.request_token().await.is_err() {
        return None;
    }

    let query = format!("track:{} artist:{}", title, artist);
    let result = spotify
        .search(&query, SearchType::Track, None, None, Some(1), None)
        .await
        .ok()?;

    if let rspotify::model::SearchResult::Tracks(page) = result {
        let found = page.items.first()?;
        let cover = found.album.images.iter()
            .max_by_key(|img| img.width.unwrap_or(0))
            .map(|img| img.url.clone())?;

        if current_url == Some(cover.as_str()) {
            return None;
        }
        Some(cover)
    } else {
        None
    }
}

fn tag_mp3_no_cover(mp3_path: &std::path::Path, track: &crate::spotify::types::SpotifyTrack) -> Result<(), String> {
    use id3::{Tag, TagLike, Version};
    let mut tag = Tag::new();
    tag.set_title(&track.name);
    tag.set_artist(track.artists.join(", "));
    tag.set_album(&track.album);
    tag.set_track(track.track_number as u32);
    if let Some(ref date) = track.release_date {
        if let Some(year) = date.split('-').next().and_then(|y| y.parse::<i32>().ok()) {
            tag.set_year(year);
        }
    }
    tag.write_to_path(mp3_path, Version::Id3v24)
        .map_err(|e| format!("Failed to write tags: {e}"))?;
    Ok(())
}
