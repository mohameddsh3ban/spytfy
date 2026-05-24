use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_spytfy_download::SpytfyDownloadExt;

use super::youtube::YtCandidate;

#[derive(Serialize)]
struct SearchRequest {
    query: String,
    #[serde(rename = "maxResults")]
    max_results: u32,
}

#[derive(Deserialize)]
struct SearchResultItem {
    #[serde(rename = "videoId")]
    video_id: String,
    title: String,
    #[serde(rename = "durationSec")]
    duration_sec: u64,
    channel: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchResultItem>,
}

#[derive(Serialize)]
struct DownloadRequest {
    #[serde(rename = "videoId")]
    video_id: String,
    #[serde(rename = "outputPath")]
    output_path: String,
    #[serde(rename = "bitrateKbps")]
    bitrate_kbps: u32,
}

#[derive(Deserialize)]
struct DownloadResponse {
    #[serde(rename = "filePath")]
    file_path: String,
}

pub async fn search_youtube_android(
    app: &AppHandle,
    artist: &str,
    title: &str,
) -> Result<Vec<YtCandidate>, String> {
    let query = format!("{artist} {title}");
    let plugin = app.spytfy_download();

    let response: SearchResponse = plugin.run_mobile_plugin("searchYoutube", SearchRequest {
        query,
        max_results: 5,
    })?;

    Ok(response
        .results
        .into_iter()
        .map(|r| YtCandidate {
            id: r.video_id,
            title: r.title,
            duration_secs: r.duration_sec,
            uploader: r.channel.clone(),
            channel_id: r.channel,
        })
        .collect())
}

pub async fn download_audio_android(
    app: &AppHandle,
    video_id: &str,
    output_path: &str,
    bitrate_kbps: u32,
) -> Result<String, String> {
    let plugin = app.spytfy_download();

    let response: DownloadResponse = plugin.run_mobile_plugin("downloadAudio", DownloadRequest {
        video_id: video_id.to_string(),
        output_path: output_path.to_string(),
        bitrate_kbps,
    })?;

    Ok(response.file_path)
}
