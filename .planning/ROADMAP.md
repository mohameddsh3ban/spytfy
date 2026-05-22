# Roadmap: Spytfy

## Milestones

- ✅ **v1.0 Desktop Release** - Phases 1-8 (shipped)
- 🚧 **v2.0 Android Mobile App** - Phases 9-14 (in progress)

## Phases

<details>
<summary>v1.0 Desktop Release (Phases 1-8) - SHIPPED</summary>

- [x] **Phase 1: Scaffold** - Tauri 2 + Angular project structure
- [x] **Phase 2: Spotify Resolve** - URL parsing and track/album/playlist resolution
- [x] **Phase 3: Single-Track Download** - yt-dlp search, download, transcode, ID3 tagging
- [x] **Phase 4: Queue & Concurrency** - Batch queue with concurrent workers
- [x] **Phase 4B: Screenshot Import** - OCR-based playlist/album import
- [x] **Phase 4C: Spotify Thumbnail Embedding** - Per-track cover art in ID3 tags
- [x] **Phase 5: Folders & Naming** - Organized output directories and naming templates
- [x] **Phase 6: Failure Recovery** - Retry, disambiguation UI, pause/cancel
- [x] **Phase 7: Polish** - Empty states, motion, error toasts, keyboard shortcuts
- [x] **Phase 8: Packaging** - Windows installer, signing, release workflow

</details>

### v2.0 Android Mobile App

**Milestone Goal:** Port Spytfy to Android as a native APK with on-device download pipeline -- no server, fully offline. Paste a Spotify link on your phone, get organized MP3 files.

**Phase Numbering:**
- Integer phases (9, 10, 11...): Planned milestone work
- Decimal phases (10.1, 10.2): Urgent insertions (marked with INSERTED)

- [ ] **Phase 9: Android Scaffold** - Tauri 2 Android build pipeline boots on device with SQLite working
- [ ] **Phase 10: Platform Abstraction** - cfg(target_os) layer keeps desktop and Android builds green
- [ ] **Phase 11: Binary Execution Engine** - Kotlin plugin bridges youtubedl-android for search and download
- [ ] **Phase 12: Download Pipeline Integration** - End-to-end Spotify URL to tagged MP3 on Android
- [ ] **Phase 13: Mobile UI** - Bottom navigation, responsive pages, touch-friendly mobile experience
- [ ] **Phase 14: Storage, Background Service & Release** - MediaStore, foreground service, signed APK

## Phase Details

### Phase 9: Android Scaffold
**Goal**: A Tauri 2 Android app boots on a physical device or emulator with the existing Angular UI visible in WebView and SQLite database initializing correctly
**Depends on**: Nothing (first phase of v2.0 milestone; v1.0 desktop is complete)
**Requirements**: BUILD-01, BUILD-02, BUILD-04, BUILD-05
**Success Criteria** (what must be TRUE):
  1. Running `cargo tauri android dev` produces an APK that installs and launches on an Android 12+ device or emulator
  2. The existing Angular UI renders in the Android WebView (pages load, navigation works)
  3. SQLite database initializes on Android without crash (migrations run, tables created)
  4. Rust backend cross-compiles to ARM64 with cdylib crate-type and loads via JNI without UnsatisfiedLinkError
**Plans**: TBD

### Phase 10: Platform Abstraction
**Goal**: Shared codebase compiles for both desktop (Windows) and Android targets without platform-specific code leaking across boundaries
**Depends on**: Phase 9
**Requirements**: BUILD-03
**Success Criteria** (what must be TRUE):
  1. A `platform.rs` module with `cfg(target_os)` gates provides data directory, output directory, and binary resolution for each platform
  2. All `dirs` crate calls are replaced with Tauri `app.path()` APIs for Android paths
  3. Desktop build (`cargo tauri dev`) still compiles and runs correctly with no regressions
  4. Android build continues to boot after platform abstraction refactor
**Plans**: TBD

### Phase 11: Binary Execution Engine
**Goal**: A custom Kotlin Tauri plugin wraps youtubedl-android to execute YouTube search and MP3 download on Android
**Depends on**: Phase 10
**Requirements**: DL-01, DL-02, DL-03
**Success Criteria** (what must be TRUE):
  1. A Kotlin Tauri plugin initializes youtubedl-android on app startup without crash or ANR
  2. YouTube search invoked from Rust returns scored candidates with video ID, title, duration, and channel (same scoring algorithm as desktop)
  3. Audio download from a YouTube video ID produces a 320 kbps MP3 file on Android device storage
  4. All youtubedl-android operations run on background threads (no ANR dialog within 30 seconds of operation start)
**Plans**: TBD

### Phase 12: Download Pipeline Integration
**Goal**: The complete download pipeline works end-to-end on Android -- paste a Spotify URL, get a tagged MP3 with cover art
**Depends on**: Phase 11
**Requirements**: DL-04, DL-05, DL-06
**Success Criteria** (what must be TRUE):
  1. Pasting a Spotify track URL resolves the track, searches YouTube, downloads MP3, and applies ID3v2.4 tags with real album cover art -- all on device
  2. Pasting a Spotify album or playlist URL resolves all tracks and downloads them as a batch with per-track progress
  3. Downloaded MP3 files pass SHA-256 verification (file integrity confirmed post-download)
  4. Queue pause, resume, and cancel work during batch downloads on Android
**Plans**: TBD

### Phase 13: Mobile UI
**Goal**: The Angular frontend is redesigned for mobile viewports with bottom navigation, touch-friendly controls, and all core pages adapted
**Depends on**: Phase 12
**Requirements**: UI-01, UI-02, UI-03, UI-04, UI-05, UI-06
**Success Criteria** (what must be TRUE):
  1. Bottom navigation bar replaces the desktop sidebar with tabs for Input, Downloads, Library, and Settings
  2. URL input page accepts paste, resolves Spotify URL, and shows track/album/playlist preview on mobile viewport
  3. Downloads page shows batch progress with per-track status, and pause/resume/cancel controls are touch-accessible (48dp minimum targets)
  4. Library page displays downloaded tracks grouped by album or playlist
  5. Settings page allows configuring output quality and Spotify credentials, and onboarding flow guides first-time credential entry
**Plans**: TBD
**UI hint**: yes

### Phase 14: Storage, Background Service & Release
**Goal**: Downloads persist to the shared Music directory visible in other apps, survive app backgrounding, and the APK is signed for distribution
**Depends on**: Phase 13
**Requirements**: STOR-01, STOR-02, STOR-03, STOR-04, BUILD-06
**Success Criteria** (what must be TRUE):
  1. Downloaded MP3s appear in the device Music directory via MediaStore and are playable in Samsung Music, Poweramp, or other Android music apps
  2. Downloads continue when the user switches to another app or locks the screen (foreground service keeps pipeline alive)
  3. A notification shows download progress with current track count (e.g., "Downloading 3/12 tracks")
  4. A signed, versioned release APK installs on Android 12+ devices without security warnings from known-source installation
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 9 -> 10 -> 11 -> 12 -> 13 -> 14
Decimal phases (if inserted) execute between their surrounding integers.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Scaffold | v1.0 | - | Complete | - |
| 2. Spotify Resolve | v1.0 | - | Complete | - |
| 3. Single-Track Download | v1.0 | - | Complete | - |
| 4. Queue & Concurrency | v1.0 | - | Complete | - |
| 4B. Screenshot Import | v1.0 | - | Complete | - |
| 4C. Thumbnail Embedding | v1.0 | - | Complete | - |
| 5. Folders & Naming | v1.0 | - | Complete | - |
| 6. Failure Recovery | v1.0 | - | Complete | - |
| 7. Polish | v1.0 | - | Complete | - |
| 8. Packaging | v1.0 | - | Complete | - |
| 9. Android Scaffold | v2.0 | 0/? | Not started | - |
| 10. Platform Abstraction | v2.0 | 0/? | Not started | - |
| 11. Binary Execution Engine | v2.0 | 0/? | Not started | - |
| 12. Download Pipeline Integration | v2.0 | 0/? | Not started | - |
| 13. Mobile UI | v2.0 | 0/? | Not started | - |
| 14. Storage, Background Service & Release | v2.0 | 0/? | Not started | - |
