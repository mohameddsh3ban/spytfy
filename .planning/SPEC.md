# Spytfy — Spec v0.2

> "Your Spotify library, downloaded as a real music library."

## Identity

| Field | Value |
|---|---|
| Codename | Spytfy |
| Type | Cross-platform desktop app (Windows / macOS / Linux) |
| Audience | Power users, DJs, archivists, self-hosted music server runners |
| Stack | Tauri 2.x + Angular 20 + Tailwind v4 + Rust/Tokio |

## Goals (v1)

1. Accept Spotify URL: track, album, playlist
2. Resolve metadata via Spotify Web API (no Premium required)
3. Match each track to best YouTube source via scoring algorithm
4. Download as MP3 (320 kbps default) with embedded ID3v2.4 tags + real cover art
5. Organize: `{output_root}/{playlist_or_album_name}/{NN} - {artist} - {title}.mp3`
6. Real-time per-track progress
7. Resume / retry failed tracks
8. Fully offline after setup. No backend, no cloud, no telemetry

## Non-Goals (v1)

- Apple Music / Tidal / Deezer (v2)
- Lossless FLAC (v2)
- DRM sources — never
- Mobile app
- Cloud sync

## Architecture

```
Angular UI (Tauri WebView)
    ↕ Tauri IPC
Rust Core: Spotify Resolver → YouTube Matcher → Download Orchestrator
    ↕ Job Queue (Tokio + SQLite)
    ↕ yt-dlp + ffmpeg (sidecars)
```

## Key Invariants

- Every MP3 has real ≥300×300 cover art. Never placeholder. Fail loudly instead.
- Cover source priority: Spotify album → Spotify playlist → YT thumbnail (cropped square) → fail
- Post-tag verification: re-read APIC, SHA-256 compare, dimension check. Fail = delete partial file.
- Spotify creds in OS keychain via tauri-plugin-keyring, never plaintext
- YT match scoring threshold ≥40 + |duration_delta| ≤5s, else NeedsReview

## Full Spec

See conversation context for complete spec v0.2 including:
- Domain model (§8)
- Core flows (§9)
- IPC contract (§10)
- UI spec (§11)
- File system contract (§12)
- YouTube match scoring (§13)
- Cover art pipeline (§14)
- Error handling matrix (§15)
- Configuration & secrets (§16)
