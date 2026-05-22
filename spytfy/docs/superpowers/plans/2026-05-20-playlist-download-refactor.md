# Playlist Download Pipeline Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the queue system with channel-based workers, batch deduplication, filesystem validation, and 9 bug fixes.

**Architecture:** Replace poll-based worker loop with `tokio::mpsc` channel + `Semaphore`. Remove in-memory pause/cancel HashSets — DB is source of truth. Add batch dedup by `source_url` and filesystem check at enqueue time. New `VerifyResult` enum stops file deletion. New `done_warning` job state.

**Tech Stack:** Rust (Tauri 2, tokio, sqlx, tokio-util for CancellationToken), Angular 20 (signals), TypeScript

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `crate/src-tauri/Cargo.toml` | Dependencies | Add `tokio-util` |
| `crate/src-tauri/src/queue/manager.rs` | QueueManager struct, batch CRUD, dedup, job helpers | Rewrite |
| `crate/src-tauri/src/queue/worker.rs` | WorkerPool struct, channel supervisor, process_job | Rewrite |
| `crate/src-tauri/src/queue/commands.rs` | Tauri command handlers | Modify |
| `crate/src-tauri/src/lib.rs` | App setup, WorkerPool init | Modify |
| `crate/src-tauri/src/download/verifier.rs` | MP3 verification | Rewrite |
| `crate/src-tauri/src/download/downloader.rs` | yt-dlp download with progress | Modify |
| `crate/libs/models/src/job.model.ts` | JobState type | Modify |
| `crate/libs/tauri-ipc/src/queue.ipc.ts` | IPC bridge, event types | Modify |
| `crate/apps/desktop/src/app/pages/input/preview-card.component.ts` | Download trigger | Modify |
| `crate/apps/desktop/src/app/pages/downloads/downloads.page.ts` | Downloads UI | Modify |

---

### Task 1: Add `tokio-util` dependency

**Files:**
- Modify: `crate/src-tauri/Cargo.toml`

- [ ] **Step 1: Add tokio-util to Cargo.toml**

In `crate/src-tauri/Cargo.toml`, add after the `tokio` line:

```toml
tokio-util = "0.7"
```

- [ ] **Step 2: Verify it compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: Compiles with no errors (new dep downloaded)

- [ ] **Step 3: Commit**

```bash
git add crate/src-tauri/Cargo.toml
git commit -m "chore: add tokio-util dependency for CancellationToken"
```

---

### Task 2: Rewrite `verifier.rs` — VerifyResult enum, no file deletion

**Files:**
- Modify: `crate/src-tauri/src/download/verifier.rs`

- [ ] **Step 1: Replace entire verifier.rs**

Replace the full contents of `crate/src-tauri/src/download/verifier.rs` with:

```rust
use id3::Tag;
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::path::Path;

pub enum VerifyResult {
    Ok,
    Warning(String),
}

pub fn verify_mp3(mp3_path: &Path, expected_cover_hash: &[u8]) -> Result<VerifyResult, String> {
    let tag =
        Tag::read_from_path(mp3_path).map_err(|e| format!("Failed to read tags for verification: {e}"))?;

    let pictures: Vec<_> = tag.pictures().collect();
    if pictures.is_empty() {
        return Ok(VerifyResult::Warning("no APIC frame found".to_string()));
    }

    let pic = &pictures[0];
    let actual_hash = Sha256::digest(&pic.data).to_vec();
    if actual_hash != expected_cover_hash {
        return Ok(VerifyResult::Warning("cover art hash mismatch".to_string()));
    }

    let img = match image::load_from_memory(&pic.data) {
        std::result::Result::Ok(img) => img,
        Err(e) => {
            return Ok(VerifyResult::Warning(format!("invalid embedded image: {e}")));
        }
    };

    let (w, h) = img.dimensions();
    if w < 300 || h < 300 {
        return Ok(VerifyResult::Warning(format!("embedded cover {w}x{h} < 300x300")));
    }

    Ok(VerifyResult::Ok)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: Compiles. Worker.rs will have warnings about unused `VerifyResult` variants — that's fine, we fix worker.rs in Task 5.

- [ ] **Step 3: Commit**

```bash
git add crate/src-tauri/src/download/verifier.rs
git commit -m "fix: verifier no longer deletes files, returns VerifyResult enum"
```

---

### Task 3: Modify `downloader.rs` — add job_id/batch_id to progress events

**Files:**
- Modify: `crate/src-tauri/src/download/downloader.rs`

- [ ] **Step 1: Update download_mp3 signature and try_download**

In `crate/src-tauri/src/download/downloader.rs`, replace the `download_mp3` function signature (line 36) with:

```rust
pub async fn download_mp3(
    app: &AppHandle,
    yt_dlp_path: &str,
    yt_url: &str,
    output_path: &Path,
    bitrate: u16,
    job_id: &str,
    batch_id: &str,
) -> Result<(), String> {
```

Update the call to `try_download` inside `download_mp3` (line 64) to pass the new params:

```rust
        match try_download(app, yt_dlp_path, yt_url, &output_template, bitrate, job_id, batch_id).await {
```

- [ ] **Step 2: Update try_download signature and progress emission**

Replace the `try_download` function signature (line 79) with:

```rust
async fn try_download(
    app: &AppHandle,
    yt_dlp_path: &str,
    yt_url: &str,
    output_template: &str,
    bitrate: u16,
    job_id: &str,
    batch_id: &str,
) -> Result<(), String> {
```

Update the progress emission (line 132) — replace:

```rust
                        let _ = app_clone.emit("download:progress", serde_json::json!({
                            "percent": pct,
                        }));
```

With (need to capture job_id and batch_id before the spawn):

Before the `tokio::spawn` block, add captures:

```rust
        let job_id_owned = job_id.to_string();
        let batch_id_owned = batch_id.to_string();
```

Then in the spawn closure, replace the emit with:

```rust
                        let _ = app_clone.emit("download:progress", serde_json::json!({
                            "jobId": job_id_owned,
                            "batchId": batch_id_owned,
                            "percent": pct,
                        }));
```

- [ ] **Step 3: Verify it compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: Errors in worker.rs because `download_mp3` now takes extra args — expected, we fix that in Task 5.

- [ ] **Step 4: Commit**

```bash
git add crate/src-tauri/src/download/downloader.rs
git commit -m "feat: add jobId/batchId to download progress events"
```

---

### Task 4: Rewrite `manager.rs` — dedup, DB-only state, channel integration

**Files:**
- Modify: `crate/src-tauri/src/queue/manager.rs`

- [ ] **Step 1: Replace entire manager.rs**

Replace the full contents of `crate/src-tauri/src/queue/manager.rs` with:

```rust
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
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

        let output_root = get_output_root(&self.pool).await;
        let output_root_path = PathBuf::from(&output_root);

        // Check for existing batch with same source_url
        let existing: Option<BatchInfo> = sqlx::query_as(
            "SELECT id, source_url, source_type, name, total_tracks, state, created_at FROM batches WHERE source_url = ? AND state != 'cancelled' LIMIT 1"
        )
        .bind(&source_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to check existing batch: {e}"))?;

        let batch_id = if let Some(batch) = existing {
            // Merge into existing batch
            let existing_ids: Vec<String> = sqlx::query_scalar(
                "SELECT spotify_id FROM jobs WHERE batch_id = ?"
            )
            .bind(&batch.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to fetch existing jobs: {e}"))?;

            let mut new_queued = 0u32;
            for track in &tracks {
                if existing_ids.contains(&track.id) {
                    continue;
                }
                let expected_path = downloader::build_output_path(&output_root_path, track);
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
            .bind(&source_url)
            .bind(source_type)
            .bind(&name)
            .bind(tracks.len() as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to insert batch: {e}"))?;

            for track in &tracks {
                let expected_path = downloader::build_output_path(&output_root_path, track);
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
            "SELECT j.id, j.batch_id, j.spotify_id, j.title, j.artist, j.album, j.duration_ms, j.state, j.yt_url, j.output_path, j.error, j.progress_pct, j.cover_url FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.batch_id = ? AND j.state = 'queued' AND b.state = 'active' ORDER BY j.rowid"
        )
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for job in jobs {
            let track = self.job_to_track(&job).await;
            let _ = self.tx.send((job, track)).await;
        }
    }

    pub async fn push_all_queued_jobs(&self) {
        let jobs: Vec<JobInfo> = sqlx::query_as(
            "SELECT j.id, j.batch_id, j.spotify_id, j.title, j.artist, j.album, j.duration_ms, j.state, j.yt_url, j.output_path, j.error, j.progress_pct, j.cover_url FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.state = 'queued' AND b.state = 'active' ORDER BY j.rowid"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for job in jobs {
            let track = self.job_to_track(&job).await;
            let _ = self.tx.send((job, track)).await;
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
                "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url FROM jobs WHERE batch_id = ? ORDER BY rowid"
            )
            .bind(bid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url FROM jobs ORDER BY rowid"
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
            "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url FROM jobs WHERE id = ?"
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

        let _ = app.emit("batch:progress", serde_json::json!({
            "batchId": batch_id,
            "done": done,
            "failed": failed,
            "total": total,
        }));

        if done + failed >= total {
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
        let reset = sqlx::query("UPDATE jobs SET state = 'queued', error = NULL WHERE state IN ('resolving', 'downloading')")
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() as u32)
            .unwrap_or(0);

        eprintln!("[CRATE] Reset {reset} stuck jobs on startup");
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
            dirs::audio_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
                .join("Crate")
                .to_string_lossy()
                .to_string()
        })
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: May have errors in commands.rs / worker.rs due to changed QueueManager API — expected, fixed in Tasks 5 and 6.

- [ ] **Step 3: Commit**

```bash
git add crate/src-tauri/src/queue/manager.rs
git commit -m "feat: rewrite QueueManager with batch dedup, DB-only state, channel integration"
```

---

### Task 5: Rewrite `worker.rs` — channel-based WorkerPool

**Files:**
- Modify: `crate/src-tauri/src/queue/worker.rs`

- [ ] **Step 1: Replace entire worker.rs**

Replace the full contents of `crate/src-tauri/src/queue/worker.rs` with:

```rust
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::manager::{get_bitrate, get_output_root, JobInfo, QueueManager};
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

        let mgr_c = mgr.clone();
        let app_c = app.clone();

        tokio::spawn(async move {
            let result = process_job(&mgr_c, &app_c, &job.id, &job.batch_id, &track).await;

            if let Err(err) = result {
                eprintln!("[CRATE] Job {} ({}) FAILED: {}", job.title, job.artist, err);
                mgr_c.update_job_state(&job.id, "failed", Some(&err)).await;
                let _ = app_c.emit("job:state", serde_json::json!({
                    "jobId": job.id, "batchId": job.batch_id, "state": "failed", "error": err,
                }));
            } else {
                eprintln!("[CRATE] Job {} ({}) DONE", job.title, job.artist);
            }

            mgr_c.check_batch_complete(&app_c, &job.batch_id).await;
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
    let yt_dlp = resolve_sidecar("yt-dlp")?;
    let yt_dlp_str = yt_dlp.to_str().ok_or("Invalid yt-dlp path")?;

    let app_data_dir = dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    let output_root = get_output_root(&mgr.pool).await;
    let bitrate = get_bitrate(&mgr.pool).await;

    let emit_state = |state: &str| {
        let _ = app.emit("job:state", serde_json::json!({
            "jobId": job_id, "batchId": batch_id, "state": state,
        }));
    };

    // Step 1: Search YouTube
    emit_state("resolving");
    mgr.update_job_state(job_id, "resolving", None).await;

    let artist = track.artists.first().cloned().unwrap_or_default();
    let candidates = youtube::search_youtube(yt_dlp_str, &artist, &track.name).await?;

    if candidates.is_empty() {
        return Err("No YouTube results found".to_string());
    }

    // Step 2: Score all candidates
    let scored = scorer::score_all_candidates(&artist, &track.name, track.duration_ms, &candidates);

    // Step 3: Try downloading — fall through to next candidate on failure
    emit_state("downloading");
    mgr.update_job_state(job_id, "downloading", None).await;

    let output_path = downloader::build_output_path(&PathBuf::from(&output_root), track);
    let mut download_err = String::new();

    for matched in &scored {
        mgr.update_job_output(job_id, &matched.url, &output_path.to_string_lossy()).await;
        match downloader::download_mp3(app, yt_dlp_str, &matched.url, &output_path, bitrate, job_id, batch_id).await {
            Ok(()) => {
                download_err.clear();
                break;
            }
            Err(e) => {
                eprintln!("[CRATE] Download failed for {}: {e}, trying next candidate", matched.url);
                download_err = e;
            }
        }
    }

    if !download_err.is_empty() && !output_path.exists() {
        return Err(format!("All download attempts failed. Last: {download_err}"));
    }

    // Step 4: Tag
    emit_state("tagging");
    mgr.update_job_state(job_id, "tagging", None).await;

    let cache_dir = cover::cover_cache_dir(&app_data_dir);
    let cache_key = if track.album_id.is_empty() { &track.id } else { &track.album_id };

    let cover_result = cover::fetch_cover(&cache_dir, cache_key, track.cover_url.as_deref()).await;

    match &cover_result {
        Ok(cr) => {
            tagger::tag_mp3(&output_path, track, cr)?;
            // Step 5: Verify
            emit_state("verifying");
            mgr.update_job_state(job_id, "verifying", None).await;

            match verifier::verify_mp3(&output_path, &cr.hash)? {
                verifier::VerifyResult::Ok => {
                    emit_state("done");
                    mgr.update_job_state(job_id, "done", None).await;
                }
                verifier::VerifyResult::Warning(msg) => {
                    eprintln!("[CRATE] Job {job_id} done with warning: {msg}");
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

    Ok(())
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: Errors in commands.rs and lib.rs — expected, fixed in Tasks 6 and 7.

- [ ] **Step 3: Commit**

```bash
git add crate/src-tauri/src/queue/worker.rs
git commit -m "feat: channel-based WorkerPool with Semaphore and CancellationToken"
```

---

### Task 6: Rewrite `commands.rs` — fix resume_queued, use new API

**Files:**
- Modify: `crate/src-tauri/src/queue/commands.rs`

- [ ] **Step 1: Replace entire commands.rs**

Replace the full contents of `crate/src-tauri/src/queue/commands.rs` with:

```rust
use tauri::{AppHandle, State};

use serde::Serialize;
use super::manager::{BatchInfo, JobInfo, QueueManager};
use crate::spotify::types::ResolvedInput;

#[derive(Serialize)]
pub struct FailedJobInfo {
    title: String,
    artist: String,
    error: String,
}

#[tauri::command]
pub async fn list_failed_jobs(
    mgr: State<'_, QueueManager>,
) -> Result<Vec<FailedJobInfo>, String> {
    let jobs: Vec<JobInfo> = sqlx::query_as(
        "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url FROM jobs WHERE state = 'failed' ORDER BY rowid"
    )
    .fetch_all(&mgr.pool)
    .await
    .map_err(|e| format!("Query failed: {e}"))?;

    Ok(jobs.into_iter().map(|j| FailedJobInfo {
        title: j.title,
        artist: j.artist,
        error: j.error.unwrap_or_default(),
    }).collect())
}

#[tauri::command]
pub async fn enqueue_download(
    app: AppHandle,
    mgr: State<'_, QueueManager>,
    input: ResolvedInput,
    source_url: String,
) -> Result<String, String> {
    mgr.enqueue_batch(&app, input, source_url).await
}

#[tauri::command]
pub async fn list_batches(
    mgr: State<'_, QueueManager>,
    limit: u32,
) -> Result<Vec<BatchInfo>, String> {
    mgr.list_batches(limit).await
}

#[tauri::command]
pub async fn list_jobs(
    mgr: State<'_, QueueManager>,
    batch_id: Option<String>,
) -> Result<Vec<JobInfo>, String> {
    mgr.list_jobs(batch_id).await
}

#[tauri::command]
pub async fn pause_batch(
    mgr: State<'_, QueueManager>,
    batch_id: String,
) -> Result<(), String> {
    mgr.pause_batch(&batch_id).await
}

#[tauri::command]
pub async fn resume_batch(
    mgr: State<'_, QueueManager>,
    batch_id: String,
) -> Result<(), String> {
    mgr.resume_batch(&batch_id).await
}

#[tauri::command]
pub async fn cancel_batch(
    mgr: State<'_, QueueManager>,
    batch_id: String,
) -> Result<(), String> {
    mgr.cancel_batch(&batch_id).await
}

#[tauri::command]
pub async fn resume_queued(
    mgr: State<'_, QueueManager>,
) -> Result<u32, String> {
    eprintln!("[CRATE] resume_queued called");

    // Reset stuck states back to queued
    let reset = mgr.reset_stuck_jobs().await;

    // Count queued jobs in active batches
    let queued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.state = 'queued' AND b.state = 'active'"
    )
    .fetch_one(&mgr.pool)
    .await
    .unwrap_or(0);

    eprintln!("[CRATE] {queued} queued jobs, pushing to channel");

    // Push queued jobs from active batches to channel
    mgr.push_all_queued_jobs().await;

    Ok(queued as u32)
}

#[tauri::command]
pub async fn retry_all_failed(
    mgr: State<'_, QueueManager>,
    batch_id: String,
) -> Result<u32, String> {
    let failed_ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM jobs WHERE batch_id = ? AND state = 'failed'"
    )
    .bind(&batch_id)
    .fetch_all(&mgr.pool)
    .await
    .map_err(|e| format!("Query failed: {e}"))?;

    let count = failed_ids.len() as u32;
    for id in &failed_ids {
        mgr.retry_job(id).await?;
    }
    Ok(count)
}

#[tauri::command]
pub async fn retry_job(
    mgr: State<'_, QueueManager>,
    job_id: String,
) -> Result<(), String> {
    mgr.retry_job(&job_id).await
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: Error in lib.rs because QueueManager::new now takes a tx param — fixed in Task 7.

- [ ] **Step 3: Commit**

```bash
git add crate/src-tauri/src/queue/commands.rs
git commit -m "fix: resume_queued no longer reopens completed batches, uses channel"
```

---

### Task 7: Update `lib.rs` — WorkerPool init at app startup

**Files:**
- Modify: `crate/src-tauri/src/lib.rs`

- [ ] **Step 1: Update lib.rs setup**

Replace the full contents of `crate/src-tauri/src/lib.rs` with:

```rust
mod commands;
mod db;
mod download;
mod ocr;
mod queue;
mod spotify;

use commands::settings;
use download::pipeline;
use ocr::commands as ocr_cmds;
use queue::commands as queue_cmds;
use spotify::{auth, resolver, scraper};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let spotify_client = auth::create_client_state();

            tauri::async_runtime::block_on(async {
                let app_data_dir = dirs::data_dir().unwrap_or_else(|| {
                    std::env::current_dir().expect("failed to get current dir")
                });

                let pool = db::init_pool(app_data_dir)
                    .await
                    .expect("failed to initialize database");

                // Create channel for worker pool
                let (tx, rx) = tokio::sync::mpsc::channel(50);

                let queue_mgr = queue::manager::QueueManager::new(pool.clone(), tx);

                // Reset stuck jobs from previous session
                queue_mgr.reset_stuck_jobs().await;

                // Spawn worker pool
                let worker_pool = queue::worker::WorkerPool::spawn(
                    rx,
                    queue_mgr.clone(),
                    app_handle.clone(),
                    2,
                );

                // Push any queued jobs from active batches
                queue_mgr.push_all_queued_jobs().await;

                app_handle.manage(pool);
                app_handle.manage(queue_mgr);
                app_handle.manage(worker_pool);

                auth::init_from_store(&app_handle, &spotify_client).await;
                app_handle.manage(spotify_client);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::update_settings,
            auth::save_spotify_credentials,
            auth::test_spotify_credentials,
            auth::has_spotify_credentials,
            resolver::resolve_url,
            scraper::debug_scrape,
            scraper::resolve_from_json,
            pipeline::download_track,
            queue_cmds::enqueue_download,
            queue_cmds::list_batches,
            queue_cmds::list_jobs,
            queue_cmds::pause_batch,
            queue_cmds::resume_batch,
            queue_cmds::cancel_batch,
            queue_cmds::retry_job,
            queue_cmds::resume_queued,
            queue_cmds::retry_all_failed,
            queue_cmds::list_failed_jobs,
            ocr_cmds::process_screenshots,
            ocr_cmds::debug_ocr,
            ocr_cmds::create_playlist_from_tracks,
            ocr_cmds::parse_text_tracklist,
            ocr_cmds::parse_spotify_html,
            ocr_cmds::scrape_playlist_tracks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Crate");
}
```

- [ ] **Step 2: Verify full Rust backend compiles**

Run: `cd crate/src-tauri && cargo check`
Expected: Clean compile with no errors.

- [ ] **Step 3: Commit**

```bash
git add crate/src-tauri/src/lib.rs
git commit -m "feat: init WorkerPool at app startup, reset stuck jobs, push queued jobs"
```

---

### Task 8: Update TypeScript models — add `done_warning` state

**Files:**
- Modify: `crate/libs/models/src/job.model.ts`

- [ ] **Step 1: Add `done_warning` to JobState**

In `crate/libs/models/src/job.model.ts`, replace the `JobState` type (line 1-10):

```typescript
export type JobState =
  | 'queued'
  | 'resolving'
  | 'downloading'
  | 'converting'
  | 'tagging'
  | 'verifying'
  | 'done'
  | 'done_warning'
  | 'failed'
  | 'needs_review';
```

- [ ] **Step 2: Commit**

```bash
git add crate/libs/models/src/job.model.ts
git commit -m "feat: add done_warning to JobState type"
```

---

### Task 9: Update `queue.ipc.ts` — add progress event with jobId

**Files:**
- Modify: `crate/libs/tauri-ipc/src/queue.ipc.ts`

- [ ] **Step 1: Add DownloadProgressEvent and listener**

In `crate/libs/tauri-ipc/src/queue.ipc.ts`, add after the `BatchProgressEvent` interface (after line 53):

```typescript
export interface DownloadProgressEvent {
  jobId: string;
  batchId: string;
  percent: number;
}

export async function onDownloadJobProgress(cb: (e: DownloadProgressEvent) => void): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>('download:progress', (e) => cb(e.payload));
}
```

- [ ] **Step 2: Export the new types from index.ts**

In `crate/libs/tauri-ipc/src/index.ts`, update the queue.ipc exports to include the new items. Replace the queue export block (lines 18-33):

```typescript
export {
  enqueueDownload,
  listBatches,
  listJobs,
  pauseBatch,
  resumeBatch,
  cancelBatch,
  retryJob,
  resumeQueued,
  retryAllFailed,
  onJobState,
  onBatchProgress,
  onBatchComplete,
  onDownloadJobProgress,
  type JobStateEvent,
  type BatchProgressEvent,
  type DownloadProgressEvent,
} from './queue.ipc';
```

- [ ] **Step 3: Commit**

```bash
git add crate/libs/tauri-ipc/src/queue.ipc.ts crate/libs/tauri-ipc/src/index.ts
git commit -m "feat: add per-job download progress event to IPC bridge"
```

---

### Task 10: Simplify `preview-card.component.ts` — remove single-track special case

**Files:**
- Modify: `crate/apps/desktop/src/app/pages/input/preview-card.component.ts`

- [ ] **Step 1: Clean up imports**

Replace line 1-6 with:

```typescript
import { Component, ChangeDetectionStrategy, input, computed, signal, OnDestroy } from '@angular/core';
import type { ResolvedInput, SpotifyTrack } from '@crate/models';
import { enqueueDownload, scrapePlaylistTracks, createPlaylistFromTracks } from '@crate/tauri-ipc';
import { Router } from '@angular/router';
import { ScreenshotModalComponent } from './screenshot-modal.component';
```

- [ ] **Step 2: Simplify the download section template**

In the template, replace the download-section block (the `<div class="download-section">` through its closing `</div>`, lines 95-123) with:

```html
      <div class="download-section">
        @if (downloadStage() === 'idle') {
          <button class="download-btn" (click)="startDownload()">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>
            </svg>
            Download {{ trackCount() === 1 ? '' : 'all ' + trackCount() + ' tracks' }}
          </button>
        } @else if (downloadStage() === 'failed') {
          <div class="download-error">
            <span>{{ downloadError() }}</span>
            <button class="download-btn retry" (click)="startDownload()">Retry</button>
          </div>
        }
      </div>
```

- [ ] **Step 3: Remove unused signals and simplify class**

Replace the class body (from `export class PreviewCardComponent` through end of file) with:

```typescript
export class PreviewCardComponent implements OnDestroy {
  resolved = input.required<ResolvedInput>();
  sourceUrl = input<string>('');
  showJson = signal(false);
  showScreenshotModal = signal(false);
  scraping = signal(false);
  scrapeError = signal('');
  scrapePercent = signal(0);
  scrapeStatus = signal('');
  scrapeLogs = signal<string[]>([]);
  showScrapeLogs = signal(false);

  downloadStage = signal<'idle' | 'failed'>('idle');
  downloadError = signal('');

  constructor(private router: Router) {}

  jsonData = computed(() => JSON.stringify(this.resolved(), null, 2));

  coverUrl = computed(() => {
    const r = this.resolved();
    return r.data.coverUrl ?? null;
  });

  title = computed(() => this.resolved().data.name);

  subtitle = computed(() => {
    const r = this.resolved();
    switch (r.type) {
      case 'track': return r.data.artists.join(', ');
      case 'album': return r.data.artists.join(', ');
      case 'playlist': return `by ${r.data.owner}`;
    }
  });

  tracks = computed<SpotifyTrack[]>(() => {
    const r = this.resolved();
    switch (r.type) {
      case 'track': return [r.data];
      case 'album': return r.data.tracks;
      case 'playlist': return r.data.tracks;
    }
  });

  trackCount = computed(() => this.tracks().length);

  formattedDuration = computed(() => {
    const totalMs = this.tracks().reduce((sum, t) => sum + t.durationMs, 0);
    const totalSec = Math.floor(totalMs / 1000);
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    return h > 0 ? `${h}h ${m}m` : `${m}m`;
  });

  formatMs(ms: number): string {
    const sec = Math.floor(ms / 1000);
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  async startDownload() {
    try {
      await enqueueDownload(this.resolved(), this.sourceUrl());
      this.router.navigate(['/downloads']);
    } catch (e: any) {
      this.downloadError.set(typeof e === 'string' ? e : e?.message || 'Failed to enqueue');
      this.downloadStage.set('failed');
    }
  }

  async autoLoadTracks() {
    this.scraping.set(true);
    this.scrapeError.set('');
    this.scrapePercent.set(0);
    this.scrapeStatus.set('Starting...');
    this.scrapeLogs.set([]);

    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<{ message: string; percent: number }>('scrape:log', (e) => {
      this.scrapeStatus.set(e.payload.message);
      this.scrapePercent.set(e.payload.percent);
      this.scrapeLogs.update(logs => [...logs, `[${e.payload.percent}%] ${e.payload.message}`]);
    });

    try {
      const tracks = await scrapePlaylistTracks(this.sourceUrl());
      if (tracks.length === 0) {
        this.scrapeError.set('No tracks found. Try the manual import.');
        return;
      }
      const r = this.resolved();
      const playlistName = r.data.name || 'Playlist';
      const coverUrl = r.data.coverUrl || null;
      const playlist = await createPlaylistFromTracks(playlistName, coverUrl, tracks);
      await enqueueDownload(playlist, this.sourceUrl());
      this.router.navigate(['/downloads']);
    } catch (e: any) {
      this.scrapeError.set(typeof e === 'string' ? e : e?.message || 'Scraping failed');
    } finally {
      unlisten();
      this.scraping.set(false);
    }
  }

  onTracksImported(playlist: ResolvedInput) {
    this.showScreenshotModal.set(false);
    enqueueDownload(playlist, this.sourceUrl()).then(() => {
      this.router.navigate(['/downloads']);
    }).catch(e => {
      this.downloadError.set(typeof e === 'string' ? e : 'Failed to enqueue');
    });
  }

  ngOnDestroy() {}
}
```

- [ ] **Step 4: Commit**

```bash
git add crate/apps/desktop/src/app/pages/input/preview-card.component.ts
git commit -m "fix: remove single-track special case, all downloads use enqueue+navigate"
```

---

### Task 11: Update `downloads.page.ts` — per-job progress, done_warning badge

**Files:**
- Modify: `crate/apps/desktop/src/app/pages/downloads/downloads.page.ts`

- [ ] **Step 1: Replace entire downloads.page.ts**

Replace the full contents of `crate/apps/desktop/src/app/pages/downloads/downloads.page.ts` with:

```typescript
import { Component, ChangeDetectionStrategy, signal, OnInit, OnDestroy } from '@angular/core';
import {
  listBatches, listJobs, pauseBatch, resumeBatch, cancelBatch, retryJob, retryAllFailed, resumeQueued,
  onJobState, onBatchProgress, onBatchComplete, onDownloadJobProgress,
  type JobStateEvent, type BatchProgressEvent, type DownloadProgressEvent,
} from '@crate/tauri-ipc';
import type { Batch, DownloadJob } from '@crate/models';
import type { UnlistenFn } from '@tauri-apps/api/event';

@Component({
  selector: 'crate-downloads-page',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="page">
      <h1>Downloads</h1>

      @if (hasStuckJobs()) {
        <button class="resume-all-btn" (click)="onResumeAll()">
          ▶ Resume {{ stuckCount() }} queued downloads
        </button>
      }

      @if (batches().length === 0) {
        <div class="empty">
          <p>No downloads yet</p>
          <p class="hint">Paste a Spotify link on the Input page to get started</p>
        </div>
      }

      @for (batch of batches(); track batch.id) {
        <div class="batch-card">
          <div class="batch-header">
            <div class="batch-info">
              <span class="batch-type">{{ batch.sourceType }}</span>
              <h2>{{ batch.name }}</h2>
              <p class="batch-stats">{{ batchStats(batch.id) }}</p>
            </div>
            <div class="batch-actions">
              @if (hasFailedJobs(batch.id)) {
                <button class="action-btn retry-all" (click)="onRetryAll(batch.id)" title="Retry all failed">↻</button>
              }
              @if (batch.state === 'active') {
                <button class="action-btn" (click)="onPause(batch.id)" title="Pause">⏸</button>
                <button class="action-btn danger" (click)="onCancel(batch.id)" title="Cancel">✕</button>
              } @else if (batch.state === 'paused') {
                <button class="action-btn" (click)="onResume(batch.id)" title="Resume">▶</button>
              }
            </div>
          </div>

          <div class="job-list">
            @for (job of getJobs(batch.id); track job.id) {
              <div class="job-row" [class]="'state-' + job.state">
                <div class="job-info">
                  <span class="job-title">{{ job.title }}</span>
                  <span class="job-artist">{{ job.artist }}</span>
                </div>
                <div class="job-status">
                  @switch (job.state) {
                    @case ('queued') { <span class="badge queued">Queued</span> }
                    @case ('resolving') { <span class="badge working">Searching…</span> }
                    @case ('downloading') {
                      <div class="download-inline">
                        <span class="badge working">Downloading</span>
                        @if (jobProgress().get(job.id); as pct) {
                          <div class="mini-progress"><div class="mini-fill" [style.width.%]="pct"></div></div>
                        }
                      </div>
                    }
                    @case ('tagging') { <span class="badge working">Tagging</span> }
                    @case ('verifying') { <span class="badge working">Verifying</span> }
                    @case ('done') { <span class="badge done">Done ✓</span> }
                    @case ('done_warning') {
                      <span class="badge done-warn" [title]="job.error || ''">Done ⚠</span>
                    }
                    @case ('failed') {
                      <span class="badge failed">Failed</span>
                      <button class="retry-btn" (click)="onRetry(job.id)">Retry</button>
                      @if (job.error) {
                        <span class="error-text">{{ job.error }}</span>
                      }
                    }
                  }
                </div>
              </div>
            }
          </div>
        </div>
      }
    </div>
  `,
  styles: `
    .resume-all-btn {
      display: flex; align-items: center; justify-content: center; gap: 8px;
      width: 100%; height: 44px; background: var(--accent); border: none; border-radius: 8px;
      color: #0c0c0e; font-family: inherit; font-size: 14px; font-weight: 600;
      cursor: pointer; margin-bottom: 16px; transition: background 150ms;
    }
    .resume-all-btn:hover { background: var(--accent-hover, #1ed760); }
    .page { padding: 32px; max-width: 800px; }
    h1 { font-size: 24px; font-weight: 600; margin-bottom: 24px; }
    .empty {
      display: flex; flex-direction: column; align-items: center; justify-content: center;
      height: 300px; color: var(--text-muted);
    }
    .empty p { font-size: 16px; }
    .empty .hint { font-size: 13px; margin-top: 8px; }
    .batch-card {
      background: var(--surface-800); border: 1px solid var(--surface-600);
      border-radius: 12px; margin-bottom: 16px; overflow: hidden;
    }
    .batch-header {
      display: flex; align-items: center; justify-content: space-between;
      padding: 16px 20px; border-bottom: 1px solid var(--surface-700);
    }
    .batch-info { min-width: 0; }
    .batch-type {
      font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em;
      color: var(--accent); margin-bottom: 4px; display: block;
    }
    h2 {
      font-family: 'Space Grotesk', sans-serif; font-size: 16px; font-weight: 600;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .batch-stats { font-size: 12px; color: var(--text-muted); margin-top: 4px; }
    .batch-actions { display: flex; gap: 8px; flex-shrink: 0; }
    .action-btn {
      width: 32px; height: 32px; border-radius: 6px; border: 1px solid var(--surface-600);
      background: var(--surface-700); color: var(--text-secondary); font-size: 14px;
      cursor: pointer; display: flex; align-items: center; justify-content: center;
      transition: all 150ms;
    }
    .action-btn:hover { background: var(--surface-600); color: var(--text-primary); }
    .action-btn.danger:hover { background: var(--error); color: white; border-color: var(--error); }
    .action-btn.retry-all { font-size: 16px; }
    .action-btn.retry-all:hover { background: var(--accent); color: #0c0c0e; border-color: var(--accent); }
    .job-list { max-height: 400px; overflow-y: auto; }
    .job-row {
      display: flex; align-items: center; justify-content: space-between;
      padding: 10px 20px; transition: background 100ms;
    }
    .job-row:hover { background: var(--surface-700); }
    .job-info { min-width: 0; flex: 1; }
    .job-title {
      font-size: 13px; font-weight: 500; display: block;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .job-artist {
      font-size: 12px; color: var(--text-muted); display: block;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .job-status { display: flex; align-items: center; gap: 8px; flex-shrink: 0; margin-left: 12px; }
    .badge {
      font-size: 11px; font-weight: 600; padding: 3px 8px; border-radius: 4px;
    }
    .badge.queued { background: var(--surface-600); color: var(--text-muted); }
    .badge.working { background: rgba(29,185,84,0.15); color: var(--accent); }
    .badge.done { background: rgba(29,185,84,0.15); color: var(--accent); }
    .badge.done-warn { background: rgba(234,179,8,0.15); color: #eab308; }
    .badge.failed { background: rgba(239,68,68,0.15); color: var(--error); }
    .download-inline { display: flex; align-items: center; gap: 6px; }
    .mini-progress {
      width: 48px; height: 4px; background: var(--surface-600); border-radius: 2px; overflow: hidden;
    }
    .mini-fill {
      height: 100%; background: var(--accent); border-radius: 2px;
      transition: width 300ms ease;
    }
    .retry-btn {
      font-size: 11px; padding: 3px 8px; border-radius: 4px; border: none;
      background: var(--surface-600); color: var(--text-secondary); cursor: pointer;
      transition: all 150ms;
    }
    .retry-btn:hover { background: var(--accent); color: #0c0c0e; }
    .error-text {
      font-size: 10px; color: var(--error); max-width: 200px;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .job-list::-webkit-scrollbar { width: 6px; }
    .job-list::-webkit-scrollbar-track { background: transparent; }
    .job-list::-webkit-scrollbar-thumb { background: var(--surface-500); border-radius: 3px; }
  `,
})
export class DownloadsPage implements OnInit, OnDestroy {
  batches = signal<Batch[]>([]);
  jobs = signal<DownloadJob[]>([]);
  stuckCount = signal(0);
  jobProgress = signal<Map<string, number>>(new Map());

  hasStuckJobs = () => this.stuckCount() > 0;

  private unlisteners: UnlistenFn[] = [];

  async ngOnInit() {
    await this.refresh();

    this.unlisteners.push(
      await onJobState((e) => this.handleJobState(e)),
      await onBatchProgress((e) => this.handleBatchProgress(e)),
      await onBatchComplete(() => this.refresh()),
      await onDownloadJobProgress((e) => this.handleDownloadProgress(e)),
    );
  }

  ngOnDestroy() {
    this.unlisteners.forEach((fn) => fn());
  }

  async refresh() {
    try {
      const [b, j] = await Promise.all([listBatches(50), listJobs()]);
      this.batches.set(b);
      this.jobs.set(j);
      const stuck = j.filter(job => ['queued', 'resolving', 'downloading'].includes(job.state)).length;
      this.stuckCount.set(stuck);
    } catch {}
  }

  async onResumeAll() {
    await resumeQueued();
    this.stuckCount.set(0);
    await this.refresh();
  }

  getJobs(batchId: string): DownloadJob[] {
    return this.jobs().filter((j) => j.batchId === batchId);
  }

  batchStats(batchId: string): string {
    const jobs = this.getJobs(batchId);
    const done = jobs.filter((j) => j.state === 'done' || j.state === 'done_warning').length;
    const failed = jobs.filter((j) => j.state === 'failed').length;
    const total = jobs.length;
    let s = `${done}/${total} done`;
    if (failed > 0) s += ` · ${failed} failed`;
    return s;
  }

  handleJobState(e: JobStateEvent) {
    this.jobs.update((jobs) =>
      jobs.map((j) =>
        j.id === e.jobId ? { ...j, state: e.state as any, error: e.error } : j
      )
    );
    // Clear progress when job leaves downloading state
    if (e.state !== 'downloading') {
      this.jobProgress.update((m) => {
        const next = new Map(m);
        next.delete(e.jobId);
        return next;
      });
    }
  }

  handleBatchProgress(_e: BatchProgressEvent) {}

  handleDownloadProgress(e: DownloadProgressEvent) {
    this.jobProgress.update((m) => {
      const next = new Map(m);
      next.set(e.jobId, e.percent);
      return next;
    });
  }

  async onPause(batchId: string) {
    await pauseBatch(batchId);
    this.batches.update((bs) => bs.map((b) => b.id === batchId ? { ...b, state: 'paused' } : b));
  }

  async onResume(batchId: string) {
    await resumeBatch(batchId);
    this.batches.update((bs) => bs.map((b) => b.id === batchId ? { ...b, state: 'active' } : b));
  }

  async onCancel(batchId: string) {
    await cancelBatch(batchId);
    await this.refresh();
  }

  hasFailedJobs(batchId: string): boolean {
    return this.getJobs(batchId).some(j => j.state === 'failed');
  }

  async onRetryAll(batchId: string) {
    await retryAllFailed(batchId);
    await this.refresh();
  }

  async onRetry(jobId: string) {
    await retryJob(jobId);
    await this.refresh();
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add crate/apps/desktop/src/app/pages/downloads/downloads.page.ts
git commit -m "feat: per-job progress bars, done_warning badge, updated batch stats"
```

---

### Task 12: Build verification

**Files:** None (verification only)

- [ ] **Step 1: Cargo check**

Run: `cd crate/src-tauri && cargo check`
Expected: Clean compile, no errors.

- [ ] **Step 2: Check Angular builds**

Run: `cd crate && npx nx build desktop`
Expected: Build succeeds. If there are type errors, fix them (likely around removed exports from `@crate/tauri-ipc`).

- [ ] **Step 3: If `downloadTrack`, `onDownloadState`, `onDownloadProgress` are still imported elsewhere, remove those imports**

Search for any remaining imports of removed symbols:
```bash
grep -r "downloadTrack\|onDownloadState\|onDownloadProgress" crate/apps/ crate/libs/
```

Remove any stale imports found. These old IPC functions may still be exported from `download.ipc.ts` — that file is untouched by this refactor, so existing exports are fine. Only fix imports that reference them from components we changed.

- [ ] **Step 4: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix: resolve build errors from refactor"
```
