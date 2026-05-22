# Spytfy v2.0 Android — Plan Patch (Apply Before Phase 9)

This is a single document you can drop into your agent's context (Claude Code, GSD, etc.) so it knows exactly what changed in the plan, what to implement, and in what order. It is written as a delta against the existing planning documents in `.planning/` — your agent should read it alongside `PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md`, and `.planning/research/ANDROID-PLAN-REVIEW.md`.

---

## 0. Instructions for the agent

You are working on **Spytfy v2.0 Android port**, branch `android`. The desktop v1 is complete and feature-stable. Your job is to execute Phases 9–14 of the Android port.

Before you begin, **read in this order**:

1. `.planning/PROJECT.md` — project background
2. `.planning/REQUIREMENTS.md` — original requirements
3. `.planning/ROADMAP.md` — original phase plan
4. `.planning/research/ANDROID-PLAN-REVIEW.md` — initial expert review
5. **This document** — supersedes earlier guidance where they conflict

When this document and earlier planning docs disagree, **this document wins**. It incorporates research findings from late-stage review and provides code-level fixes that the earlier docs did not have.

After reading, post a short summary back to the human covering: (a) what this document changes vs. the original plan, (b) which phase you intend to start with, and (c) any clarifying questions before you begin. Do not start coding without confirmation.

---

## 1. Context refresher

**Goal:** Port Spytfy (Spotify URL → 320 kbps MP3 downloader) from desktop to Android as a native APK. ~80% of the existing Rust backend is reusable. The hard part is the binary execution layer — Tauri's shell plugin does not support spawning processes on Android.

**Stack:** Tauri 2 + Angular 20 + Rust + SQLite (existing). Adding: `io.github.junkfood02.youtubedl-android` (Kotlin lib) + custom Tauri Kotlin plugins (new).

**Distribution:** Signed APK, direct download or F-Droid. No Google Play submission in v2.0.

**Phases:** 9 (Scaffold) → 10 (Platform Abstraction) → 11 (Binary Execution Engine, HIGH RISK) → 12 (Pipeline Integration) → 13 (Mobile UI) → 14 (Storage + Background + Release).

---

## 2. What changed since the original plan

The earlier `ANDROID-PLAN-REVIEW.md` flagged Phase 11 as HIGH risk and listed open questions. Subsequent research resolved most of them and surfaced two new findings. Net effect:

| Topic | Original plan | Updated decision |
|---|---|---|
| youtubedl-android dependency | `com.github.yausername.youtubedl-android` | `io.github.junkfood02.youtubedl-android` (Maven Central mirror, same version, actively used by Seal — 26k★) |
| yt-dlp runtime update | "May need YTDLnis-style package system" | Built into the library — call `YoutubeDL.updateYoutubeDL()` on every app launch |
| Background work on API 34+ | `FOREGROUND_SERVICE_DATA_SYNC` | **UIDT JobScheduler** on API 34+, foreground service only as fallback for API 29–33 (because Android 15+ caps `dataSync` at 6h/24h) |
| Plan B if Phase 11 fails | "NewPipeExtractor + ffmpeg-kit" | ~~ffmpeg-kit was retired April 1, 2025~~. Plan B is now NewPipeExtractor + a community ffmpeg fork — significantly weaker, which means **Plan A must succeed**. Bundling ffmpeg via youtubedl-android sidesteps the entire ffmpeg crisis. |
| minSdkVersion | 31 (Android 12, ~76% coverage) | **29** (Android 10, ~92% coverage). Tauri's own Google Play example uses 28. |
| Phase 11 acceptance | "Start with proof-of-concept" | **8-point pass/fail checklist + hard kill criterion at end of day 2.** No sunk-cost extension. |
| Total requirements | 22 | **26** (4 added: DL-07, STOR-05, STOR-06, BUILD-07) |
| Timeline | 12–15 working days | **16–20 working days** (added: dual-spike day, UIDT impl, MediaStore IS_PENDING flow, slack budget) |

---

## 3. New & updated requirements

Replace the requirement count "22" with **26**. New rows below.

### Added requirements

| ID | Requirement | Phase |
|----|---|---|
| **DL-07** | yt-dlp binary is updatable at runtime via `YoutubeDL.updateYoutubeDL(STABLE)` on every app launch, without an APK rebuild. Failure to update is non-fatal — last-known-good binary continues to serve. | 11 |
| **STOR-05** | Downloads on API 34+ use UIDT JobScheduler (`setUserInitiated(true)`); API 29–33 use a `FOREGROUND_SERVICE_DATA_SYNC` foreground service. Implementation is version-gated at `DownloadOrchestrator`. | 14 |
| **STOR-06** | MediaStore writes use the `IS_PENDING=1 → stream bytes → IS_PENDING=0` flow so that ID3 tags written by the Rust tagger are preserved end-to-end. | 14 |
| **BUILD-07** | `minSdkVersion = 29` in `tauri.conf.json > bundle > android`. Targets ~92% of Android devices. | 9 |

### Modified requirements

- **BUILD-06** (release APK signed) — add: **keystore must be backed up to a password manager + 2 separate physical locations before first signed release**. Loss of keystore = inability to ship any updates to users who installed v2.0.x.
- **STOR-03** (background downloads) — replaces "foreground service" with "version-gated UIDT or foreground service" per STOR-05.

---

## 4. Stack updates

`.planning/research/STACK.md` should be updated:

```diff
- youtubedl-android 0.18.1 (yausername)
+ io.github.junkfood02.youtubedl-android:library 0.18.1
+ io.github.junkfood02.youtubedl-android:ffmpeg  0.18.1

- ffmpeg-kit (Plan B)
+ ffmpeg is bundled inside youtubedl-android — no separate ffmpeg dependency.
+ ffmpeg-kit was retired April 1, 2025 and is no longer a viable Plan B.

- minSdkVersion: 31
+ minSdkVersion: 29
```

---

## 5. The Phase 11 dual-spike — start here

Phase 11 is the milestone bottleneck. Do not begin Phase 9 implementation work until you have read this section.

### What you're proving

That a Tauri 2 Android app can call `io.github.junkfood02.youtubedl-android` through a custom Kotlin plugin and produce a playable MP3 file on a physical device, end-to-end, in under 2 working days.

### Test asset (constant across all spike attempts)

- URL: `https://www.youtube.com/watch?v=dQw4w9WgXcQ`
- Hardware: Samsung Galaxy S24 Ultra (ARM64, Android 14+). Primary test device for all phases. Use USB debugging.

### Pass/fail checklist (binary, no debate)

| # | Criterion | Verification |
|---|---|---|
| 1 | Tauri app installs and launches on device | `adb install`; app icon appears; UI renders |
| 2 | `YoutubeDL.getInstance().init()` completes without exception | logcat: "youtubedl-android ready" |
| 3 | `searchYoutube` for "rick astley never gonna give you up" returns ≥3 candidates | JSON in logcat with valid videoIds |
| 4 | `downloadAudio` produces a file at expected path | `adb shell ls /data/data/.../cache/` shows MP3 |
| 5 | Downloaded MP3 plays in VLC | audio audible, not corrupt |
| 6 | File size ≥1 MB for a 3-minute track at 320 kbps | `ls -la` |
| 7 | Progress callback fires ≥5 times | logcat count |
| 8 | End-to-end search→download finishes in <90s on Wi-Fi | wall clock |

### Kill criterion

**End of working day 2 of Phase 11.** If criteria 1–5 are not all PASS, stop. Do not extend. Surface the failure to the human with logs, then switch to fallback: NewPipeExtractor-based extraction with custom ffmpeg build (Plan B in `ANDROID-PLAN-REVIEW.md` §10, but note the ffmpeg-kit retirement complicates this).

---

## 6. Code templates the agent should use directly

These are the canonical patterns for each integration point. Use them as the starting point — do not invent variations unless you've tried these first.

### 6.1 Gradle (in `src-tauri/gen/android/app/build.gradle.kts` once Android scaffold exists)

```kotlin
dependencies {
    val youtubedlAndroid = "0.18.1"
    implementation("io.github.junkfood02.youtubedl-android:library:$youtubedlAndroid")
    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:$youtubedlAndroid")
    // No aria2c — saves ~5MB. Add later only if downloads are too slow without it.
}

android {
    packagingOptions {
        jniLibs {
            useLegacyPackaging = true  // == extractNativeLibs=true
        }
    }
}
```

### 6.2 AndroidManifest.xml additions

```xml
<!-- Required to extract bundled native libs -->
<application android:extractNativeLibs="true" ...>

<!-- Network -->
<uses-permission android:name="android.permission.INTERNET"/>
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/>

<!-- Notifications (foreground service AND UIDT both require this on API 33+) -->
<uses-permission android:name="android.permission.POST_NOTIFICATIONS"/>

<!-- Foreground service tier (API 29-33 fallback) -->
<uses-permission android:name="android.permission.FOREGROUND_SERVICE"/>
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC"/>

<!-- DO NOT add WRITE_EXTERNAL_STORAGE or MANAGE_EXTERNAL_STORAGE -->
<!-- DO NOT add READ_MEDIA_AUDIO -->

<application>
    <service
        android:name=".DownloadForegroundService"
        android:foregroundServiceType="dataSync"
        android:exported="false"/>
    <service
        android:name=".DownloadJobService"
        android:permission="android.permission.BIND_JOB_SERVICE"
        android:exported="false"/>
</application>
```

### 6.3 Cargo.toml deltas

```toml
[lib]
name = "spytfy_lib"
crate-type = ["staticlib", "cdylib", "rlib"]  # cdylib REQUIRED for Android

[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "bundled"] }

[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"
```

### 6.4 Tauri config (`src-tauri/tauri.conf.json`)

```json
{
  "bundle": {
    "android": {
      "minSdkVersion": 29
    }
  }
}
```

### 6.5 Kotlin Tauri plugin — `DownloadPlugin.kt`

This is the complete Phase 11 deliverable. Place at `src-tauri/tauri-plugin-spytfy-download/android/src/main/java/com/spytfy/download/DownloadPlugin.kt`.

```kotlin
package com.spytfy.download

import android.app.Activity
import android.util.Log
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import com.yausername.youtubedl_android.YoutubeDLException
import com.yausername.ffmpeg.FFmpeg
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.json.JSONObject

@InvokeArg
class SearchArgs {
    lateinit var query: String
    var maxResults: Int = 5
}

@InvokeArg
class DownloadArgs {
    lateinit var videoId: String
    lateinit var outputPath: String   // app-private temp file from Rust
    var bitrateKbps: Int = 320
}

@InvokeArg
class CancelArgs {
    lateinit var processId: String
}

@TauriPlugin
class DownloadPlugin(private val activity: Activity) : Plugin(activity) {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    @Volatile private var initialized = false

    override fun load(webView: WebView) {
        super.load(webView)
        scope.launch {
            try {
                YoutubeDL.getInstance().init(activity.application)
                FFmpeg.getInstance().init(activity.application)
                
                runCatching {
                    YoutubeDL.getInstance().updateYoutubeDL(
                        activity.application,
                        YoutubeDL.UpdateChannel.STABLE
                    )
                }.onFailure { Log.w(TAG, "yt-dlp update check failed", it) }
                
                initialized = true
                Log.i(TAG, "youtubedl-android ready")
            } catch (e: Exception) {
                Log.e(TAG, "init failed", e)
            }
        }
    }

    @Command
    fun searchYoutube(invoke: Invoke) {
        if (!initialized) { invoke.reject("youtubedl not initialized yet"); return }
        val args = invoke.parseArgs(SearchArgs::class.java)
        scope.launch {
            try {
                val req = YoutubeDLRequest("ytsearch${args.maxResults}:${args.query}")
                req.addOption("--dump-json")
                req.addOption("--flat-playlist")
                req.addOption("--no-warnings")
                val response = YoutubeDL.getInstance().execute(req)

                val results = mutableListOf<JSObject>()
                response.out.lineSequence().forEach { line ->
                    if (line.isBlank()) return@forEach
                    try {
                        val obj = JSONObject(line)
                        results.add(JSObject().apply {
                            put("videoId", obj.optString("id"))
                            put("title", obj.optString("title"))
                            put("durationSec", obj.optInt("duration"))
                            put("channel", obj.optString("channel", obj.optString("uploader")))
                        })
                    } catch (e: Exception) {
                        Log.w(TAG, "skipping malformed search line", e)
                    }
                }
                invoke.resolve(JSObject().apply { put("results", results) })
            } catch (e: YoutubeDLException) {
                invoke.reject("search failed: ${e.message}")
            }
        }
    }

    @Command
    fun downloadAudio(invoke: Invoke) {
        if (!initialized) { invoke.reject("youtubedl not initialized yet"); return }
        val args = invoke.parseArgs(DownloadArgs::class.java)
        val processId = "dl-${args.videoId}-${System.currentTimeMillis()}"

        scope.launch {
            try {
                val req = YoutubeDLRequest("https://youtube.com/watch?v=${args.videoId}")
                req.addOption("-f", "bestaudio")
                req.addOption("-x")
                req.addOption("--audio-format", "mp3")
                req.addOption("--audio-quality", "${args.bitrateKbps}K")
                req.addOption("-o", args.outputPath)
                req.addOption("--no-mtime")

                YoutubeDL.getInstance().execute(req, processId) { progress, etaSec, _ ->
                    trigger("download:progress", JSObject().apply {
                        put("processId", processId)
                        put("progress", progress)
                        put("etaSec", etaSec)
                    })
                }

                invoke.resolve(JSObject().apply {
                    put("filePath", args.outputPath)
                    put("processId", processId)
                })
            } catch (e: YoutubeDLException) {
                invoke.reject("download failed: ${e.message}")
            }
        }
    }

    @Command
    fun cancelDownload(invoke: Invoke) {
        val args = invoke.parseArgs(CancelArgs::class.java)
        YoutubeDL.getInstance().destroyProcessById(args.processId)
        invoke.resolve()
    }

    companion object { const val TAG = "SpytfyDownload" }
}
```

### 6.6 Background work orchestrator — `DownloadOrchestrator.kt`

Place at the same Kotlin package level. Used by Phase 14.

```kotlin
class DownloadOrchestrator(private val context: Context) {
    fun startBatchDownload(batchId: String, trackCount: Int, estimatedBytes: Long) {
        if (Build.VERSION.SDK_INT >= 34) {
            startAsUidtJob(batchId, estimatedBytes)
        } else {
            startAsForegroundService(batchId, trackCount)
        }
    }

    @RequiresApi(34)
    private fun startAsUidtJob(batchId: String, estimatedBytes: Long) {
        val networkRequest = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()
        val extras = PersistableBundle().apply { putString("batch_id", batchId) }
        val jobInfo = JobInfo.Builder(
            batchId.hashCode(),
            ComponentName(context, DownloadJobService::class.java)
        )
            .setUserInitiated(true)
            .setRequiredNetwork(networkRequest)
            .setEstimatedNetworkBytes(estimatedBytes, 0L)
            .setExtras(extras)
            .build()
        val scheduler = context.getSystemService(JobScheduler::class.java)
        if (scheduler.schedule(jobInfo) != JobScheduler.RESULT_SUCCESS) {
            startAsForegroundService(batchId, 0)
        }
    }

    private fun startAsForegroundService(batchId: String, trackCount: Int) {
        val intent = Intent(context, DownloadForegroundService::class.java).apply {
            putExtra("batch_id", batchId)
            putExtra("track_count", trackCount)
        }
        ContextCompat.startForegroundService(context, intent)
    }
}
```

### 6.7 UIDT JobService — `DownloadJobService.kt`

```kotlin
class DownloadJobService : JobService() {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    override fun onStartJob(params: JobParameters): Boolean {
        val batchId = params.extras.getString("batch_id") ?: return false
        val notification = buildProgressNotification(batchId, 0)
        setNotification(
            params, NOTIFICATION_ID, notification,
            JOB_END_NOTIFICATION_POLICY_REMOVE
        )

        scope.launch {
            try {
                runBatch(batchId, params)
                jobFinished(params, false)
            } catch (e: CancellationException) {
                jobFinished(params, true)
            } catch (e: Exception) {
                jobFinished(params, false)
            }
        }
        return true
    }

    override fun onStopJob(params: JobParameters): Boolean {
        scope.coroutineContext.cancelChildren()
        return true
    }

    private fun buildProgressNotification(batchId: String, progress: Int): Notification {
        TODO("implement notification builder")
    }

    private suspend fun runBatch(batchId: String, params: JobParameters) {
        TODO("implement batch runner")
    }

    companion object { const val NOTIFICATION_ID = 1001 }
}
```

### 6.8 Foreground service (API 29–33 fallback) — `DownloadForegroundService.kt`

```kotlin
class DownloadForegroundService : Service() {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val batchId = intent?.getStringExtra("batch_id") ?: return START_NOT_STICKY
        val notification = buildNotification(batchId)
        startForeground(NOTIFICATION_ID, notification)
        scope.launch {
            try {
                runBatch(batchId)
            } finally {
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
            }
        }
        return START_REDELIVER_INTENT
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onTimeout(startId: Int, fgsType: Int) {
        saveQueueState()
        stopSelf()
    }

    private fun saveQueueState() { /* persist current batch position */ }
    private fun buildNotification(batchId: String): Notification { TODO() }
    private suspend fun runBatch(batchId: String) { TODO() }

    companion object { const val NOTIFICATION_ID = 1002 }
}
```

### 6.9 MediaStore writer with IS_PENDING — `MediaStorePlugin.kt`

Critical: the Rust tagger MUST write ID3 tags **before** this is called. The file passed in `sourceTempPath` is already tagged.

```kotlin
@InvokeArg
class SaveArgs {
    lateinit var sourceTempPath: String
    lateinit var displayName: String
    lateinit var relativePath: String
}

@TauriPlugin
class MediaStorePlugin(private val activity: Activity) : Plugin(activity) {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    @Command
    fun saveToMusicLibrary(invoke: Invoke) {
        val args = invoke.parseArgs(SaveArgs::class.java)
        scope.launch {
            try {
                val resolver = activity.contentResolver
                val collection = MediaStore.Audio.Media.getContentUri(
                    MediaStore.VOLUME_EXTERNAL_PRIMARY
                )

                val pending = ContentValues().apply {
                    put(MediaStore.Audio.Media.DISPLAY_NAME, args.displayName)
                    put(MediaStore.Audio.Media.MIME_TYPE, "audio/mpeg")
                    put(MediaStore.Audio.Media.RELATIVE_PATH, args.relativePath)
                    put(MediaStore.Audio.Media.IS_PENDING, 1)
                }
                val uri = resolver.insert(collection, pending)
                    ?: return@launch invoke.reject("MediaStore insert returned null")

                resolver.openFileDescriptor(uri, "w")?.use { pfd ->
                    FileInputStream(args.sourceTempPath).use { input ->
                        FileOutputStream(pfd.fileDescriptor).use { output ->
                            input.copyTo(output)
                        }
                    }
                }

                val complete = ContentValues().apply {
                    put(MediaStore.Audio.Media.IS_PENDING, 0)
                }
                resolver.update(uri, complete, null, null)

                File(args.sourceTempPath).delete()
                invoke.resolve(JSObject().apply { put("uri", uri.toString()) })
            } catch (e: Exception) {
                invoke.reject("MediaStore save failed: ${e.message}")
            }
        }
    }
}
```

### 6.10 Rust platform abstraction — `src-tauri/src/download/youtube_android.rs`

Use `app.run_mobile_plugin(command_name, payload)` — the correct Tauri 2 mobile plugin API. The plugin handle is obtained via the plugin's extension trait on `AppHandle`.

```rust
#[cfg(target_os = "android")]
pub async fn search_youtube_android(
    app: &tauri::AppHandle,
    query: &str,
    max_results: u32,
) -> Result<Vec<Candidate>, DownloadError> {
    let handle = app.spytfy_download(); // plugin extension trait
    let response: SearchResponse = handle
        .run_mobile_plugin("searchYoutube", SearchArgs { query: query.into(), max_results })
        .map_err(DownloadError::from)?;
    Ok(response.results.into_iter().filter_map(|r| Candidate::try_from(r).ok()).collect())
}

#[cfg(target_os = "android")]
pub async fn download_audio_android(
    app: &tauri::AppHandle,
    video_id: &str,
    output_path: &Path,
    bitrate_kbps: u32,
) -> Result<PathBuf, DownloadError> {
    let handle = app.spytfy_download();
    let response: DownloadResponse = handle
        .run_mobile_plugin("downloadAudio", DownloadArgs {
            video_id: video_id.into(),
            output_path: output_path.to_string_lossy().into(),
            bitrate_kbps,
        })
        .map_err(DownloadError::from)?;
    Ok(PathBuf::from(response.file_path))
}
```

### 6.11 Platform-gated dispatcher in existing pipeline

```rust
// in src-tauri/src/platform.rs (new module from Phase 10)
pub async fn search_youtube(
    app: &tauri::AppHandle,
    query: &str,
    max_results: u32,
) -> Result<Vec<Candidate>, DownloadError> {
    #[cfg(target_os = "android")]
    return crate::download::youtube_android::search_youtube_android(app, query, max_results).await;

    #[cfg(not(target_os = "android"))]
    return crate::download::youtube::search_youtube_desktop(query, max_results).await;
}
```

Existing `scorer.rs`, `tagger.rs`, `cover.rs`, `verifier.rs`, `queue/*` — **do not modify**. They are already platform-agnostic.

---

## 7. Updated phase plan

### Phase 9 — Android Scaffold (1.5–2 days)

**Requirements:** BUILD-01, BUILD-02, BUILD-04, BUILD-05, **BUILD-07**

**Tasks:**
1. Install Android SDK, NDK r26+, JDK (Android Studio's JBR is fine).
2. Add Rust targets: `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`, `i686-linux-android`.
3. Update `Cargo.toml`: add `cdylib` crate-type, add `bundled` feature to SQLx (§6.3).
4. Run `cargo tauri android init`.
5. Replace `dirs::data_dir()` with `app.path().app_data_dir()` in `db.rs`. Do NOT do other platform abstraction here — that's Phase 10.
6. Set `minSdkVersion: 29` in `tauri.conf.json` (§6.4).
7. Run `cargo tauri android dev` against Samsung Galaxy S24 Ultra via USB debugging.

**Acceptance:** APK installs, Angular UI renders, SQLite tables created in app-private storage, no `UnsatisfiedLinkError` in logcat.

### Phase 10 — Platform Abstraction (0.5–1 day)

**Requirements:** BUILD-03

**Tasks:**
1. Create `src-tauri/src/platform.rs` with `cfg(target_os)` gates for: binary resolution, data dir, output dir, and the search/download dispatcher stubs (§6.11).
2. Replace all remaining `dirs::*` calls.
3. Run desktop build (`cargo tauri dev`) — must work unchanged.
4. Re-run Android build — must still boot.

**Acceptance:** Both platforms compile. Desktop functionality unchanged. Android scaffold still boots.

### Phase 11 — Binary Execution Engine ⚠️ HIGH RISK (3–5 days)

**Requirements:** DL-01, DL-02, DL-03, **DL-07**

**Day 1 (spike):**
1. Add Gradle deps (§6.1).
2. Add AndroidManifest entries (§6.2).
3. Generate Tauri plugin scaffold: `cargo tauri plugin new spytfy-download --android`.
4. Drop in `DownloadPlugin.kt` (§6.5). Stub Rust side enough to invoke once.
5. Hardcode test: search "rick astley never gonna give you up" from a button.

**Day 2 (acceptance):**
1. Wire `downloadAudio` end-to-end with the hardcoded test URL.
2. Verify against the 8-point checklist (§5).
3. **If criteria 1–5 pass: proceed.** If not: STOP, escalate, switch to Plan B.

**Day 3–5 (productionize):**
- Real Spotify-search-derived queries (replace hardcoded).
- Progress event wiring to Angular.
- Cancel/pause via `destroyProcessById`.
- Confirm `updateYoutubeDL` runs on launch.

**Acceptance:** All 8 checklist criteria PASS on Samsung Galaxy S24 Ultra.

### Phase 12 — Pipeline Integration (2–3 days)

**Requirements:** DL-04, DL-05, DL-06

**Tasks:**
1. Wire `platform::search_youtube` → existing `scorer.rs` → `platform::download_audio` → existing `tagger.rs` → existing `verifier.rs`.
2. Confirm reusable Rust tagger writes ID3v2.4 + cover art to the file that the Kotlin plugin downloaded.
3. Wire to existing queue manager for batch downloads.
4. Test pause/resume/cancel across the full pipeline.
5. Default Android concurrency to 1 (thermal/battery).

**Acceptance:** Spotify track URL → tagged MP3 with cover art in app-private temp dir. Album/playlist URL → batch downloads with per-track progress. SHA-256 verifier passes.

### Phase 13 — Mobile UI (2–3 days)

**Requirements:** UI-01 through UI-06

**Tasks:** Per `ROADMAP.md` original Phase 13. No changes from research findings.

**Acceptance:** Bottom nav with 4 tabs; URL input + resolve + preview; downloads page with 48dp touch targets; library page; settings; onboarding for Spotify credentials.

### Phase 14 — Storage, Background, Release (3–4 days)

**Requirements:** STOR-01, STOR-02, **STOR-05**, **STOR-06**, STOR-04, BUILD-06

**Tasks:**
1. Implement `MediaStorePlugin` (§6.9). Wire as last step of pipeline.
2. Implement `DownloadOrchestrator` (§6.6).
3. Implement `DownloadJobService` for UIDT (§6.7).
4. Implement `DownloadForegroundService` with `onTimeout` (§6.8).
5. Implement notification builder with progress (e.g., "Downloading 3/12 tracks").
6. ABI splits in Gradle for ARM64-only release APK.
7. Generate release keystore. **Back up keystore + password to password manager + 2 physical locations BEFORE first signed build.**
8. Sign release APK.

**Acceptance:**
- MP3s in `Music/Spytfy/{playlist}/` visible in Samsung Music, VLC, Poweramp.
- Downloads continue when app backgrounded or screen locked.
- Notification shows live progress.
- Signed APK installs on Android 10–16 without warnings.

---

## 8. Updated risk matrix

| Risk | Probability | Impact | Phase | Mitigation |
|---|---|---|---|---|
| Phase 11 dual-spike fails by end of day 2 | Low | Critical | 11 | Hard kill criterion; escalate to human, do not silently extend |
| Python 3.8 deprecated upstream forces APK rebuild | Medium | High | 11 | DL-07 keeps yt-dlp itself current; Python bundled and locked |
| YouTube breaks yt-dlp (every 2–6 weeks) | High | **Low** | Ongoing | `updateYoutubeDL` auto-pulls fix on next app launch |
| Android 15+ 6h dataSync limit | Medium | **Low** | 14 | UIDT on API 34+ has no 6h limit; `onTimeout` defense on FGS path |
| ANR from blocking plugin call | High | Medium | 11 | All ops on `CoroutineScope(Dispatchers.IO)` — non-negotiable |
| MediaStore strips ID3 tags | Low | Medium | 14 | IS_PENDING flow (§6.9): tag before insert, not after |
| Keystore loss | Low | **Critical** | 14 | 3-location backup mandatory before first signed release |
| Tauri Android quirks | Medium | Low | All | 15–20% slack budget per phase |
| Concurrent downloads thermal-throttle | High | Low | 12 | Default concurrency = 1 on Android; UI explains |
| ffmpeg-kit availability for Plan B | N/A (Plan A wins) | N/A | N/A | Sidestepped by bundling ffmpeg via youtubedl-android |

---

## 9. Open questions that are now resolved

- ~~Does youtubedl-android work inside Tauri's Gradle build?~~ → To be confirmed by Phase 11 spike. If pass-by-day-2 fails, stop.
- ~~Does MediaStore preserve ID3 tags?~~ → Yes, with the IS_PENDING flow (§6.9).
- ~~Android 14+ foreground service 6-hour limit?~~ → Use UIDT on API 34+ (§6.6, §6.7).
- ~~yt-dlp update path post-release?~~ → `YoutubeDL.updateYoutubeDL()` built-in.
- ~~APK size?~~ → Expect ~80MB single-ABI ARM64 release. ABI splits in Gradle.

## 10. Still-open questions for the agent to resolve in-flight

1. Does the `rspotify` dep chain require `openssl-sys` on Android? Verify in Phase 9 — switch to `rustls-tls` feature if so.
2. youtubedl-android thread safety for concurrent downloads — assume serial until proven otherwise. Document findings in Phase 12.
3. x86_64 emulator + SQLite + Python + ffmpeg — assume broken, use ARM64 device only.

---

## 11. Definition of done for v2.0

- All 26 requirements implemented and verified.
- Signed release APK builds reproducibly via `cargo tauri android build --release`.
- APK installs on physical devices running Android 10, 12, 14, and 15.
- Single hardcoded Spotify playlist URL → ≥10 tagged 320 kbps MP3s in `Music/Spytfy/` visible in VLC and one third-party music app.
- Download survives 5 minutes of screen-off + app-backgrounded.
- No `UnsatisfiedLinkError`, no `ForegroundServiceDidNotStopInTimeException`, no ANR.
- `updateYoutubeDL` succeeds at least once during testing window.
- Keystore + password backed up in 3 locations.
- README updated with Android install + Spotify credentials guidance.
- Desktop build still works unchanged.

---

## 12. Clarifications from expert review

### Physical device
Samsung Galaxy S24 Ultra (ARM64, Android 14+) confirmed as primary test device. Use USB debugging. S24 Ultra validates UIDT path (API 34+) on same device. For foreground-service fallback testing (API 29-33), use ARM64 emulator image at API 30 — Phase 14 concern only.

### Tauri plugin invoke API
`app.run_mobile_plugin(command_name, payload)` is the correct API. Section 6.10 code has been updated to use the actual Tauri 2 pattern via plugin extension trait. Defer to official docs at `v2.tauri.app/develop/plugins/develop-mobile/` and reference implementations in `tauri-plugin-dialog` or `tauri-plugin-notification` for canonical examples.

### Phase ordering
Execute sequentially: Phase 9 → 10 → 11. Read Phase 11 spike criteria (§5) before starting Phase 9 so scaffold decisions (device target, manifest, Cargo.toml structure) align with Phase 11 needs.

### minSdkVersion
BUILD-07 (`minSdkVersion = 29`) supersedes BUILD-01's original "API 31+" everywhere. Update any stale "API 31" references in older planning docs as encountered.

---

*This document supersedes earlier guidance in `.planning/ROADMAP.md` and `.planning/research/ANDROID-PLAN-REVIEW.md` where they conflict. Keep both docs for context; this one wins on disagreements.*
