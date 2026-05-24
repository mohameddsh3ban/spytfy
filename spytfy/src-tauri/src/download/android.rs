use serde::{Deserialize, Serialize};
use tauri::AppHandle;

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

    let response: SearchResponse = app
        .run_mobile_plugin("spytfy-download", "searchYoutube", SearchRequest {
            query,
            max_results: 5,
        })
        .map_err(|e| format!("Android search failed: {e}"))?;

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
    let response: DownloadResponse = app
        .run_mobile_plugin("spytfy-download", "downloadAudio", DownloadRequest {
            video_id: video_id.to_string(),
            output_path: output_path.to_string(),
            bitrate_kbps,
        })
        .map_err(|e| format!("Android download failed: {e}"))?;

    Ok(response.file_path)
}
