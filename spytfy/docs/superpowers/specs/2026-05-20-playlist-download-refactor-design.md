# Playlist Download Pipeline Refactor

**Date:** 2026-05-20
**Status:** Approved
**Scope:** Queue system rewrite with channel-based architecture, batch deduplication, filesystem validation, and bug fixes

## Problem Statement

The playlist download pipeline has 9 identified bugs that cause duplicate batches, blocked queues, lost files, and inconsistent state across app restarts. The user sees the same playlist appearing multiple times in the Downloads page, and re-submitting a playlist creates entirely new jobs even when tracks are already downloaded on disk.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Duplicate playlist handling | Merge into existing batch | Avoids UI clutter, preserves progress, handles playlist growth |
| Filesystem validation | Path + size > 1KB check | Fast, no I/O overhead, good enough for music files |
| Verification failure | Keep file, mark `done_warning` | Music matters more than cover art perfection |
| Pause/cancel persistence | Persist to DB | Survives app restarts, single source of truth |
| Worker architecture | Channel + Semaphore | Eliminates polling, prevents duplicate pools, clean shutdown |

## 1. Batch Deduplication & Filesystem Check

### Enqueue Flow

When `enqueue_batch` is called:

1. Query: `SELECT * FROM batches WHERE source_url = ? AND state != 'cancelled'`
2. If existing batch found:
   - Get existing job `spotify_id` set for that batch
   - For each track in new input:
     - Skip if `spotify_id` already exists in batch
     - Skip if output file exists on disk (path exists + size > 1KB) — insert as `'done'`
     - Otherwise: INSERT new job with state `'queued'`
   - Update `batch.total_tracks` to reflect new count
   - If batch was `'complete'` or `'paused'` → set state to `'active'`
   - Return existing `batch_id`
3. If no existing batch:
   - Create new batch record
   - For each track:
     - Check output file on disk (build expected path, check exists + size > 1KB)
     - If exists → INSERT job with state `'done'` (pre-completed)
     - If not → INSERT job with state `'queued'`
   - If ALL tracks pre-completed → batch state = `'complete'`, no workers spawned
   - Return new `batch_id`

### Output Path Resolution

Filesystem check requires building the expected output path at enqueue time. Reuse `downloader::build_output_path(output_root, track)` to ensure path matches what the worker would generate. `output_root` read from settings DB at enqueue time.

## 2. Channel-Based Worker Pool

### Architecture

Replace the poll-based `run_workers` loop with a channel-driven `WorkerPool`.

```
                    +----------------+
  enqueue_batch --> |                |
  retry_job     --> | mpsc channel   | --> Worker 1 --> process_job()
  resume_queued --> | (bounded = 50) | --> Worker 2 --> process_job()
                    +----------------+
                          ^
                    QueueManager owns
                    single WorkerPool
```

### WorkerPool Struct

```rust
pub struct WorkerPool {
    tx: mpsc::Sender<(JobInfo, SpotifyTrack)>,
    shutdown: CancellationToken,
    handle: JoinHandle<()>,
}
```

### Supervisor Loop

```rust
let semaphore = Arc::new(Semaphore::new(concurrency));

loop {
    tokio::select! {
        Some((job, track)) = rx.recv() => {
            // Check if batch still active (DB query)
            // If paused/cancelled, skip this job
            let permit = semaphore.clone().acquire_owned().await;
            tokio::spawn(async move {
                process_job(&job, &track).await;
                drop(permit);
            });
        }
        _ = shutdown.cancelled() => break,
    }
}
```

### Key Properties

- **Bounded channel (50):** Natural backpressure. If channel full, sender awaits.
- **Semaphore:** Enforces exact concurrency limit. No race conditions (replaces `AtomicU8`).
- **Single instance:** `QueueManager` owns one `WorkerPool`. No duplicate pools.
- **CancellationToken:** Clean shutdown. In-flight jobs finish, no new jobs claimed.
- **Batch state check at pickup:** Jobs whose batch became paused between enqueue and pickup are skipped. The supervisor resets their state to `'queued'` in DB: `UPDATE jobs SET state = 'queued' WHERE id = ? AND state = 'resolving'`. This ensures they get picked up again when the batch is resumed.

### Job Submission

`enqueue_batch`, `retry_job`, and `resume_queued` all follow same pattern:

1. Insert/update jobs in DB
2. Query DB for queued jobs in active batches
3. Send each to channel via `worker_pool.tx.send(job)`

## 3. Pause/Cancel Persistence

### Remove In-Memory State

Delete from `QueueManager`:
- `paused_batches: Arc<RwLock<HashSet<String>>>`
- `cancelled_batches: Arc<RwLock<HashSet<String>>>`

### DB Is Source of Truth

Batch state column already supports: `active`, `paused`, `complete`, `cancelled`.

### State Transitions

```
active --> paused --> active (resume)
  |          |
  |          +-----> cancelled
  |
  +-----> complete (all jobs done/done_warning/failed)
  |
  +-----> cancelled
```

### Revised claim_next_job

```sql
SELECT j.* FROM jobs j
JOIN batches b ON j.batch_id = b.id
WHERE j.state = 'queued'
  AND b.state = 'active'
ORDER BY j.rowid
LIMIT 1
```

Single query. No in-memory sets. Pausing one batch no longer blocks other batches.

### Pause Command

```rust
pub async fn pause_batch(&self, batch_id: &str) {
    sqlx::query("UPDATE batches SET state = 'paused' WHERE id = ?")
        .bind(batch_id)
        .execute(&self.pool).await;
}
```

### Cancel Command

```rust
pub async fn cancel_batch(&self, batch_id: &str) {
    // Mark queued/resolving jobs as failed
    sqlx::query("UPDATE jobs SET state = 'failed', error = 'Cancelled' WHERE batch_id = ? AND state IN ('queued', 'resolving')")
        .bind(batch_id).execute(&self.pool).await;
    // Mark batch cancelled
    sqlx::query("UPDATE batches SET state = 'cancelled' WHERE id = ?")
        .bind(batch_id).execute(&self.pool).await;
    // In-flight jobs (downloading/tagging/verifying) finish naturally.
    // Worker checks batch state after completion — if cancelled, no further action.
}
```

### Resume Command

```rust
pub async fn resume_batch(&self, batch_id: &str) {
    sqlx::query("UPDATE batches SET state = 'active' WHERE id = ?")
        .bind(batch_id)
        .execute(&self.pool).await;
    // Push queued jobs from this batch to channel
    self.push_queued_jobs_to_channel(batch_id).await;
}
```

## 4. Startup Recovery

On app launch:

1. Reset stuck jobs: `UPDATE jobs SET state = 'queued' WHERE state IN ('resolving', 'downloading')`
2. Do NOT touch `batches.state` — paused stays paused, complete stays complete
3. Query queued jobs from active batches, push to worker pool channel
4. Worker pool supervisor handles them normally

### Fix `resume_queued` Command

- Only resets stuck jobs (`resolving`/`downloading` → `queued`)
- Does NOT update `batches.state` at all
- Pushes queued jobs from active batches to channel

## 5. Verifier — No File Deletion

### New Return Type

```rust
pub enum VerifyResult {
    Ok,
    Warning(String),
}

pub fn verify_mp3(path: &Path, expected_hash: &[u8]) -> Result<VerifyResult, String> {
    // Same checks, but:
    // - NEVER call fs::remove_file
    // - Hash mismatch → Ok with Warning("cover art hash mismatch")
    // - Small dimensions → Ok with Warning("embedded cover 200x200 < 300x300")
    // - Only Err for truly corrupt/unreadable files
}
```

### New Job State: `done_warning`

```
queued → resolving → downloading → tagging → verifying → done
                                                        → done_warning
                                                        → failed
```

Worker integration:

```rust
match verifier::verify_mp3(&output_path, &cr.hash)? {
    VerifyResult::Ok => {
        update_job_state(job_id, "done", None);
    }
    VerifyResult::Warning(msg) => {
        update_job_state(job_id, "done_warning", Some(&msg));
    }
}
```

No DB migration needed — `state` column is TEXT, not a constrained enum.

### Batch Completion Check

`done_warning` counts as completed:

```sql
SELECT COUNT(*) FROM jobs WHERE batch_id = ? AND state IN ('done', 'done_warning')
```

## 6. Progress Events & Single-Track Cleanup

### Enriched Progress Events

`download_mp3` signature gains `job_id` and `batch_id`:

```rust
pub async fn download_mp3(
    app: &AppHandle,
    yt_dlp_path: &str,
    yt_url: &str,
    output_path: &Path,
    bitrate: u16,
    job_id: &str,
    batch_id: &str,
) -> Result<(), String>
```

Emits:

```json
{
    "jobId": "...",
    "batchId": "...",
    "percent": 45.2
}
```

### Remove Single-Track Special Case

`preview-card.component.ts` simplifies to:

```typescript
async startDownload() {
    try {
        await enqueueDownload(this.resolved(), this.sourceUrl());
        this.router.navigate(['/downloads']);
    } catch (e: any) {
        this.downloadError.set(typeof e === 'string' ? e : e?.message || 'Failed to enqueue');
        this.downloadStage.set('failed');
    }
}
```

Remove: `downloadStage`, `downloadPercent`, `stageLabel`, `unlistenState`, `unlistenProgress` signals. Remove: `onDownloadState`, `onDownloadProgress` listener setup.

All download progress shown exclusively on downloads page.

### Frontend Downloads Page Updates

- Listen to `download:progress` events with `jobId` — show per-job progress bar
- Render `done_warning` as green badge with warning indicator: "Done ⚠"
- `batchStats()` counts `done` + `done_warning` as completed

## Files Touched

| File | Changes |
|------|---------|
| `queue/manager.rs` | Dedup logic in `enqueue_batch`, remove in-memory HashSets, new `WorkerPool` struct, revised `claim_next_job` with JOIN query, `push_queued_jobs_to_channel` helper |
| `queue/worker.rs` | Channel-based supervisor loop, `Semaphore` concurrency, `CancellationToken` shutdown, batch-state check before processing |
| `queue/commands.rs` | Fix `resume_queued` (no batch state changes), pass jobs to channel instead of spawning new worker pool |
| `download/downloader.rs` | Accept `job_id`/`batch_id` params, emit enriched `download:progress` events |
| `download/verifier.rs` | `VerifyResult` enum, remove all `fs::remove_file` calls, return warnings instead of errors |
| `preview-card.component.ts` | Remove single-track special case, remove unused signals/listeners, all downloads → enqueue + navigate |
| `downloads.page.ts` | Per-job progress bars, `done_warning` badge, updated `batchStats()` to count warnings as done |
| `queue.ipc.ts` | Update progress event type to include `jobId`/`batchId` |

## Files NOT Touched

- `download/scorer.rs` — scoring logic unchanged
- `download/tagger.rs` — tagging logic unchanged
- `download/cover.rs` — cover fetch/cache unchanged
- `download/youtube.rs` — YouTube search unchanged
- `spotify/resolver.rs` — Spotify API unchanged
- `ocr/browser.rs` — scraping unchanged
- DB migrations — no new columns needed (`state` is TEXT)
