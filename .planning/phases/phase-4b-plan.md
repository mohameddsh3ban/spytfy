# Phase 4B — Screenshot Import for Playlists/Albums

**Date**: 2026-05-20
**Estimate**: 1.5 days
**Why**: Spotify API requires Premium. Screenshots bypass this entirely.

---

## Flow

1. Paste playlist/album URL → Fetch → gets name + cover from scraping
2. Preview shows "0 tracks" → "Import from Screenshots" button appears
3. Modal opens: drag-drop zone for screenshots
4. User drops 1+ screenshots of the Spotify playlist track list
5. Tesseract OCR extracts text from each image
6. Parser extracts structured data: track#, title, artist, album, duration
7. Navigate to playlist view: header (cover + name) + track table
8. User clicks "Download All" or selects individual tracks
9. Queue processes all tracks → output to `~/Music/Spytfy/{playlist_name}/`

---

## Step 1 — Tesseract Sidecar
**Time**: ~15 min

1. Download Tesseract Windows binary + eng.traineddata
2. Place in `src-tauri/binaries/tesseract-x86_64-pc-windows-gnu.exe`
3. Place trained data in `src-tauri/binaries/tessdata/eng.traineddata`
4. Add to `tauri.conf.json` externalBin

---

## Step 2 — OCR Module (Rust)
**Time**: ~30 min

1. Create `src-tauri/src/ocr/mod.rs`, `ocr.rs`
2. Run: `tesseract <image_path> stdout --psm 6` (assume uniform block of text)
3. Parse stdout → raw text lines
4. Return raw text for further parsing

---

## Step 3 — Track List Parser (Rust)
**Time**: ~30 min

1. Create `src-tauri/src/ocr/parser.rs`
2. Parse Spotify screenshot text format:
   - Each track row: `{number} {title} {artist} {album} {date} {duration}`
   - Duration format: `M:SS`
   - Artist might have commas (multiple artists)
3. Heuristics:
   - Lines starting with a number = track row
   - Duration at end of line (regex `\d+:\d{2}$`)
   - Album name before date (date format: `Mon DD, YYYY`)
   - Title is bold (first text after number), artist is below/after
4. Return `Vec<ParsedTrack { title, artist, album, duration_ms }>`

---

## Step 4 — IPC Commands
**Time**: ~10 min

1. `process_screenshots(image_paths: Vec<String>) -> Vec<ParsedTrack>`
2. `create_playlist_from_parsed(playlist_name, cover_url, tracks: Vec<ParsedTrack>) -> ResolvedInput`

---

## Step 5 — Screenshot Modal UI
**Time**: ~30 min

1. When playlist/album shows "0 tracks", show "Import from Screenshots" button
2. Modal: drag-drop zone for images, supports multiple
3. Shows thumbnail of each dropped image
4. "Process" button → calls OCR → shows progress
5. After processing: shows extracted track count
6. "Continue" → navigates to playlist view

---

## Step 6 — Playlist View Page
**Time**: ~40 min

1. New route: `/playlist/:id` or sub-component
2. Header: cover art + playlist name + track count + total duration
3. Table: # | Title | Artist | Album | Duration
4. Checkbox per track (all selected by default)
5. "Download All" / "Download Selected" buttons
6. Download → enqueues to queue → navigates to Downloads page

---

## Step 7 — Folder Naming + Metadata
**Time**: ~15 min

1. Output path: `{output_root}/{playlist_name}/{NN} - {artist} - {title}.mp3`
2. ID3 tag: set album to playlist name (or original album if known)
3. Track numbering: sequential within playlist

---

## Step 8 — Verify
**Time**: ~15 min

1. Paste playlist URL → Import screenshots → Process → See track list
2. Download All → tracks download to playlist folder
3. Check MP3 tags, folder structure, cover art

---

## Acceptance Criteria

- [ ] Tesseract OCR extracts text from Spotify screenshots
- [ ] Parser correctly identifies track title, artist, album, duration
- [ ] Playlist view shows extracted tracks in table format
- [ ] Download All enqueues all tracks to queue
- [ ] Output organized in playlist-named folder
- [ ] Works with multiple screenshots (long playlists)
