# Phase 4 — Queue & Concurrency | Implementation Plan

**Date**: 2026-05-20
**Estimate**: 1 day
**Prerequisites**: Phase 3 complete (single-track pipeline working)

---

## Step 1 — Queue Manager (Rust)
**Time**: ~30 min

1. Create `src-tauri/src/queue/mod.rs`, `manager.rs`, `worker.rs`
2. `QueueManager` struct managed by Tauri state:
   ```rust
   struct QueueManager {
       pool: SqlitePool,
       concurrency: Arc<AtomicU8>,
       active_workers: Arc<AtomicU8>,
       cancel_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
   }
   ```
3. Methods:
   - `enqueue_batch(input: ResolvedInput, output_root: String) -> BatchId`
   - `pause_batch(batch_id)` / `resume_batch(batch_id)` / `cancel_batch(batch_id)`
   - `retry_job(job_id)`
   - `set_concurrency(n: u8)`
4. On enqueue: insert batch row + job rows into SQLite, then spawn workers
5. Workers pull from `jobs WHERE state = 'queued' LIMIT 1` with row locking

**Output**: Queue manager that persists jobs and spawns concurrent workers.

---

## Step 2 — Worker Pool (Rust)
**Time**: ~30 min

1. Create `src-tauri/src/queue/worker.rs`
2. Worker loop:
   ```
   loop {
       if active_workers >= concurrency: wait
       claim next queued job (UPDATE state='resolving' WHERE state='queued' LIMIT 1)
       if none: break
       run download pipeline (from Phase 3)
       update job state at each stage
       emit events
   }
   ```
3. Each stage update: `UPDATE jobs SET state=?, updated_at=now WHERE id=?`
4. Emit per-job events: `job:state { jobId, batchId, state, error? }`
5. Emit per-job progress: `job:progress { jobId, percent }`
6. On batch complete: emit `batch:complete { batchId, total, succeeded, failed }`
7. Cancellation: check `CancellationToken` before each stage

**Output**: Workers process jobs concurrently, update SQLite, emit events.

---

## Step 3 — IPC Commands
**Time**: ~15 min

1. New Tauri commands:
   - `enqueue_download(input: ResolvedInput) -> String` (returns batch_id)
   - `list_batches(limit: u32) -> Vec<Batch>`
   - `list_jobs(batch_id: Option<String>) -> Vec<DownloadJob>`
   - `pause_batch(batch_id: String)`
   - `resume_batch(batch_id: String)`
   - `cancel_batch(batch_id: String)`
   - `retry_job(job_id: String)`
2. Register all in `lib.rs` invoke_handler

**Output**: Frontend can control the queue via IPC.

---

## Step 4 — TypeScript IPC + Models
**Time**: ~10 min

1. Add to `libs/tauri-ipc/src/queue.ipc.ts`:
   ```typescript
   enqueueDownload(input: ResolvedInput): Promise<string>
   listBatches(limit: number): Promise<Batch[]>
   listJobs(batchId?: string): Promise<DownloadJob[]>
   pauseBatch / resumeBatch / cancelBatch / retryJob
   onJobState / onJobProgress / onBatchComplete (event listeners)
   ```
2. Export from index

**Output**: Typed IPC wrappers for queue management.

---

## Step 5 — Downloads Page UI
**Time**: ~40 min

1. Rewrite `apps/desktop/src/app/pages/downloads/downloads.page.ts`
2. On init: `listBatches()` + `listJobs()` to load existing state
3. Subscribe to events: `job:state`, `job:progress`, `batch:complete`
4. Layout:
   - Batch cards (grouped): cover art, name, summary "12/47 done · 2 failed"
   - Per-track rows inside batch: `[#] [title] [artist] [state badge] [progress bar]`
   - State badges: queued (gray), resolving (blue), downloading (green animated), tagging (purple), done (green check), failed (red)
5. Batch actions: Pause / Resume / Cancel / Retry Failed / Open Folder
6. Empty state: "No downloads yet. Paste a Spotify link to get started."

**Output**: Live queue view with real-time updates.

---

## Step 6 — Wire Preview Card → Queue
**Time**: ~15 min

1. Update preview card "Download" button:
   - Single track: `enqueueDownload(resolvedInput)` instead of `downloadTrack(track)`
   - Album/playlist: same — enqueues all tracks
2. After enqueue: auto-navigate to Downloads page
3. Remove old `downloadTrack` direct call from preview card
4. Show "Queued! View Downloads" link after enqueue

**Output**: Download button enqueues to queue instead of direct download.

---

## Step 7 — Concurrency Settings
**Time**: ~10 min

1. Add to Settings page:
   - "Concurrent Downloads" slider: 1-8, default 3
   - Persists to SQLite settings table
   - Changes take effect immediately via `set_concurrency()`
2. Display current active worker count

**Output**: User can control parallelism from Settings.

---

## Step 8 — Verify End-to-End
**Time**: ~20 min

1. Paste playlist URL → Fetch → Download All → navigates to Downloads
2. Verify: 3 tracks downloading in parallel, others queued
3. Progress bars updating per-track
4. State badges transitioning correctly
5. Pause batch → workers stop → Resume → continues
6. Cancel batch → remaining jobs cancelled
7. Failed track → Retry → re-downloads
8. Close app → reopen → Downloads page shows persisted state
9. Settings: change concurrency slider → takes effect

**Output**: Phase 4 complete.

---

## Dependency Graph

```
Step 1 (Queue Manager) → Step 2 (Workers) → Step 3 (IPC) → Step 6 (Wire Preview)
Step 4 (TS Models) → Step 5 (Downloads UI) → Step 6
                                              Step 7 (Settings)
All → Step 8 (Verify)
```

---

## Acceptance Criteria

- [ ] Albums/playlists download all tracks via queue
- [ ] 3 concurrent downloads by default
- [ ] Jobs persisted in SQLite (survive app restart)
- [ ] Downloads page shows live progress per track
- [ ] Batch pause/resume/cancel works
- [ ] Failed tracks can be retried
- [ ] Concurrency slider in Settings (1-8)
- [ ] State badges: queued → resolving → downloading → tagging → done/failed
