# Spytfy

## What This Is

Cross-platform music downloader that converts Spotify URLs (tracks, albums, playlists) into high-quality 320 kbps MP3 files with embedded ID3v2.4 tags and real album artwork. Desktop app (Windows) is complete. Now building an Android mobile app for on-device downloading.

## Core Value

Paste a Spotify link on your phone, get organized MP3 files with full metadata — no server, no cloud, no subscription, fully offline.

## Current Milestone: v2.0 Android Mobile App

**Goal:** Port Spytfy to Android as a native APK with on-device download pipeline — no server, fully offline.

**Target features:**
- Tauri 2 Android build with WebView-based Angular frontend
- On-device download engine (ARM yt-dlp + ffmpeg binaries OR embedded Rust libs)
- Spotify OAuth2 with manual Client ID/Secret entry
- URL input → resolve → batch download → organized MP3 output
- Library view for downloaded tracks
- Simplified settings (output directory, bitrate)
- Android 12+ (API 31) minimum

## Requirements

### Validated

- ✓ Spotify URL resolve (track/album/playlist) — v1.0
- ✓ YouTube search + scoring algorithm — v1.0
- ✓ MP3 download with ffmpeg transcode — v1.0
- ✓ ID3v2.4 tagging with real cover art — v1.0
- ✓ Batch queue with concurrent workers — v1.0
- ✓ Pause/resume/cancel batches — v1.0
- ✓ Retry with disambiguation UI — v1.0
- ✓ SQLite persistence — v1.0
- ✓ Organized folder output — v1.0
- ✓ OCR screenshot import — v1.0

### Active

- [ ] Android Tauri 2 build pipeline
- [ ] On-device ARM download engine (yt-dlp + ffmpeg or Rust-native)
- [ ] Mobile-adapted UI (Angular + Tailwind responsive)
- [ ] Android file storage (Music directory, scoped storage)
- [ ] Spotify auth on Android (OAuth2, credential storage)
- [ ] Mobile library view
- [ ] Mobile settings page

### Out of Scope

- OCR screenshot import — Desktop-only for v2, complex on mobile
- iOS build — Android first, iOS later
- Cloud sync — Offline-first, no backend
- Streaming/playback — Download tool, not a music player
- PKCE OAuth — Manual creds simpler for v2, consider for v3

## Context

- Desktop v1 is complete: Windows NSIS installer, all 8 phases shipped
- Tauri 2 has Android support (experimental but functional)
- Key challenge: yt-dlp and ffmpeg are sidecar EXEs on desktop — need ARM solution for mobile
- Options: bundle ARM64 Linux binaries (Termux-proven) or find Rust-native alternatives
- Android 12+ scoped storage requires SAF or MediaStore for Music directory access
- Existing Rust backend code is ~80% reusable (Spotify resolver, scorer, tagger, queue)
- Angular frontend needs responsive redesign for mobile viewports
- **Dev tooling**: Android Studio installed — use for emulator management, APK inspection, logcat debugging, and layout testing across each phase. Emulators provide fast feedback loop before physical device testing.

## Constraints

- **Platform**: Android 12+ (API 31) — modern scoped storage, Material You
- **Architecture**: On-device only, no remote server or cloud dependency
- **Binary size**: ARM yt-dlp (~18MB) + ffmpeg (~30MB ARM) = ~50MB added to APK
- **Stack**: Tauri 2 + Angular 20 + Rust (shared codebase with desktop)
- **Storage**: Must use Android Music directory or app-specific storage

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| minSdkVersion 29 (was 31) | ~92% device coverage, Tauri uses 28 | ✓ Good |
| youtubedl-android via Kotlin plugin | Battle-tested, behavioral parity with desktop yt-dlp | ✓ Good — spike 8/8 PASS |
| junkfood02 Maven Central fork | Same lib, better distribution, used by Seal (26k stars) | ✓ Good |
| App-private storage + MediaStore copy | Scoped storage blocks /sdcard writes; copy after tagging | ✓ Good |
| register_android_plugin() required | Plugin compiles without it but never loads — undocumented | ✓ Good — fixed |
| CARGO_TARGET_DIR for spaces in path | MinGW dlltool breaks on spaces; C:\spytfy-target workaround | ✓ Good |
| Same auth as desktop (manual creds) | Proven flow, simpler implementation, consistent UX | ✓ Good |
| No OCR for mobile v1 | Reduces scope, OCR deps complex on Android | ✓ Good |
| Separate branch (android) | Isolate mobile work from desktop stability | ✓ Good |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-25 after Phase 12 completion (Phases 9-12 done, downloads working on S24 Ultra)*
