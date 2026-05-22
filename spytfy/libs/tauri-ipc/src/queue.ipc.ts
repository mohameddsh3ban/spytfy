import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ResolvedInput, Batch, DownloadJob } from '@spytfy/models';

export async function enqueueDownload(input: ResolvedInput, sourceUrl: string): Promise<string> {
  return invoke<string>('enqueue_download', { input, sourceUrl });
}

export async function listBatches(limit: number = 50): Promise<Batch[]> {
  return invoke<Batch[]>('list_batches', { limit });
}

export async function listJobs(batchId?: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>('list_jobs', { batchId: batchId ?? null });
}

export async function pauseBatch(batchId: string): Promise<void> {
  return invoke('pause_batch', { batchId });
}

export async function resumeBatch(batchId: string): Promise<void> {
  return invoke('resume_batch', { batchId });
}

export async function cancelBatch(batchId: string): Promise<void> {
  return invoke('cancel_batch', { batchId });
}

export async function retryJob(jobId: string): Promise<void> {
  return invoke('retry_job', { jobId });
}

export async function resumeQueued(): Promise<number> {
  return invoke<number>('resume_queued');
}

export async function retryAllFailed(batchId: string): Promise<number> {
  return invoke<number>('retry_all_failed', { batchId });
}

export async function pickCandidate(jobId: string, ytUrl: string): Promise<void> {
  return invoke('pick_candidate', { jobId, ytUrl });
}

export interface JobStateEvent {
  jobId: string;
  batchId: string;
  state: string;
  error?: string;
}

export interface BatchProgressEvent {
  batchId: string;
  done: number;
  failed: number;
  total: number;
}

export async function onJobState(cb: (e: JobStateEvent) => void): Promise<UnlistenFn> {
  return listen<JobStateEvent>('job:state', (e) => cb(e.payload));
}

export async function onBatchProgress(cb: (e: BatchProgressEvent) => void): Promise<UnlistenFn> {
  return listen<BatchProgressEvent>('batch:progress', (e) => cb(e.payload));
}

export async function onBatchComplete(cb: (e: BatchProgressEvent) => void): Promise<UnlistenFn> {
  return listen<BatchProgressEvent>('batch:complete', (e) => cb(e.payload));
}

export interface DownloadProgressEvent {
  jobId: string;
  batchId: string;
  percent: number;
}

export async function onDownloadJobProgress(cb: (e: DownloadProgressEvent) => void): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>('download:progress', (e) => cb(e.payload));
}

export interface JobCoverEvent {
  jobId: string;
  batchId: string;
  coverUrl: string;
}

export async function onJobCover(cb: (e: JobCoverEvent) => void): Promise<UnlistenFn> {
  return listen<JobCoverEvent>('job:cover', (e) => cb(e.payload));
}
