# Phase 3 — Single-Track Download | Discussion Notes

**Date**: 2026-05-20

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Sidecars | Tauri bundled (yt-dlp + ffmpeg) | No user setup. ~80MB app size acceptable. |
| Scope | Full single-track pipeline | Complete spec §9.2. No shortcuts. |
| YT matching | Full scoring algorithm (§13) | Duration + title + channel + negative signals. |
| Output dir | ~/Music/Spytfy/ | Standard location. Configurable via settings. |

## Deliverables

1. Sidecar config for yt-dlp + ffmpeg binaries
2. YouTube search via yt-dlp ytsearch
3. Full match scoring (spec §13)
4. Download + convert to MP3 320kbps
5. Cover art: Spotify fetch → validate → cache → embed
6. ID3v2.4 tagging with id3 crate
7. Post-tag verification (APIC present, SHA-256 match, dimensions)
8. UI: Download button → progress → done state
