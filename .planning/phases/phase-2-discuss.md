# Phase 2 — Spotify Resolve | Discussion Notes

**Date**: 2026-05-19

## Decisions

| Decision | Choice |
|---|---|
| Credentials | Onboarding wizard in app → OS keychain |
| URL types | Track + Album + Playlist (full spec) |
| Auth flow | Client Credentials (no Premium) |
| Spotify crate | rspotify |

## Deliverables

1. Rust: rspotify Client Credentials auth, token cache + auto-refresh
2. Rust: URL parser (regex → {type, id})
3. Rust: `resolve_url` IPC command → returns metadata
4. Rust: Credential storage (tauri-plugin-keyring) + test_spotify_credentials command
5. UI: Onboarding wizard (first-run flow)
6. UI: Preview card (cover art, name, tracks, duration)

## Notes

- User already built polished Input page with URL type detection + paste support
- No Tailwind — all plain CSS with CSS variables
- Sidebar upgraded with SVG icons + Space Grotesk font
