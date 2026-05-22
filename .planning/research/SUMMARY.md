# Project Research Summary

**Project:** Spytfy v2.0 Android
**Domain:** Cross-platform Tauri 2 desktop-to-Android port (music downloader)
**Researched:** 2026-05-22
**Confidence:** MEDIUM

## Executive Summary

Spytfy's Android port is a Tauri 2 cross-platform migration where roughly 80% of the existing Rust backend (Spotify resolution, scoring, ID3 tagging, cover art, queue management, SQLite persistence) can be reused without modification. The critical challenge is replacing the desktop download execution layer: Tauri's shell plugin does NOT support process spawning on Android, so the entire sidecar/subprocess approach that powers yt-dlp and ffmpeg on desktop is dead on arrival. The recommended solution is integrating `youtubedl-android` (a battle-tested Kotlin library with 1800+ GitHub stars) as a Gradle dependency, bridged to Rust through custom Tauri Kotlin plugins. This preserves behavioral parity with the desktop app -- same yt-dlp engine, same search results, same audio quality -- at the cost of ~50-80MB APK size and a Kotlin bridge layer.

Three custom Kotlin plugins are needed: (1) a download engine plugin wrapping youtubedl-android for YouTube search and MP3 download, (2) a MediaStore plugin for writing finished MP3s to the shared Music directory without requiring storage permissions on API 31+, and (3) a foreground service plugin to keep downloads alive when the app is backgrounded. The Android-specific Rust code is minimal -- a `platform.rs` module with `cfg(target_os = "android")` gates that redirect binary resolution, data directory paths, and output directory paths away from the `dirs` crate (which returns None on Android) to Tauri's `app.path()` API. The frontend needs a responsive layout swap (sidebar to bottom navigation) but no IPC changes -- Tauri's invoke/event system works identically on Android.

The biggest risk is the lack of existing Tauri + youtubedl-android integration precedent. No one has published a working example of a Tauri Kotlin plugin calling youtubedl-android, so the Phase 3 (binary execution) integration will require hands-on experimentation. Secondary risks include SQLite compilation issues on x86_64 emulator targets (use NDK r26+ and test on ARM devices), scoped storage confusion (use app-specific external storage for v2.0, MediaStore for v2.1+), and ANR crashes from blocking the main thread in Kotlin plugins (all subprocess work must run on `Dispatchers.IO`).

## Key Findings

### Recommended Stack

The stack preserves the existing Tauri 2 + Angular + Rust architecture and extends it with Android-specific components. All pure-Rust dependencies (rspotify, reqwest with rustls-tls, id3, image, tokio, serde, SQLx) cross-compile to Android without issues. The only new dependencies are on the Android/Kotlin side.

**Core technologies:**
- **Tauri 2 (Android target):** App framework -- already used for desktop, Android build pipeline is stable and documented
- **youtubedl-android 0.18.1:** Download engine -- bundles Python 3.8 + yt-dlp + ffmpeg as native libs, handles W^X/SELinux internally
- **Custom Kotlin Tauri plugins (3x):** Platform bridge -- required because shell plugin is desktop-only; plugins use `@TauriPlugin` / `@Command` annotations
- **MediaStore API:** Storage -- write MP3s to shared Music directory without permissions on API 31+
- **Android Foreground Service (dataSync type):** Background execution -- keeps downloads alive when app is backgrounded
- **SQLx with `bundled` feature:** Database -- bundled SQLite compilation avoids NDK dynamic linking issues
- **API 31+ (Android 12) minimum:** Target platform -- scoped storage built-in, Chrome 96+ WebView guaranteed, 65%+ market coverage

**Critical version requirements:**
- NDK r26+ (fixes x86_64 128-bit float intrinsic linker errors for SQLite)
- `extractNativeLibs="true"` in AndroidManifest.xml (required for youtubedl-android binary extraction)
- `crate-type = ["staticlib", "cdylib", "rlib"]` in Cargo.toml (cdylib required for Android .so JNI export)

### Expected Features

**Must have (table stakes):**
- Paste Spotify URL and download MP3 with ID3 tags + cover art (core value proposition)
- Track/album/playlist resolution (reusable Rust code)
- Download queue with batch processing and progress indication
- Pause/resume/cancel downloads
- Library view of downloaded tracks
- Settings (bitrate configuration)
- Spotify credential setup (onboarding flow)

**Should have (differentiators):**
- Share intent (receive Spotify URLs from Spotify app -- natural mobile UX)
- Background download service with notification progress
- yt-dlp runtime updates (youtubedl-android supports this natively)
- Retry with candidate picker (desktop feature, needs mobile UI)

**Defer to v2.1+:**
- OCR screenshot import (complex on mobile, camera permissions, ML models)
- Music playback (not our value prop)
- Google Play distribution (store policies may conflict with download functionality)
- PKCE OAuth (manual creds work fine for v2.0)
- Battery-aware download throttling (enhancement)
- Folder customization (scoped storage makes this complex)

### Architecture Approach

The architecture follows a platform abstraction pattern using `cfg(target_os = "android")` conditional compilation -- the same pattern Tauri itself uses in its plugins (desktop.rs / mobile.rs). Rust compiles as a shared library (.so) on Android loaded via JNI, rather than a standalone binary. The Kotlin plugin layer sits between Rust and Android APIs, handling youtubedl-android calls, MediaStore writes, and foreground service lifecycle. IPC between Angular and Rust is identical to desktop (invoke/events via postMessage on Android WebView).

**Major components:**
1. **Platform abstraction module** (`platform.rs`) -- `cfg`-gated binary resolution, data dir, and output dir for desktop vs Android
2. **Download engine Kotlin plugin** -- wraps youtubedl-android for YouTube search + MP3 download + ffmpeg transcode
3. **MediaStore Kotlin plugin** -- writes finished MP3s from app temp dir to shared Music/Spytfy/ directory
4. **Foreground service Kotlin plugin** -- manages Android foreground service lifecycle with download progress notification
5. **Mobile layout components** -- bottom navigation bar replacing sidebar, responsive page layouts
6. **Shared Rust core** (unchanged) -- Spotify resolver, scorer, ID3 tagger, cover art fetcher, queue manager, SQLite persistence

### Critical Pitfalls

1. **Shell plugin sidecar is desktop-only** -- Do not attempt `app.shell().sidecar()` on Android. It silently fails. Use youtubedl-android via Kotlin plugin from day 1.
2. **`dirs` crate returns None on Android** -- Replace ALL `dirs::data_dir()` / `dirs::audio_dir()` calls with `app.path().app_data_dir()` before any Android testing. App will crash on startup otherwise.
3. **Missing `cdylib` crate type** -- Cargo.toml must include `cdylib` in crate-type or the Android app crashes with `UnsatisfiedLinkError` on launch.
4. **Scoped storage blocks arbitrary file writes** -- Cannot write to `/sdcard/Music/` on Android 12+. Use app-specific external storage for v2.0, MediaStore for music app visibility later.
5. **ANR from blocking Kotlin plugin** -- All youtubedl-android calls (search, download) must run on `Dispatchers.IO` coroutines. Blocking the main thread for >5 seconds triggers Android's kill dialog.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Android Scaffold and Build Pipeline
**Rationale:** Nothing else works without a compiling, booting Android app. Environment setup has known pitfalls (env vars, NDK version, crate-type) that must be resolved first.
**Delivers:** A Tauri Android app that boots on a device/emulator with the existing Angular UI visible in WebView, SQLite database initializing correctly.
**Addresses:** Android build pipeline (table stakes prerequisite), Spotify credential setup (onboarding works if SQLite works)
**Avoids:** Pitfall 2 (dirs crate), Pitfall 3 (SQLite compilation), Pitfall 8 (missing cdylib), Pitfall 9 (env vars)

### Phase 2: Platform Abstraction Layer
**Rationale:** Before building Android-specific features, the shared codebase needs clean platform boundaries so desktop builds are not broken. This is a refactoring phase with no new features.
**Delivers:** `platform.rs` module with `cfg(target_os)` gates for binary resolution, data directory, and output directory. All `dirs::*` calls replaced. Desktop build still passes all tests.
**Addresses:** Cross-platform code organization (architecture foundation)
**Avoids:** Pitfall 2 (dirs crate on Android), anti-pattern of hardcoded Windows paths

### Phase 3: Binary Execution Engine (Kotlin Plugin)
**Rationale:** This is the highest-risk phase and the core technical challenge. The Tauri + youtubedl-android integration is unproven. Must be tackled early so blockers surface before significant UI work.
**Delivers:** A working Kotlin Tauri plugin that can: (1) initialize youtubedl-android, (2) execute YouTube search and return JSON results to Rust, (3) download and transcode an MP3 from a YouTube video ID.
**Addresses:** Binary executor plugin (table stakes prerequisite for all downloads)
**Avoids:** Pitfall 1 (shell plugin), Pitfall 6 (ANR from blocking), Pitfall 7 (binary naming)
**Risk:** HIGH -- no existing reference implementation. Budget extra time for experimentation.

### Phase 4: Download Pipeline Integration
**Rationale:** With the binary execution engine working, reconnect it to the existing Rust download pipeline (Spotify resolution -> YouTube matching/scoring -> download -> tag -> verify). Most logic is shared; only the execution call site changes.
**Delivers:** End-to-end download: paste Spotify URL, resolve track, search YouTube, download MP3, apply ID3 tags + cover art, save to app storage.
**Addresses:** Core download workflow (primary table stakes), queue with batch download, pause/resume/cancel
**Avoids:** Pitfall 4 (scoped storage -- use app-specific dir), Pitfall 5 (IPC payload size for large playlists)

### Phase 5: Mobile UI Adaptation
**Rationale:** With the backend working, adapt the frontend for mobile. This is lower risk (standard responsive design) and can be done in parallel with Phase 4 polish.
**Delivers:** Bottom navigation replacing sidebar, responsive input/downloads/library/settings pages, touch-friendly controls (48dp minimum targets).
**Addresses:** Mobile-adapted UI, library view, settings page
**Avoids:** Pitfall 13 (WebView compatibility -- minSdk 31 guarantees Chrome 96+)

### Phase 6: Background Service and Polish
**Rationale:** Background download service is a differentiator, not a blocker. The app works foreground-only without it. Ship this as the final phase before release.
**Delivers:** Foreground service with notification progress, download survival when app is backgrounded, MediaStore integration for music app visibility, APK size optimization (ABI splits), share intent for receiving URLs from Spotify app.
**Addresses:** Background download service (differentiator), download notifications (differentiator), share intent (differentiator)
**Avoids:** Pitfall 11 (APK size -- add ABI splits), Pitfall 12 (battery drain -- default concurrency to 1)

### Phase Ordering Rationale

- **Scaffold first** because every other phase depends on a booting Android app with working SQLite.
- **Platform abstraction second** because the refactoring is low-risk and prevents desktop regressions during later Android-specific work.
- **Binary execution third** because it is the highest-uncertainty task. If youtubedl-android integration hits a wall, we need to know before investing in UI work. This is the "fail fast" phase.
- **Download pipeline fourth** because it reconnects the proven Rust core to the new Android execution engine. Most code is shared, so this phase is primarily integration testing.
- **Mobile UI fifth** because it is independent of backend uncertainty and follows well-documented responsive design patterns.
- **Background service last** because it is a differentiator that adds complexity (foreground service lifecycle, notification management) without being required for a functional MVP.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Binary Execution):** No existing Tauri + youtubedl-android reference. Needs hands-on prototyping to validate the Kotlin plugin -> youtubedl-android -> Rust result flow. Research the exact JNI call pattern and async result passing.
- **Phase 6 (Background Service):** Android foreground service types have changed significantly in Android 14+. Research `dataSync` type restrictions and notification channel requirements.

Phases with standard patterns (skip additional research):
- **Phase 1 (Scaffold):** Tauri Android init is well-documented. Follow official prerequisites guide.
- **Phase 2 (Platform Abstraction):** Standard `cfg(target_os)` pattern used throughout the Rust ecosystem.
- **Phase 5 (Mobile UI):** Standard responsive design. Bottom navigation is a solved pattern in Angular.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All technologies verified against official docs. youtubedl-android is battle-tested (1800+ stars, active maintenance). Pure Rust crates confirmed cross-compilable. |
| Features | HIGH | Feature list derived from existing desktop app analysis. Table stakes vs differentiators clearly separable. Anti-features well-justified. |
| Architecture | MEDIUM | Tauri 2 Android build pipeline is stable, but the Kotlin plugin -> youtubedl-android bridge is novel. IPC and conditional compilation patterns are well-documented. |
| Pitfalls | MEDIUM-HIGH | Critical pitfalls (shell plugin, dirs crate, scoped storage) verified against official docs. ANR and binary naming confirmed by Android documentation. x86_64 SQLite issue sourced from GitHub issue tracker. |

**Overall confidence:** MEDIUM -- The individual components are well-understood, but their integration (specifically Tauri Kotlin plugin calling youtubedl-android and returning results to Rust) is unproven territory. Phase 3 is the confidence bottleneck.

### Gaps to Address

- **Tauri Kotlin plugin async pattern:** Research confirms Kotlin plugins use `@Command` annotation, but the exact pattern for long-running async operations (download taking 30+ seconds) with progress events back to Rust needs prototyping. Validate during Phase 3 planning.
- **youtubedl-android thread safety:** Unclear if youtubedl-android supports concurrent downloads from multiple threads. Desktop uses 3 workers; Android should default to 1, but verify the library's concurrency model.
- **MediaStore write pattern from Rust/Kotlin:** The two-step pattern (write to temp dir in Rust, copy to Music via MediaStore in Kotlin) needs validation. Confirm that ID3 tags survive the MediaStore copy operation.
- **APK signing and distribution:** Research covers build but not distribution. For v2.0, direct APK distribution is planned (no Play Store), but signing configuration and update mechanism need attention during Phase 6.
- **Android emulator vs physical device testing:** SQLite x86_64 issue affects emulators specifically. Establish whether ARM64 emulator or physical device is the primary test target.

## Sources

### Primary (HIGH confidence)
- Tauri 2 Prerequisites: https://v2.tauri.app/start/prerequisites/
- Tauri Shell Plugin (mobile limitations): https://v2.tauri.app/plugin/shell/
- Tauri Mobile Plugin Development: https://v2.tauri.app/develop/plugins/develop-mobile/
- Tauri Sidecar Docs: https://v2.tauri.app/develop/sidecar/
- Tauri IPC Concepts: https://v2.tauri.app/concept/inter-process-communication/
- Tauri Android Config: https://v2.tauri.app/reference/config/
- Tauri Android Code Signing: https://v2.tauri.app/distribute/sign/android/
- Tauri File System Plugin: https://v2.tauri.app/plugin/file-system/
- youtubedl-android: https://github.com/yausername/youtubedl-android
- youtubedl-android Maven: https://mvnrepository.com/artifact/io.github.junkfood02.youtubedl-android
- Android MediaStore: https://developer.android.com/training/data-storage/shared/media
- Android Foreground Services: https://developer.android.com/about/versions/14/changes/fgs-types-required
- Android Scoped Storage: https://developer.android.com/training/data-storage
- Android Share Intent: https://developer.android.com/training/sharing/receive

### Secondary (MEDIUM confidence)
- Tauri Sidecar Android bug: https://github.com/tauri-apps/tauri/issues/9774
- Tauri Android SQLite Issue 6047: https://github.com/tauri-apps/tauri/issues/6047
- DeepWiki Tauri Mobile Architecture: https://deepwiki.com/tauri-apps/tauri/8.1-mobile-architecture-overview
- Tauri File Management on Android: https://philrich.dev/tauri-fs-android/
- Execute Native Binaries Android Q+: https://www.androidbugfix.com/2022/01/android-can-execute-process-for-android.html
- rusty_ytdl (future reference): https://github.com/Mithronn/rusty_ytdl
- Symphonia (future reference): https://github.com/pdeljanov/Symphonia

### Tertiary (LOW confidence)
- Termux W^X analysis: https://github.com/termux/termux-app/issues/2155 -- context on Android execution restrictions, not directly applicable

---
*Research completed: 2026-05-22*
*Ready for roadmap: yes*
