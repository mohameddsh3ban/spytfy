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

- [x] **Phase 9: Android Scaffold** - Tauri 2 Android build pipeline boots on S24 Ultra with SQLite working
- [x] **Phase 10: Platform Abstraction** - cfg(target_os) layer keeps desktop and Android builds green
- [x] **Phase 11: Binary Execution Engine** - Kotlin plugin bridges youtubedl-android for search and download (spike PASSED 8/8)
- [x] **Phase 12: Download Pipeline Integration** - End-to-end Spotify URL to tagged MP3 on Android with MediaStore visibility
- [ ] **Phase 13: Mobile UI** - Bottom navigation, responsive pages, touch-friendly mobile experience
- [ ] **Phase 14: Background Service & Release** - Foreground service, signed APK, distribution

## Phase Details

### Phase 9: Android Scaffold -- COMPLETE (2026-05-22)
**Goal**: A Tauri 2 Android app boots on a physical device with the existing Angular UI visible in WebView and SQLite database initializing correctly
**Requirements**: BUILD-01 ✅, BUILD-02 ✅, BUILD-04 ✅, BUILD-05 ✅, BUILD-07 ✅
**Verified on**: Samsung Galaxy S24 Ultra (SM-S928B, API 36, Android 16)

### Phase 10: Platform Abstraction -- COMPLETE (2026-05-23)
**Goal**: Shared codebase compiles for both desktop and Android without platform-specific code leaking
**Requirements**: BUILD-03 ✅
**Key artifact**: `src-tauri/src/platform.rs`

### Phase 11: Binary Execution Engine -- COMPLETE (2026-05-24)
**Goal**: Custom Kotlin Tauri plugin wraps youtubedl-android for YouTube search and MP3 download
**Requirements**: DL-01 ✅, DL-02 ✅, DL-03 ✅, DL-07 ✅
**Key artifact**: `src-tauri/tauri-plugin-spytfy-download/`
**Spike result**: 8/8 PASS -- search 5 candidates in 4s, download 8.13MB MP3 in 28.5s

### Phase 12: Download Pipeline Integration -- COMPLETE (2026-05-25)
**Goal**: End-to-end download pipeline on Android with MediaStore visibility
**Requirements**: DL-04 ✅, DL-05 ✅, DL-06 ✅, STOR-01 ✅ (early), STOR-02 ✅ (early)
**Key artifact**: `src-tauri/src/download/android.rs`
**Verified**: Tracks download with ID3 tags + cover art, appear in Samsung Music

### Phase 13: Mobile UI -- NEXT
**Goal**: The Angular frontend is redesigned for mobile viewports with bottom navigation, touch-friendly controls, and all core pages adapted
**Depends on**: Phase 12 ✅
**Requirements**: UI-01, UI-02, UI-03, UI-04, UI-05, UI-06
**Success Criteria** (what must be TRUE):
  1. Bottom navigation bar replaces the desktop sidebar with tabs for Input, Downloads, Library, and Settings
  2. URL input page accepts paste, resolves Spotify URL, and shows track/album/playlist preview on mobile viewport
  3. Downloads page shows batch progress with per-track status, and pause/resume/cancel controls are touch-accessible (48dp minimum targets)
  4. Library page displays downloaded tracks grouped by album or playlist
  5. Settings page allows configuring output quality and Spotify credentials, and onboarding flow guides first-time credential entry
**Plans**: TBD
**UI hint**: yes

### Phase 14: Background Service & Release -- PENDING
**Goal**: Downloads survive app backgrounding, signed APK ready for distribution
**Depends on**: Phase 13
**Requirements**: STOR-03, STOR-04, STOR-05, STOR-06, BUILD-06
**Note**: STOR-01 and STOR-02 delivered early in Phase 12
**Success Criteria** (what must be TRUE):
  1. Downloads continue when the user switches to another app or locks the screen (foreground service or UIDT keeps pipeline alive)
  2. A notification shows download progress with current track count
  3. A signed, versioned release APK installs on Android 10+ devices without security warnings
  4. Keystore backed up to password manager + 2 physical locations

## Progress

| Phase | Milestone | Status | Completed |
|-------|-----------|--------|-----------|
| 1-8 | v1.0 | ✅ Complete | Shipped |
| 9. Android Scaffold | v2.0 | ✅ Complete | 2026-05-22 |
| 10. Platform Abstraction | v2.0 | ✅ Complete | 2026-05-23 |
| 11. Binary Execution Engine | v2.0 | ✅ Complete | 2026-05-24 |
| 12. Download Pipeline Integration | v2.0 | ✅ Complete | 2026-05-25 |
| 13. Mobile UI | v2.0 | ○ Next | - |
| 14. Background Service & Release | v2.0 | ○ Pending | - |
