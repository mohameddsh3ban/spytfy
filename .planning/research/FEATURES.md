# Feature Landscape: Android Port

**Domain:** Mobile music downloader (Android port of desktop app)
**Researched:** 2026-05-22

## Table Stakes

Features users expect from the Android port. Missing = product feels broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Paste Spotify URL and download | Core value prop, desktop parity | High | Requires full download pipeline working on Android |
| Track/album/playlist resolution | Core feature from desktop | Low | Reusable Rust code |
| MP3 output with ID3 tags + cover art | Core feature from desktop | Low | Reusable Rust code |
| Queue with batch download | Desktop parity | Low | Reusable queue manager |
| Download progress indication | Mobile users expect visual feedback | Medium | Events work cross-platform, need mobile UI |
| Pause/resume/cancel downloads | Desktop parity | Low | Reusable queue commands |
| Library view of downloaded tracks | Users need to find their downloads | Medium | Need mobile-optimized list view |
| Settings (output dir, bitrate) | Desktop parity | Medium | Simplified for mobile |
| Spotify credential setup | Required for any functionality | Low | Same onboarding flow |
| Offline operation (no server) | Core architectural constraint | Low | Already offline-first |

## Differentiators

Features that set the mobile version apart. Valued but not strictly expected.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Share intent (receive Spotify URLs from other apps) | One-tap from Spotify app to download | Medium | Android intent filter, very natural UX |
| Background download service | Downloads continue when app is backgrounded | High | Android foreground service with notification |
| Download notifications with progress | System-level progress visibility | Medium | Android NotificationManager integration |
| Retry with candidate picker | Disambiguation when wrong match | Low | Already exists in desktop, just needs mobile UI |
| yt-dlp runtime updates | Keep download engine current without app update | Low | youtubedl-android supports this natively |
| Battery-aware download throttling | Reduce battery drain during large batches | Medium | Monitor battery state, throttle concurrency |

## Anti-Features

Features to explicitly NOT build for Android v2.0.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| OCR screenshot import | Complex on mobile (camera permissions, ML models, heavy deps), low ROI | Defer to v3.0 if demanded |
| Music playback | Not our value prop, many better players exist | Suggest opening in default music app |
| Cloud sync | Violates offline-first constraint, adds server dependency | Not in scope |
| iOS support | Split focus, Android first | Separate milestone after Android stable |
| Google Play distribution (v2.0) | Store policies may conflict with download functionality | Distribute APK directly, consider Play later |
| PKCE OAuth | Over-engineering for v2.0, manual creds work fine | Consider for v3.0 |
| Streaming downloads | Adds complexity, not core to batch download use case | Keep batch-only |
| Folder customization on mobile | Scoped storage makes this complex | Use fixed app-specific directory |

## Feature Dependencies

```
Android Build Pipeline -> Everything
  |
  +-> Binary Executor Plugin -> Download Pipeline
  |     |
  |     +-> YouTube Search (needs yt-dlp execution)
  |     +-> MP3 Download (needs yt-dlp + ffmpeg execution)
  |
  +-> Platform Path Abstraction -> Database Init, Output Dir
  |     |
  |     +-> SQLite Database (needs correct Android path)
  |     +-> File Output (needs scoped storage path)
  |
  +-> Mobile UI Layout -> All Pages
        |
        +-> Bottom Nav replaces Sidebar
        +-> Input Page (responsive)
        +-> Downloads Page (mobile list)
        +-> Library Page (mobile grid/list)
        +-> Settings Page (mobile form)
```

## MVP Recommendation

Prioritize for Android v2.0:

1. **Android build pipeline** -- nothing works without this
2. **Binary executor plugin** -- enables all downloads
3. **Platform path abstraction** -- enables database and file output
4. **URL input + download pipeline** -- core value
5. **Download queue with progress** -- batch downloads are the main workflow
6. **Mobile-adapted UI** -- bottom nav, responsive pages
7. **Library view** -- users need to find their downloads
8. **Settings** -- bitrate, basic config

Defer to v2.1+:
- **Share intent:** Adds significant value but not blocking MVP
- **Background download service:** Complex Android service, can work foreground-only first
- **Download notifications:** Enhancement, not blocking
- **yt-dlp runtime updates:** youtubedl-android supports it, but not needed day 1

## Sources

- [Android Share Intent](https://developer.android.com/training/sharing/receive) -- HIGH confidence
- [Android Foreground Services](https://developer.android.com/develop/background-work/services/foreground-services) -- HIGH confidence
- Project analysis of existing desktop features -- HIGH confidence
