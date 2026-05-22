# Architecture Patterns: Android Port of Spytfy

**Domain:** Cross-platform Tauri 2 desktop-to-Android port
**Researched:** 2026-05-22
**Overall confidence:** MEDIUM (Tauri 2 Android is functional but documentation is sparse on edge cases)

## Recommended Architecture

### High-Level Overview

```
+----------------------------------------------------------+
|                    Android APK                            |
|                                                          |
|  +------------------+    +----------------------------+  |
|  | Angular 20       |    | Tauri Rust Library (.so)   |  |
|  | WebView Frontend |<-->| - Spotify resolver         |  |
|  | (System WebView) |IPC | - YouTube matcher/scorer   |  |
|  | Responsive UI    |    | - Queue manager            |  |
|  +------------------+    | - SQLite (SQLx)            |  |
|         ^                | - ID3 tagger               |  |
|         |                | - Cover art fetcher        |  |
|         v                +----------------------------+  |
|  +------------------+               |                    |
|  | Tauri Android    |               | JNI / Plugin       |
|  | Runtime (Kotlin) |               v                    |
|  | - WRY WebView    |    +----------------------------+  |
|  | - IPC Bridge     |    | Kotlin Plugin Layer        |  |
|  +------------------+    | - Binary executor          |  |
|                          | - Storage access           |  |
|                          | - Permissions manager      |  |
|                          +----------------------------+  |
|                                     |                    |
|                                     v                    |
|                          +----------------------------+  |
|                          | Bundled ARM Binaries       |  |
|                          | - yt-dlp + Python 3.8      |  |
|                          | - ffmpeg (ARM64 static)    |  |
|                          | (in jniLibs/ as .so files) |  |
|                          +----------------------------+  |
+----------------------------------------------------------+
```

### How Rust Runs on Android

**Confidence: HIGH** (verified via Tauri docs + DeepWiki)

Tauri 2 compiles Rust code as a **shared library** (`.so`) rather than a standalone binary on Android. The entry point uses `#[cfg_attr(mobile, tauri::mobile_entry_point)]` -- which the existing `lib.rs` already has. The compiled `.so` is placed in `gen/android/app/src/main/jniLibs/{abi}/lib{name}.so` and loaded by the Android runtime via JNI.

The existing `main.rs` (with `windows_subsystem = "windows"`) is only used for desktop builds. On Android, the library entry point in `lib.rs` is used directly.

**Critical change needed:** The `Cargo.toml` must add `cdylib` to `crate-type` for Android:

```toml
[lib]
name = "spytfy_lib"
crate-type = ["staticlib", "cdylib", "rlib"]
```

The `staticlib` + `cdylib` targets are needed for mobile; `rlib` remains for desktop.

## Component Boundaries

### Reusable Components (No Changes Needed)

| Component | File(s) | Why Reusable |
|-----------|---------|-------------|
| Spotify URL parser | `spotify/parser.rs` | Pure string parsing, no platform deps |
| Spotify types | `spotify/types.rs` | Data structures only |
| YouTube scorer | `download/scorer.rs` | Pure algorithm, no I/O |
| ID3 tagger | `download/tagger.rs` | Uses `id3` crate, cross-platform |
| Cover art fetcher | `download/cover.rs` | Uses `reqwest` + `image`, cross-platform |
| MP3 verifier | `download/verifier.rs` | Pure file I/O, cross-platform |
| Output path builder | `download/downloader.rs` (`build_output_path`) | Pure path logic |
| Spotify auth | `spotify/auth.rs` | Uses `rspotify`, cross-platform |
| Spotify resolver | `spotify/resolver.rs` | HTTP-based, cross-platform |
| Queue manager (logic) | `queue/manager.rs` | SQLx queries, cross-platform |
| Queue commands | `queue/commands.rs` | Tauri command wrappers |
| Settings commands | `commands/settings.rs` | SQLx queries |
| Database migrations | `migrations/` | SQLx migrations, cross-platform |

### Components Needing Android-Specific Implementation

| Component | Current Impl | Android Change | Severity |
|-----------|-------------|---------------|----------|
| **Sidecar resolution** | `pipeline.rs` + `worker.rs` `resolve_sidecar()` | Must use `nativeLibraryDir` path from Kotlin plugin | CRITICAL |
| **Binary execution** | `downloader.rs` `tokio::process::Command` | Must route through Kotlin plugin `Runtime.exec()` or use `nativeLibraryDir` exec | CRITICAL |
| **YouTube search** | `youtube.rs` uses yt-dlp subprocess | Same binary execution change needed | CRITICAL |
| **Database path** | `db.rs` uses `dirs::data_dir()` | `dirs` crate does NOT support Android; must use `app.path().app_data_dir()` from Tauri | HIGH |
| **Output directory** | `pipeline.rs` uses `dirs::audio_dir()` | Must use Android app-specific external storage or SAF | HIGH |
| **OCR module** | `ocr/` (entire module) | Excluded from Android build entirely | LOW (out of scope) |
| **App layout** | Sidebar-based desktop layout | Bottom nav / drawer for mobile | MEDIUM |

### New Components (Android-Only)

| Component | Purpose | Implementation |
|-----------|---------|---------------|
| **Binary Executor Plugin** | Execute yt-dlp/ffmpeg ARM binaries on Android | Kotlin Tauri plugin that manages `Runtime.exec()` calls |
| **Storage Manager Plugin** | Manage Android file paths and permissions | Kotlin plugin wrapping `getExternalFilesDir()` / MediaStore |
| **Platform Bridge** | Abstract platform-specific operations | Rust trait with desktop/mobile impls via `cfg(target_os)` |
| **Mobile Layout Component** | Bottom navigation bar for mobile | Angular component with responsive breakpoints |

## Data Flow: Download Pipeline (Android)

### Current Desktop Flow
```
Frontend invoke("download_track") 
  -> Rust: resolve sidecar path (Windows .exe)
  -> Rust: tokio::process::Command(yt-dlp) search YouTube
  -> Rust: tokio::process::Command(yt-dlp) download MP3
  -> Rust: tag MP3, verify
  -> Frontend: emit events for progress
```

### Android Flow (Proposed)
```
Frontend invoke("download_track")
  -> Rust: detect platform via cfg(target_os = "android")
  -> Rust: call Kotlin plugin via PluginHandle::run_mobile_plugin()
  -> Kotlin: resolve binary path from nativeLibraryDir
  -> Kotlin: Runtime.exec() yt-dlp with args, capture stdout/stderr
  -> Kotlin: return result to Rust via invoke.resolve()
  -> Rust: tag MP3, verify (same as desktop)
  -> Frontend: emit events for progress (same IPC)
```

### Alternative: Direct Exec from Rust on Android

There is a simpler alternative that avoids the Kotlin roundtrip:

```
Frontend invoke("download_track")
  -> Rust: detect platform via cfg(target_os = "android")
  -> Rust: resolve binary path (passed from Kotlin at app init via JNI)
  -> Rust: tokio::process::Command(binary_path) -- works if binary is in nativeLibraryDir
  -> Rust: tag, verify (identical to desktop)
```

**Recommendation:** Try direct `tokio::process::Command` first with the `nativeLibraryDir` path. If Android blocks it (W^X policy varies by OEM), fall back to the Kotlin plugin bridge. The `nativeLibraryDir` approach is proven by `youtubedl-android` which uses `Runtime.exec()` on extracted native libraries.

## Sidecar Binary Strategy (CRITICAL)

**Confidence: MEDIUM** (approach is proven by youtubedl-android, but integration with Tauri is novel)

### The Problem

Tauri's shell plugin on Android **only supports `open` (URL opening)** -- it cannot execute binaries. The `externalBin` sidecar feature is desktop-only. The existing code uses `tokio::process::Command` with Windows `.exe` paths.

### The Solution: Native Library Packaging

Package yt-dlp and ffmpeg as "native libraries" in the APK:

1. **Rename binaries** with `lib` prefix and `.so` suffix (Android requirement):
   - `yt-dlp` ARM64 binary -> `lib_ytdlp.so`  
   - `ffmpeg` ARM64 static binary -> `lib_ffmpeg.so`
   - Python 3.8 ARM64 -> `lib_python.so`

2. **Place in jniLibs directory**:
   ```
   gen/android/app/src/main/jniLibs/
     arm64-v8a/
       lib_ytdlp.so
       lib_ffmpeg.so
       lib_python.so
     armeabi-v7a/
       lib_ytdlp.so
       lib_ffmpeg.so
       lib_python.so
   ```

3. **Set `extractNativeLibs="true"`** in AndroidManifest.xml:
   ```xml
   <application android:extractNativeLibs="true" ...>
   ```

4. **Resolve path at runtime** via Kotlin:
   ```kotlin
   val libDir = applicationInfo.nativeLibraryDir
   val ytdlp = "$libDir/lib_ytdlp.so"
   ```

5. **Execute**:
   ```kotlin
   val process = Runtime.getRuntime().exec(
     arrayOf(ytdlp, "ytsearch5:query", "--print", "..."),
     arrayOf("LD_LIBRARY_PATH=$libDir"),
     workDir
   )
   ```

### Binary Sources

| Binary | Source | Size (ARM64) | Notes |
|--------|--------|-------------|-------|
| yt-dlp + Python | [youtubedl-android](https://github.com/yausername/youtubedl-android) | ~18MB | Proven, maintained, lazy extractors build |
| ffmpeg | [ffmpeg-binary-android](https://github.com/Khang-NT/ffmpeg-binary-android) or FFmpegKit | ~30MB | Static build, no dynamic deps |

### Alternative: youtubedl-android as Gradle Dependency

Instead of manually packaging binaries, use `youtubedl-android` as a Gradle dependency in the Tauri Android project. This library already handles:
- Bundling yt-dlp + Python 3.8
- FFmpeg integration (optional module)
- Binary extraction and execution
- yt-dlp updates at runtime

**Trade-off:** Adds a Java/Kotlin API layer that must be bridged to Rust via Tauri plugin, but eliminates binary packaging complexity.

**Recommendation:** Use `youtubedl-android` library. It is battle-tested, handles all the binary packaging/extraction/execution complexity, and supports runtime updates. Build a Tauri Kotlin plugin that wraps its API and exposes it to Rust.

## SQLite on Android

**Confidence: HIGH** (verified via multiple sources)

### The Problem

1. `dirs::data_dir()` returns `None` on Android (crate doesn't support `target_os = "android"`)
2. SQLx with bundled SQLite can have compilation issues on Android x86_64 targets (linker errors for 128-bit float intrinsics)
3. The current `db.rs` hardcodes `dirs::data_dir()` for the database path

### The Solution

1. **Replace `dirs::data_dir()`** with Tauri's `app.path().app_data_dir()`:
   ```rust
   // Before (desktop-only):
   let app_data_dir = dirs::data_dir().unwrap_or_else(|| ...);
   
   // After (cross-platform):
   let app_data_dir = app.path().app_data_dir()
       .expect("failed to resolve app data dir");
   ```

2. **Use SQLx `bundled` feature** for SQLite compilation:
   ```toml
   sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "bundled"] }
   ```
   The `bundled` feature compiles SQLite from source, avoiding dynamic linking issues. The x86_64 linker issue (issue #6047) was specific to older NDK versions and is resolved with current NDK (r26+).

3. **Android database path** resolves to: `/data/user/0/com.spytfy.app/files/db.sqlite`

### Migration Compatibility

The existing 4 SQLx migrations are pure SQL and will work identically on Android. No migration changes needed.

## IPC Differences: Desktop vs Android

**Confidence: HIGH** (verified via Tauri docs)

### Protocol Difference

| Aspect | Desktop | Android |
|--------|---------|---------|
| Transport | Custom protocol (`ipc://`) preferred | **postMessage only** (Android WebView limitation) |
| Payload efficiency | Binary data via custom protocol | JSON serialization only |
| Large payloads | Direct transfer | Must use Channel API |
| Events | Full support | Full support (same API) |
| Commands | Full support | Full support (same API) |

### Practical Impact

The existing IPC commands and events work identically on Android. The `invoke()` calls from Angular and `app.emit()` from Rust require **zero changes**. The transport difference is transparent to application code.

**One caveat:** If any command returns large binary data (>1MB), use the Channel API instead of direct return values. The current app returns small JSON payloads, so this is not an issue.

### Plugin API Gaps

| Plugin | Desktop | Android | Impact |
|--------|---------|---------|--------|
| `tauri-plugin-shell` | Full (spawn, execute, sidecar) | **open() only** | CRITICAL -- no binary execution |
| `tauri-plugin-fs` | Full | Read/write to app dirs + designated external dirs | HIGH -- output path changes |
| `tauri-plugin-dialog` | Full | Full | None |
| `tauri-plugin-store` | Full | Full | None |

## File Storage Strategy

**Confidence: MEDIUM**

### Output Directory Options

| Option | Path | Pros | Cons |
|--------|------|------|------|
| App-specific external | `/storage/emulated/0/Android/data/com.spytfy.app/Music/` | No permissions needed, survives app updates | Deleted on uninstall, not visible in music apps |
| Shared Music dir via MediaStore | `/storage/emulated/0/Music/Spytfy/` | Visible in music apps, survives uninstall | Requires MediaStore API, complex |
| Documents dir | `/storage/emulated/0/Documents/Spytfy/` | Simple, no special permissions on Android 10+ | Not standard for music |

**Recommendation:** Use **app-specific external storage** for v2.0 (simplest, no permissions). Add MediaStore integration in a future version for music app visibility. The path is accessible via Tauri's fs plugin.

### Database Storage

Store SQLite database in internal app data: `/data/user/0/com.spytfy.app/` via `app.path().app_data_dir()`. This is private, requires no permissions, and is the standard location.

## Platform Abstraction Pattern

### Recommended: `cfg(target_os)` Conditional Compilation

```rust
// src/platform.rs
#[cfg(not(target_os = "android"))]
pub mod imp {
    pub fn resolve_binary(name: &str, app: &tauri::AppHandle) -> Result<PathBuf, String> {
        // Desktop: existing sidecar resolution logic
    }
    
    pub fn get_data_dir(app: &tauri::AppHandle) -> PathBuf {
        dirs::data_dir().unwrap_or_else(|| ...)
    }
    
    pub fn get_output_dir(app: &tauri::AppHandle) -> PathBuf {
        dirs::audio_dir().unwrap_or_else(|| ...)
    }
}

#[cfg(target_os = "android")]
pub mod imp {
    pub fn resolve_binary(name: &str, app: &tauri::AppHandle) -> Result<PathBuf, String> {
        // Android: get path from Kotlin plugin
    }
    
    pub fn get_data_dir(app: &tauri::AppHandle) -> PathBuf {
        app.path().app_data_dir().expect("app data dir")
    }
    
    pub fn get_output_dir(app: &tauri::AppHandle) -> PathBuf {
        app.path().app_data_dir()
            .expect("app data dir")
            .join("Music")
    }
}

pub use imp::*;
```

### Alternative: Trait-Based Abstraction

```rust
pub trait Platform {
    fn resolve_binary(&self, name: &str) -> Result<PathBuf, String>;
    fn data_dir(&self) -> PathBuf;
    fn output_dir(&self) -> PathBuf;
}
```

**Recommendation:** Use `cfg(target_os)` modules -- simpler, no runtime dispatch, matches Tauri's own pattern (desktop.rs / mobile.rs in plugins).

## Angular Mobile Layout Strategy

**Confidence: HIGH** (standard responsive design patterns)

### Current Layout
- Desktop: Left sidebar (240px/72px collapsed) + main content area
- Horizontal navigation with 4 items: Home, Downloads, Library, Settings
- Keyboard shortcuts (Ctrl+1-4)

### Mobile Layout Changes

1. **Replace sidebar with bottom navigation bar** on mobile viewports:
   ```typescript
   // app.component.ts
   @Component({
     template: `
       <div class="shell" [class.mobile]="isMobile()">
         @if (!isMobile()) {
           <spytfy-sidebar />
         }
         <main class="main">
           <router-outlet />
         </main>
         @if (isMobile()) {
           <spytfy-bottom-nav />
         }
       </div>
     `
   })
   ```

2. **Breakpoint strategy:** Use `matchMedia('(max-width: 768px)')` or Tailwind's `md:` breakpoint
3. **Touch targets:** Minimum 48x48dp for all interactive elements
4. **No keyboard shortcuts** on mobile (HostListener for keydown can remain, just won't trigger)

### Pages Requiring Redesign

| Page | Desktop | Mobile Change |
|------|---------|--------------|
| Input | Wide URL input + preview cards | Full-width input, stacked cards |
| Downloads | Batch list with job details | Compact card list, expandable |
| Library | Grid/list of tracks | Single-column list |
| Settings | Form layout | Full-width stacked form |
| Onboarding | Center dialog | Full-screen flow |

## Build Pipeline

**Confidence: HIGH** (verified via Tauri docs)

### Prerequisites

```powershell
# Windows environment variables
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
$env:ANDROID_HOME = "$env:LocalAppData\Android\Sdk"
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\26.1.10909125"  # or latest

# Rust Android targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android
```

### Init Command

```bash
# From spytfy/src-tauri directory
cargo tauri android init
```

This generates `gen/android/` with:
```
gen/android/
  app/
    build.gradle.kts
    src/main/
      AndroidManifest.xml
      java/com/spytfy/app/
        MainActivity.kt
      jniLibs/           # ARM binaries go here
      res/               # Icons, splash
  build.gradle.kts
  gradle.properties
  settings.gradle.kts
```

### Development

```bash
cargo tauri android dev          # Run on connected device/emulator
cargo tauri android dev --open   # Open in Android Studio
```

### Release Build

```bash
cargo tauri android build --apk  # Build APK
cargo tauri android build        # Build both APK + AAB
```

Output: `gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`

### Signing

```bash
# Generate keystore
keytool -genkey -v -keystore spytfy-release.keystore -alias spytfy -keyalg RSA -keysize 2048 -validity 10000

# Configure in gen/android/app/build.gradle.kts
# (signing config section)
```

### Platform-Specific Config

Create `src-tauri/tauri.android.conf.json`:
```json
{
  "bundle": {
    "android": {
      "minSdkVersion": 31
    }
  },
  "app": {
    "security": {
      "csp": null
    }
  }
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Using `dirs` Crate on Android
**What:** Calling `dirs::data_dir()`, `dirs::audio_dir()`, etc. on Android
**Why bad:** Returns `None` or incorrect Linux fallback paths. App crashes or writes to wrong location.
**Instead:** Use `app.path().app_data_dir()` from Tauri's path API, which resolves correctly on all platforms.

### Anti-Pattern 2: Direct `tokio::process::Command` with Hardcoded Paths
**What:** Calling `Command::new("yt-dlp")` or using Windows `.exe` paths
**Why bad:** Binary not in PATH on Android, wrong file extension, execution may be blocked.
**Instead:** Resolve binary path from `nativeLibraryDir` and use full absolute path with `.so` extension.

### Anti-Pattern 3: Relying on Shell Plugin for Binary Execution
**What:** Using `app.shell().sidecar("yt-dlp")` on Android
**Why bad:** Shell plugin on Android only supports `open()`. Sidecar feature is desktop-only.
**Instead:** Build a custom Kotlin Tauri plugin that wraps `Runtime.exec()`.

### Anti-Pattern 4: Writing to Arbitrary File System Paths
**What:** Writing downloads to `/sdcard/Music/` or similar hardcoded paths
**Why bad:** Scoped storage on Android 12+ prevents this. Will get `FileNotFoundException`.
**Instead:** Use app-specific external storage or MediaStore API.

### Anti-Pattern 5: Blocking the Main Thread in Kotlin Plugin
**What:** Running yt-dlp process synchronously in a `@Command` method
**Why bad:** Causes ANR (Application Not Responding) dialog after 5 seconds.
**Instead:** Launch coroutines on `Dispatchers.IO` and use invoke.resolve() asynchronously.

## Scalability Considerations

| Concern | Phone (typical) | Tablet | Notes |
|---------|-----------------|--------|-------|
| Concurrent downloads | 1-2 max | 2-3 | CPU/memory limited, battery drain |
| SQLite connections | 2-3 max | 3-5 | Less RAM available |
| Queue size | Hundreds OK | Hundreds OK | SQLite handles this fine |
| Binary size (APK) | ~80MB total | Same | yt-dlp + ffmpeg + Python add ~50MB |
| Battery impact | Significant during download | Same | Consider battery optimization |

## Sources

- [Tauri 2 Sidecar Documentation](https://v2.tauri.app/develop/sidecar/) -- HIGH confidence
- [Tauri Shell Plugin](https://v2.tauri.app/plugin/shell/) -- HIGH confidence (Android: open only)
- [Tauri Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/) -- HIGH confidence
- [Tauri IPC Concepts](https://v2.tauri.app/concept/inter-process-communication/) -- HIGH confidence
- [Tauri Android Code Signing](https://v2.tauri.app/distribute/sign/android/) -- HIGH confidence
- [Tauri File System Plugin](https://v2.tauri.app/plugin/file-system/) -- HIGH confidence
- [DeepWiki: Tauri Mobile Architecture](https://deepwiki.com/tauri-apps/tauri/8.1-mobile-architecture-overview) -- MEDIUM confidence
- [youtubedl-android](https://github.com/yausername/youtubedl-android) -- HIGH confidence (proven library)
- [Execute Native Binaries on Android Q+](https://www.androidbugfix.com/2022/01/android-can-execute-process-for-android.html) -- MEDIUM confidence
- [Tauri Android SQLite Issue #6047](https://github.com/tauri-apps/tauri/issues/6047) -- MEDIUM confidence
- [Tauri File Management on Android](https://philrich.dev/tauri-fs-android/) -- MEDIUM confidence
- [Android Scoped Storage](https://developer.android.com/training/data-storage) -- HIGH confidence
