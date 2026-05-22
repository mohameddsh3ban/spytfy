export type JobState =
  | 'queued'
  | 'pending'
  | 'resolving'
  | 'downloading'
  | 'converting'
  | 'tagging'
  | 'verifying'
  | 'done'
  | 'done_warning'
  | 'failed'
  | 'needs_review';

export interface DownloadJob {
  id: string;
  batchId: string;
  spotifyId: string;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  state: JobState;
  ytUrl?: string;
  ytScore?: number;
  outputPath?: string;
  error?: string;
  progressPct?: number;
  coverUrl?: string;
  candidatesJson?: string;
  createdAt?: string;
  updatedAt?: string;
}

export interface Batch {
  id: string;
  sourceUrl: string;
  sourceType: 'track' | 'album' | 'playlist';
  name: string;
  totalTracks: number;
  state: string;
  createdAt: string;
  updatedAt: string;
}
