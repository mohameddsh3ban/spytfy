export interface SpotifyTrack {
  id: string;
  name: string;
  artists: string[];
  album: string;
  albumId: string;
  trackNumber: number;
  discNumber: number;
  durationMs: number;
  isrc?: string;
  coverUrl?: string;
  releaseDate?: string;
}

export interface SpotifyAlbum {
  id: string;
  name: string;
  artists: string[];
  tracks: SpotifyTrack[];
  coverUrl?: string;
  releaseDate: string;
}

export interface SpotifyPlaylist {
  id: string;
  name: string;
  owner: string;
  tracks: SpotifyTrack[];
  coverUrl?: string;
}

export type ResolvedInput =
  | { type: 'track'; data: SpotifyTrack }
  | { type: 'album'; data: SpotifyAlbum }
  | { type: 'playlist'; data: SpotifyPlaylist };
