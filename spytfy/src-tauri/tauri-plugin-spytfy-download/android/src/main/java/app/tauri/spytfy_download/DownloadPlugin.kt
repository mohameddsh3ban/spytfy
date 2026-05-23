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
