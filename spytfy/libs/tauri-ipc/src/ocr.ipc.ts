import { invoke } from '@tauri-apps/api/core';
import type { ResolvedInput } from '@spytfy/models';

export interface ParsedTrack {
  trackNumber: number;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  coverUrl?: string;
}

export async function processScreenshots(imagePaths: string[]): Promise<ParsedTrack[]> {
  return invoke<ParsedTrack[]>('process_screenshots', { imagePaths });
}

export async function debugOcr(imagePath: string): Promise<string> {
  return invoke<string>('debug_ocr', { imagePath });
}

export async function parseTextTracklist(text: string): Promise<ParsedTrack[]> {
  return invoke<ParsedTrack[]>('parse_text_tracklist', { text });
}

export async function scrapePlaylistTracks(url: string): Promise<ParsedTrack[]> {
  return invoke<ParsedTrack[]>('scrape_playlist_tracks', { url });
}

export async function parseSpotifyHtml(html: string): Promise<ParsedTrack[]> {
  return invoke<ParsedTrack[]>('parse_spotify_html', { html });
}

export async function createPlaylistFromTracks(
  playlistName: string,
  coverUrl: string | null,
  tracks: ParsedTrack[]
): Promise<ResolvedInput> {
  return invoke<ResolvedInput>('create_playlist_from_tracks', {
    playlistName,
    coverUrl,
    tracks,
  });
}
