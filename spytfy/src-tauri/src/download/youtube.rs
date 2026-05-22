use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct YtCandidate {
    pub id: String,
    pub title: String,
    pub duration_secs: u64,
    pub uploader: String,
    pub channel_id: String,
}

pub async fn search_youtube(
    yt_dlp_path: &str,
    artist: &str,
    title: &str,
) -> Result<Vec<YtCandidate>, String> {
    let query = format!("{artist} {title}");
    let output = Command::new(yt_dlp_path)
        .args([
            &format!("ytsearch5:{query}"),
            "--print",
            "%(id)s|%(title)s|%(duration)s|%(uploader)s|%(channel_id)s",
            "--no-download",
            "--no-warnings",
            "--no-playlist",
            "--sleep-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    // Don't check exit code — yt-dlp returns non-zero if ANY result has an error
    // (e.g., unavailable video) but still outputs valid results for others
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If no stdout at all, then it truly failed
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp search failed: {stderr}"));
    }
    let candidates: Vec<YtCandidate> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(5, '|').collect();
            if parts.len() < 5 {
                return None;
            }
            Some(YtCandidate {
                id: parts[0].to_string(),
                title: parts[1].to_string(),
                duration_secs: parts[2].parse().unwrap_or(0),
                uploader: parts[3].to_string(),
                channel_id: parts[4].to_string(),
            })
        })
        .collect();

    Ok(candidates)
}
