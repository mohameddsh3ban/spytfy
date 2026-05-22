<p align="center">
  <img src="spytfy/src-tauri/icons/icon.png" alt="Spytfy Logo" width="128" />
</p>

<h1 align="center">Spytfy</h1>

<p align="center">
  <strong>Your Spotify library, downloaded as a real music library.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version" />
  <img src="https://img.shields.io/badge/platform-Windows-0078D6?logo=windows" alt="Platform" />
  <img src="https://img.shields.io/badge/Tauri-2.x-FFC131?logo=tauri" alt="Tauri" />
  <img src="https://img.shields.io/badge/Angular-20-DD0031?logo=angular" alt="Angular" />
  <img src="https://img.shields.io/badge/Rust-Tokio-DEA584?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
</p>

---

## What is Spytfy?

Spytfy is a cross-platform desktop application that downloads music from Spotify as high-quality MP3 files. Paste a Spotify link (track, album, or playlist), and Spytfy resolves the metadata, finds the best YouTube match, downloads it as **320 kbps MP3**, embeds **ID3v2.4 tags** with **real album artwork**, and organizes everything into a clean folder structure.

**No Spotify Premium required. No cloud. No telemetry. Fully offline after setup.**

### Who is it for?

- Music archivists building offline libraries
- DJs preparing sets from Spotify playlists
- Self-hosted music server runners (Plex, Navidrome, Jellyfin)
- Anyone who wants to own their music

---

## Features

| Feature | Description |
|---------|-------------|
| **Spotify URL Import** | Paste any track, album, or playlist URL |
| **Screenshot Import (OCR)** | Screenshot a Spotify playlist &rarr; OCR extracts all tracks automatically |
| **Smart YouTube Matching** | Scoring algorithm with duration validation (&plusmn;5s tolerance) |
| **High-Quality Audio** | 320 kbps MP3 via ffmpeg transcoding |
| **Full Metadata** | ID3v2.4 tags: title, artist, album, track number, cover art |
| **Real Cover Art** | Album artwork embedded at &ge;300&times;300px (never placeholder) |
| **Batch Downloads** | Queue entire playlists/albums with concurrent workers |
| **Real-Time Progress** | Per-track progress bars with live status updates |
| **Pause / Resume / Cancel** | Full control over download batches |
| **Retry & Disambiguation** | Failed tracks get top-3 YouTube candidates for manual selection |
| **Smart File Organization** | `{output}/{playlist_name}/{01 - Artist - Title}.mp3` |
| **Keyboard Shortcuts** | `Ctrl+1-4` for instant page navigation |
| **Persistent Queue** | SQLite-backed &mdash; survives app restarts |
| **Offline-First** | No backend server, no accounts, no cloud dependency |

---

## Screenshots

> *Coming soon &mdash; the app is in active development (v0.1.0)*

---

## Architecture

```
+---------------------------------------------+
|         Angular 20 Desktop App              |
|        (Tauri WebView2 @ :4200)             |
+---------------------------------------------+
|  Pages: Input | Downloads | Library | Settings
|  Components: Sidebar, Toast, Preview Cards  |
+---------------------------------------------+
               | Tauri IPC |
+---------------------------------------------+
|           Rust Core (Tokio async)           |
|                                             |
|  +-- Spotify Module --+  +-- OCR Module --+ |
|  | OAuth2 auth        |  | Screenshot     | |
|  | URL resolver       |  | Tesseract OCR  | |
|  | Playlist scraper   |  | Track parser   | |
|  +--------------------+  +----------------+ |
|                                             |
|  +-- Download Pipeline -------------------+ |
|  | YouTube search (yt-dlp)                | |
|  | Candidate scoring (>=40 + ±5s)         | |
|  | MP3 download (ffmpeg)                  | |
|  | ID3v2.4 tagging + cover art embedding  | |
|  | SHA-256 post-tag verification           | |
|  +-----------------------------------------+ |
|                                             |
|  +-- Queue Manager -----------------------+ |
|  | SQLite persistence (batches + jobs)    | |
|  | Tokio worker pool (concurrent DL)      | |
|  | Pause / resume / cancel / retry        | |
|  +-----------------------------------------+ |
+---------------------------------------------+
|        Sidecar Binaries                     |
|   yt-dlp  |  ffmpeg  |  WebView2Loader.dll  |
+---------------------------------------------+
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Frontend** | Angular 20, Tailwind CSS v4, RxJS 7.8 |
| **Desktop** | Tauri 2.x (WebView2) |
| **Backend** | Rust, Tokio 1 (async runtime) |
| **Database** | SQLite via SQLx 0.8 |
| **Spotify API** | rspotify 0.13 (OAuth2, no Premium needed) |
| **Audio** | yt-dlp (search/download), ffmpeg (transcode) |
| **Metadata** | id3 crate (ID3v2.4 tags), image crate (cover art) |
| **Build System** | Nx 22, pnpm, Cargo |
| **Installer** | NSIS (Windows) |

---

## Getting Started

### Prerequisites

- **Node.js** 20+
- **pnpm** 9+
- **Rust** toolchain (1.70+)
- **Spotify Developer Account** &mdash; [Create an app](https://developer.spotify.com/dashboard) to get Client ID & Secret

### Sidecar Binaries

Download and place in `spytfy/src-tauri/binaries/`:

| Binary | Source | Rename to |
|--------|--------|-----------|
| [yt-dlp](https://github.com/yt-dlp/yt-dlp/releases) | Latest release | `yt-dlp-x86_64-pc-windows-gnu.exe` |
| [ffmpeg](https://github.com/BtbN/FFmpeg-Builds/releases) | `ffmpeg-master-latest-win64-gpl` | `ffmpeg-x86_64-pc-windows-gnu.exe` |

### Installation

```bash
# Clone the repo
git clone https://github.com/mohameddsh3ban/spytfy.git
cd spytfy

# Install frontend dependencies
cd spytfy
pnpm install

# Run in development mode
cd ..
npm run dev
```

### First Run

1. Launch the app &rarr; you'll see the **Onboarding** page
2. Enter your Spotify **Client ID** and **Client Secret**
3. Credentials are stored securely in your OS keychain
4. Set your output directory in **Settings**
5. Paste a Spotify URL in the **Input** page and hit download

---

## Development

### Project Structure

```
spytfy/
├── scripts/              # Dev launcher & process cleanup
├── spytfy/               # Main workspace
│   ├── apps/desktop/     # Angular 20 frontend
│   │   └── src/app/
│   │       ├── pages/    # Input, Downloads, Library, Settings
│   │       ├── layout/   # Sidebar, Toast
│   │       └── guards/   # Auth guard
│   ├── libs/
│   │   ├── models/       # Shared TypeScript types
│   │   ├── tauri-ipc/    # Tauri command & event wrappers
│   │   └── ui/           # Shared UI components
│   ├── src-tauri/        # Rust backend
│   │   ├── src/
│   │   │   ├── spotify/  # Auth, resolver, scraper, parser
│   │   │   ├── download/ # Pipeline, scorer, tagger, cover, verifier
│   │   │   ├── queue/    # Manager, worker pool, commands
│   │   │   └── ocr/      # Engine, parser, browser, commands
│   │   ├── migrations/   # SQLite schema (4 migrations)
│   │   └── binaries/     # yt-dlp, ffmpeg sidecars
│   ├── build.ps1         # Windows build automation
│   └── nx.json           # Nx monorepo config
└── package.json          # Root scripts (dev, build, stop)
```

### Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server (hot-reload frontend + Rust backend) |
| `npm run build` | Build production installer (NSIS) |
| `npm run stop` | Kill dev server processes |

### Database Schema

SQLite with 4 migrations:

| Table | Purpose |
|-------|---------|
| `batches` | Track download groups (playlist/album/artist) |
| `jobs` | Individual track state machine (queued &rarr; downloading &rarr; done/failed) |
| `settings` | App configuration (output path, bitrate, cover options) |

---

## How It Works

### Download Pipeline

```
Spotify URL
    │
    ▼
1. Resolve metadata via Spotify Web API
   (track title, artist, album, duration, cover URL)
    │
    ▼
2. Search YouTube via yt-dlp
   (query: "{artist} - {title}")
    │
    ▼
3. Score candidates
   (title similarity + duration match within ±5s)
   Score >= 40 → auto-select
   Score < 40  → NeedsReview (user picks from top 3)
    │
    ▼
4. Download audio via yt-dlp + ffmpeg
   (extract audio → transcode to 320 kbps MP3)
    │
    ▼
5. Embed metadata
   (ID3v2.4: title, artist, album, track#, cover art ≥300×300)
    │
    ▼
6. Verify integrity
   (re-read APIC tag, SHA-256 compare, dimension check)
    │
    ▼
7. Move to organized folder
   ({output}/{playlist_name}/{01 - Artist - Title}.mp3)
```

### OCR Screenshot Import

1. User takes a screenshot of a Spotify playlist/album
2. App processes the image through Tesseract OCR
3. Extracts track names and artists from the recognized text
4. Creates a batch download job for all discovered tracks

---

## Key Design Decisions

- **Tauri over Electron** &mdash; 10x smaller bundle, native Rust performance, no Chromium bloat
- **Angular 20 standalone components** &mdash; OnPush change detection for snappy UI
- **SQLite for queue persistence** &mdash; survives crashes, no external DB needed
- **Channel-based worker pool** &mdash; Tokio semaphore controls concurrency, cancellation tokens for clean shutdown
- **Cover art verification** &mdash; every MP3 must have real &ge;300&times;300 artwork, verified post-embed via SHA-256

---

## Roadmap

| Phase | Status |
|-------|--------|
| Scaffold (Tauri + Angular + Nx) | Done |
| Spotify URL Resolver | Done |
| Single-Track Download Pipeline | Done |
| Queue & Concurrent Downloads | Done |
| Screenshot Import (OCR) | Done |
| Cover Art Embedding | Done |
| Folders & File Naming | Done |
| Failure Recovery & Retry | Done |
| UI Polish & Keyboard Shortcuts | Done |
| Packaging & Distribution | In Progress |

### Future (v2)

- macOS and Linux builds
- FLAC / lossless output
- Apple Music & Tidal support
- Drag-and-drop URL input
- Download history & search

---


## License

MIT License. See [LICENSE](LICENSE) for details.

---

## Disclaimer

This tool is for personal use only. Respect copyright laws and the terms of service of Spotify and YouTube. The developers are not responsible for misuse of this software.
