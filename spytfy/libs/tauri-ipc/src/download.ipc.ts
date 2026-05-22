import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { SpotifyTrack } from '@spytfy/models';

export interface DownloadResult {
  outputPath: string;
  ytUrl: string;
  ytScore: number;
}

export interface DownloadStateEvent {
  stage: 'searching' | 'matching' | 'downloading' | 'tagging' | 'verifying' | 'done' | 'failed';
}

export interface DownloadProgressEvent {
  percent: number;
}

export async function downloadTrack(track: SpotifyTrack): Promise<DownloadResult> {
  return invoke<DownloadResult>('download_track', { track });
}

export async function onDownloadState(
  callback: (event: DownloadStateEvent) => void
): Promise<UnlistenFn> {
  return listen<DownloadStateEvent>('download:state', (e) => callback(e.payload));
}

export async function onDownloadProgress(
  callback: (event: DownloadProgressEvent) => void
): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>('download:progress', (e) => callback(e.payload));
}
