# Spytfy Technical Documentation

Complete technical deep-dive covering every module, every command, every data flow.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Project Structure](#2-project-structure)
3. [Application Bootstrap](#3-application-bootstrap)
4. [Rust Backend — Module by Module](#4-rust-backend--module-by-module)
   - 4.1 [Entry Point (lib.rs / main.rs)](#41-entry-point)
   - 4.2 [Platform Abstraction (platform.rs)](#42-platform-abstraction)
   - 4.3 [Database (db.rs)](#43-database)
   - 4.4 [Spotify Module](#44-spotify-module)
   - 4.5 [Download Module](#45-download-module)
   - 4.6 [Queue Module](#46-queue-module)
   - 4.7 [OCR Module (Desktop Only)](#47-ocr-module)
   - 4.8 [Settings Commands](#48-settings-commands)
5. [Kotlin Plugin — Android Download Engine](#5-kotlin-plugin--android-download-engine)
6. [Angular Frontend — Page by Page](#6-angular-frontend--page-by-page)
   - 6.1 [App Shell & Routing](#61-app-shell--routing)
   - 6.2 [Onboarding Page](#62-onboarding-page)
   - 6.3 [Input Page](#63-input-page)
   - 6.4 [Downloads Page](#64-downloads-page)
   - 6.5 [Library Page](#65-library-page)
   - 6.6 [Settings Page](#66-settings-page)
7. [Shared Libraries](#7-shared-libraries)
   - 7.1 [Models (@spytfy/models)](#71-models)
   - 7.2 [IPC Layer (@spytfy/tauri-ipc)](#72-ipc-layer)
   - 7.3 [UI Components (@spytfy/ui)](#73-ui-components)
8. [IPC Protocol — Full Command & Event Reference](#8-ipc-protocol)
9. [Database Schema](#9-database-schema)
10. [Data Flows — End to End](#10-data-flows)
11. [Scoring Algorithm](#11-scoring-algorithm)
12. [Error Handling & Recovery](#12-error-handling--recovery)
13. [Platform Differences (Desktop vs Android)](#13-platform-differences)
14. [Build System & Configuration](#14-build-system--configuration)
15. [Dependencies](#15-dependencies)

---

## 1. Architecture Overview

Spytfy is a Spotify music downloader built with:

- **Frontend:** Angular 20 (standalone components, signals) running in a WebView
- **Backend:** Rust with Tokio async runtime
- **Framework:** Tauri 2 (bridges frontend ↔ backend via IPC)
- **Database:** SQLite with sqlx ORM
- **Platforms:** Windows/macOS/Linux (desktop) and Android (mobile)

```
┌──────────────────────────────────────────────────────────────┐
│                    Angular Frontend                          │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌────────┐          │
│  │  Input   │ │Downloads │ │ Library │ │Settings│          │
│  │  Page    │ │  Page    │ │  Page   │ │  Page  │          │
│  └────┬─────┘ └────┬─────┘ └────┬────┘ └───┬────┘          │
│       │             │            │           │               │
│  ┌────┴─────────────┴────────────┴───────────┴────┐         │
│  │           @spytfy/tauri-ipc                     │         │
│  │  (typed wrappers around Tauri invoke + listen)  │         │
│  └────────────────────┬───────────────────────────┘         │
└───────────────────────┼─────────────────────────────────────┘
                        │ Tauri IPC (JSON serialization)
┌───────────────────────┼─────────────────────────────────────┐
│                  Rust Backend                                │
│  ┌────────────────────┴───────────────────────────┐         │
│  │            Tauri Command Router                 │         │
│  │         (31 commands, cfg-gated)                │         │
│  └──┬──────┬──────┬──────┬──────┬────────────────┘         │
│     │      │      │      │      │                           │
│  ┌──┴──┐┌──┴──┐┌──┴──┐┌──┴──┐┌──┴──┐                      │
│  │Spot-││Down-││Queue││OCR  ││Sett-│                       │
│  │ify  ││load ││     ││(win)││ings │                       │
│  └──┬──┘└──┬──┘└──┬──┘└─────┘└──┬──┘                      │
│     │      │      │              │                           │
│  ┌──┴──────┴──────┴──────────────┴──┐                       │
│  │           SQLite (sqlx)           │                       │
│  └──────────────────────────────────┘                       │
│                                                              │
│  ┌─────────────────────────────────┐                        │
│  │  Platform Layer (cfg gates)     │                        │
│  │  Desktop: yt-dlp + ffmpeg       │                        │
│  │  Android: Kotlin plugin (JNI)   │                        │
│  └─────────────────────────────────┘                        │
└──────────────────────────────────────────────────────────────┘
```

The app is fully offline and self-contained. No server component. Everything runs on-device.

---

## 2. Project Structure

```
spytfy/
├── apps/
│   └── desktop/                    # Angular 20 desktop app
│       └── src/
│           ├── main.ts             # Bootstrap
│           └── app/
│               ├── app.component.ts
│               ├── app.config.ts
│               ├── app.routes.ts
│               ├── guards/         # Route guards (auth)
│               ├── layout/         # Sidebar, toast
│               └── pages/          # Onboarding, input, downloads, library, settings
├── libs/
│   ├── models/src/                 # TypeScript data models
│   │   ├── spotify.model.ts        # SpotifyTrack, SpotifyAlbum, SpotifyPlaylist, ResolvedInput
│   │   ├── job.model.ts            # DownloadJob, Batch, JobState
│   │   └── settings.model.ts       # Settings, SettingsPatch
│   ├── tauri-ipc/src/              # Typed Tauri invoke wrappers
│   │   ├── spotify.ipc.ts          # resolveUrl, auth commands
│   │   ├── download.ipc.ts         # downloadTrack, events
│   │   ├── queue.ipc.ts            # enqueue, batch/job management, events
│   │   ├── ocr.ipc.ts              # OCR commands (desktop only)
│   │   └── settings.ipc.ts         # getSettings, updateSettings
│   └── ui/src/                     # Shared UI components
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                 # Windows entry point
│   │   ├── lib.rs                  # App setup, plugin init, command registration
│   │   ├── platform.rs             # cfg(target_os) platform abstraction
│   │   ├── db.rs                   # SQLite pool initialization
│   │   ├── spotify/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs             # Credential storage & Spotify client init
│   │   │   ├── types.rs            # SpotifyTrack, SpotifyAlbum, etc.
│   │   │   ├── parser.rs           # URL parsing (regex)
│   │   │   ├── resolver.rs         # API resolution (rspotify)
│   │   │   └── scraper.rs          # HTML scraping fallback
│   │   ├── download/
│   │   │   ├── mod.rs
│   │   │   ├── pipeline.rs         # Main download_track command
│   │   │   ├── youtube.rs          # yt-dlp search wrapper
│   │   │   ├── scorer.rs           # YouTube candidate scoring
│   │   │   ├── downloader.rs       # yt-dlp download + retry
│   │   │   ├── cover.rs            # Cover art fetch + cache
│   │   │   ├── tagger.rs           # ID3v2.4 tag writing
│   │   │   ├── verifier.rs         # Post-download integrity check
│   │   │   └── android.rs          # Android bridge (cfg-gated)
│   │   ├── queue/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs          # QueueManager state machine
│   │   │   ├── worker.rs           # WorkerPool (concurrent job processor)
│   │   │   └── commands.rs         # Tauri command handlers
│   │   ├── ocr/                    # Desktop-only OCR module
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs           # Tesseract + Windows OCR
│   │   │   ├── parser.rs           # Screenshot text → tracks
│   │   │   ├── html_parser.rs      # Spotify HTML → tracks
│   │   │   ├── browser.rs          # Web scraping
│   │   │   └── commands.rs         # Tauri command handlers
│   │   └── commands/
│   │       └── settings.rs         # Settings CRUD commands
│   ├── migrations/                 # SQLite migration files
│   ├── binaries/                   # yt-dlp, ffmpeg sidecars (desktop)
│   ├── tauri-plugin-spytfy-download/  # Custom Tauri plugin
│   │   ├── src/lib.rs              # Plugin registration (Rust side)
│   │   ├── Cargo.toml
│   │   └── android/
│   │       └── src/main/java/app/tauri/spytfy_download/
│   │           └── DownloadPlugin.kt  # Kotlin plugin (youtubedl-android)
│   ├── Cargo.toml                  # Rust dependencies
│   ├── tauri.conf.json             # Desktop Tauri config
│   └── tauri.android.conf.json     # Android Tauri config overrides
├── package.json                    # Node dependencies
├── nx.json                         # NX monorepo config
└── tsconfig.base.json              # TypeScript path aliases
```

---

## 3. Application Bootstrap

### What happens when the app starts:

**Step 1 — Rust backend initializes (lib.rs)**

```
main.rs → run() in lib.rs
  │
  ├─ Register plugins: dialog, shell, store, fs, spytfy-download
  │
  ├─ Setup callback (runs after Tauri is ready):
  │   ├─ Init SQLite pool → db.rs::init_pool(app_data_dir)
  │   │   └─ Runs all pending migrations (creates tables if first run)
  │   │
  │   ├─ Create QueueManager (wraps pool + mpsc channel, capacity 50)
  │   │
  │   ├─ Spawn WorkerPool (2 concurrent workers, reads from channel)
  │   │
  │   ├─ Merge duplicate batches (dedup by name+type from prior crash)
  │   │
  │   ├─ Reset stuck jobs (pending/resolving/downloading → queued)
  │   │
  │   ├─ Push all queued jobs to channel (resume interrupted downloads)
  │   │
  │   └─ Init Spotify client from stored credentials
  │
  └─ Register command handlers (31 commands, cfg-gated for platform)
```

**Step 2 — Angular frontend loads in WebView**

```
main.ts → bootstrapApplication(AppComponent, appConfig)
  │
  ├─ appConfig provides: router, guards, services
  │
  ├─ AppComponent renders: sidebar + <router-outlet> + toasts
  │
  └─ Router checks spotifyAuthGuard:
      ├─ Has credentials? → Navigate to /input
      └─ No credentials? → Redirect to /onboarding
```

**Step 3 — On Android, additionally:**

```
Kotlin DownloadPlugin.load(webView):
  │
  ├─ YoutubeDL.getInstance().init(application)   # Extract bundled yt-dlp
  ├─ FFmpeg.getInstance().init(application)       # Extract bundled ffmpeg
  └─ YoutubeDL.getInstance().updateYoutubeDL()    # Check for yt-dlp updates
```

---

## 4. Rust Backend — Module by Module

### 4.1 Entry Point

**`main.rs`** — Windows-specific entry (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`). Calls `run()`.

**`lib.rs`** — The real entry point. Does everything described in Section 3, Step 1.

Key design: Two separate `generate_handler!` blocks gated by `#[cfg(target_os)]`. Desktop gets all 31 commands. Android gets 25 (OCR commands excluded: `process_screenshots`, `debug_ocr`, `create_playlist_from_tracks`, `parse_text_tracklist`, `parse_spotify_html`, `scrape_playlist_tracks`).

State managed via Tauri's `app.manage()`:
- `SqlitePool` — database connection pool
- `QueueManager` — download queue state machine
- `WorkerPool` — concurrent download workers
- `Arc<RwLock<Option<ClientCredsSpotify>>>` — Spotify API client

### 4.2 Platform Abstraction

**`platform.rs`** — Three functions, each with `#[cfg(target_os)]` gates:

| Function | Desktop | Android |
|----------|---------|---------|
| `data_dir(app)` | `app.path().app_data_dir()` fallback to `dirs::data_dir()` | `app.path().app_data_dir()` fallback to `current_dir()` |
| `default_output_dir(app)` | `dirs::audio_dir()` (e.g., `C:\Users\X\Music`) | `{app_data}/Music` |
| `default_concurrency()` | `3` | `1` |

### 4.3 Database

**`db.rs`** — Creates SQLite connection pool at `{app_data_dir}/.spytfy/db.sqlite`. Runs sqlx migrations on startup. Max 5 connections.

Migration files create the schema described in [Section 9](#9-database-schema).

### 4.4 Spotify Module

#### 4.4.1 Authentication (`spotify/auth.rs`)

Manages Spotify API credentials (Client ID + Client Secret). Uses `rspotify` with Client Credentials flow (no user login needed — only public API access).

**State:** `Arc<RwLock<Option<ClientCredsSpotify>>>` — shared across all async tasks.

**Commands:**
- `save_spotify_credentials(client_id, client_secret)` — Stores in `tauri-plugin-store` (credentials.json), creates new `ClientCredsSpotify`, acquires token
- `test_spotify_credentials()` — Attempts token acquisition, returns error if invalid
- `has_spotify_credentials()` — Checks if credentials.json exists and has values

**Startup:** `init_from_store()` loads credentials from store, creates client if found. Falls back to hardcoded default credentials if none stored.

#### 4.4.2 Types (`spotify/types.rs`)

Core domain models shared between Rust modules:

```rust
SpotifyTrack {
    id: String,              // Spotify track ID
    name: String,            // Track title
    artists: Vec<String>,    // Artist names
    album: String,           // Album name
    album_id: String,        // For cover art caching
    track_number: u32,
    disc_number: u32,
    duration_ms: u64,        // Track length in milliseconds
    isrc: String,            // International Standard Recording Code
    cover_url: Option<String>,
    release_date: Option<String>,
}

SpotifyAlbum {
    id: String,
    name: String,
    artists: Vec<String>,
    tracks: Vec<SpotifyTrack>,
    cover_url: Option<String>,
    release_date: String,
}

SpotifyPlaylist {
    id: String,
    name: String,
    owner: String,
    tracks: Vec<SpotifyTrack>,
    cover_url: Option<String>,
}

ResolvedInput — enum:
    Track(SpotifyTrack)
    Album(SpotifyAlbum)
    Playlist(SpotifyPlaylist)

SpotifyUrl {
    kind: SpotifyUrlKind,    // Track | Album | Playlist
    id: String,              // Spotify ID extracted from URL
}
```

#### 4.4.3 URL Parser (`spotify/parser.rs`)

`parse_spotify_url(input)` — Accepts two formats:
- Web URL: `https://open.spotify.com/track|album|playlist/{id}?si=...`
- URI: `spotify:track:{id}`

Uses regex to extract the type and ID. Strips query parameters.

#### 4.4.4 Resolver (`spotify/resolver.rs`)

`resolve_url(url)` — Main entry command. Flow:

```
parse_spotify_url(input)
  │
  ├─ Track → resolve_track(id)
  │   └─ rspotify::track(id) → SpotifyTrack
  │
  ├─ Album → resolve_album(id)
  │   └─ rspotify::album(id) + album_track_stream() → SpotifyAlbum
  │
  └─ Playlist → resolve_playlist(id)
      └─ rspotify::playlist(id) + playlist_items_stream() → SpotifyPlaylist

If API fails → fallback to scraper::resolve_url_scraping(url)
```

Album and playlist resolution use paginated streams (rspotify's `_stream()` methods) to fetch all tracks regardless of count.

#### 4.4.5 Scraper (`spotify/scraper.rs`)

Fallback when Spotify API fails (no credentials, rate limited, etc.).

`resolve_url_scraping(url)` — Fetches the Spotify web page with reqwest, parses:
- `__NEXT_DATA__` script tag (JSON embedded in page)
- `og:title`, `og:description`, `og:image` meta tags
- Duration from ISO 8601 (`PT3M42S`), `M:SS`, or `X min Y sec` text

Returns a `ResolvedInput` with whatever metadata it can extract.

`debug_scrape(url)` — Returns raw extraction details for debugging.
`resolve_from_json(json_str)` — Creates a `SpotifyTrack` from user-pasted JSON.

### 4.5 Download Module

The download pipeline is the core of the app. It converts a `SpotifyTrack` into a tagged MP3 file.

#### 4.5.1 Pipeline (`download/pipeline.rs`)

`download_track(track)` — The main download command. Orchestrates the full pipeline:

```
Step 1: SEARCH YOUTUBE
  ├─ Desktop: youtube::search_youtube(yt_dlp, artist, title)
  │   └─ Spawns yt-dlp process: ytsearch5:"{artist} {title}"
  └─ Android: android::search_youtube_android(app, artist, title)
      └─ Calls Kotlin plugin via run_mobile_plugin("searchYoutube")

Step 2: SCORE CANDIDATES
  └─ scorer::score_candidates(artist, title, duration, candidates)
      └─ Returns best match (or first candidate as fallback)

Step 3: DOWNLOAD MP3
  ├─ Desktop: downloader::download_mp3(yt_dlp, url, path, bitrate)
  │   └─ Spawns yt-dlp process: -x --audio-format mp3
  └─ Android: android::download_audio_android(app, video_id, path, bitrate)
      └─ Calls Kotlin plugin via run_mobile_plugin("downloadAudio")

Step 4: TAG MP3 (shared, both platforms)
  ├─ cover::fetch_cover(cache_dir, album_id, cover_url)
  │   └─ Downloads, validates, resizes, caches cover art
  ├─ tagger::tag_mp3(path, track, cover)
  │   └─ Writes ID3v2.4 tags (title, artist, album, track#, cover, ISRC)
  └─ verifier::verify_mp3(path, cover_hash)
      └─ Validates embedded cover art matches expected hash
```

Emits `download:state` events at each stage so the frontend can show progress.

Helper: `resolve_sidecar_path(app, name)` — Finds bundled binaries (yt-dlp, ffmpeg) in dev mode or production. Checks `./binaries/`, `CARGO_MANIFEST_DIR/binaries/`, and Tauri resource directory.

Helper: `load_settings_for_pipeline(pool)` — Reads settings from DB with platform-aware defaults.

#### 4.5.2 YouTube Search (`download/youtube.rs`)

`search_youtube(yt_dlp_path, artist, title)` — Desktop only.

Spawns yt-dlp as a child process:
```
yt-dlp "ytsearch5:{artist} {title}"
    --print "%(id)s|%(title)s|%(duration)s|%(uploader)s|%(channel_id)s"
    --no-download --no-warnings --flat-playlist
```

Parses pipe-delimited output into `Vec<YtCandidate>`:
```rust
YtCandidate {
    id: String,           // YouTube video ID (e.g., "dQw4w9WgXcQ")
    title: String,        // Video title
    duration_secs: u64,   // Video length
    uploader: String,     // Channel name
    channel_id: String,   // YouTube channel ID
}
```

#### 4.5.3 Scoring (`download/scorer.rs`)

`score_candidates(artist, title, duration_ms, candidates)` — Picks the best YouTube match for a Spotify track.

**Algorithm:**

```
Base score: 100 points

Duration penalty:
  delta = |candidate_duration - track_duration|
  penalty = delta_seconds * 2
  score -= penalty

Title similarity (Levenshtein):
  similarity = 1.0 - (edit_distance / max_length)
  penalty = (1.0 - similarity) * 50
  score -= penalty

Bonuses (added to score):
  +15  title contains "official audio"
  +20  uploader contains "topic" or "vevo"
  +25  uploader matches track artist (Levenshtein > 0.8)

Penalties (subtracted from score):
  -25  title contains "live"
  -40  title contains "cover"
  -30  title contains "remix"
  -50  title contains "sped up", "slowed", "8d", "nightcore"
  -30  duration > track_duration * 1.5 (oversized)

Selection threshold:
  - Duration known: score >= 40 AND duration within ±10 seconds
  - Duration unknown: score >= 30
  - Returns None if no candidate passes → triggers needs_review
```

`score_all_candidates()` — Returns all candidates with scores, used for manual review UI.

#### 4.5.4 Downloader (`download/downloader.rs`)

`download_mp3(app, yt_dlp, url, output_path, bitrate, job_id, batch_id)` — Desktop download.

```
Pre-download:
  - Clean stale .part and .webm files in output directory
  - Skip if output file exists and > 1000 bytes

yt-dlp invocation:
  yt-dlp "{url}"
    -x                          # Extract audio
    --audio-format mp3          # Convert to MP3
    --audio-quality {bitrate}K  # e.g., 320K
    -o "{output_path}"          # Output file path
    --no-playlist               # Single video only
    --retries 5                 # yt-dlp internal retries
    --no-mtime                  # Don't set file modification time

Retry logic:
  - 3 attempts with exponential backoff: 5s, 10s, 15s
  - Each attempt spawns a new yt-dlp process
  - Failure after 3 attempts → returns error string
```

`build_output_path(root, track, folder_name, template)` — Renders output path from template:

```
Template: "{folder}/{number} - {artist} - {title}"
Example:  "My Playlist/01 - Drake - God's Plan.mp3"

Placeholders:
  {folder}  → album or playlist name
  {artist}  → first artist
  {title}   → track title
  {album}   → album name
  {number}  → zero-padded track number (01, 02, ...)

Sanitization: removes / \ : * ? " < > | from filenames
```

#### 4.5.5 Cover Art (`download/cover.rs`)

`fetch_cover(cache_dir, cache_key, cover_url)` — Downloads and processes album art.

```
Cache check:
  {cache_dir}/{album_id}.jpg exists? → Return cached CoverResult

Download:
  reqwest::get(cover_url) → raw bytes

Validation:
  - Decode as JPEG or PNG
  - Must be >= 300x300 pixels

Processing:
  - Resize to <= 1000x1000 (preserving aspect ratio)
  - Crop to square (center crop)
  - Encode as JPEG quality 90

Cache:
  - Write to {cache_dir}/{album_id}.jpg
  - Compute SHA-256 hash of final bytes

Returns:
  CoverResult {
    bytes: Vec<u8>,     # JPEG bytes for embedding
    hash: String,       # SHA-256 for verification
  }
```

#### 4.5.6 Tagger (`download/tagger.rs`)

`tag_mp3(path, track, cover_result)` — Writes ID3v2.4 tags to MP3 file.

Tags written:
| Tag | Value |
|-----|-------|
| Title (TIT2) | `track.name` |
| Artist (TPE1) | `track.artists.join(", ")` |
| Album (TALB) | `track.album` |
| Track # (TRCK) | `track.track_number` |
| Disc # (TPOS) | `track.disc_number` |
| Year (TDRC) | First 4 chars of `release_date` |
| ISRC (TSRC) | `track.isrc` |
| Cover (APIC) | Embedded JPEG, type "Front cover" |
| Comment (COMM) | "Downloaded by Spytfy" |

#### 4.5.7 Verifier (`download/verifier.rs`)

`verify_mp3(path, expected_hash)` — Post-download integrity check.

Checks:
1. File exists and is readable
2. ID3 tags are parseable
3. Embedded APIC frame exists
4. Cover image >= 300x300 pixels
5. Cover SHA-256 hash matches `expected_hash`

Returns `VerifyResult::Ok` or `VerifyResult::Warning(message)`.

#### 4.5.8 Android Bridge (`download/android.rs`)

Rust-side bridge to the Kotlin plugin. Only compiled on `#[cfg(target_os = "android")]`.

`search_youtube_android(app, artist, title)`:
```rust
app.run_mobile_plugin("spytfy-download", "searchYoutube", SearchRequest {
    query: "{artist} {title}",
    max_results: 5,
})
// Returns Vec<YtCandidate> (same struct as desktop)
```

`download_audio_android(app, video_id, output_path, bitrate_kbps)`:
```rust
app.run_mobile_plugin("spytfy-download", "downloadAudio", DownloadRequest {
    video_id,
    output_path,
    bitrate_kbps,
})
// Returns file path string
```

Both use Tauri's `run_mobile_plugin()` which calls into Kotlin via JNI.

### 4.6 Queue Module

The queue module manages batch downloads. It's a state machine with persistent storage (SQLite) and concurrent workers.

#### 4.6.1 Queue Manager (`queue/manager.rs`)

`QueueManager` holds:
- `SqlitePool` — database connection
- `mpsc::Sender<(JobInfo, SpotifyTrack)>` — channel to workers (capacity 50)

**`enqueue_batch(input, source_url)`** — Creates a batch and its jobs:

```
1. Parse ResolvedInput → extract tracks[]
2. Normalize source URL (strip ?si= params)
3. Check for existing batch with same name + type:
   ├─ Found → merge jobs into existing batch (skip duplicates by title+artist)
   └─ Not found → create new batch row
4. For each track:
   ├─ Check if output file already exists (> 1000 bytes):
   │   ├─ Exists → insert job with state "done" (skip download)
   │   └─ Not exists → insert job with state "queued"
   └─ Insert into jobs table
5. Push queued jobs to channel (sends to workers)
6. Return batch_id
```

**`push_queued_jobs(batch_id)`** — Sends queued jobs to the worker channel:
```
1. Query all jobs where state = "queued" AND batch.state = "active"
2. For each job:
   ├─ Update state to "pending" (claimed by channel)
   ├─ Try send to channel:
   │   ├─ Success → job is now in worker pipeline
   │   └─ Channel full → revert state to "queued"
   └─ Emit job:state event
```

**Batch state machine:**
```
active ──pause──→ paused ──resume──→ active
  │                                     │
  └────cancel────→ cancelled            │
  │                                     │
  └──(all done)──→ complete             │
                                        │
  paused ──cancel──→ cancelled          │
```

**Job state machine:**
```
queued → pending → resolving → downloading → converting → tagging → verifying → done
                                                                              → done_warning
  Any state can → failed
  failed → queued (retry)
  downloading can → needs_review (all candidates failed)
  needs_review → queued (after pick_candidate)
```

#### 4.6.2 Worker Pool (`queue/worker.rs`)

`WorkerPool::spawn(rx, mgr, app, concurrency)` — Creates a Tokio supervisor task.

```
Supervisor loop:
  │
  ├─ Receive (JobInfo, SpotifyTrack) from channel
  │
  ├─ Check batch state:
  │   ├─ paused/cancelled → reset job to "queued", continue
  │   └─ active → proceed
  │
  ├─ Acquire semaphore permit (limits to {concurrency} parallel jobs)
  │
  ├─ Sleep 2 seconds (YouTube rate limiting between requests)
  │
  └─ Spawn async task → process_job()
```

**`process_job()`** — Full single-job pipeline:

```
1. Check for pre-selected yt_url (from pick_candidate):
   ├─ Has URL → skip search, download directly
   └─ No URL → proceed to search

2. Set state → "resolving"

3. Search YouTube:
   ├─ Desktop: youtube::search_youtube()
   └─ Android: android::search_youtube_android()

4. Score candidates with scorer::score_candidates()

5. Try top 3 candidates:
   For each candidate (with 3-second delay between):
   │
   ├─ Set state → "downloading"
   ├─ Download MP3:
   │   ├─ Desktop: downloader::download_mp3()
   │   └─ Android: android::download_audio_android()
   ├─ Success → break to step 6
   └─ Failure → try next candidate

   All failed → store candidates_json, set state → "needs_review", return

6. Fetch real cover art:
   ├─ If track has Spotify ID → re-fetch track metadata from API
   │   └─ Gets higher-quality cover URL than initial resolution
   └─ cover::fetch_cover(cache_dir, album_id, cover_url)

7. Set state → "tagging"
   └─ tagger::tag_mp3(path, track, cover)

8. Set state → "verifying"
   └─ verifier::verify_mp3(path, cover_hash)

9. Set final state:
   ├─ Verify OK → "done"
   ├─ Verify warning → "done_warning"
   └─ Any error → "failed" with error message

10. Emit events: job:state, job:cover
11. check_batch_complete() → maybe emit batch:complete
12. push_one_queued_job() → feed next job from queue
```

#### 4.6.3 Queue Commands (`queue/commands.rs`)

Thin wrappers exposing `QueueManager` methods as Tauri commands:

| Command | What it does |
|---------|-------------|
| `enqueue_download(input, url)` | Creates batch + jobs, starts processing |
| `list_batches(limit)` | Returns recent batches with metadata |
| `list_jobs(batch_id?)` | Returns jobs, optionally filtered by batch |
| `pause_batch(batch_id)` | Pauses batch (workers skip paused jobs) |
| `resume_batch(batch_id)` | Resumes batch, pushes queued jobs |
| `cancel_batch(batch_id)` | Marks queued jobs failed, batch cancelled |
| `retry_job(job_id)` | Resets single job to queued, re-sends |
| `retry_all_failed(batch_id)` | Retries all failed jobs in batch |
| `resume_queued()` | Resets stuck jobs + pushes all queued |
| `pick_candidate(job_id, url)` | User selects YouTube URL for needs_review job |
| `list_failed_jobs()` | Returns failed jobs with error messages |

### 4.7 OCR Module

**Desktop only.** Entire module excluded from Android build via `#[cfg(not(target_os = "android"))]`.

This module handles importing tracks from screenshots of Spotify playlists and from HTML copy-paste.

#### 4.7.1 OCR Engine (`ocr/engine.rs`)

`ocr_image(path)` — Extracts text from a Spotify screenshot:
1. Crop left 4% of image (removes album art thumbnails that confuse OCR)
2. Try Tesseract first: `tesseract.exe {image} stdout`
3. Fallback to Windows OCR: PowerShell `Windows.Media.Ocr.OcrEngine`
4. Returns raw text

#### 4.7.2 Screenshot Parser (`ocr/parser.rs`)

`parse_spotify_screenshot(text)` — Converts OCR text to structured tracks:
- Regex patterns for "N Title\nArtist\nAlbum" format
- Extracts duration (M:SS) from rightmost column
- Removes date strings, cleans whitespace
- Returns `Vec<ParsedTrack>` with title, artist, album, duration

#### 4.7.3 HTML Parser (`ocr/html_parser.rs`)

`parse_spotify_html(html)` — Parses HTML from Spotify web player copy-paste:
- Regex: `aria-label="Play {title} by {artist}"`
- Regex: duration in specific CSS class spans
- Regex: album links `href="/album/{id}"`

#### 4.7.4 Browser Scraper (`ocr/browser.rs`)

`scrape_playlist_tracks(url)` — Fetches playlist track list from Spotify web page.

This is the command that fails on Android (the error in the screenshot: "Command scrape_playlist_tracks not found"). It's excluded from the Android command handler registration.

#### 4.7.5 OCR Commands (`ocr/commands.rs`)

| Command | What it does |
|---------|-------------|
| `process_screenshots(paths)` | OCR multiple images → deduplicated tracks |
| `debug_ocr(path)` | Returns raw OCR text for debugging |
| `parse_text_tracklist(text)` | Parses "Artist - Title" text lines |
| `parse_spotify_html(html)` | Parses Spotify HTML table |
| `scrape_playlist_tracks(url)` | Web scrapes playlist page |
| `create_playlist_from_tracks(name, url, tracks)` | Creates ResolvedInput from parsed tracks |

### 4.8 Settings Commands

**`commands/settings.rs`**

`get_settings()` — Loads all settings from DB, returns with defaults:

| Setting | Default (Desktop) | Default (Android) |
|---------|-------------------|-------------------|
| `output_root` | `~/Music/Spytfy` | `/storage/emulated/0/Music/Spytfy` |
| `concurrency` | `3` | `1` |
| `bitrate_kbps` | `320` | `320` |
| `overwrite_existing` | `false` | `false` |
| `write_cover_jpg` | `true` | `true` |
| `naming_template` | `{folder}/{number} - {artist} - {title}` | same |

`update_settings(patch)` — Upserts changed settings into `settings` table.

`open_folder(path)` — Opens file explorer at path. Platform-gated:
- Windows: `explorer.exe {path}`
- macOS: `open {path}`
- Linux: `xdg-open {path}`
- Android: no-op

---

## 5. Kotlin Plugin — Android Download Engine

**Location:** `src-tauri/tauri-plugin-spytfy-download/android/src/main/java/app/tauri/spytfy_download/DownloadPlugin.kt`

This plugin wraps the `youtubedl-android` library (a Java/Kotlin port of yt-dlp) to provide YouTube search and download on Android without needing external binaries.

### Plugin Structure

```kotlin
@TauriPlugin
class DownloadPlugin(activity: Activity) : Plugin(activity) {
    // Coroutine scope for background operations
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    // Tracks initialization state
    @Volatile private var initialized = false
}
```

### Initialization (`load()`)

Runs when the plugin is loaded (app startup):
1. `YoutubeDL.getInstance().init(application)` — Extracts bundled yt-dlp Python runtime
2. `FFmpeg.getInstance().init(application)` — Extracts bundled ffmpeg binary
3. `YoutubeDL.getInstance().updateYoutubeDL(STABLE)` — Checks for yt-dlp updates (non-blocking, failure logged as warning)
4. Sets `initialized = true`

All operations run on `Dispatchers.IO` (background thread pool). If init fails, commands return "youtubedl not initialized yet".

### Commands

#### `searchYoutube(invoke)`

**Input:** `SearchArgs { query: String, maxResults: Int = 5 }`

**Process:**
```
1. Create YoutubeDLRequest("ytsearch{maxResults}:{query}")
2. Add options: --dump-json, --flat-playlist, --no-warnings
3. Execute request (blocks on IO dispatcher)
4. Parse response.out as newline-delimited JSON
5. Extract per result: id, title, duration, channel/uploader
6. Return JSObject with results array
```

**Output:** `{ results: [{ videoId, title, durationSec, channel }, ...] }`

#### `downloadAudio(invoke)`

**Input:** `DownloadArgs { videoId: String, outputPath: String, bitrateKbps: Int = 320 }`

**Process:**
```
1. Create YoutubeDLRequest("https://youtube.com/watch?v={videoId}")
2. Add options:
   -f bestaudio          # Best audio stream
   -x                    # Extract audio
   --audio-format mp3    # Convert to MP3
   --audio-quality {bitrate}K
   -o {outputPath}       # Output file
   --no-mtime            # Don't set file time
3. Generate processId: "dl-{videoId}-{timestamp}"
4. Execute with progress callback:
   - Emits "download:progress" events via trigger()
   - Includes processId, progress %, ETA seconds
5. Return { filePath, processId }
```

#### `cancelDownload(invoke)`

**Input:** `CancelArgs { processId: String }`

Calls `YoutubeDL.getInstance().destroyProcessById(processId)` to kill the yt-dlp process.

### How Rust Calls Kotlin

The bridge is in `download/android.rs`. Rust calls:
```rust
app.run_mobile_plugin("spytfy-download", "searchYoutube", payload)
```

Tauri serializes `payload` to JSON, sends it via JNI to the Kotlin plugin, which deserializes into the `@InvokeArg` class, executes the command, and returns JSON back through JNI.

---

## 6. Angular Frontend — Page by Page

### 6.1 App Shell & Routing

**`app.component.ts`** — Root component. Renders:
- Sidebar (left) — navigation icons for home, downloads, library, settings
- `<router-outlet>` (center) — active page
- Toast container (overlay) — notification toasts

**`app.routes.ts`** — Route definitions:

| Path | Page | Guard |
|------|------|-------|
| `/` | Redirect to `/input` | — |
| `/onboarding` | Onboarding | None (always accessible) |
| `/input` | Input | `spotifyAuthGuard` |
| `/downloads` | Downloads | `spotifyAuthGuard` |
| `/library` | Library | `spotifyAuthGuard` |
| `/settings` | Settings | `spotifyAuthGuard` |

**`guards/auth.guard.ts`** — `spotifyAuthGuard`:
- Calls `hasSpotifyCredentials()` IPC
- Credentials exist → allow route
- No credentials → redirect to `/onboarding`

### 6.2 Onboarding Page

First-time setup. User enters Spotify API credentials.

**Flow:**
1. Display input fields for Client ID and Client Secret
2. User clicks "Test" → calls `testSpotifyCredentials()`
3. Valid → calls `saveSpotifyCredentials(id, secret)` → navigates to `/input`
4. Invalid → shows error toast

### 6.3 Input Page

Main entry point for downloading. User pastes a Spotify URL.

**Flow:**
```
1. User pastes URL into input field
2. Regex detects type: track, album, or playlist
3. Calls resolveUrl(url) → backend resolves metadata
4. Shows preview card:
   - Cover art image
   - Title, artist/owner
   - Track count and total duration
   - Track list (for album/playlist)
5. User clicks "Load Tracks" (for playlists):
   - Calls scrapePlaylistTracks(url) ← THIS FAILS ON ANDROID
   - Or "Import manually" for text/JSON input
6. User clicks "Download" or "Download All":
   - Calls enqueueDownload(resolvedInput, sourceUrl)
   - Shows toast: "Enqueued {n} tracks"
   - Navigates to /downloads

Additional features:
  - "Show JSON" — displays raw resolved data
  - "Debug Scrape" — shows scraping debug output
  - "Paste API JSON" — creates track from raw JSON
```

### 6.4 Downloads Page

Batch download monitoring and management.

**Layout:**
- Batch list (grouped by source URL)
- Per-batch: header with name, state badge, track count, controls
- Per-job: row with cover art, title, artist, state indicator, progress bar

**Batch controls:**
- Pause / Resume — `pauseBatch()` / `resumeBatch()`
- Cancel — `cancelBatch()`
- Retry All Failed — `retryAllFailed()`

**Job states displayed:**

| State | Visual |
|-------|--------|
| `queued` | Gray dot |
| `pending` | Pulsing dot |
| `resolving` | Searching icon |
| `downloading` | Progress bar (percentage) |
| `converting` | Spinner |
| `tagging` | Tag icon |
| `verifying` | Check icon |
| `done` | Green check |
| `done_warning` | Yellow check |
| `failed` | Red X with error message |
| `needs_review` | Orange alert, shows candidate picker |

**Real-time updates via Tauri events:**
- `job:state` — Individual job state transition
- `job:cover` — Cover art resolved (updates thumbnail)
- `batch:progress` — Aggregate progress (done/failed/total)
- `batch:complete` — Batch finished notification
- `download:progress` — Per-job download percentage

**Candidate picker (for needs_review):**
When all auto-selected YouTube candidates fail, the user sees a list of candidates with scores. Clicking one calls `pickCandidate(jobId, ytUrl)` and retries.

### 6.5 Library Page

Browse downloaded files organized by album/playlist.

Shows files in the configured output directory. Uses OS file system to list tracks.

### 6.6 Settings Page

Configuration for download behavior.

| Setting | Control | Description |
|---------|---------|-------------|
| Output folder | Path picker | Where MP3s are saved |
| Concurrency | Slider (1-8) | Simultaneous downloads |
| Bitrate | Selector (128/192/256/320) | MP3 quality in kbps |
| Naming template | Text input | File naming pattern |
| Write cover.jpg | Toggle | Save separate cover art file |
| Overwrite existing | Toggle | Re-download existing files |

Changes saved via `updateSettings(patch)` IPC.

---

## 7. Shared Libraries

### 7.1 Models

**Path alias:** `@spytfy/models` → `libs/models/src/index.ts`

TypeScript interfaces mirroring Rust structs (keep in sync manually):

```typescript
// spotify.model.ts
interface SpotifyTrack {
  id: string; name: string; artists: string[];
  album: string; albumId: string;
  trackNumber: number; discNumber: number;
  durationMs: number; isrc?: string;
  coverUrl?: string; releaseDate?: string;
}

interface ResolvedInput {
  type: 'track' | 'album' | 'playlist';
  data: SpotifyTrack | SpotifyAlbum | SpotifyPlaylist;
}

// job.model.ts
type JobState = 'queued' | 'pending' | 'resolving' | 'downloading' |
  'converting' | 'tagging' | 'verifying' | 'done' |
  'done_warning' | 'failed' | 'needs_review';

interface DownloadJob {
  id: string; batchId: string; spotifyId: string;
  title: string; artist: string; album: string;
  durationMs: number; state: JobState;
  ytUrl?: string; ytScore?: number;
  outputPath?: string; error?: string;
  progressPct?: number; coverUrl?: string;
  candidatesJson?: string;
}

interface Batch {
  id: string; sourceUrl: string;
  sourceType: 'track' | 'album' | 'playlist';
  name: string; totalTracks: number;
  state: string; createdAt: string;
}

// settings.model.ts
interface Settings {
  outputRoot: string; concurrency: number;
  bitrateKbps: number; overwriteExisting: boolean;
  writeCoverJpg: boolean; namingTemplate: string;
}
```

### 7.2 IPC Layer

**Path alias:** `@spytfy/tauri-ipc` → `libs/tauri-ipc/src/index.ts`

Typed wrappers around `@tauri-apps/api/core::invoke()` and `@tauri-apps/api/event::listen()`.

Each file exports async functions that call the corresponding Rust command:

```typescript
// spotify.ipc.ts
export const resolveUrl = (url: string) => invoke<ResolvedInput>('resolve_url', { url });
export const saveSpotifyCredentials = (clientId: string, clientSecret: string) => ...;
export const hasSpotifyCredentials = () => invoke<boolean>('has_spotify_credentials');

// queue.ipc.ts
export const enqueueDownload = (input: ResolvedInput, sourceUrl: string) =>
  invoke<string>('enqueue_download', { input, sourceUrl });
export const listBatches = (limit?: number) => invoke<Batch[]>('list_batches', { limit });

// Event listeners return unlisten functions
export const onJobState = (callback: (e: JobStateEvent) => void) =>
  listen<JobStateEvent>('job:state', (e) => callback(e.payload));
```

### 7.3 UI Components

**Path alias:** `@spytfy/ui` → `libs/ui/src/index.ts`

Shared Angular components used across pages (buttons, cards, inputs, etc.).

---

## 8. IPC Protocol

### Commands (Frontend → Backend, async request/response)

```
┌─────────────────────────────────────────────────────────────────┐
│ SPOTIFY                                                         │
├─────────────────────────────────────────────────────────────────┤
│ resolve_url(url: string) → ResolvedInput                        │
│ save_spotify_credentials(clientId, clientSecret) → void          │
│ test_spotify_credentials() → void                                │
│ has_spotify_credentials() → bool                                 │
│ debug_scrape(url: string) → string                               │
│ resolve_from_json(jsonStr: string) → ResolvedInput               │
├─────────────────────────────────────────────────────────────────┤
│ SETTINGS                                                        │
├─────────────────────────────────────────────────────────────────┤
│ get_settings() → Settings                                        │
│ update_settings(patch: SettingsPatch) → Settings                 │
│ open_folder(path: string) → void                                 │
├─────────────────────────────────────────────────────────────────┤
│ DOWNLOAD                                                        │
├─────────────────────────────────────────────────────────────────┤
│ download_track(track: SpotifyTrack) → DownloadResult             │
├─────────────────────────────────────────────────────────────────┤
│ QUEUE                                                           │
├─────────────────────────────────────────────────────────────────┤
│ enqueue_download(input: ResolvedInput, sourceUrl) → string       │
│ list_batches(limit?: number) → Batch[]                           │
│ list_jobs(batchId?: string) → DownloadJob[]                      │
│ pause_batch(batchId: string) → void                              │
│ resume_batch(batchId: string) → void                             │
│ cancel_batch(batchId: string) → void                             │
│ retry_job(jobId: string) → void                                  │
│ retry_all_failed(batchId: string) → u32                          │
│ resume_queued() → u32                                            │
│ pick_candidate(jobId: string, ytUrl: string) → void              │
│ list_failed_jobs() → DownloadJob[]                               │
├─────────────────────────────────────────────────────────────────┤
│ OCR (Desktop only — excluded from Android build)                │
├─────────────────────────────────────────────────────────────────┤
│ process_screenshots(paths: string[]) → ParsedTrack[]             │
│ debug_ocr(path: string) → string                                │
│ parse_text_tracklist(text: string) → ParsedTrack[]               │
│ parse_spotify_html(html: string) → ParsedTrack[]                 │
│ scrape_playlist_tracks(url: string) → ParsedTrack[]              │
│ create_playlist_from_tracks(name, coverUrl, tracks) → Resolved   │
└─────────────────────────────────────────────────────────────────┘
```

### Events (Backend → Frontend, async push)

```
┌──────────────────────────────────────────────────────────────┐
│ EVENT                    │ PAYLOAD                           │
├──────────────────────────┼──────────────────────────────────┤
│ download:state           │ { stage: string }                │
│ download:progress        │ { percent: number }              │
│ job:state                │ { jobId, batchId, state, error? }│
│ job:cover                │ { jobId, batchId, coverUrl }     │
│ batch:progress           │ { batchId, done, failed,         │
│                          │   needsReview, total }           │
│ batch:complete           │ { batchId, total, succeeded,     │
│                          │   failed }                       │
│ download:progress (K)    │ { processId, progress, etaSec }  │
│  (from Kotlin plugin)    │                                  │
└──────────────────────────┴──────────────────────────────────┘
```

---

## 9. Database Schema

SQLite database at `{app_data}/.spytfy/db.sqlite`.

### Tables

#### `settings`
```sql
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT,
    updated_at TEXT DEFAULT (datetime('now'))
);
```

#### `batches`
```sql
CREATE TABLE batches (
    id           TEXT PRIMARY KEY,           -- UUID
    source_url   TEXT,                       -- Original Spotify URL
    source_type  TEXT CHECK(source_type IN ('track','album','playlist')),
    name         TEXT,                       -- Album/playlist/track name
    total_tracks INTEGER,
    state        TEXT DEFAULT 'active',      -- active|paused|cancelled|complete
    created_at   TEXT DEFAULT (datetime('now')),
    updated_at   TEXT DEFAULT (datetime('now'))
);
```

#### `jobs`
```sql
CREATE TABLE jobs (
    id              TEXT PRIMARY KEY,        -- UUID
    batch_id        TEXT REFERENCES batches(id),
    spotify_id      TEXT,                    -- Spotify track ID
    title           TEXT,
    artist          TEXT,
    album           TEXT,
    duration_ms     INTEGER,
    state           TEXT DEFAULT 'queued',   -- See job state machine
    yt_url          TEXT,                    -- Selected YouTube URL
    yt_score        INTEGER,                 -- Match score
    output_path     TEXT,                    -- Final MP3 file path
    error           TEXT,                    -- Error message if failed
    progress_pct    REAL,                    -- Download progress 0-100
    cover_url       TEXT,                    -- Resolved cover art URL
    track_number    INTEGER DEFAULT 1,
    candidates_json TEXT,                    -- JSON array of YouTube candidates
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_jobs_batch ON jobs(batch_id);
CREATE INDEX idx_jobs_state ON jobs(state);
```

### Migrations

| File | Change |
|------|--------|
| `001_initial.sql` | Creates `settings`, `batches`, `jobs` tables + indexes |
| `002_add_cover_url.sql` | Adds `cover_url` column to `jobs` |
| `003_add_track_number.sql` | Adds `track_number` column to `jobs` |
| `004_add_candidates.sql` | Adds `candidates_json` column to `jobs` |

---

## 10. Data Flows

### Flow 1: Paste Spotify URL → Download Single Track

```
User pastes: https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC

Frontend (Input Page):
  1. Regex detects "track" type
  2. invoke('resolve_url', { url }) ──────────────────────► Backend

Backend (resolver.rs):
  3. parse_spotify_url() → SpotifyUrl { kind: Track, id: "4uLU6..." }
  4. rspotify::track("4uLU6...") → Spotify API HTTP request
  5. Map to SpotifyTrack { name: "God's Plan", artists: ["Drake"], ... }
  6. Return ResolvedInput::Track(track)  ◄──────────────── Frontend

Frontend (Preview Card):
  7. Display: cover art, "God's Plan", "Drake", "3:19"
  8. User clicks "Download All"
  9. invoke('enqueue_download', { input, sourceUrl }) ───► Backend

Backend (manager.rs):
  10. Create Batch { id: uuid, name: "God's Plan", type: "track", state: "active" }
  11. Create Job { id: uuid, batch_id, title: "God's Plan", state: "queued" }
  12. Send (job, track) to mpsc channel ─────────────────► Worker

Backend (worker.rs):
  13. Receive job from channel
  14. Set state → "resolving", emit job:state event ────► Frontend updates UI
  15. search_youtube("Drake", "God's Plan") → 5 candidates
  16. score_candidates() → best: score 87, "Drake - God's Plan (Official Audio)"
  17. Set state → "downloading", emit job:state ────────► Frontend shows progress
  18. download_mp3(url, "Music/Spytfy/God's Plan/01 - Drake - God's Plan.mp3")
      └─ yt-dlp extracts audio, converts to MP3
  19. Set state → "tagging"
  20. fetch_cover(cover_url) → download, resize, cache 300x300 JPEG
  21. tag_mp3() → write ID3v2.4: title, artist, album, cover, ISRC
  22. Set state → "verifying"
  23. verify_mp3() → check cover hash matches, image valid
  24. Set state → "done", emit job:state ───────────────► Frontend shows ✓
  25. check_batch_complete() → batch 1/1 done
  26. Emit batch:complete ──────────────────────────────► Frontend shows notification
```

### Flow 2: Download Full Playlist (Batch)

```
User pastes: https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M

Frontend:
  1. resolveUrl() → SpotifyPlaylist { name: "Today's Top Hits", tracks: [50 tracks] }
  2. Preview card shows 50 tracks
  3. User clicks "Download All 50"
  4. enqueueDownload(playlist, url)

Backend (manager.rs):
  5. Create Batch { total_tracks: 50, state: "active" }
  6. For each of 50 tracks:
     ├─ Check if file exists → skip (state: "done")
     └─ Create Job (state: "queued")
  7. Push queued jobs to channel (up to channel capacity 50)

Backend (worker.rs):
  8. Supervisor receives jobs one by one
  9. Semaphore limits to 2 concurrent (desktop) or 1 (Android)
  10. 2-second delay between jobs (YouTube rate limiting)
  11. Each job follows the same pipeline as Flow 1 steps 13-26
  12. After each job completes:
      ├─ emit batch:progress { done: N, failed: M, total: 50 }
      └─ push_one_queued_job() → feed next job from DB

User can:
  - Pause: pauseBatch() → workers skip paused batch's jobs
  - Resume: resumeBatch() → re-push queued jobs
  - Cancel: cancelBatch() → mark remaining as failed
  - Retry failed: retryAllFailed() → re-queue failed jobs
```

### Flow 3: Needs Review (Manual YouTube Selection)

```
Backend (worker.rs):
  1. All 3 auto-selected YouTube candidates fail to download
  2. Store top 5 candidates in job.candidates_json
  3. Set state → "needs_review"
  4. Emit job:state ────────────────────────────► Frontend

Frontend (Downloads Page):
  5. Job row shows orange alert icon
  6. User expands candidate list (parsed from candidates_json)
  7. Each candidate shows: title, channel, duration, score
  8. User clicks preferred candidate
  9. invoke('pick_candidate', { jobId, ytUrl }) ──► Backend

Backend:
  10. Update job.yt_url = selected URL
  11. Reset job.state → "queued"
  12. Send job to channel → worker retries with pre-selected URL
  13. Worker skips search, downloads directly from selected URL
```

### Flow 4: App Crash Recovery

```
Scenario: App crashes while 3 jobs are downloading, 10 are queued

On next startup (lib.rs setup callback):
  1. init_pool() → SQLite migrations run (no-op if already current)
  
  2. merge_duplicate_batches():
     └─ Find batches with same name+type, merge jobs into earliest batch

  3. reset_all_stuck_jobs():
     ├─ Find jobs in state "pending" → reset to "queued"
     ├─ Find jobs in state "resolving" → reset to "queued"
     └─ Find jobs in state "downloading" → reset to "queued"
     (These states mean the job was mid-pipeline when crash occurred.
      Channel is empty after restart, so safe to re-queue.)

  4. push_all_queued_jobs():
     └─ Query all "queued" jobs from active batches → send to channel

  5. WorkerPool starts processing from where it left off
     (Already-downloaded files are detected and skipped via >1000 byte check)
```

---

## 11. Scoring Algorithm

The scoring algorithm in `download/scorer.rs` determines which YouTube video best matches a Spotify track. This is critical because a bad match means downloading the wrong song.

### Score Calculation

```
For each YouTube candidate:

1. START WITH 100 POINTS

2. DURATION PENALTY (most important signal)
   If Spotify provides track duration:
     delta = |youtube_duration - spotify_duration| in seconds
     penalty = delta * 2
     score -= penalty
   
   Example: Track is 3:30 (210s), YouTube is 3:45 (225s)
     delta = 15s → penalty = 30 → score = 70

3. TITLE SIMILARITY (Levenshtein distance)
   normalized_title = lowercase, strip "official video/audio/lyrics"
   similarity = 1 - (edit_distance / max_length)  # 0.0 to 1.0
   penalty = (1 - similarity) * 50
   score -= penalty

   Example: "God's Plan" vs "Drake - God's Plan Official Audio"
     After normalization: "god's plan" vs "drake - god's plan"
     similarity ≈ 0.65 → penalty = 17.5 → score -= 17.5

4. BONUSES (positive signals)
   +15  title contains "official audio"     # Likely the right version
   +20  uploader contains "topic" or "vevo" # YouTube auto-generated channels
   +25  uploader ≈ artist (Levenshtein > 0.8)

5. PENALTIES (negative signals)
   -25  title contains "live"               # Live performances differ
   -40  title contains "cover"              # Not the original artist
   -30  title contains "remix"              # Modified version
   -50  title contains "sped up/slowed/8d/nightcore"  # Altered audio
   -30  duration > track_duration * 1.5     # Likely a compilation

6. SELECTION THRESHOLD
   If duration known:  score >= 40 AND |duration_delta| <= 10 seconds
   If duration unknown: score >= 30
   
   No candidate passes? → Return None → Job enters "needs_review"
```

### Why This Works

- Duration is the strongest signal: wrong songs almost always have wrong duration
- Title similarity catches mismatches but allows for YouTube's verbose titles
- Channel matching (VEVO/Topic) strongly correlates with official uploads
- Negative keywords filter out versions users don't want
- The 10-second duration window allows for slight YouTube encoding differences

---

## 12. Error Handling & Recovery

### Download Failures

| Scenario | Handling |
|----------|---------|
| YouTube search returns 0 results | Job fails with "No YouTube results found" |
| All scored candidates below threshold | Job enters `needs_review`, candidates stored |
| yt-dlp download fails | 3 retries with exponential backoff (5/10/15 seconds) |
| All 3 top candidates fail to download | Job enters `needs_review` |
| Cover art download fails | Tags without cover (basic ID3 only) |
| Cover art too small (<300x300) | Warning logged, tags without cover |
| ID3 tag write fails | Job fails with error |
| File verification fails (hash mismatch) | Job completes with `done_warning` state |

### Batch State Recovery

| Scenario | Handling |
|----------|---------|
| App crashes mid-download | `reset_all_stuck_jobs()` on startup → re-queues |
| Duplicate batches from rapid re-queue | `merge_duplicate_batches()` on startup |
| Channel full (50 jobs buffered) | Job reverts to "queued", retried on next slot |
| Batch paused while jobs in-flight | Workers detect paused batch, reset job to "queued" |

### Spotify API Failures

| Scenario | Handling |
|----------|---------|
| Invalid credentials | `test_spotify_credentials()` returns error before saving |
| Rate limited | Falls back to web scraping (`scraper.rs`) |
| Track not found | API returns 404, shown as resolution error |
| Playlist too large | Paginated stream handles any size |

### Network Failures

| Scenario | Handling |
|----------|---------|
| No internet | yt-dlp/reqwest timeout → job fails with network error |
| Partial download (connection drop) | `.part` files cleaned before retry |
| Spotify API timeout | reqwest has default timeout, falls back to scraper |

---

## 13. Platform Differences

### Command Availability

| Command | Desktop | Android |
|---------|:-------:|:-------:|
| All Spotify commands | Yes | Yes |
| All Settings commands | Yes | Yes |
| All Queue commands | Yes | Yes |
| `download_track` | Yes | Yes |
| `process_screenshots` | Yes | **No** |
| `debug_ocr` | Yes | **No** |
| `parse_text_tracklist` | Yes | **No** |
| `parse_spotify_html` | Yes | **No** |
| `scrape_playlist_tracks` | Yes | **No** |
| `create_playlist_from_tracks` | Yes | **No** |

### Download Engine

| Aspect | Desktop | Android |
|--------|---------|---------|
| YouTube search | yt-dlp sidecar process | Kotlin plugin (youtubedl-android) |
| Audio download | yt-dlp sidecar process | Kotlin plugin (youtubedl-android) |
| MP3 conversion | ffmpeg sidecar | ffmpeg bundled in youtubedl-android |
| Binary location | `src-tauri/binaries/` | Extracted to app data on first run |
| Concurrency default | 3 workers | 1 worker |
| Output directory | `~/Music/Spytfy/` | `/storage/emulated/0/Music/Spytfy/` |
| File browser | OS explorer (`open_folder`) | Not implemented |
| OCR import | Tesseract + Windows OCR | Not available |

### Build Targets

| Target | Architecture | Build command |
|--------|-------------|---------------|
| Windows | x86_64 | `cargo tauri build` |
| Android | ARM64 (aarch64) | `cargo tauri android build --apk` |
| Android dev | ARM64 (USB debug) | `cargo tauri android dev` |

---

## 14. Build System & Configuration

### Monorepo Structure (NX)

```
nx.json         → Workspace configuration
package.json    → Node scripts and dependencies
tsconfig.base.json → Path aliases (@spytfy/models, @spytfy/tauri-ipc, @spytfy/ui)
```

**Key scripts:**
- `nx serve desktop` → Start Angular dev server (localhost:4200)
- `nx build desktop` → Production Angular build → `dist/apps/desktop/browser/`
- `cargo tauri dev` → Start Tauri dev mode (Rust backend + Angular frontend)
- `cargo tauri build` → Production desktop build (installer)
- `cargo tauri android dev` → Deploy to connected Android device
- `cargo tauri android build --apk` → Build release APK

### Tauri Configuration

**`tauri.conf.json`** (Desktop):
```json
{
  "productName": "Spytfy",
  "identifier": "com.spytfy.app",
  "build": {
    "frontendDist": "../dist/apps/desktop/browser",
    "devUrl": "http://localhost:4200"
  },
  "app": {
    "windows": [{ "width": 1100, "height": 700, "minWidth": 900, "minHeight": 600 }]
  },
  "bundle": {
    "externalBin": ["binaries/yt-dlp", "binaries/ffmpeg"],
    "resources": {}
  },
  "plugins": {
    "shell": { "open": true },
    "dialog": {},
    "fs": {},
    "store": {}
  }
}
```

**`tauri.android.conf.json`** (Android overrides):
```json
{
  "bundle": {
    "externalBin": [],           // No sidecars on Android
    "resources": {},
    "android": {
      "minSdkVersion": 29       // Android 10+
    }
  }
}
```

### Rust Build Configuration

**`Cargo.toml`** key settings:
```toml
[lib]
crate-type = ["staticlib", "cdylib", "rlib"]
# staticlib: iOS (future)
# cdylib: Android (JNI shared library)
# rlib: Desktop (standard Rust library)
```

---

## 15. Dependencies

### Rust (Cargo.toml)

| Crate | Version | Purpose |
|-------|---------|---------|
| `tauri` | 2.x | App framework, window management, IPC |
| `tauri-plugin-dialog` | 2.x | File picker dialogs |
| `tauri-plugin-shell` | 2.x | Open URLs, file explorer |
| `tauri-plugin-store` | 2.x | Persistent key-value store |
| `tauri-plugin-fs` | 2.x | File system access |
| `sqlx` | 0.8 | SQLite async ORM with migrations |
| `tokio` | 1.x | Async runtime (full features) |
| `rspotify` | 0.13 | Spotify Web API client |
| `reqwest` | 0.12 | HTTP client (rustls TLS) |
| `id3` | 1.x | MP3 ID3v2 tag reading/writing |
| `image` | 0.25 | JPEG/PNG decode, resize, crop |
| `strsim` | 0.11 | Levenshtein distance for scoring |
| `regex` | 1.x | URL parsing, text extraction |
| `sha2` | 0.10 | SHA-256 hashing for verification |
| `uuid` | 1.x | UUID generation for batch/job IDs |
| `chrono` | 0.4 | Date/time formatting |
| `dirs` | 5.x | System directory resolution |
| `serde` | 1.x | JSON serialization/deserialization |
| `serde_json` | 1.x | JSON parsing |

### Node (package.json)

| Package | Purpose |
|---------|---------|
| `@angular/*` 20.x | Frontend framework |
| `@tauri-apps/api` 2.x | Frontend IPC bridge |
| `@tauri-apps/cli` 2.x | Tauri build tooling |
| `tailwindcss` | Utility-first CSS |
| `rxjs` | Reactive event streams |
| `nx` | Monorepo task runner |
| `typescript` 5.8 | Type checking |

### Android (Kotlin plugin)

| Library | Purpose |
|---------|---------|
| `youtubedl-android` | Java port of yt-dlp (YouTube download) |
| `ffmpeg-android` | Bundled ffmpeg for audio conversion |
| `kotlinx-coroutines` | Async background operations |
| Tauri Android SDK | Plugin framework, JNI bridge |

---

## Appendix: Known Gaps (as of current build)

1. **`scrape_playlist_tracks` missing on Android** — OCR module entirely excluded. Frontend doesn't check platform before calling it. Causes the "Command not found" error on mobile.

2. **Phase 11 incomplete** — Kotlin plugin compiles and registers, but no confirmed end-to-end test of search → download on real device.

3. **No MediaStore integration** — Downloaded files go to app-private storage, not visible in other music apps (Phase 14 work).

4. **No foreground service** — Downloads stop when app is backgrounded on Android (Phase 14 work).

5. **No mobile UI adaptation** — Desktop sidebar layout shows on mobile viewport. Bottom navigation not implemented (Phase 13 work).

6. **ROADMAP.md not updated** — Shows phases 9-14 as "Not started" despite 9 and 10 being committed.
