use strsim::normalized_levenshtein;

use super::youtube::YtCandidate;

#[derive(Debug, Clone)]
pub struct ScoredMatch {
    pub candidate: YtCandidate,
    pub score: i32,
    pub url: String,
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn score_candidates(
    spotify_artist: &str,
    spotify_title: &str,
    spotify_duration_ms: u64,
    candidates: &[YtCandidate],
) -> Option<ScoredMatch> {
    let spotify_dur_secs = (spotify_duration_ms / 1000) as i64;
    let expected = format!("{spotify_artist} {spotify_title}");
    let norm_expected = normalize(&expected);

    let has_duration = spotify_duration_ms > 0;

    let mut scored: Vec<ScoredMatch> = candidates
        .iter()
        .map(|c| {
            let mut score: i32 = 100;

            // Duration penalty (skip if no Spotify duration available)
            let dur_delta = (c.duration_secs as i64 - spotify_dur_secs).abs();
            if has_duration {
                score -= (dur_delta * 2) as i32;
            }

            // Title similarity
            let norm_title = normalize(&c.title);
            let similarity = normalized_levenshtein(&norm_title, &norm_expected);
            score -= ((1.0 - similarity) * 50.0) as i32;

            let title_lower = c.title.to_lowercase();
            let uploader_lower = c.uploader.to_lowercase();

            // Positive signals
            if title_lower.contains("official audio") {
                score += 15;
            }
            if title_lower.contains("topic") || uploader_lower.contains("vevo") {
                score += 20;
            }
            let artist_similarity =
                normalized_levenshtein(&uploader_lower, &normalize(spotify_artist));
            if artist_similarity >= 0.8 {
                score += 25;
            }

            // Negative signals
            if title_lower.contains("live") {
                score -= 25;
            }
            if title_lower.contains("cover") {
                score -= 40;
            }
            if title_lower.contains("remix") && !spotify_title.to_lowercase().contains("remix") {
                score -= 30;
            }
            if title_lower.contains("sped up")
                || title_lower.contains("slowed")
                || title_lower.contains("8d audio")
                || title_lower.contains("nightcore")
            {
                score -= 50;
            }
            if c.duration_secs > (spotify_dur_secs as u64 * 3 / 2) {
                score -= 30;
            }

            ScoredMatch {
                candidate: c.clone(),
                score,
                url: format!("https://www.youtube.com/watch?v={}", c.id),
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.cmp(&a.score));

    scored.into_iter().next().filter(|m| {
        if has_duration {
            let dur_delta = (m.candidate.duration_secs as i64 - spotify_dur_secs).abs();
            m.score >= 40 && dur_delta <= 10
        } else {
            m.score >= 30
        }
    })
}

pub fn score_all_candidates(
    spotify_artist: &str,
    spotify_title: &str,
    spotify_duration_ms: u64,
    candidates: &[YtCandidate],
) -> Vec<ScoredMatch> {
    let spotify_dur_secs = (spotify_duration_ms / 1000) as i64;
    let has_duration = spotify_duration_ms > 0;
    let expected = format!("{spotify_artist} {spotify_title}");
    let norm_expected = normalize(&expected);

    let mut scored: Vec<ScoredMatch> = candidates
        .iter()
        .map(|c| {
            let mut score: i32 = 100;
            if has_duration {
                let dur_delta = (c.duration_secs as i64 - spotify_dur_secs).abs();
                score -= (dur_delta * 2) as i32;
            }
            let norm_title = normalize(&c.title);
            let similarity = normalized_levenshtein(&norm_title, &norm_expected);
            score -= ((1.0 - similarity) * 50.0) as i32;

            let title_lower = c.title.to_lowercase();
            let uploader_lower = c.uploader.to_lowercase();
            if title_lower.contains("official audio") { score += 15; }
            if title_lower.contains("topic") || uploader_lower.contains("vevo") { score += 20; }
            let artist_similarity = normalized_levenshtein(&uploader_lower, &normalize(spotify_artist));
            if artist_similarity >= 0.8 { score += 25; }
            if title_lower.contains("live") { score -= 25; }
            if title_lower.contains("cover") { score -= 40; }
            if title_lower.contains("remix") && !spotify_title.to_lowercase().contains("remix") { score -= 30; }
            if title_lower.contains("sped up") || title_lower.contains("slowed") || title_lower.contains("8d audio") || title_lower.contains("nightcore") { score -= 50; }
            if has_duration && c.duration_secs > (spotify_dur_secs as u64 * 3 / 2) { score -= 30; }

            ScoredMatch {
                candidate: c.clone(),
                score,
                url: format!("https://www.youtube.com/watch?v={}", c.id),
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored
}
