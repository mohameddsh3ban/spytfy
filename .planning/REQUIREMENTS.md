# Requirements: Spytfy

**Defined:** 2026-05-22
**Core Value:** Paste a Spotify link on your phone, get organized MP3 files -- no server, no cloud, fully offline.

## v2.0 Requirements

Requirements for Android mobile release. Each maps to roadmap phases.

### Build & Platform

- [x] **BUILD-01**: App builds as Android APK targeting API 29+ via Tauri 2 Android
- [x] **BUILD-02**: Rust backend cross-compiles to ARM64 with cdylib crate-type
- [x] **BUILD-03**: Platform abstraction layer (cfg target_os) keeps desktop builds green
- [x] **BUILD-04**: All `dirs` crate calls replaced with Tauri `app.path()` APIs on Android
- [x] **BUILD-05**: SQLite database works on Android (bundled feature, correct path)
- [ ] **BUILD-06**: Release APK is signed and versioned for distribution
- [x] **BUILD-07**: `minSdkVersion = 29` in tauri.conf.json (targets ~92% of Android devices)

### Download Engine

- [x] **DL-01**: Custom Kotlin Tauri plugin bridges youtubedl-android library to Rust
- [x] **DL-02**: YouTube search returns scored candidates on Android (same scoring as desktop)
- [x] **DL-03**: Audio downloads as 320 kbps MP3 via youtubedl-android ffmpeg
- [x] **DL-04**: ID3v2.4 tags + cover art embedded on downloaded MP3s (reuse desktop tagger)
- [x] **DL-05**: Post-download SHA-256 verification (reuse desktop verifier)
- [x] **DL-06**: Full pipeline: Spotify resolve -> YT search -> score -> download -> tag -> verify
- [x] **DL-07**: yt-dlp binary updatable at runtime via YoutubeDL.updateYoutubeDL() on app launch

### Storage & Background

- [x] **STOR-01**: Downloaded MP3s written to Music/ directory via MediaStore API
- [x] **STOR-02**: MP3s visible in other Android music apps (Samsung Music, Poweramp, etc.)
- [ ] **STOR-03**: Foreground service or UIDT keeps downloads alive when app is backgrounded
- [ ] **STOR-04**: Notification shows download progress with track count
- [ ] **STOR-05**: Downloads on API 34+ use UIDT JobScheduler; API 29-33 use foreground service
- [ ] **STOR-06**: MediaStore writes use IS_PENDING flow preserving ID3 tags end-to-end

### Mobile UI

- [ ] **UI-01**: Bottom navigation replacing desktop sidebar (Input, Downloads, Library, Settings)
- [ ] **UI-02**: Mobile-adapted URL input page with paste + resolve + preview
- [ ] **UI-03**: Downloads page with batch progress, per-track status, pause/resume/cancel
- [ ] **UI-04**: Library page showing downloaded tracks grouped by album/playlist
- [ ] **UI-05**: Settings page (output quality, Spotify credentials)
- [ ] **UI-06**: Onboarding page for Spotify Client ID/Secret entry

## v3.0 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Enhanced Mobile

- **MOBILE-01**: Android share intent (receive Spotify URLs from share sheet)
- **MOBILE-02**: Material You dynamic theming
- **MOBILE-03**: OCR screenshot import for mobile
- **MOBILE-04**: In-app music playback preview

### Cross-Platform

- **XPLAT-01**: iOS build via Tauri 2
- **XPLAT-02**: Rust-native YouTube extraction (replace youtubedl-android)
- **XPLAT-03**: PKCE OAuth flow (no client secret needed)

## Out of Scope

| Feature | Reason |
|---------|--------|
| iOS build | Android first, iOS in v3+ |
| Cloud sync | Core value is offline-first, no backend |
| Music playback/streaming | Download tool, not a music player |
| Google Play distribution | Play Store may reject; direct APK/F-Droid instead |
| MANAGE_EXTERNAL_STORAGE | Play Store rejection risk; use MediaStore instead |
| Rust-native yt-dlp replacement | crates not mature enough (rusty_ytdl, symphonia lack Opus) |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BUILD-01 | Phase 9 | ✅ Complete |
| BUILD-02 | Phase 9 | ✅ Complete |
| BUILD-03 | Phase 10 | ✅ Complete |
| BUILD-04 | Phase 9 | ✅ Complete |
| BUILD-05 | Phase 9 | ✅ Complete |
| BUILD-06 | Phase 14 | Pending |
| BUILD-07 | Phase 9 | ✅ Complete |
| DL-01 | Phase 11 | ✅ Complete |
| DL-02 | Phase 11 | ✅ Complete |
| DL-03 | Phase 11 | ✅ Complete |
| DL-04 | Phase 12 | ✅ Complete |
| DL-05 | Phase 12 | ✅ Complete |
| DL-06 | Phase 12 | ✅ Complete |
| DL-07 | Phase 11 | ✅ Complete |
| STOR-01 | Phase 12 | ✅ Complete (early) |
| STOR-02 | Phase 12 | ✅ Complete (early) |
| STOR-03 | Phase 14 | Pending |
| STOR-04 | Phase 14 | Pending |
| STOR-05 | Phase 14 | Pending |
| STOR-06 | Phase 14 | Pending |
| UI-01 | Phase 13 | Pending |
| UI-02 | Phase 13 | Pending |
| UI-03 | Phase 13 | Pending |
| UI-04 | Phase 13 | Pending |
| UI-05 | Phase 13 | Pending |
| UI-06 | Phase 13 | Pending |

**Coverage:**
- v2.0 requirements: 26 total
- Complete: 17
- Pending: 9
- Unmapped: 0

---
*Requirements defined: 2026-05-22*
*Last updated: 2026-05-25 after Phase 12 completion*
