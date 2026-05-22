export { getSettings, updateSettings, openFolder } from './settings.ipc';
export {
  resolveUrl,
  saveSpotifyCredentials,
  testSpotifyCredentials,
  hasSpotifyCredentials,
  debugScrape,
  resolveFromJson,
} from './spotify.ipc';
export {
  downloadTrack,
  onDownloadState,
  onDownloadProgress,
  type DownloadResult,
  type DownloadStateEvent,
} from './download.ipc';
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
  pickCandidate,
  onJobState,
  onBatchProgress,
  onBatchComplete,
  onDownloadJobProgress,
  onJobCover,
  type JobStateEvent,
  type BatchProgressEvent,
  type DownloadProgressEvent,
  type JobCoverEvent,
} from './queue.ipc';
export {
  processScreenshots,
  debugOcr,
  createPlaylistFromTracks,
  parseTextTracklist,
  parseSpotifyHtml,
  scrapePlaylistTracks,
  type ParsedTrack,
} from './ocr.ipc';
