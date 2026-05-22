# Phase 2 — Spotify Resolve | Implementation Plan

**Date**: 2026-05-19
**Estimate**: 1 day
**Prerequisites**: Phase 1 complete, Spotify dev account for testing

---

## Step 1 — Add Rust Dependencies
**Time**: ~5 min

1. Add to `src-tauri/Cargo.toml`:
   ```toml
   rspotify = { version = "0.13", features = ["client-credentials"] }
   tauri-plugin-store = "2"
   regex = "1"
   ```
2. `cargo check` to verify deps resolve

**Output**: Dependencies locked.

---

## Step 2 — Spotify URL Parser
**Time**: ~15 min

1. Create `src-tauri/src/spotify/mod.rs` + `src-tauri/src/spotify/parser.rs`
2. Regex patterns:
   - `https://open.spotify.com/(track|album|playlist)/([a-zA-Z0-9]+)`
   - Handle `?si=...` query params
   - Handle `spotify:track:ID` URI format
3. Returns `SpotifyUrl { kind: "track"|"album"|"playlist", id: String }`
4. Unit tests for all URL formats

**Output**: URL parser with tests.

---

## Step 3 — Spotify Auth (Client Credentials)
**Time**: ~20 min

1. Create `src-tauri/src/spotify/auth.rs`
2. Read client_id + client_secret from tauri-plugin-store (encrypted local storage)
3. `rspotify::ClientCredsSpotify` with auto-refresh
4. Singleton pattern: store authenticated client in Tauri managed state
5. IPC commands:
   - `save_spotify_credentials(client_id, client_secret)` → store + init client
   - `test_spotify_credentials()` → attempt auth, return success/error
   - `has_spotify_credentials()` → bool check

**Output**: Auth flow working, creds persisted securely.

---

## Step 4 — Resolve URL Command
**Time**: ~30 min

1. Create `src-tauri/src/spotify/resolver.rs`
2. Domain types in `src-tauri/src/spotify/types.rs`:
   ```rust
   struct SpotifyTrack { id, name, artists, album, album_id, track_number, 
                          disc_number, duration_ms, isrc, cover_url, release_date }
   struct SpotifyAlbum { id, name, artists, tracks, cover_url, release_date }
   struct SpotifyPlaylist { id, name, owner, tracks, cover_url }
   enum ResolvedInput { Track(SpotifyTrack), Album(SpotifyAlbum), Playlist(SpotifyPlaylist) }
   ```
3. IPC command: `resolve_url(url: String) -> Result<ResolvedInput, AppError>`
4. For playlists: paginate all tracks (Spotify returns 100 per page)
5. Map rspotify types to our domain types
6. Error handling: 401 (bad creds), 404 (not found), 429 (rate limit)

**Output**: `resolve_url` returns full metadata for any Spotify URL.

---

## Step 5 — TypeScript Models Update
**Time**: ~10 min

1. Add to `libs/models/src/`:
   ```typescript
   interface SpotifyTrack { id, name, artists, album, albumId, trackNumber, 
                             discNumber, durationMs, isrc?, coverUrl?, releaseDate? }
   interface SpotifyAlbum { id, name, artists, tracks, coverUrl?, releaseDate }
   interface SpotifyPlaylist { id, name, owner, tracks, coverUrl? }
   type ResolvedInput = 
     | { type: 'track'; data: SpotifyTrack }
     | { type: 'album'; data: SpotifyAlbum }
     | { type: 'playlist'; data: SpotifyPlaylist }
   ```
2. Add to `libs/tauri-ipc/src/`:
   ```typescript
   resolveUrl(url: string): Promise<ResolvedInput>
   saveSpotifyCredentials(clientId, clientSecret): Promise<void>
   testSpotifyCredentials(): Promise<void>
   hasSpotifyCredentials(): Promise<boolean>
   ```

**Output**: Typed IPC wrappers for frontend.

---

## Step 6 — Onboarding Wizard UI
**Time**: ~30 min

1. Create `apps/desktop/src/app/pages/onboarding/` component
2. Multi-step wizard:
   - Step 1: "Welcome to Spytfy" — explain what the app does
   - Step 2: "Connect Spotify" — explain why API keys needed (catalog only, no Premium)
   - Step 3: Link to developer.spotify.com/dashboard, instructions to create app
   - Step 4: Paste client ID + client secret fields
   - Step 5: "Test Connection" button → success animation → done
3. Route guard: redirect to `/onboarding` if `hasSpotifyCredentials()` returns false
4. Settings page: "Spotify Connection" section with re-configure option

**Output**: First-run wizard, guards input/downloads/library routes until creds set.

---

## Step 7 — Preview Card UI
**Time**: ~30 min

1. Update Input page: after `resolve_url` succeeds, show preview card
2. Preview card shows:
   - Cover art (large, from `coverUrl`)
   - Name + type badge (Track / Album / Playlist)
   - Artist(s) / Owner
   - Track count + total duration (formatted as "1h 23m")
   - For album/playlist: scrollable track list with checkboxes
3. "Download All" / "Download N Selected" CTA button (wired in Phase 3)
4. Loading state: skeleton placeholder while resolving
5. Error state: friendly message + retry

**Output**: Full preview card rendering after URL resolve.

---

## Step 8 — Verify End-to-End
**Time**: ~15 min

1. Fresh app launch → onboarding wizard appears
2. Enter Spotify dev credentials → test connection → success
3. Paste track URL → resolves → preview card shows metadata + cover art
4. Paste album URL → resolves → track list renders
5. Paste playlist URL → resolves → all tracks paginated
6. Invalid URL → error message
7. Bad credentials → clear error

**Output**: Phase 2 complete. Spotify resolve working end-to-end.

---

## Dependency Graph

```
Step 1 (deps) → Step 2 (parser) → Step 4 (resolver)
                Step 3 (auth)   → Step 4 (resolver)
Step 5 (TS models) → Step 6 (onboarding) → Step 8 (verify)
                     Step 7 (preview)     → Step 8 (verify)
```

**Parallelizable**: Steps 2+3 after Step 1. Steps 5+6+7 can start alongside Rust work.

---

## Acceptance Criteria

- [ ] `resolve_url` returns metadata for track/album/playlist URLs
- [ ] Spotify credentials stored in local encrypted store
- [ ] Onboarding wizard guides first-time setup
- [ ] Preview card shows cover art, track list, duration
- [ ] Pagination works for playlists with 100+ tracks
- [ ] Error states: bad URL, bad creds, not found, rate limited
