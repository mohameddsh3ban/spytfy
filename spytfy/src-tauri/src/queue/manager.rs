use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::download::downloader;
use crate::spotify::types::{ResolvedInput, SpotifyTrack};

#[derive(Clone)]
pub struct QueueManager {
    pub pool: SqlitePool,
    pub tx: mpsc::Sender<(JobInfo, SpotifyTrack)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BatchInfo {
    pub id: String,
    pub source_url: String,
    pub source_type: String,
    pub name: String,
    pub total_tracks: i64,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: String,
    pub batch_id: String,
    pub spotify_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    pub state: String,
    pub yt_url: Option<String>,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub progress_pct: Option<f64>,
    pub cover_url: Option<String>,
    pub candidates_json: Option<String>,
}

impl QueueManager {
    pub fn new(pool: SqlitePool, tx: mpsc::Sender<(JobInfo, SpotifyTrack)>) -> Self {
        Self { pool, tx }
    }

    pub async fn enqueue_batch(
        &self,
        app: &AppHandle,
        input: ResolvedInput,
        source_url: String,
    ) -> Result<String, String> {
        let (source_type, name, tracks) = match &input {
            ResolvedInput::Track(t) => ("track", t.name.clone(), vec![t.clone()]),
            ResolvedInput::Album(a) => ("album", a.name.clone(), a.tracks.clone()),
            ResolvedInput::Playlist(p) => ("playlist", p.name.clone(), p.tracks.clone()),
        };

        let normalized_url = normalize_spotify_url(&source_url);
        let output_root = get_output_root(&self.pool).await;
        let output_root_path = PathBuf::from(&output_root);
        let naming_template = get_naming_template(&self.pool).await;

        // Check for existing batch with same source_url (normalized)
        let existing: Option<BatchInfo> = sqlx::query_as(
            "SELECT id, source_url, source_type, name, total_tracks, state, created_at FROM batches WHERE source_url = ? AND state != 'cancelled' LIMIT 1"
        )
        .bind(&normalized_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to check existing batch: {e}"))?;

        // Fallback: match by name + source_type if URL didn't match
        let existing = if existing.is_some() {
            existing
        } else {
            sqlx::query_as(
                "SELECT id, source_url, source_type, name, total_tracks, state, created_at FROM batches WHERE name = ? AND source_type = ? AND state != 'cancelled' LIMIT 1"
            )
            .bind(&name)
            .bind(source_type)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
        };

        let batch_id = if let Some(batch) = existing {
            // Merge into existing batch
            let existing_keys: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT spotify_id, title, artist FROM jobs WHERE batch_id = ?"
            )
            .bind(&batch.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to fetch existing jobs: {e}"))?;

            let mut new_queued = 0u32;
            for track in &tracks {
                let artist_str = track.artists.join(", ");
                let already_exists = existing_keys.iter().any(|(sid, t, a)| {
                    sid == &track.id || (t == &track.name && a == &artist_str)
                });
                if already_exists {
                    continue;
                }
                let expected_path = downloader::build_output_path(&output_root_path, track, &name, &naming_template);
                let file_exists = expected_path.exists()
                    && expected_path.metadata().map(|m| m.len() > 1000).unwrap_or(false);

                let job_id = Uuid::new_v4().to_string();
                let state = if file_exists { "done" } else { "queued" };

                sqlx::query(
                    "INSERT INTO jobs (id, batch_id, spotify_id, title, artist, album, duration_ms, state, cover_url, track_number, output_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&job_id)
                .bind(&batch.id)
                .bind(&track.id)
                .bind(&track.name)
                .bind(track.artists.join(", "))
                .bind(&track.album)
                .bind(track.duration_ms as i64)
                .bind(state)
                .bind(&track.cover_url)
                .bind(track.track_number as i64)
                .bind(if file_exists { Some(expected_path.to_string_lossy().to_string()) } else { None })
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to insert job: {e}"))?;

                if !file_exists {
                    new_queued += 1;
                }
            }

            // Update total_tracks
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE batch_id = ?")
                .bind(&batch.id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

            sqlx::query("UPDATE batches SET total_tracks = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(total)
                .bind(&batch.id)
                .execute(&self.pool)
                .await
                .ok();

            // Reactivate batch if it was complete or paused
            if batch.state == "complete" || batch.state == "paused" {
                sqlx::query("UPDATE batches SET state = 'active', updated_at = datetime('now') WHERE id = ?")
                    .bind(&batch.id)
                    .execute(&self.pool)
                    .await
                    .ok();
            }

            if new_queued > 0 {
                self.push_queued_jobs(&batch.id).await;
            }

            batch.id
        } else {
            // Create new batch
            let batch_id = Uuid::new_v4().to_string();
            let mut queued_count = 0u32;

            sqlx::query(
                "INSERT INTO batches (id, source_url, source_type, name, total_tracks, state) VALUES (?, ?, ?, ?, ?, 'active')"
            )
            .bind(&batch_id)
            .bind(&normalized_url)
            .bind(source_type)
            .bind(&name)
            .bind(tracks.len() as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to insert batch: {e}"))?;

            for track in &tracks {
                let expected_path = downloader::build_output_path(&output_root_path, track, &name, &naming_template);
                let file_exists = expected_path.exists()
                    && expected_path.metadata().map(|m| m.len() > 1000).unwrap_or(false);

                let job_id = Uuid::new_v4().to_string();
                let state = if file_exists { "done" } else { "queued" };

                sqlx::query(
                    "INSERT INTO jobs (id, batch_id, spotify_id, title, artist, album, duration_ms, state, cover_url, track_number, output_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&job_id)
                .bind(&batch_id)
                .bind(&track.id)
                .bind(&track.name)
                .bind(track.artists.join(", "))
                .bind(&track.album)
                .bind(track.duration_ms as i64)
                .bind(state)
                .bind(&track.cover_url)
                .bind(track.track_number as i64)
                .bind(if file_exists { Some(expected_path.to_string_lossy().to_string()) } else { None })
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to insert job: {e}"))?;

                if !file_exists {
                    queued_count += 1;
                }
            }

            // If all tracks already on disk, mark batch complete
            if queued_count == 0 {
                sqlx::query("UPDATE batches SET state = 'complete' WHERE id = ?")
                    .bind(&batch_id)
                    .execute(&self.pool)
                    .await
                    .ok();
            } else {
                self.push_queued_jobs(&batch_id).await;
            }

            batch_id
        };

        Ok(batch_id)
    }

    pub async fn push_queued_jobs(&self, batch_id: &str) {
        let jobs: Vec<JobInfo> = sqlx::query_as(
            "SELECT j.id, j.batch_id, j.spotify_id, j.title, j.artist, j.album, j.duration_ms, j.state, j.yt_url, j.output_path, j.error, j.progress_pct, j.cover_url, j.candidates_json FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.batch_id = ? AND j.state = 'queued' AND b.state = 'active' ORDER BY j.rowid"
        )
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for job in jobs {
            let _ = sqlx::query("UPDATE jobs SET state = 'pending' WHERE id = ? AND state = 'queued'")
                .bind(&job.id)
                .execute(&self.pool)
                .await;

            let track = self.job_to_track(&job).await;
            if self.tx.try_send((job.clone(), track)).is_err() {
                let _ = sqlx::query("UPDATE jobs SET state = 'queued' WHERE id = ? AND state = 'pending'")
                    .bind(&job.id)
                    .execute(&self.pool)
                    .await;
                break;
            }
        }
    }

    pub async fn push_all_queued_jobs(&self) {
        let jobs: Vec<JobInfo> = sqlx::query_as(
            "SELECT j.id, j.batch_id, j.spotify_id, j.title, j.artist, j.album, j.duration_ms, j.state, j.yt_url, j.output_path, j.error, j.progress_pct, j.cover_url, j.candidates_json FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.state = 'queued' AND b.state = 'active' ORDER BY j.rowid"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for job in jobs {
            let _ = sqlx::query("UPDATE jobs SET state = 'pending' WHERE id = ? AND state = 'queued'")
                .bind(&job.id)
                .execute(&self.pool)
                .await;

            let track = self.job_to_track(&job).await;
            if self.tx.try_send((job.clone(), track)).is_err() {
                // Channel full — revert to queued so it can be picked up later
                let _ = sqlx::query("UPDATE jobs SET state = 'queued' WHERE id = ? AND state = 'pending'")
                    .bind(&job.id)
                    .execute(&self.pool)
                    .await;
                break;
            }
        }
    }

    async fn job_to_track(&self, job: &JobInfo) -> SpotifyTrack {
        let track_num: i64 = sqlx::query_scalar("SELECT track_number FROM jobs WHERE id = ?")
            .bind(&job.id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(1);

        SpotifyTrack {
            id: job.spotify_id.clone(),
            name: job.title.clone(),
            artists: job.artist.split(", ").map(String::from).collect(),
            album: job.album.clone(),
            album_id: String::new(),
            track_number: track_num as u16,
            disc_number: 1,
            duration_ms: job.duration_ms as u64,
            isrc: None,
            cover_url: job.cover_url.clone(),
            release_date: None,
        }
    }

    pub async fn list_batches(&self, limit: u32) -> Result<Vec<BatchInfo>, String> {
        let rows: Vec<BatchInfo> = sqlx::query_as(
            "SELECT id, source_url, source_type, name, total_tracks, state, created_at FROM batches ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list batches: {e}"))?;

        Ok(rows)
    }

    pub async fn list_jobs(&self, batch_id: Option<String>) -> Result<Vec<JobInfo>, String> {
        let rows: Vec<JobInfo> = if let Some(bid) = batch_id {
            sqlx::query_as(
                "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url, candidates_json FROM jobs WHERE batch_id = ? ORDER BY rowid"
            )
            .bind(bid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url, candidates_json FROM jobs ORDER BY rowid"
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| format!("Failed to list jobs: {e}"))?;

        Ok(rows)
    }

    pub async fn pause_batch(&self, batch_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE batches SET state = 'paused', updated_at = datetime('now') WHERE id = ?")
            .bind(batch_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to pause: {e}"))?;
        Ok(())
    }

    pub async fn resume_batch(&self, batch_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE batches SET state = 'active', updated_at = datetime('now') WHERE id = ?")
            .bind(batch_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to resume: {e}"))?;

        self.push_queued_jobs(batch_id).await;
        Ok(())
    }

    pub async fn cancel_batch(&self, batch_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE jobs SET state = 'failed', error = 'Cancelled' WHERE batch_id = ? AND state IN ('queued', 'resolving')")
            .bind(batch_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to cancel jobs: {e}"))?;

        sqlx::query("UPDATE batches SET state = 'cancelled', updated_at = datetime('now') WHERE id = ?")
            .bind(batch_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to cancel batch: {e}"))?;

        Ok(())
    }

    pub async fn retry_job(&self, job_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE jobs SET state = 'queued', error = NULL, progress_pct = NULL WHERE id = ?")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to retry: {e}"))?;

        // Also ensure batch is active
        sqlx::query("UPDATE batches SET state = 'active' WHERE id = (SELECT batch_id FROM jobs WHERE id = ?) AND state IN ('complete', 'paused')")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .ok();

        let job: Option<JobInfo> = sqlx::query_as(
            "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url, candidates_json FROM jobs WHERE id = ?"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        if let Some(job) = job {
            let track = self.job_to_track(&job).await;
            let _ = self.tx.send((job, track)).await;
        }

        Ok(())
    }

    pub async fn push_one_queued_job(&self) {
        let job: Option<JobInfo> = sqlx::query_as(
            "SELECT j.id, j.batch_id, j.spotify_id, j.title, j.artist, j.album, j.duration_ms, j.state, j.yt_url, j.output_path, j.error, j.progress_pct, j.cover_url, j.candidates_json FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.state = 'queued' AND b.state = 'active' ORDER BY j.rowid LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        if let Some(job) = job {
            let _ = sqlx::query("UPDATE jobs SET state = 'pending' WHERE id = ? AND state = 'queued'")
                .bind(&job.id)
                .execute(&self.pool)
                .await;

            let track = self.job_to_track(&job).await;
            if self.tx.try_send((job.clone(), track)).is_err() {
                let _ = sqlx::query("UPDATE jobs SET state = 'queued' WHERE id = ? AND state = 'pending'")
                    .bind(&job.id)
                    .execute(&self.pool)
                    .await;
            }
        }
    }

    pub async fn pick_candidate(&self, app: &AppHandle, job_id: &str, yt_url: &str) -> Result<(), String> {
        sqlx::query("UPDATE jobs SET state = 'queued', yt_url = ?, error = NULL, candidates_json = NULL WHERE id = ?")
            .bind(yt_url)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update job: {e}"))?;

        // Ensure batch is active
        sqlx::query("UPDATE batches SET state = 'active' WHERE id = (SELECT batch_id FROM jobs WHERE id = ?) AND state IN ('complete', 'paused')")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .ok();

        let job: Option<JobInfo> = sqlx::query_as(
            "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url, candidates_json FROM jobs WHERE id = ?"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        if let Some(job) = job {
            let track = self.job_to_track(&job).await;
            let _ = self.tx.send((job, track)).await;
        }

        Ok(())
    }

    pub async fn update_job_state(&self, job_id: &str, state: &str, error: Option<&str>) {
        let _ = sqlx::query("UPDATE jobs SET state = ?, error = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(state)
            .bind(error)
            .bind(job_id)
            .execute(&self.pool)
            .await;
    }

    pub async fn update_job_progress(&self, job_id: &str, pct: f64) {
        let _ = sqlx::query("UPDATE jobs SET progress_pct = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(pct)
            .bind(job_id)
            .execute(&self.pool)
            .await;
    }

    pub async fn update_job_cover(&self, job_id: &str, cover_url: &str) {
        let _ = sqlx::query("UPDATE jobs SET cover_url = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(cover_url)
            .bind(job_id)
            .execute(&self.pool)
            .await;
    }

    pub async fn update_job_output(&self, job_id: &str, yt_url: &str, output_path: &str) {
        let _ = sqlx::query("UPDATE jobs SET yt_url = ?, output_path = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(yt_url)
            .bind(output_path)
            .bind(job_id)
            .execute(&self.pool)
            .await;
    }

    pub async fn check_batch_complete(&self, app: &AppHandle, batch_id: &str) {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE batch_id = ?")
            .bind(batch_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let done: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE batch_id = ? AND state IN ('done', 'done_warning')")
            .bind(batch_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let failed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE batch_id = ? AND state = 'failed'")
            .bind(batch_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let needs_review: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE batch_id = ? AND state = 'needs_review'")
            .bind(batch_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let _ = app.emit("batch:progress", serde_json::json!({
            "batchId": batch_id,
            "done": done,
            "failed": failed,
            "needsReview": needs_review,
            "total": total,
        }));

        if done + failed + needs_review >= total {
            let _ = sqlx::query("UPDATE batches SET state = 'complete', updated_at = datetime('now') WHERE id = ? AND state = 'active'")
                .bind(batch_id)
                .execute(&self.pool)
                .await;

            let _ = app.emit("batch:complete", serde_json::json!({
                "batchId": batch_id,
                "total": total,
                "succeeded": done,
                "failed": failed,
            }));
        }
    }

    pub async fn reset_stuck_jobs(&self) -> u32 {
        // Only reset truly stuck states — NOT 'pending' (those are already in the channel)
        let reset = sqlx::query("UPDATE jobs SET state = 'queued', error = NULL WHERE state IN ('resolving', 'downloading')")
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() as u32)
            .unwrap_or(0);

        eprintln!("[SPYTFY] Reset {reset} stuck jobs");
        reset
    }

    pub async fn merge_duplicate_batches(&self) {
        let batches: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, name, source_type FROM batches WHERE state != 'cancelled' ORDER BY created_at ASC"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let mut seen: std::collections::HashMap<(String, String), String> = std::collections::HashMap::new();
        for (id, name, stype) in &batches {
            let key = (name.clone(), stype.clone());
            if let Some(keep_id) = seen.get(&key) {
                // Move jobs from duplicate to keeper
                let _ = sqlx::query("UPDATE jobs SET batch_id = ? WHERE batch_id = ?")
                    .bind(keep_id)
                    .bind(id)
                    .execute(&self.pool)
                    .await;
                let _ = sqlx::query("DELETE FROM batches WHERE id = ?")
                    .bind(id)
                    .execute(&self.pool)
                    .await;
                eprintln!("[SPYTFY] Merged duplicate batch '{name}' ({id}) into {keep_id}");
            } else {
                seen.insert(key, id.clone());
            }
        }

        // Update total_tracks counts
        for (_, keep_id) in &seen {
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE batch_id = ?")
                .bind(keep_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
            let _ = sqlx::query("UPDATE batches SET total_tracks = ? WHERE id = ?")
                .bind(total)
                .bind(keep_id)
                .execute(&self.pool)
                .await;
        }

        // Deduplicate jobs within merged batches (same title + artist)
        for (_, keep_id) in &seen {
            let jobs: Vec<(String, String, String, String)> = sqlx::query_as(
                "SELECT id, title, artist, state FROM jobs WHERE batch_id = ? ORDER BY rowid"
            )
            .bind(keep_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            let mut job_seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
            for (jid, title, artist, state) in &jobs {
                let key = (title.clone(), artist.clone());
                if !job_seen.insert(key) {
                    // Keep the one that's done, delete the queued duplicate
                    if state == "queued" || state == "pending" {
                        let _ = sqlx::query("DELETE FROM jobs WHERE id = ?")
                            .bind(jid)
                            .execute(&self.pool)
                            .await;
                    }
                }
            }
        }
    }

    pub async fn reset_all_stuck_jobs(&self) -> u32 {
        // Full reset on app startup — includes 'pending' since channel is empty
        let reset = sqlx::query("UPDATE jobs SET state = 'queued', error = NULL WHERE state IN ('pending', 'resolving', 'downloading')")
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() as u32)
            .unwrap_or(0);

        eprintln!("[SPYTFY] Reset {reset} stuck jobs on startup");
        reset
    }
}

pub async fn get_output_root(pool: &sqlx::SqlitePool) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'output_root'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
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
        })
}

pub async fn get_naming_template(pool: &sqlx::SqlitePool) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'naming_template'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "{folder}/{number} - {artist} - {title}".to_string())
}

pub async fn get_write_cover_jpg(pool: &sqlx::SqlitePool) -> bool {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'write_cover_jpg'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(true)
}

pub async fn get_bitrate(pool: &sqlx::SqlitePool) -> u16 {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'bitrate_kbps'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(320)
}

fn normalize_spotify_url(url: &str) -> String {
    let url = url.split('?').next().unwrap_or(url);
    let url = url.trim_end_matches('/');
    url.to_string()
}
