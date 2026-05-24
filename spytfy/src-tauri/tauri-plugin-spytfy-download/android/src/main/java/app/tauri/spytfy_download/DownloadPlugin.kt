package app.tauri.spytfy_download

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
import org.json.JSONArray
import java.io.File

@InvokeArg
class SearchArgs {
    lateinit var query: String
    var maxResults: Int = 5
}

@InvokeArg
class DownloadArgs {
    lateinit var videoId: String
    lateinit var outputPath: String
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
                runSpikeTest()
            } catch (e: Exception) {
                Log.e(TAG, "init failed", e)
            }
        }
    }

    private fun runSpikeTest() {
        scope.launch {
            val startTime = System.currentTimeMillis()
            Log.i(TAG, "=== SPIKE TEST START ===")

            // Checklist #3: Search
            try {
                val query = "rick astley never gonna give you up"
                Log.i(TAG, "SPIKE: Searching '$query'...")
                val req = YoutubeDLRequest("ytsearch5:$query")
                req.addOption("--dump-json")
                req.addOption("--flat-playlist")
                req.addOption("--no-warnings")
                val response = YoutubeDL.getInstance().execute(req)

                val results = mutableListOf<JSONObject>()
                response.out.lineSequence().forEach { line ->
                    if (line.isBlank()) return@forEach
                    runCatching {
                        results.add(JSONObject(line))
                    }
                }
                Log.i(TAG, "SPIKE: Search returned ${results.size} candidates")
                if (results.size >= 3) {
                    Log.i(TAG, "SPIKE CHECK #3: PASS (${results.size} >= 3 candidates)")
                } else {
                    Log.e(TAG, "SPIKE CHECK #3: FAIL (${results.size} < 3 candidates)")
                    return@launch
                }

                results.forEachIndexed { i, obj ->
                    Log.i(TAG, "SPIKE:   [$i] id=${obj.optString("id")} title=${obj.optString("title")} dur=${obj.optInt("duration")}s")
                }

                // Checklist #4-6: Download first result
                val videoId = results[0].optString("id")
                val cacheDir = activity.cacheDir.absolutePath
                val outputPath = "$cacheDir/spike-test.mp3"
                File(outputPath).delete() // clean previous

                Log.i(TAG, "SPIKE: Downloading videoId=$videoId to $outputPath")
                val dlReq = YoutubeDLRequest("https://youtube.com/watch?v=$videoId")
                dlReq.addOption("-f", "bestaudio")
                dlReq.addOption("-x")
                dlReq.addOption("--audio-format", "mp3")
                dlReq.addOption("--audio-quality", "320K")
                dlReq.addOption("-o", outputPath)
                dlReq.addOption("--no-mtime")

                var progressCount = 0
                YoutubeDL.getInstance().execute(dlReq, "spike-test") { progress, etaSec, _ ->
                    progressCount++
                    if (progressCount <= 5 || progressCount % 10 == 0) {
                        Log.i(TAG, "SPIKE: Progress #$progressCount: ${progress}% eta=${etaSec}s")
                    }
                }

                // Checklist #4: File exists
                val file = File(outputPath)
                if (file.exists()) {
                    Log.i(TAG, "SPIKE CHECK #4: PASS (file exists at $outputPath)")
                } else {
                    Log.e(TAG, "SPIKE CHECK #4: FAIL (file not found)")
                    return@launch
                }

                // Checklist #6: File size >= 1MB
                val sizeMB = file.length() / (1024.0 * 1024.0)
                if (file.length() >= 1_000_000) {
                    Log.i(TAG, "SPIKE CHECK #6: PASS (${String.format("%.2f", sizeMB)} MB)")
                } else {
                    Log.e(TAG, "SPIKE CHECK #6: FAIL (${file.length()} bytes, need >= 1MB)")
                }

                // Checklist #7: Progress callbacks >= 5
                if (progressCount >= 5) {
                    Log.i(TAG, "SPIKE CHECK #7: PASS ($progressCount progress callbacks)")
                } else {
                    Log.e(TAG, "SPIKE CHECK #7: FAIL ($progressCount < 5 callbacks)")
                }

                // Checklist #8: Total time < 90s
                val elapsed = (System.currentTimeMillis() - startTime) / 1000.0
                if (elapsed < 90) {
                    Log.i(TAG, "SPIKE CHECK #8: PASS (${String.format("%.1f", elapsed)}s < 90s)")
                } else {
                    Log.e(TAG, "SPIKE CHECK #8: FAIL (${String.format("%.1f", elapsed)}s >= 90s)")
                }

                Log.i(TAG, "=== SPIKE TEST COMPLETE === (${String.format("%.1f", elapsed)}s)")
                // Checklist #5 (plays in VLC) must be verified manually

            } catch (e: Exception) {
                Log.e(TAG, "SPIKE TEST FAILED: ${e.message}", e)
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

                val resultsArray = JSONArray()
                response.out.lineSequence().forEach { line ->
                    if (line.isBlank()) return@forEach
                    try {
                        val obj = JSONObject(line)
                        val item = JSONObject()
                        item.put("videoId", obj.optString("id"))
                        item.put("title", obj.optString("title"))
                        item.put("durationSec", obj.optInt("duration"))
                        item.put("channel", obj.optString("channel", obj.optString("uploader")))
                        resultsArray.put(item)
                    } catch (e: Exception) {
                        Log.w(TAG, "skipping malformed search line", e)
                    }
                }
                val ret = JSObject()
                ret.put("results", resultsArray)
                invoke.resolve(ret)
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

                val ret = JSObject()
                ret.put("filePath", args.outputPath)
                ret.put("processId", processId)
                invoke.resolve(ret)
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
