import { invoke } from '@tauri-apps/api/core';
import type { ResolvedInput } from '@spytfy/models';

export async function resolveUrl(url: string): Promise<ResolvedInput> {
  return invoke<ResolvedInput>('resolve_url', { url });
}

export async function saveSpotifyCredentials(
  clientId: string,
  clientSecret: string
): Promise<void> {
  return invoke('save_spotify_credentials', { clientId, clientSecret });
}

export async function testSpotifyCredentials(): Promise<void> {
  return invoke('test_spotify_credentials');
}

export async function hasSpotifyCredentials(): Promise<boolean> {
  return invoke<boolean>('has_spotify_credentials');
}

export async function debugScrape(url: string): Promise<string> {
  return invoke<string>('debug_scrape', { url });
}

export async function resolveFromJson(jsonStr: string): Promise<ResolvedInput> {
  return invoke<ResolvedInput>('resolve_from_json', { jsonStr });
}
