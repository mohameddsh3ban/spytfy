# Spytfy v2.0 Android Mobile App — Full Plan Documentation

**Prepared for:** Expert Technical Review  
**Date:** 2026-05-22  
**Author:** Development Team (AI-assisted with Claude Code)  
**Status:** Planning complete, awaiting validation before execution  
**Repository:** https://github.com/mohameddsh3ban/spytfy (branch: `android`)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Project Background](#2-project-background)
3. [Milestone Scope & Goals](#3-milestone-scope--goals)
4. [Technical Architecture](#4-technical-architecture)
5. [Technology Stack](#5-technology-stack)
6. [Requirements Specification](#6-requirements-specification)
7. [Phase-by-Phase Execution Plan](#7-phase-by-phase-execution-plan)
8. [Risk Analysis](#8-risk-analysis)
9. [Known Pitfalls & Mitigations](#9-known-pitfalls--mitigations)
10. [Alternatives Considered](#10-alternatives-considered)
11. [Open Questions](#11-open-questions)
12. [Confidence Assessment](#12-confidence-assessment)
13. [Sources & References](#13-sources--references)

---

## 1. Executive Summary

**What:** Port the existing Spytfy desktop music downloader to Android as a native APK.

**Why:** Users want to download Spotify music directly from their phones -- paste a Spotify link, get organized 320 kbps MP3 files with full ID3v2.4 metadata and real album artwork. Fully offline, no server, no cloud dependency.

**How:** Tauri 2 supports Android targets. ~80% of the existing Rust backend (Spotify resolver, YouTube scorer, ID3 tagger, cover art fetcher, queue manager, SQLite persistence) cross-compiles to Android without changes. The critical challenge is replacing the desktop download engine -- Tauri's shell plugin does NOT support process spawning on Android, so we bridge `youtubedl-android` (a battle-tested Kotlin library) to Rust via custom Tauri Kotlin plugins.

**Risk:** MEDIUM overall. Phase 11 (Binary Execution Engine) is HIGH risk -- no existing Tauri + youtubedl-android integration example exists. Positioned early in the roadmap for fail-fast discovery.

**Timeline:** 6 phases (Phase 9-14), estimated ~12-15 working days.

**Deliverable:** Signed Android APK (API 31+) distributable via direct download or F-Droid.

---

## 2. Project Background

### What Spytfy Does

Spytfy is a cross-platform desktop application that converts Spotify URLs into high-quality MP3 files:

1. User pastes a Spotify URL (track, album, or playlist)
2. App resolves metadata via Spotify Web API (no Premium required)
3. Searches YouTube for each track using a scoring algorithm (title similarity + duration match within +/-5 seconds)
4. Downloads as 320 kbps MP3 via yt-dlp + ffmpeg
5. Embeds ID3v2.4 tags (title, artist, album, track number) with real album artwork (>=300x300px, verified post-embed via SHA-256)
6. Organizes files: `{output}/{playlist_name}/{01 - Artist - Title}.mp3`

### Desktop v1.0 (Complete)

| Phase | What Shipped |
|-------|-------------|
| Phase 1: Scaffold | Tauri 2 + Angular 20 + Nx monorepo |
| Phase 2: Spotify Resolve | URL parser, OAuth2 auth, track/album/playlist resolution |
| Phase 3: Single-Track Download | yt-dlp search, ffmpeg transcode, ID3 tagging, cover art |
| Phase 4: Queue & Concurrency | SQLite-backed batch queue, Tokio worker pool, concurrent downloads |
| Phase 4B: Screenshot Import | OCR via Tesseract, playlist extraction from screenshots |
| Phase 4C: Cover Art | Per-track Spotify thumbnail embedding with verification |
| Phase 5: Folders & Naming | Organized output directories, naming templates |
| Phase 6: Failure Recovery | Retry with top-3 YouTube candidate disambiguation, batch pause/cancel |
| Phase 7: Polish | Empty states, toast notifications, keyboard shortcuts (Ctrl+1-4) |
| Phase 8: Packaging | Windows NSIS installer (~53 MB) |

**Stack:** Tauri 2.x, Angular 20, Tailwind CSS v4, Rust/Tokio, SQLite (SQLx), rspotify, yt-dlp + ffmpeg sidecars.

### Why Android Now

- Desktop v1 is feature-complete and stable
- Users want mobile access -- download from phone, music ready when they get home
- Tauri 2 has Android support (experimental but functional)
- ~80% Rust code reusable -- the port is primarily a platform integration challenge, not a rewrite

---

## 3. Milestone Scope & Goals

### Goal

Port Spytfy to Android as a native APK with on-device download pipeline -- no server, fully offline.

### In Scope (v2.0)

- Tauri 2 Android build with WebView-based Angular frontend
- On-device download engine via youtubedl-android Kotlin library
- Spotify OAuth2 with manual Client ID/Secret entry (same as desktop)
- URL input -> resolve -> batch download -> organized MP3 output
- Library view for downloaded tracks
- Simplified settings (output directory, bitrate)
- Background downloads via Android foreground service
- MediaStore integration for Music directory visibility
- Signed release APK
- Android 12+ (API 31) minimum target

### Out of Scope (v2.0)

| Feature | Reason |
|---------|--------|
| OCR screenshot import | Complex on mobile (camera permissions, ML models, heavy deps) |
| iOS build | Android first, iOS in v3+ after Android is stable |
| Cloud sync | Core value is offline-first, no backend server |
| Music playback/streaming | Download tool, not a music player |
| Google Play distribution | Store policies may conflict; direct APK/F-Droid instead |
| MANAGE_EXTERNAL_STORAGE | Play Store rejection risk; use MediaStore instead |
| Rust-native yt-dlp replacement | Crates not mature enough (rusty_ytdl, symphonia lack Opus decoder) |
| PKCE OAuth | Manual creds simpler for v2, consider for v3 |
| Share intent | Deferred to v2.1 -- adds value but not blocking MVP |

### Key Constraints

- **Platform:** Android 12+ (API 31) -- scoped storage built-in, Chrome 96+ WebView guaranteed
- **Architecture:** On-device only, no remote server
- **Binary size:** ~80 MB APK (yt-dlp + Python 3.8 + ffmpeg bundled via youtubedl-android)
- **Stack:** Shared codebase with desktop (Tauri 2 + Angular 20 + Rust)
- **Storage:** MediaStore API for Music directory; app-specific for temp/DB

---

## 4. Technical Architecture

### System Overview

```
+----------------------------------------------------------+
|                    Android APK (~80 MB)                   |
|                                                          |
|  +------------------+    +----------------------------+  |
|  | Angular 20       |    | Tauri Rust Library (.so)   |  |
|  | WebView Frontend |<-->| - Spotify resolver (shared)|  |
|  | (System WebView) |IPC | - YouTube scorer (shared)  |  |
|  | Responsive UI    |    | - Queue manager (shared)   |  |
|  +------------------+    | - SQLite/SQLx (shared)     |  |
|         ^                | - ID3 tagger (shared)      |  |
|         |                | - Cover art (shared)       |  |
|         v                | - Platform abstraction     |  |
|  +------------------+    +----------------------------+  |
|  | Tauri Android    |               |                    |
|  | Runtime (Kotlin) |               | Plugin Bridge      |
|  | - WRY WebView    |               v                    |
|  | - IPC Bridge     |    +----------------------------+  |
|  +------------------+    | Custom Kotlin Plugins (3x)  |  |
|                          | 1. Download Engine          |  |
|                          |    (youtubedl-android)      |  |
|                          | 2. MediaStore Writer        |  |
|                          | 3. Foreground Service       |  |
|                          +----------------------------+  |
|                                     |                    |
|                                     v                    |
|                          +----------------------------+  |
|                          | youtubedl-android Bundled   |  |
|                          | - Python 3.8 (ARM64)       |  |
|                          | - yt-dlp (latest)          |  |
|                          | - ffmpeg (ARM64 static)    |  |
|                          | (in nativeLibraryDir)      |  |
|                          +----------------------------+  |
+----------------------------------------------------------+
```

### Code Reuse Analysis

| Component | Files | Reusable? | Notes |
|-----------|-------|-----------|-------|
| Spotify URL parser | `spotify/parser.rs` | 100% | Pure string parsing |
| Spotify types | `spotify/types.rs` | 100% | Data structures |
| Spotify auth | `spotify/auth.rs` | 100% | rspotify, cross-platform |
| Spotify resolver | `spotify/resolver.rs` | 100% | HTTP via reqwest/rustls |
| Spotify scraper | `spotify/scraper.rs` | 100% | HTTP-based |
| YouTube scorer | `download/scorer.rs` | 100% | Pure algorithm |
| ID3 tagger | `download/tagger.rs` | 100% | id3 crate, cross-platform |
| Cover art fetcher | `download/cover.rs` | 100% | reqwest + image crate |
| MP3 verifier | `download/verifier.rs` | 100% | Pure file I/O |
| Queue manager | `queue/manager.rs` | 100% | SQLx queries |
| Queue commands | `queue/commands.rs` | 100% | Tauri command wrappers |
| Worker pool | `queue/worker.rs` | ~80% | Needs Android concurrency limits |
| Settings commands | `commands/settings.rs` | 100% | SQLx queries |
| Database init | `db.rs` | **Needs changes** | Replace `dirs::data_dir()` |
| Download pipeline | `download/pipeline.rs` | **Needs changes** | Platform-gated execution |
| YouTube search | `download/youtube.rs` | **Desktop only** | Uses yt-dlp subprocess |
| Downloader | `download/downloader.rs` | **Desktop only** | Uses yt-dlp subprocess |
| OCR module | `ocr/*` | **Excluded** | Out of scope for mobile |

**Summary:** ~80% reusable. Only the binary execution layer (youtube.rs, downloader.rs) and path resolution (db.rs) need Android-specific implementations.

### Data Flow: Desktop vs Android

**Desktop (current):**
```
Spotify URL -> Rust resolve -> tokio::process::Command("yt-dlp") -> ffmpeg transcode -> tag -> verify -> save
```

**Android (proposed):**
```
Spotify URL -> Rust resolve -> Kotlin Plugin -> youtubedl-android -> MP3 file -> Rust tag -> Rust verify -> Kotlin MediaStore -> Music/
```

### IPC (Frontend <-> Backend)

Tauri IPC works identically on Android. The `invoke()` calls from Angular and `app.emit()` from Rust require zero changes. Transport switches from custom protocol to `postMessage` on Android WebView, but this is transparent to application code.

**Existing IPC channels (all reusable):**
- Commands: `resolve_url`, `download_track`, `enqueue_download`, `process_screenshots` (excluded on mobile)
- Events: `download:state`, `download:progress`, `job:state`, `batch:progress`, `batch:complete`, `job:cover`

---

## 5. Technology Stack

### Existing (Shared with Desktop)

| Technology | Version | Purpose | Android Compatible |
|------------|---------|---------|-------------------|
| Tauri | 2.x | App framework | Yes (Android target) |
| Angular | 20 | Frontend | Yes (WebView) |
| Tailwind CSS | v4 | Styling | Yes |
| Rust + Tokio | 1.70+ / 1.x | Backend runtime | Yes (ARM64 cross-compile) |
| SQLx + SQLite | 0.8 | Database | Yes (with `bundled` feature) |
| rspotify | 0.13 | Spotify API | Yes (reqwest/rustls-tls) |
| reqwest | 0.12 | HTTP client | Yes (rustls-tls) |
| id3 | 1.x | MP3 tagging | Yes (pure Rust) |
| image | 0.25 | Cover art | Yes (pure Rust) |
| sha2 | 0.10 | File verification | Yes (pure Rust) |

### New (Android-Specific)

| Technology | Version | Purpose | Why This |
|------------|---------|---------|----------|
| youtubedl-android | 0.18.1 | Download engine | Battle-tested (1800+ stars), bundles Python + yt-dlp + ffmpeg, handles W^X/SELinux |
| Kotlin Tauri Plugins (3x) | Custom | Platform bridge | Shell plugin is desktop-only; custom plugins use @TauriPlugin annotation |
| MediaStore API | Android SDK | Music dir writes | No permissions needed on API 31+; files visible to other music apps |
| Foreground Service | Android SDK | Background downloads | Android kills background work after ~10s; foreground service + notification required |
| Android NDK | r26+ | Rust cross-compilation | Required for aarch64-linux-android target; r26+ fixes SQLite linker issue |
| JDK | Android Studio JBR | Gradle builds | Bundled with Android Studio |

### Cargo.toml Changes Required

```toml
[lib]
name = "spytfy_lib"
crate-type = ["staticlib", "cdylib", "rlib"]  # cdylib required for Android .so

[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "bundled"] }  # bundled for Android

[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"  # JNI bindings for Kotlin plugin communication
```

### Android Permissions Required

```xml
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
<!-- No WRITE_EXTERNAL_STORAGE needed on API 31+ when using MediaStore -->
<!-- No READ_MEDIA_AUDIO needed -- app only accesses files it created -->
```

---

## 6. Requirements Specification

### v2.0 Requirements (22 total)

#### Build & Platform (6 requirements)

| ID | Requirement | Phase |
|----|------------|-------|
| BUILD-01 | App builds as Android APK targeting API 31+ via Tauri 2 Android | 9 |
| BUILD-02 | Rust backend cross-compiles to ARM64 with cdylib crate-type | 9 |
| BUILD-03 | Platform abstraction layer (cfg target_os) keeps desktop builds green | 10 |
| BUILD-04 | All `dirs` crate calls replaced with Tauri `app.path()` APIs on Android | 9 |
| BUILD-05 | SQLite database works on Android (bundled feature, correct path) | 9 |
| BUILD-06 | Release APK is signed and versioned for distribution | 14 |

#### Download Engine (6 requirements)

| ID | Requirement | Phase |
|----|------------|-------|
| DL-01 | Custom Kotlin Tauri plugin bridges youtubedl-android library to Rust | 11 |
| DL-02 | YouTube search returns scored candidates on Android (same scoring as desktop) | 11 |
| DL-03 | Audio downloads as 320 kbps MP3 via youtubedl-android ffmpeg | 11 |
| DL-04 | ID3v2.4 tags + cover art embedded on downloaded MP3s (reuse desktop tagger) | 12 |
| DL-05 | Post-download SHA-256 verification (reuse desktop verifier) | 12 |
| DL-06 | Full pipeline: Spotify resolve -> YT search -> score -> download -> tag -> verify | 12 |

#### Storage & Background (4 requirements)

| ID | Requirement | Phase |
|----|------------|-------|
| STOR-01 | Downloaded MP3s written to Music/ directory via MediaStore API | 14 |
| STOR-02 | MP3s visible in other Android music apps (Samsung Music, Poweramp) | 14 |
| STOR-03 | Foreground service keeps downloads alive when app is backgrounded | 14 |
| STOR-04 | Notification shows download progress with track count | 14 |

#### Mobile UI (6 requirements)

| ID | Requirement | Phase |
|----|------------|-------|
| UI-01 | Bottom navigation replacing desktop sidebar (Input, Downloads, Library, Settings) | 13 |
| UI-02 | Mobile-adapted URL input page with paste + resolve + preview | 13 |
| UI-03 | Downloads page with batch progress, per-track status, pause/resume/cancel | 13 |
| UI-04 | Library page showing downloaded tracks grouped by album/playlist | 13 |
| UI-05 | Settings page (output quality, Spotify credentials) | 13 |
| UI-06 | Onboarding page for Spotify Client ID/Secret entry | 13 |

### Traceability

All 22 requirements mapped to phases. 0 unmapped.

---

## 7. Phase-by-Phase Execution Plan

### Phase 9: Android Scaffold

**Goal:** A Tauri 2 Android app boots on a device/emulator with WebView UI and SQLite working.

**Requirements:** BUILD-01, BUILD-02, BUILD-04, BUILD-05

**What gets done:**
1. Set up Android SDK, NDK, JDK environment variables
2. Add Rust Android targets (`aarch64-linux-android`, etc.)
3. Update Cargo.toml: add `cdylib` crate-type, `bundled` SQLite feature
4. Run `cargo tauri android init` to generate Android project structure
5. Replace `dirs::data_dir()` with `app.path().app_data_dir()` in db.rs
6. Configure `minSdkVersion: 31` in Tauri Android config
7. Verify: APK installs on emulator, Angular UI renders, SQLite initializes

**Success Criteria:**
1. `cargo tauri android dev` produces APK that installs on Android 12+ emulator
2. Existing Angular UI renders in WebView (pages load, navigation works)
3. SQLite database initializes (migrations run, tables created)
4. Rust backend loads via JNI without `UnsatisfiedLinkError`

**Depends on:** Nothing (first phase)  
**Estimated effort:** 1-2 days  
**Risk:** LOW -- well-documented Tauri Android setup

---

### Phase 10: Platform Abstraction

**Goal:** Shared codebase compiles for both Windows desktop and Android without leaking platform-specific code.

**Requirements:** BUILD-03

**What gets done:**
1. Create `platform.rs` module with `cfg(target_os)` gates
2. Abstract: binary resolution, data directory, output directory
3. Replace all remaining `dirs::*` calls
4. Verify desktop build still works with no regressions

**Success Criteria:**
1. `platform.rs` provides platform-gated binary resolution, data dir, output dir
2. All `dirs` crate calls replaced with Tauri `app.path()` for Android
3. Desktop build (`cargo tauri dev`) compiles and runs correctly
4. Android build continues to boot after refactor

**Depends on:** Phase 9  
**Estimated effort:** 0.5-1 day  
**Risk:** LOW -- standard Rust conditional compilation

---

### Phase 11: Binary Execution Engine (HIGH RISK)

**Goal:** Custom Kotlin Tauri plugin wraps youtubedl-android for YouTube search and MP3 download.

**Requirements:** DL-01, DL-02, DL-03

**What gets done:**
1. Add youtubedl-android as Gradle dependency
2. Set `extractNativeLibs="true"` in AndroidManifest.xml
3. Create Kotlin Tauri plugin with `@TauriPlugin` and `@Command` annotations
4. Implement YouTube search (returns JSON: video IDs, titles, durations, channels)
5. Implement audio download + transcode to 320 kbps MP3
6. Bridge results back to Rust via `invoke.resolve()`
7. All operations on `Dispatchers.IO` (prevent ANR)

**Success Criteria:**
1. Kotlin plugin initializes youtubedl-android on app startup without crash/ANR
2. YouTube search from Rust returns scored candidates (same scoring algorithm)
3. Audio download produces 320 kbps MP3 on device storage
4. All operations run on background threads (no ANR within 30 seconds)

**Depends on:** Phase 10  
**Estimated effort:** 3-5 days  
**Risk:** HIGH

**Why high risk:**
1. **No existing reference.** Nobody has published a Tauri Kotlin plugin calling youtubedl-android. We are the first integration of these two systems.
2. **Three integration boundaries.** Rust <-> Tauri Plugin API <-> Kotlin <-> youtubedl-android. Each boundary is a potential failure point.
3. **Android binary execution is hostile.** W^X enforcement (API 29+), SELinux policies, app sandbox. youtubedl-android handles this internally, but whether it works inside Tauri's Gradle build is unproven.

**Mitigation strategy:** Start with minimal proof-of-concept (single video search + download) before building full integration. If this phase fails, the entire milestone is blocked.

---

### Phase 12: Download Pipeline Integration

**Goal:** End-to-end: paste Spotify URL, get tagged MP3 with cover art on Android.

**Requirements:** DL-04, DL-05, DL-06

**What gets done:**
1. Wire Spotify resolve -> YouTube search (via Kotlin plugin) -> score -> download
2. Connect reusable Rust tagger (ID3v2.4 + cover art) to downloaded MP3s
3. Run SHA-256 verification post-tag
4. Wire queue manager for batch downloads (albums/playlists)
5. Test pause/resume/cancel on Android

**Success Criteria:**
1. Spotify track URL -> resolves, downloads, applies tags + cover art on device
2. Album/playlist URL -> batch download with per-track progress
3. MP3 files pass SHA-256 verification
4. Queue pause/resume/cancel works

**Depends on:** Phase 11  
**Estimated effort:** 2-3 days  
**Risk:** MEDIUM (integration testing, most code shared)

---

### Phase 13: Mobile UI

**Goal:** Angular frontend redesigned for mobile with bottom navigation and responsive pages.

**Requirements:** UI-01 through UI-06

**What gets done:**
1. Replace sidebar with bottom navigation bar (4 tabs)
2. Responsive URL input page (full-width, stacked preview cards)
3. Downloads page (compact card list, touch-friendly pause/resume/cancel, 48dp targets)
4. Library page (single-column list, grouped by album/playlist)
5. Settings page (full-width stacked form, bitrate, credentials)
6. Onboarding flow (full-screen, Spotify Client ID/Secret entry)

**Success Criteria:**
1. Bottom nav with Input, Downloads, Library, Settings tabs
2. URL input accepts paste, resolves, shows preview on mobile viewport
3. Downloads page: batch progress, per-track status, 48dp touch targets
4. Library: tracks grouped by album/playlist
5. Settings + onboarding flow for credentials

**Depends on:** Phase 12  
**Estimated effort:** 2-3 days  
**Risk:** LOW (standard responsive design)

---

### Phase 14: Storage, Background Service & Release

**Goal:** Downloads persist to shared Music directory, survive backgrounding, signed APK ready.

**Requirements:** STOR-01 through STOR-04, BUILD-06

**What gets done:**
1. Kotlin MediaStore plugin: write MP3s to Music/Spytfy/ directory
2. Verify files visible in Samsung Music, Poweramp, etc.
3. Kotlin Foreground Service plugin: TYPE_DATA_SYNC, notification with progress
4. Download survival when app backgrounded or screen locked
5. ABI splits in Gradle (ARM64-only APK for size reduction)
6. APK signing with release keystore
7. Version management

**Success Criteria:**
1. MP3s in Music/ via MediaStore, playable in other music apps
2. Downloads continue when app backgrounded or screen locked
3. Notification shows "Downloading 3/12 tracks" style progress
4. Signed APK installs without security warnings on API 31+

**Depends on:** Phase 13  
**Estimated effort:** 2-3 days  
**Risk:** MEDIUM (foreground service complexity, MediaStore edge cases)

---

## 8. Risk Analysis

### Risk Matrix

| Risk | Probability | Impact | Phase | Mitigation |
|------|-------------|--------|-------|------------|
| youtubedl-android + Tauri integration fails | Medium | **Critical** (blocks entire milestone) | 11 | Fail-fast positioning; proof-of-concept first; fallback to direct jniLibs binary execution |
| SQLite fails on x86_64 emulator | Medium | Low (workaround exists) | 9 | Use NDK r26+; test on ARM64 emulator or physical device |
| ANR from blocking Kotlin plugin | High | Medium | 11 | Mandatory `Dispatchers.IO` for all plugin operations |
| Scoped storage confusion | Medium | Medium | 14 | Use app-specific dir for temp; MediaStore for final output |
| Large APK size (~80 MB) | Low | Low | 14 | ABI splits; ARM64-only APK reduces by ~40% |
| Android kills background downloads | High | Medium | 14 | Foreground service with TYPE_DATA_SYNC; battery optimization guidance |
| Desktop regression from platform abstraction | Low | Medium | 10 | Run desktop build after every change; cfg gates isolate changes |
| WebView CSS/JS incompatibility | Low | Low | 13 | minSdk 31 guarantees Chrome 96+ |

### Critical Path

```
Phase 9 (Scaffold) -> Phase 10 (Abstraction) -> Phase 11 (Binary Engine) -> Phase 12 (Pipeline) -> Phase 13 (UI) -> Phase 14 (Release)
```

**Bottleneck:** Phase 11. If the Kotlin plugin + youtubedl-android integration fails, everything downstream is blocked. No workaround exists that doesn't require significant re-architecture.

**Fallback plan for Phase 11 failure:**
1. Try direct `tokio::process::Command` with `nativeLibraryDir` paths (bypasses Kotlin, but may hit W^X on some OEMs)
2. If that fails, try manually packaging ARM yt-dlp + ffmpeg as `lib*.so` in jniLibs and executing directly
3. If all binary execution fails on Android, pivot to a server-assisted architecture (out of current scope)

---

## 9. Known Pitfalls & Mitigations

### Critical (blocks progress)

| # | Pitfall | What Happens | Prevention | Phase |
|---|---------|-------------|------------|-------|
| 1 | Shell plugin sidecar on Android | `app.shell().sidecar("yt-dlp")` silently fails | Don't use shell plugin; build Kotlin plugin from day 1 | 11 |
| 2 | `dirs` crate returns None on Android | App crashes on startup, database fails to initialize | Replace with `app.path().app_data_dir()` | 9 |
| 3 | SQLite x86_64 emulator linker errors | Cannot build for emulator | Use NDK r26+; test on ARM64 emulator or physical device | 9 |
| 4 | Scoped storage blocks file writes | `FileNotFoundException` writing to /Music/ | Use MediaStore API (no permissions on API 31+) | 14 |

### Moderate (causes delays)

| # | Pitfall | What Happens | Prevention | Phase |
|---|---------|-------------|------------|-------|
| 5 | IPC payload size limits | Large playlist resolution fails silently on Android | Use Channel API for 100+ track playlists | 12 |
| 6 | ANR from blocking Kotlin plugin | Android kills app after 5 seconds | All operations on `Dispatchers.IO` coroutines | 11 |
| 7 | Binary naming convention | ARM binaries not extracted from APK | Must use `lib` prefix + `.so` suffix; `extractNativeLibs="true"` | 11 |
| 8 | Missing `cdylib` crate type | App crashes with `UnsatisfiedLinkError` | Add `cdylib` to crate-type before android init | 9 |
| 9 | Environment variables not persisting | `cargo tauri android init` fails | Use `[System.Environment]::SetEnvironmentVariable()` with "User" scope | 9 |

### Minor (quality/polish)

| # | Pitfall | What Happens | Prevention | Phase |
|---|---------|-------------|------------|-------|
| 10 | Dev server URL on physical device | Blank screen on physical device | Use emulator for initial dev; check firewall for port 4200 | 9 |
| 11 | Large APK size (~80 MB) | Distribution concerns | ABI splits in Gradle; ARM64-only build | 14 |
| 12 | Concurrent downloads drain battery | User complaints | Default concurrency to 1 on Android | 12 |
| 13 | WebView CSS differences | Visual glitches on some devices | minSdk 31 guarantees Chrome 96+; test on multiple API levels | 13 |

---

## 10. Alternatives Considered

| Decision | Chosen | Alternative | Why Not Alternative |
|----------|--------|-------------|-------------------|
| Download engine | youtubedl-android (Kotlin lib) | Manual ARM yt-dlp sidecar binaries | Tauri shell plugin doesn't support process spawning on Android; manual packaging is error-prone |
| Download engine | youtubedl-android | Rust-native (rusty_ytdl + symphonia) | rusty_ytdl: 47K downloads, last release Aug 2024; symphonia lacks Opus decoder; too risky for v2.0 |
| Storage | MediaStore API | MANAGE_EXTERNAL_STORAGE | Google Play rejects for most app categories |
| Storage | MediaStore API | SAF file picker | Poor UX -- user must pick directory every time |
| Storage | MediaStore API | App-specific directory only | Files invisible to other music apps |
| Background work | Foreground Service | WorkManager | WorkManager is for deferrable work; downloads should happen immediately |
| TLS | rustls-tls (existing) | OpenSSL / native-tls | OpenSSL requires complex NDK linking; rustls is pure Rust |
| Platform abstraction | `cfg(target_os)` modules | Trait-based dispatch | cfg modules simpler, no runtime overhead, matches Tauri's own pattern |
| Min SDK | API 31 (Android 12) | API 24 (Android 7) | Scoped storage, Material You, Chrome 96+ WebView guaranteed; 65%+ market |
| Auth flow | Manual Client ID/Secret | PKCE OAuth | Manual creds proven, simpler, consistent with desktop; PKCE for v3 |

---

## 11. Open Questions

These need answers during execution, not before:

| # | Question | When to Answer | Impact |
|---|---------|----------------|--------|
| 1 | Does youtubedl-android work inside Tauri's Gradle build? | Phase 11 (day 1) | Critical -- blocks milestone if no |
| 2 | Can `tokio::process::Command` execute from `nativeLibraryDir` on Android? | Phase 11 (fallback) | High -- alternative to Kotlin bridge |
| 3 | Does MediaStore copy preserve ID3 tags? | Phase 14 | Medium -- may need tag after MediaStore write |
| 4 | What is actual APK size with youtubedl-android? | Phase 11 (after build) | Low -- informational for distribution planning |
| 5 | youtubedl-android thread safety for concurrent downloads? | Phase 12 | Medium -- affects worker pool design |
| 6 | Android 14+ foreground service 6-hour limit for large playlists? | Phase 14 | Medium -- may need checkpoint/resume for 500+ tracks |
| 7 | Does `rspotify` dependency chain avoid `openssl-sys` on Android? | Phase 9 | Medium -- may need feature flag adjustment |
| 8 | x86_64 emulator SQLite linking with NDK r26+? | Phase 9 | Low -- ARM device is fallback |

---

## 12. Confidence Assessment

| Area | Confidence | Evidence |
|------|-----------|---------|
| Tauri 2 Android build pipeline | HIGH | Official documentation, stable since Tauri 2.0 release |
| Rust cross-compilation to Android | HIGH | Standard Rust target, well-documented |
| Existing Rust code reusability | HIGH | All reusable crates are pure Rust (no platform-specific deps) |
| SQLite on Android | HIGH | SQLx bundled feature documented; known issues have workarounds |
| IPC (Angular <-> Rust) | HIGH | Same API on desktop and mobile; transparent transport switch |
| youtubedl-android library | HIGH | 1800+ stars, active maintenance, proven on Android |
| Kotlin Tauri plugin bridge | MEDIUM | Pattern documented by Tauri, but wrapping youtubedl-android is novel |
| MediaStore integration | HIGH | Official Android API, well-documented, no permissions on API 31+ |
| Foreground service | MEDIUM | Requires custom plugin; Android 14+ restrictions add complexity |
| Overall integration | **MEDIUM** | Individual components proven; their combination is uncharted territory |

**Confidence bottleneck:** Phase 11 (Binary Execution Engine). No existing Tauri + youtubedl-android reference implementation. This is the single point where the plan could require significant revision.

---

## 13. Sources & References

### Primary (HIGH confidence)

| Source | URL | Used For |
|--------|-----|----------|
| Tauri 2 Prerequisites | https://v2.tauri.app/start/prerequisites/ | Android setup, env vars |
| Tauri Shell Plugin | https://v2.tauri.app/plugin/shell/ | Confirmed Android limitation |
| Tauri Mobile Plugin Dev | https://v2.tauri.app/develop/plugins/develop-mobile/ | Kotlin plugin architecture |
| Tauri Sidecar Docs | https://v2.tauri.app/develop/sidecar/ | Confirmed desktop-only |
| Tauri IPC Concepts | https://v2.tauri.app/concept/inter-process-communication/ | Android IPC behavior |
| Tauri Android Config | https://v2.tauri.app/reference/config/ | minSdkVersion, bundle config |
| Tauri Android Signing | https://v2.tauri.app/distribute/sign/android/ | APK signing |
| Tauri FS Plugin | https://v2.tauri.app/plugin/file-system/ | Android file access |
| youtubedl-android | https://github.com/yausername/youtubedl-android | Download engine library |
| Android MediaStore | https://developer.android.com/training/data-storage/shared/media | Music directory access |
| Android Foreground Services | https://developer.android.com/about/versions/14/changes/fgs-types-required | Background download |
| Android Scoped Storage | https://developer.android.com/training/data-storage | Storage architecture |
| Android Share Intent | https://developer.android.com/training/sharing/receive | Future: share intent |

### Secondary (MEDIUM confidence)

| Source | URL | Used For |
|--------|-----|----------|
| Tauri Sidecar Android Issue | https://github.com/tauri-apps/tauri/issues/9774 | Sidecar limitation confirmation |
| Tauri SQLite Issue #6047 | https://github.com/tauri-apps/tauri/issues/6047 | x86_64 emulator workaround |
| Tauri FS on Android (blog) | https://philrich.dev/tauri-fs-android/ | Practical file management tips |
| DeepWiki Tauri Mobile | https://deepwiki.com/tauri-apps/tauri/8.1-mobile-architecture-overview | Architecture overview |
| Android Binary Execution | https://www.androidbugfix.com/2022/01/android-can-execute-process-for-android.html | W^X enforcement details |

---

## Review Checklist for Expert

Please validate:

- [ ] **Architecture:** Is the Tauri 2 + youtubedl-android + Kotlin plugin bridge approach sound? Are there better alternatives we missed?
- [ ] **Phase ordering:** Does the fail-fast positioning of Phase 11 (binary execution) make sense? Should anything be reordered?
- [ ] **Risk assessment:** Are the HIGH risk items correctly identified? Are there hidden risks we haven't considered?
- [ ] **Scope:** Is the v2.0 scope realistic for 12-15 working days? Is anything missing that's truly table stakes?
- [ ] **youtubedl-android:** Is this the right library? Are there better Android yt-dlp wrappers?
- [ ] **Storage strategy:** MediaStore for Music/ visibility -- correct approach for API 31+?
- [ ] **Foreground service:** TYPE_DATA_SYNC correct for download use case? Android 14+ implications?
- [ ] **Code reuse estimate:** Is 80% realistic? Any hidden platform dependencies we missed?
- [ ] **Fallback plan:** If Phase 11 fails, are the fallback options viable?
- [ ] **Distribution:** Direct APK vs F-Droid -- right call for v2.0?

---

*Document generated: 2026-05-22*  
*Branch: `android`*  
*Repository: https://github.com/mohameddsh3ban/spytfy*
