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
        "SELECT id, batch_id, spotify_id, title, artist, album, duration_ms, state, yt_url, output_path, error, progress_pct, cover_url, candidates_json FROM jobs WHERE state = 'failed' ORDER BY rowid"
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
    eprintln!("[SPYTFY] resume_queued called");

    // Reset stuck states back to queued
    let reset = mgr.reset_stuck_jobs().await;

    // Count queued jobs in active batches
    let queued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM jobs j JOIN batches b ON j.batch_id = b.id WHERE j.state = 'queued' AND b.state = 'active'"
    )
    .fetch_one(&mgr.pool)
    .await
    .unwrap_or(0);

    eprintln!("[SPYTFY] {queued} queued jobs, pushing to channel");

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

#[tauri::command]
pub async fn pick_candidate(
    app: AppHandle,
    mgr: State<'_, QueueManager>,
    job_id: String,
    yt_url: String,
) -> Result<(), String> {
    mgr.pick_candidate(&app, &job_id, &yt_url).await
}
