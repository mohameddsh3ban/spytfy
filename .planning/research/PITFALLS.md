# Domain Pitfalls: Android Port of Tauri 2 Desktop App

**Domain:** Tauri 2 desktop-to-Android port with sidecar binaries
**Researched:** 2026-05-22

## Critical Pitfalls

Mistakes that cause rewrites, crashes, or blocked progress.

### Pitfall 1: Shell Plugin Sidecar on Android
**What goes wrong:** Attempting to use `app.shell().sidecar("yt-dlp")` or `externalBin` on Android. The build succeeds but the sidecar never executes.
**Why it happens:** Tauri's shell plugin on Android only supports `open()` (URL opening). The sidecar/spawn/execute APIs are desktop-only. This is not documented prominently.
**Consequences:** The entire download pipeline is dead. No YouTube search, no MP3 download.
**Prevention:** Do NOT use shell plugin for binary execution on Android. Build a Kotlin Tauri plugin that wraps `Runtime.exec()` or integrate `youtubedl-android` as a Gradle dependency.
**Detection:** First `cargo tauri android dev` run -- nothing downloads. Shell plugin calls silently fail or return errors.

### Pitfall 2: `dirs` Crate Returns None on Android
**What goes wrong:** `dirs::data_dir()`, `dirs::audio_dir()`, `dirs::home_dir()` return `None` or incorrect Linux fallback paths on Android.
**Why it happens:** The `dirs` crate supports Linux/macOS/Windows/Redox. Android (`target_os = "android"`) is not explicitly supported -- it falls back to Linux conventions which don't map to Android's sandboxed filesystem.
**Consequences:** Database initialization crashes (`expect("failed to initialize database")`). Output files written to wrong or inaccessible location. App unusable.
**Prevention:** Replace all `dirs::*` calls with Tauri's `app.path().app_data_dir()` which resolves correctly on Android to `/data/user/0/com.spytfy.app/`. Use `cfg(target_os)` to provide platform-specific paths.
**Detection:** App crashes immediately on Android startup. SQLite pool creation fails.

### Pitfall 3: SQLite Compilation Failure on Android x86_64
**What goes wrong:** `sqlx` with `sqlite` feature fails to link on Android x86_64 emulator targets with undefined symbol errors (`__extenddftf2`, `__multf3`).
**Why it happens:** The bundled SQLite C code uses 128-bit float operations that some NDK toolchains don't provide intrinsics for. This is specific to x86_64 emulator targets, not ARM devices.
**Consequences:** Cannot build for Android emulator. Development iteration blocked.
**Prevention:** Use NDK r26+ (has the required intrinsics). Add `bundled` feature to SQLx. For x86_64 emulator issues, test on ARM64 emulator or physical device. Set `minSdkVersion = 31` in android config.
**Detection:** Linker errors during `cargo tauri android build` targeting x86_64.

### Pitfall 4: Android Scoped Storage Blocks File Access
**What goes wrong:** Writing MP3 files to `/storage/emulated/0/Music/` or similar "obvious" paths fails with `FileNotFoundException`.
**Why it happens:** Android 12+ enforces scoped storage. Apps cannot write to arbitrary external storage paths even with `WRITE_EXTERNAL_STORAGE` permission (which is ignored on API 30+).
**Consequences:** Downloads appear to succeed but files are not saved. Users cannot find their music.
**Prevention:** Use app-specific external storage (`getExternalFilesDir("Music")`) which requires no permissions. Or use MediaStore API for shared Music directory access (more complex). For v2.0, app-specific external storage is the right choice.
**Detection:** Files not appearing where expected. No error from Tauri fs plugin but files invisible in other apps.

## Moderate Pitfalls

### Pitfall 5: IPC Payload Size on Android
**What goes wrong:** Large JSON payloads in IPC commands/events fail silently or cause lag on Android.
**Why it happens:** Android uses `postMessage` for IPC (cannot use custom protocol). postMessage has practical size limits compared to the custom protocol used on desktop. Large playlist resolutions (100+ tracks) may hit this.
**Prevention:** Use Tauri's Channel API for large data transfers. Keep individual IPC payloads small. Paginate large playlist results.
**Detection:** Large playlist resolution silently fails or is very slow on Android but works on desktop.

### Pitfall 6: ANR (Application Not Responding) from Blocking Kotlin Plugin
**What goes wrong:** Android shows "Application Not Responding" dialog when download takes too long.
**Why it happens:** Kotlin `@Command` methods in Tauri plugins run on the main thread by default. If yt-dlp execution (which can take 30+ seconds) blocks this thread, Android kills the app.
**Prevention:** Always launch subprocess execution on `Dispatchers.IO` in Kotlin coroutines. Return immediately from the command and use events/callbacks for results.
**Detection:** ANR dialog appearing after ~5 seconds of download starting.

### Pitfall 7: Binary Naming Convention for Android
**What goes wrong:** ARM binaries packaged in APK don't extract or aren't executable.
**Why it happens:** Android's APK packaging requires native libraries to have `lib` prefix and `.so` suffix. Binaries without this naming are silently ignored during extraction. Also requires `extractNativeLibs="true"` in the manifest.
**Consequences:** Binaries not found at runtime. Download pipeline cannot start.
**Prevention:** Rename all binaries: `yt-dlp` -> `lib_ytdlp.so`, `ffmpeg` -> `lib_ffmpeg.so`. Set `extractNativeLibs="true"` in AndroidManifest.xml. Or use `youtubedl-android` library which handles all of this.
**Detection:** `File not found` errors when trying to execute binaries.

### Pitfall 8: Missing `cdylib` Crate Type
**What goes wrong:** Android build compiles but the `.so` library is empty or missing JNI symbols.
**Why it happens:** The current `Cargo.toml` has `crate-type = ["rlib"]` only. Android needs `cdylib` (and `staticlib`) for JNI symbol export.
**Consequences:** App crashes on launch with native library loading failure.
**Prevention:** Update `crate-type` to `["staticlib", "cdylib", "rlib"]`.
**Detection:** App crashes immediately on Android with `java.lang.UnsatisfiedLinkError`.

### Pitfall 9: Environment Variable Setup on Windows
**What goes wrong:** `cargo tauri android init` fails with "NDK_HOME not set" even though NDK is installed.
**Why it happens:** Windows environment variables set via PowerShell session don't persist to new terminals. Android Studio installs NDK in a version-specific subdirectory that must be referenced exactly.
**Prevention:** Set environment variables via System Properties (persistent) or `[System.Environment]::SetEnvironmentVariable(name, value, "User")`. Restart terminal/IDE after setting. Verify with `echo $env:NDK_HOME`.
**Detection:** `cargo tauri android init` errors about missing JAVA_HOME, ANDROID_HOME, or NDK_HOME.

## Minor Pitfalls

### Pitfall 10: Dev Server URL on Physical Device
**What goes wrong:** `cargo tauri android dev` on physical device shows blank screen.
**Why it happens:** Tauri's `devUrl: "http://localhost:4200"` doesn't work on physical devices (localhost refers to the phone, not the dev machine). Tauri auto-substitutes the machine's IP, but firewall may block it.
**Prevention:** Ensure development machine firewall allows port 4200 on the local network. Use an emulator for initial development.
**Detection:** Blank WebView with console errors about connection refused.

### Pitfall 11: Large APK Size
**What goes wrong:** APK is 80-100MB, too large for easy distribution.
**Why it happens:** yt-dlp (~18MB) + Python 3.8 + ffmpeg (~30MB) + Rust library + Angular bundle.
**Prevention:** Use ABI splits in Gradle to build per-architecture APKs instead of universal. ARM64-only reduces binary size by ~40%. Consider lazy-loading ffmpeg on first use.
**Detection:** APK size check after build.

### Pitfall 12: Concurrent Downloads Drain Battery
**What goes wrong:** Users complain about battery drain during large playlist downloads.
**Why it happens:** Desktop defaults to 3 concurrent workers. On mobile, each worker runs yt-dlp subprocess + network I/O + CPU transcoding simultaneously.
**Prevention:** Default concurrency to 1 on Android (detect via `cfg(target_os = "android")`). Let users increase manually. Show battery usage warning for large batches.
**Detection:** User reports or battery usage monitoring.

### Pitfall 13: WebView Compatibility
**What goes wrong:** Angular app renders incorrectly or CSS features don't work on some Android devices.
**Why it happens:** Tauri uses the system WebView (Android System WebView), which varies by device and Android version. Older devices may have outdated Chrome-based WebView.
**Prevention:** Set `minSdkVersion = 31` (Android 12), which guarantees Chrome 96+. Test on multiple Android versions. Avoid bleeding-edge CSS features.
**Detection:** Visual glitches or JavaScript errors on specific devices.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Build pipeline setup | Env vars not persisting (Pitfall 9) | Document exact setup steps, verify before proceeding |
| Android init | Missing crate-type (Pitfall 8) | Update Cargo.toml before `cargo tauri android init` |
| Database on Android | dirs crate failure (Pitfall 2) | Replace dirs calls first, before any Android testing |
| Binary execution | Shell plugin won't work (Pitfall 1) | Plan Kotlin plugin from day 1, don't try shell plugin |
| Download pipeline | ANR from blocking (Pitfall 6) | Design async plugin architecture upfront |
| File output | Scoped storage blocks (Pitfall 4) | Use app-specific dir, don't attempt /Music/ |
| Mobile UI | WebView compat (Pitfall 13) | Test on emulator with different API levels |
| Release build | APK size (Pitfall 11) | Add ABI splits early in Gradle config |

## Sources

- [Tauri Shell Plugin Platform Support](https://v2.tauri.app/plugin/shell/) -- HIGH confidence (Android: open only)
- [Tauri Sidecar Docs](https://v2.tauri.app/develop/sidecar/) -- HIGH confidence (desktop-only)
- [Tauri Android SQLite Issue #6047](https://github.com/tauri-apps/tauri/issues/6047) -- MEDIUM confidence
- [Android Scoped Storage](https://developer.android.com/training/data-storage) -- HIGH confidence
- [Tauri File Management on Android](https://philrich.dev/tauri-fs-android/) -- MEDIUM confidence
- [dirs crate](https://crates.io/crates/dirs) -- HIGH confidence (no Android support documented)
- [youtubedl-android](https://github.com/yausername/youtubedl-android) -- HIGH confidence
- [Tauri Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/) -- HIGH confidence
- [Execute Native Binaries Android Q+](https://www.androidbugfix.com/2022/01/android-can-execute-process-for-android.html) -- MEDIUM confidence
