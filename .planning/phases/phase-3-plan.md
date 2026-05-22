# Phase 3 — Single-Track Download | Implementation Plan

**Date**: 2026-05-20
**Estimate**: 2 days
**Prerequisites**: Phase 2 complete, yt-dlp + ffmpeg binaries downloaded

---

## Step 1 — Acquire Sidecar Binaries
**Time**: ~15 min

1. Download yt-dlp Windows binary from GitHub releases → `src-tauri/binaries/yt-dlp-x86_64-pc-windows-gnu.exe`
2. Download ffmpeg Windows binary (essentials build) → `src-tauri/binaries/ffmpeg-x86_64-pc-windows-gnu.exe`
3. Configure `tauri.conf.json` → `bundle.externalBin: ["binaries/yt-dlp", "binaries/ffmpeg"]`
4. Tauri auto-appends target triple to binary name at build time
5. Add `binaries/` to `.gitignore` (too large for git)

**Output**: Sidecars resolve at runtime via `app.shell().sidecar("yt-dlp")`.

---

## Step 2 — YouTube Search Module
**Time**: ~30 min

1. Create `src-tauri/src/download/mod.rs` + `youtube.rs`
2. Run: `yt-dlp "ytsearch10:{artist} {title}" --print "%(id)s|%(title)s|%(duration)s|%(uploader)s|%(channel_id)s" --no-download`
3. Parse stdout lines → `Vec<YtCandidate { id, title, duration_secs, uploader, channel_id }>`
4. Shell out via `tauri_plugin_shell::ShellExt` sidecar API
5. Handle: no results, yt-dlp errors, timeout (30s)

**Output**: `search_youtube(artist, title) -> Vec<YtCandidate>`.

---

## Step 3 — Match Scoring Algorithm (Spec §13)
**Time**: ~30 min

1. Create `src-tauri/src/download/scorer.rs`
2. Implement scoring:
   ```
   base = 100
   base -= abs(yt_duration - spotify_duration) * 2
   base -= levenshtein(normalize(yt_title), normalize("{artist} {title}")) * 0.5
   +15 if "official audio"
   +20 if "topic" / VEVO channel
   +25 if channel matches artist (fuzzy ≥ 0.8)
   -25 if "live"
   -40 if "cover"
   -30 if "remix" (when not in query)
   -50 if "sped up" / "slowed" / "8d audio" / "nightcore"
   -30 if duration > spotify * 1.5
   ```
3. Decision: score ≥ 40 AND |duration_delta| ≤ 5s → accept. Else → best candidate with warning.
4. Add `strsim` crate for Levenshtein distance
5. Normalize: lowercase, strip punctuation, collapse whitespace

**Output**: `score_candidates(spotify_track, candidates) -> Option<ScoredMatch>`.

---

## Step 4 — Download + Convert to MP3
**Time**: ~30 min

1. Create `src-tauri/src/download/downloader.rs`
2. Run: `yt-dlp <url> -x --audio-format mp3 --audio-quality 0 -o "{output_path}" --no-playlist`
3. Parse stdout for progress: `[download] 34.2%` → emit Tauri event `download:progress`
4. Output path: `{output_root}/{album_or_playlist}/{NN} - {artist} - {title}.mp3`
5. Sanitize filenames: strip `/ \ : * ? " < > |`, trim, max 200 chars
6. Handle: disk full, yt-dlp crash, ffmpeg failure

**Output**: MP3 file on disk, progress events emitted.

---

## Step 5 — Cover Art Pipeline (Spec §14)
**Time**: ~30 min

1. Create `src-tauri/src/download/cover.rs`
2. Source priority: Spotify album cover (largest from images[]) → YT thumbnail fallback
3. Fetch via reqwest, cache at `.spytfy/cache/covers/{album_id}.jpg`
4. Validate:
   - Magic bytes: `FF D8 FF` (JPEG) or `89 50 4E 47` (PNG)
   - Min dimensions: 300×300 (use `image` crate to read headers)
   - Max file size: 1MB (re-encode to 1000×1000 JPEG q90 if exceeded)
   - Square aspect ratio (crop center if 16:9 YT thumbnail)
5. Convert PNG → JPEG before embedding
6. If all sources fail → return error (never placeholder)
7. Add `image` and `reqwest` crates

**Output**: `fetch_cover(album_id, cover_url) -> Result<Vec<u8>>` returning validated JPEG bytes.

---

## Step 6 — ID3 Tagging + Cover Embed
**Time**: ~20 min

1. Create `src-tauri/src/download/tagger.rs`
2. Write ID3v2.4 tags via `id3` crate:
   - TIT2 (title), TPE1 (artist), TALB (album), TRCK (track#), TPOS (disc#)
   - TDRC (year from release_date), TSRC (ISRC if available)
   - COMM (comment: "Spytfy")
3. Embed cover: APIC frame, picture type 0x03 (front cover), MIME `image/jpeg`
4. One APIC frame only, empty description

**Output**: MP3 file with all tags + embedded cover art.

---

## Step 7 — Post-Tag Verification (Spec §14.6)
**Time**: ~15 min

1. Create `src-tauri/src/download/verifier.rs`
2. Re-open MP3 read-only:
   - APIC frame present ✓
   - Embedded image bytes SHA-256 matches input cover ✓
   - Image dimensions ≥ 300×300 ✓
3. If any check fails: delete partial file, return error
4. Optionally write `cover.jpg` to folder if settings.write_cover_jpg is true

**Output**: Verified MP3 or error + cleanup.

---

## Step 8 — Orchestrator + IPC Command
**Time**: ~30 min

1. Create `src-tauri/src/download/pipeline.rs`
2. Orchestrate full flow:
   ```
   resolve_track → search_youtube → score → pick_best
   → download_mp3 → fetch_cover → tag_mp3 → verify
   ```
3. IPC command: `download_track(track: SpotifyTrack) -> Result<DownloadResult>`
4. Emit events at each stage: `download:state { stage: "searching" | "downloading" | "converting" | "tagging" | "verifying" | "done" | "failed" }`
5. Emit progress: `download:progress { percent, speed_kbps, eta_seconds }`
6. Return: `DownloadResult { output_path, duration_ms, file_size_bytes }`

**Output**: Single IPC command that runs the full pipeline.

---

## Step 9 — Wire to Frontend
**Time**: ~30 min

1. Add `downloadTrack` to `libs/tauri-ipc/src/`
2. Listen to `download:state` and `download:progress` events via `@tauri-apps/api/event`
3. Update Input page "Download" button:
   - Click → call `downloadTrack(tracks[0])` (single track for now)
   - Show progress bar during download
   - Show state transitions: Searching → Downloading 34% → Converting → Tagging → Done ✓
   - Error state with retry button
4. On success: show "Open folder" button via `tauri_plugin_shell::open`

**Output**: Full end-to-end flow from paste URL → download MP3 with cover art.

---

## Step 10 — Verify End-to-End
**Time**: ~20 min

1. Paste a Spotify track URL → Fetch → Download
2. Verify output MP3:
   - File exists at ~/Music/Spytfy/{album}/{NN} - {artist} - {title}.mp3
   - Open in media player — audio plays correctly
   - Check ID3 tags in foobar2000/VLC/mp3tag
   - Cover art visible in player
3. Test edge cases: long title, special chars in name, very short track
4. Test failure: invalid YT match, network down mid-download

**Output**: Phase 3 complete.

---

## Dependency Graph

```
Step 1 (binaries) → Step 2 (YT search) → Step 3 (scoring) → Step 8 (orchestrator)
                    Step 4 (download)   → Step 8
                    Step 5 (cover art)  → Step 6 (tagging) → Step 7 (verify) → Step 8
Step 8 (orchestrator) → Step 9 (frontend) → Step 10 (verify)
```

## New Crates Needed

- `strsim` — Levenshtein distance for title matching
- `image` — Read image dimensions for cover validation
- `reqwest` — HTTP client for cover art download (already pulled in by rspotify)
- `sha2` — SHA-256 for post-tag verification (already in deps tree)
- `id3` — MP3 ID3 tagging

---

## Acceptance Criteria

- [ ] yt-dlp + ffmpeg bundled as Tauri sidecars
- [ ] YouTube search returns candidates with metadata
- [ ] Scoring picks correct match ≥80% of the time
- [ ] MP3 downloaded at 320kbps
- [ ] Cover art embedded (≥300×300, real Spotify cover, not placeholder)
- [ ] All ID3 tags correct (title, artist, album, track#, year, ISRC)
- [ ] Post-tag verification passes
- [ ] UI shows progress through pipeline stages
- [ ] Output file plays correctly in media players
