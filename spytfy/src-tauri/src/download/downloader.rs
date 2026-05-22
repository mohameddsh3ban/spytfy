use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::AppHandle;
use tokio::process::Command;

use crate::spotify::types::SpotifyTrack;

fn sanitize_filename(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let trimmed = sanitized.trim().to_string();
    if trimmed.len() > 200 {
        trimmed[..200].to_string()
    } else {
        trimmed
    }
}

pub fn build_output_path(
    output_root: &Path,
    track: &SpotifyTrack,
    folder_name: &str,
    template: &str,
) -> PathBuf {
    let folder = sanitize_filename(if folder_name.is_empty() { &track.album } else { folder_name });
    let artist = sanitize_filename(&track.artists.join(", "));
    let title = sanitize_filename(&track.name);
    let album = sanitize_filename(&track.album);
    let number = format!("{:02}", track.track_number);

    let rendered = template
        .replace("{folder}", &folder)
        .replace("{artist}", &artist)
        .replace("{title}", &title)
        .replace("{album}", &album)
        .replace("{number}", &number);

    let path = output_root.join(format!("{rendered}.mp3"));
    path
}

pub async fn download_mp3(
    app: &AppHandle,
    yt_dlp_path: &str,
    yt_url: &str,
    output_path: &Path,
    bitrate: u16,
    job_id: &str,
    batch_id: &str,
) -> Result<(), String> {
    // Skip if already downloaded
    if output_path.exists() && output_path.metadata().map(|m| m.len() > 1000).unwrap_or(false) {
        return Ok(());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {e}"))?;
    }

    // Clean up stale partial downloads that cause [Errno 22] on resume
    let base_no_ext = output_path.with_extension("");
    let base_str = base_no_ext.to_string_lossy();
    if let Some(parent) = output_path.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.to_string_lossy();
                if name.starts_with(base_str.as_ref())
                    && (name.ends_with(".part") || name.ends_with(".webm") || name.ends_with(".webm.part"))
                {
                    eprintln!("[SPYTFY] Cleaning up stale file: {}", p.display());
                    std::fs::remove_file(&p).ok();
                }
            }
        }
    }

    let output_template = output_path
        .to_str()
        .ok_or("Invalid output path")?
        .to_string();

    // Retry up to 3 times with increasing backoff
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            let backoff = 5 * (attempt as u64 + 1);
            eprintln!("[SPYTFY] Download retry {attempt} for {yt_url}, waiting {backoff}s");
            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
        }
        match try_download(app, yt_dlp_path, yt_url, &output_template, bitrate, job_id, batch_id).await {
            Ok(()) => {
                if output_path.exists() {
                    return Ok(());
                }
                last_err = "Download completed but output file not found".to_string();
            }
            Err(e) => {
                last_err = e;
            }
        }
    }
    Err(last_err)
}

async fn try_download(
    _app: &AppHandle,
    yt_dlp_path: &str,
    yt_url: &str,
    output_template: &str,
    bitrate: u16,
    _job_id: &str,
    _batch_id: &str,
) -> Result<(), String> {
    let base = output_template.trim_end_matches(".mp3");
    let temp_template = format!("{base}.%(ext)s");

    // Use output() instead of spawn+pipe — avoids Windows pipe deadlocks
    // that cause [Errno 22] when tokio pipe readers block yt-dlp's stdout
    let output = Command::new(yt_dlp_path)
        .args([
            yt_url,
            "-x",
            "--audio-format",
            "mp3",
            "--audio-quality",
            &format!("{bitrate}K"),
            "-o",
            &temp_template,
            "--no-playlist",
            "--no-warnings",
            "--no-continue",
            "--force-overwrites",
            "--retries",
            "5",
            "--fragment-retries",
            "5",
            "--sleep-requests",
            "1.5",
            "--retry-sleep",
            "http:5",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_lines: Vec<&str> = stderr.lines()
            .filter(|l| l.contains("ERROR"))
            .collect();
        let detail = if error_lines.is_empty() {
            format!("exit code {:?}", output.status.code())
        } else {
            error_lines.join("; ")
        };
        return Err(format!("yt-dlp failed: {detail}"));
    }

    Ok(())
}
