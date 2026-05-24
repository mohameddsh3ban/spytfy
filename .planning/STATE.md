# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-25)

**Core value:** Paste a Spotify link on your phone, get organized MP3 files -- no server, no cloud, fully offline.
**Current focus:** Phase 13 - Mobile UI (responsive layout, bottom nav, touch targets)

## Current Position

Phase: 13 of 14 (Mobile UI)
Plan: 0 of ? in current phase
Status: Ready to plan
Last activity: 2026-05-25 -- Phases 9-12 complete, downloads working on S24 Ultra with MediaStore

Progress: [██████████████████████████████████░░░░░░░░░░░░░░░░░░] 67%

## Performance Metrics

**Velocity:**
- Total plans completed: 4 phases (v2.0 milestone)
- Phases 9-12 executed: 2026-05-22 to 2026-05-25
- Total execution time: ~3 working days for 4 phases

**By Phase:**

| Phase | Duration | Notes |
|-------|----------|-------|
| 9. Android Scaffold | ~1 day | Env setup, Cargo.toml, android init, first boot on S24 Ultra |
| 10. Platform Abstraction | ~0.5 day | platform.rs, cfg gates, dirs replacement |
| 11. Binary Execution Engine | ~1.5 days | Kotlin plugin, youtubedl-android, spike test PASS (28.5s, 8.13MB) |
| 12. Download Pipeline Integration | ~0.5 day | Rust-Kotlin bridge, pipeline wiring, MediaStore registration |

## What Was Built (Phases 9-12)

### Phase 9: Android Scaffold -- COMPLETE
- Environment: Android SDK, NDK r30, JDK (Android Studio JBR), 4 Rust ARM targets
- Cargo.toml: cdylib + staticlib crate-types, bundled SQLite, jni dep
- tauri.conf.json: minSdkVersion 29, tauri.android.conf.json for sidecar override
- `cargo tauri android init` -- Android project generated
- dirs::data_dir() replaced with app.path().app_data_dir() in lib.rs
- OCR module gated behind cfg(not(target_os = "android"))
- Angular serve host set to 0.0.0.0 for phone network access
- Workarounds: CARGO_TARGET_DIR for MinGW space-in-path, Windows Developer Mode for symlinks
- **Verified:** APK boots on Samsung Galaxy S24 Ultra (API 36), Angular UI renders, SQLite initializes

### Phase 10: Platform Abstraction -- COMPLETE
- Created platform.rs with cfg(target_os) gates: data_dir(), default_output_dir(), default_concurrency()
- Replaced all 7 unguarded dirs:: calls across 5 files
- Android defaults: concurrency=1 (battery), app-private Music dir
- Desktop builds unchanged

### Phase 11: Binary Execution Engine -- COMPLETE (was HIGH RISK, now resolved)
- Created tauri-plugin-spytfy-download (Kotlin + Rust)
- DownloadPlugin.kt: searchYoutube, downloadAudio, cancelDownload, registerInMediaStore commands
- youtubedl-android 0.18.1 (junkfood02 fork) + ffmpeg as Gradle deps
- Plugin registered via register_android_plugin() with PluginHandle extension trait
- Package: app.tauri.spytfy_download (matches Tauri convention)
- All operations on Dispatchers.IO (no ANR)
- **Spike test PASS (8/8):** Search 5 candidates in 4s, download 8.13MB MP3 in 28.5s, 24 progress callbacks

### Phase 12: Download Pipeline Integration -- COMPLETE
- download/android.rs: search_youtube_android() + download_audio_android() via PluginHandle
- pipeline.rs + worker.rs: cfg(target_os) gates routing through Kotlin plugin on Android
- Shared code unchanged: scorer.rs, tagger.rs, cover.rs, verifier.rs, queue manager
- Fixed scoped storage error: output path changed from /storage/emulated/0/Music to app-private dir
- MediaStore registration added: registerInMediaStore() copies tagged MP3 to Music/Spytfy/ via ContentResolver
- **Verified:** Tracks download, get ID3 tags + cover art, appear in Samsung Music

### STOR-01 & STOR-02: Delivered early (originally Phase 14)
- MediaStore IS_PENDING flow implemented in DownloadPlugin.kt
- Downloaded MP3s visible in Samsung Music, VLC, Poweramp after download completes

## Accumulated Context

### Decisions

- [Phase 9]: CARGO_TARGET_DIR=C:\spytfy-target required to avoid MinGW spaces-in-path build failure
- [Phase 9]: Windows Developer Mode required for symlink creation during Tauri Android build
- [Phase 9]: minSdkVersion set to 29 (Android 10, ~92% coverage) per patch document
- [Phase 10]: cfg(target_os) module pattern chosen over trait-based dispatch (matches Tauri's own pattern)
- [Phase 11]: register_android_plugin() is REQUIRED -- without it, plugin compiles but never loads
- [Phase 11]: Plugin package must be app.tauri.<plugin_id> -- custom packages silently fail
- [Phase 11]: youtubedl-android spike PASSED -- Plan A confirmed, no need for Plan B
- [Phase 12]: App-private storage for downloads, MediaStore copy after tagging for visibility
- [Phase 12]: rspotify already uses reqwest-rustls-tls -- no openssl-sys on Android (open question resolved)

### Pending Todos

- Remove build artifacts from git (plugin build/ directory was accidentally committed)
- Update REQUIREMENTS.md and PROJECT.md with patch document additions (DL-07, STOR-05, STOR-06, BUILD-07)

### Blockers/Concerns

- Dev mode requires USB connection + dev server running -- user can't test independently yet (Phase 14 fixes with signed APK)
- "Made It On Our Own" track initially failed with needs_review (scoring issue with some tracks) -- worked after retry with fresh path
- Old failed jobs remain in SQLite with stale paths -- user must try fresh downloads after path fix

## Deferred Items

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Mobile | OCR screenshot import | Deferred to v3.0 | v2.0 init |
| Mobile | Share intent (receive URLs) | Deferred to v3.0 | v2.0 init |
| Mobile | Material You dynamic theming | Deferred to v3.0 | v2.0 init |
| Platform | iOS build | Deferred to v3.0 | v2.0 init |
| Auth | PKCE OAuth flow | Deferred to v3.0 | v2.0 init |

## Session Continuity

Last session: 2026-05-25
Stopped at: Phases 9-12 complete. Downloads working on S24 Ultra with MediaStore visibility. Ready to plan Phase 13 (Mobile UI).
Resume with: `/gsd:plan-phase 13` in a fresh session (context was too long)
