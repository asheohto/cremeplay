use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub time_sec: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsPayload {
    pub synced: bool,
    pub source: String, // "LRCLIB" | "YouTube Captions" | "YouTube Music" | "None"
    pub instrumental: bool,
    pub lines: Vec<LyricLine>,
    pub plain_lyrics: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibResponse {
    #[serde(default)]
    pub instrumental: bool,
    #[serde(default)]
    pub plain_lyrics: Option<String>,
    #[serde(default)]
    pub synced_lyrics: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
}

#[derive(Clone)]
pub struct LrclibClient {
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, LyricsPayload>>>,
}

impl LrclibClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("Cremeplay/0.1.0 (https://github.com/asheohto/cremeplay)")
            .build()
            .unwrap_or_default();

        Self {
            client,
            cache: Arc::new(Mutex::new(HashMap::with_capacity(64))),
        }
    }

    pub fn get_cached(&self, video_id: &str) -> Option<LyricsPayload> {
        let cache = self.cache.lock().unwrap();
        cache.get(video_id).cloned()
    }

    pub fn set_cached(&self, video_id: &str, payload: LyricsPayload) {
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= 64 {
            // Keep memory bound small: clear oldest half when cache exceeds 64 items
            let keys_to_remove: Vec<String> = cache.keys().take(32).cloned().collect();
            for k in keys_to_remove {
                cache.remove(&k);
            }
        }
        cache.insert(video_id.to_string(), payload);
    }

    pub async fn fetch_synced_lyrics(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_sec: Option<f64>,
    ) -> Option<LyricsPayload> {
        let clean_title = clean_track_title(title);
        let clean_artist = clean_artist_name(artist);

        if clean_title.trim().is_empty() {
            return None;
        }

        // 1. Direct GET lookup
        if let Some(payload) = self
            .get_exact(&clean_title, &clean_artist, album, duration_sec)
            .await
        {
            return Some(payload);
        }

        // 2. Search fallback
        if let Some(payload) = self
            .search_fallback(&clean_title, &clean_artist, duration_sec)
            .await
        {
            return Some(payload);
        }

        // 3. Fallback searching by title alone (in case artist string is messy or collab)
        if clean_title.len() >= 3 {
            if let Some(payload) = self
                .search_fallback(&clean_title, "", duration_sec)
                .await
            {
                return Some(payload);
            }
        }

        None
    }

    async fn get_exact(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_sec: Option<f64>,
    ) -> Option<LyricsPayload> {
        let mut params = vec![
            ("track_name", title.to_string()),
            ("artist_name", artist.to_string()),
        ];
        if let Some(alb) = album {
            if !alb.trim().is_empty() {
                params.push(("album_name", alb.trim().to_string()));
            }
        }
        if let Some(dur) = duration_sec {
            if dur > 1.0 {
                params.push(("duration", dur.round().to_string()));
            }
        }

        let url = Url::parse_with_params("https://lrclib.net/api/get", &params).ok()?;
        let resp = self.client.get(url).send().await.ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let data = resp.json::<LrclibResponse>().await.ok()?;
        Self::convert_lrclib_response(data)
    }

    async fn search_fallback(
        &self,
        title: &str,
        artist: &str,
        target_duration: Option<f64>,
    ) -> Option<LyricsPayload> {
        let params = [("track_name", title), ("artist_name", artist)];
        let url = Url::parse_with_params("https://lrclib.net/api/search", &params).ok()?;
        let resp = self.client.get(url).send().await.ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let results = resp.json::<Vec<LrclibResponse>>().await.ok()?;
        if results.is_empty() {
            // General query fallback: "clean_title clean_artist"
            let general_query = format!("{} {}", title, artist);
            let url2 = Url::parse_with_params("https://lrclib.net/api/search", &[("q", &general_query)]).ok()?;
            let resp2 = self.client.get(url2).send().await.ok()?;
            if resp2.status().is_success() {
                if let Ok(mut results2) = resp2.json::<Vec<LrclibResponse>>().await {
                    return Self::pick_best_candidate(&mut results2, target_duration);
                }
            }
            return None;
        }

        let mut candidate_list = results;
        Self::pick_best_candidate(&mut candidate_list, target_duration)
    }

    fn pick_best_candidate(
        candidates: &mut Vec<LrclibResponse>,
        target_duration: Option<f64>,
    ) -> Option<LyricsPayload> {
        if candidates.is_empty() {
            return None;
        }

        // Sort candidates:
        // 1. Has synced_lyrics first
        // 2. Minimum duration difference if duration is known
        candidates.sort_by(|a, b| {
            let a_has_synced = a.synced_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
            let b_has_synced = b.synced_lyrics.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);

            if a_has_synced != b_has_synced {
                return b_has_synced.cmp(&a_has_synced);
            }

            if let Some(target) = target_duration {
                let a_diff = a.duration.map(|d| (d - target).abs()).unwrap_or(9999.0);
                let b_diff = b.duration.map(|d| (d - target).abs()).unwrap_or(9999.0);
                return a_diff.partial_cmp(&b_diff).unwrap_or(std::cmp::Ordering::Equal);
            }

            std::cmp::Ordering::Equal
        });

        let best = candidates.first()?;
        Self::convert_lrclib_response(best.clone())
    }

    fn convert_lrclib_response(resp: LrclibResponse) -> Option<LyricsPayload> {
        if resp.instrumental {
            return Some(LyricsPayload {
                synced: true,
                source: "LRCLIB".to_string(),
                instrumental: true,
                lines: vec![LyricLine {
                    time_sec: 0.0,
                    text: "♪ Instrumental ♪".to_string(),
                }],
                plain_lyrics: "Instrumental".to_string(),
            });
        }

        if let Some(synced) = resp.synced_lyrics {
            let lines = parse_lrc(&synced);
            if !lines.is_empty() {
                let plain = resp.plain_lyrics.unwrap_or_else(|| {
                    lines
                        .iter()
                        .map(|l| l.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                });

                return Some(LyricsPayload {
                    synced: true,
                    source: "LRCLIB".to_string(),
                    instrumental: false,
                    lines,
                    plain_lyrics: plain,
                });
            }
        }

        if let Some(plain) = resp.plain_lyrics {
            if !plain.trim().is_empty() {
                return Some(LyricsPayload {
                    synced: false,
                    source: "LRCLIB".to_string(),
                    instrumental: false,
                    lines: Vec::new(),
                    plain_lyrics: plain,
                });
            }
        }

        None
    }
}

/// Strips common YouTube video noise, brackets, and badges from song titles.
pub fn clean_track_title(title: &str) -> String {
    let mut cleaned = title.to_string();

    let noise_patterns = [
        "(official music video)",
        "[official music video]",
        "(official video)",
        "[official video]",
        "(official audio)",
        "[official audio]",
        "(official lyric video)",
        "[official lyric video]",
        "(official visualizer)",
        "[official visualizer]",
        "(lyric video)",
        "[lyric video]",
        "(lyrics video)",
        "[lyrics video]",
        "(performance video)",
        "[performance video]",
        "(performance)",
        "[performance]",
        "(visualizer)",
        "[visualizer]",
        "(music video)",
        "[music video]",
        "(official 4k video)",
        "(animated video)",
        "[animated video]",
        "(live version)",
        "[live version]",
        "(live)",
        "[live]",
        "(audio)",
        "[audio]",
        "(lyrics)",
        "[lyrics]",
        "(video)",
        "[video]",
        "(official)",
        "[official]",
        "(mv)",
        "[mv]",
        "(hd)",
        "[hd]",
        "(hq)",
        "[hq]",
        "(4k)",
        "[4k]",
        "(remastered)",
        "[remastered]",
        "(extended version)",
        "[extended version]",
    ];

    for pattern in &noise_patterns {
        cleaned = remove_case_insensitive(&cleaned, pattern);
    }

    // Strip leading "Artist - " if present (e.g. "Coldplay - Yellow" -> "Yellow")
    if let Some(pos) = cleaned.find(" - ") {
        if pos > 0 && pos + 3 < cleaned.len() {
            let potential_title = cleaned[pos + 3..].trim();
            if !potential_title.is_empty() {
                cleaned = potential_title.to_string();
            }
        }
    }

    // Collapse multiple spaces into single space
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    words.join(" ").trim().to_string()
}

/// Strips YouTube Topic suffixes and splits multiple collaborating artists.
pub fn clean_artist_name(artist: &str) -> String {
    let mut cleaned = artist.trim().to_string();

    if let Some(stripped) = cleaned.strip_suffix(" - Topic") {
        cleaned = stripped.to_string();
    }

    // If bullet separator or comma/ampersand is present, take primary artist for search
    if let Some(first) = cleaned.split('•').next() {
        cleaned = first.to_string();
    }
    if let Some(first) = cleaned.split(" feat. ").next() {
        cleaned = first.to_string();
    }
    if let Some(first) = cleaned.split(" ft. ").next() {
        cleaned = first.to_string();
    }

    cleaned.trim().to_string()
}

fn remove_case_insensitive(haystack: &str, needle: &str) -> String {
    let mut result = String::with_capacity(haystack.len());
    let mut start = 0;
    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();

    while let Some(pos) = haystack_lower[start..].find(&needle_lower) {
        let abs_pos = start + pos;
        result.push_str(&haystack[start..abs_pos]);
        start = abs_pos + needle.len();
    }
    result.push_str(&haystack[start..]);
    result
}

/// Parses standard LRC formatted strings into sorted LyricLines.
pub fn parse_lrc(lrc_text: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    let mut offset_sec = 0.0;

    for raw_line in lrc_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        // Check for offset tag: [offset:+/-milliseconds]
        if line.starts_with("[offset:") && line.ends_with(']') {
            let offset_str = &line[8..line.len() - 1];
            if let Ok(offset_ms) = offset_str.trim().parse::<f64>() {
                offset_sec = offset_ms / 1000.0;
            }
            continue;
        }

        // Skip metadata header tags: [ar:], [ti:], [al:], [by:], [length:], etc.
        if line.starts_with("[ar:")
            || line.starts_with("[ti:")
            || line.starts_with("[al:")
            || line.starts_with("[by:")
            || line.starts_with("[length:")
            || line.starts_with("[re:")
            || line.starts_with("[ve:")
        {
            continue;
        }

        // Extract one or more timestamps at beginning of line: [mm:ss.xx] or [mm:ss:xx]
        let mut timestamps = Vec::new();
        let mut text_start_idx = 0;
        let bytes = line.as_bytes();
        let len = bytes.len();

        let mut idx = 0;
        while idx < len && bytes[idx] == b'[' {
            if let Some(close_idx) = line[idx..].find(']') {
                let abs_close = idx + close_idx;
                let tag = &line[idx + 1..abs_close];
                if let Some(time) = parse_timestamp(tag) {
                    timestamps.push(time);
                    idx = abs_close + 1;
                    text_start_idx = idx;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if timestamps.is_empty() {
            continue;
        }

        let lyric_text = line[text_start_idx..].trim();
        let final_text = clean_lrc_line(lyric_text);

        for t in timestamps {
            let adjusted_time = (t + offset_sec).max(0.0);
            lines.push(LyricLine {
                time_sec: (adjusted_time * 100.0).round() / 100.0,
                text: final_text.clone(),
            });
        }
    }

    lines.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

fn clean_lrc_line(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "♪".to_string();
    }
    let lower = trimmed.to_lowercase();
    if lower == "[music]"
        || lower == "(music)"
        || lower == "[musique]"
        || lower == "(musique)"
        || lower == "[música]"
        || lower == "(música)"
        || lower == "[♪]"
        || lower == "♪"
    {
        return "♪".to_string();
    }
    let mut cleaned = trimmed.to_string();
    for tag in &[
        "[music]", "[Music]", "[MUSIC]",
        "(music)", "(Music)", "(MUSIC)",
        "[musique]", "[Musique]",
        "[música]", "[Música]",
    ] {
        cleaned = cleaned.replace(tag, "");
    }
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let final_res = collapsed.trim();
    if final_res.is_empty() {
        "♪".to_string()
    } else {
        final_res.to_string()
    }
}

fn parse_timestamp(tag: &str) -> Option<f64> {
    // Formats: mm:ss.xx, mm:ss.xxx, mm:ss:xx
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() == 2 {
        let minutes: f64 = parts[0].trim().parse().ok()?;
        let seconds: f64 = parts[1].trim().parse().ok()?;
        Some(minutes * 60.0 + seconds)
    } else if parts.len() == 3 {
        // e.g. 01:23:45 -> 1 min, 23 sec, 45 hundredths
        let minutes: f64 = parts[0].trim().parse().ok()?;
        let seconds: f64 = parts[1].trim().parse().ok()?;
        let fraction: f64 = parts[2].trim().parse::<f64>().ok()? / 100.0;
        Some(minutes * 60.0 + seconds + fraction)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_track_title() {
        assert_eq!(
            clean_track_title("Bohemian Rhapsody (Official Video)"),
            "Bohemian Rhapsody"
        );
        assert_eq!(
            clean_track_title("Never Gonna Give You Up (Official Music Video) [HD]"),
            "Never Gonna Give You Up"
        );
        assert_eq!(
            clean_track_title("Starboy (Lyric Video)"),
            "Starboy"
        );
    }

    #[test]
    fn test_clean_artist_name() {
        assert_eq!(clean_artist_name("Queen - Topic"), "Queen");
        assert_eq!(clean_artist_name("Daft Punk • Pharrell Williams"), "Daft Punk");
        assert_eq!(clean_artist_name("The Weeknd feat. Daft Punk"), "The Weeknd");
    }

    #[test]
    fn test_parse_lrc_standard() {
        let lrc = r#"
[ti:Bohemian Rhapsody]
[ar:Queen]
[00:00.80]Is this the real life?
[00:04.50]Is this just fantasy?
[00:09.12]Caught in a landslide
"#;
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].time_sec, 0.80);
        assert_eq!(parsed[0].text, "Is this the real life?");
        assert_eq!(parsed[1].time_sec, 4.50);
        assert_eq!(parsed[2].time_sec, 9.12);
    }

    #[test]
    fn test_parse_lrc_multi_timestamp() {
        let lrc = "[00:10.00][00:20.00]Chorus line";
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].time_sec, 10.00);
        assert_eq!(parsed[0].text, "Chorus line");
        assert_eq!(parsed[1].time_sec, 20.00);
        assert_eq!(parsed[1].text, "Chorus line");
    }

    #[test]
    fn test_parse_lrc_offset_and_empty_line() {
        let lrc = r#"
[offset:500]
[00:01.00]First line
[00:05.00]
"#;
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].time_sec, 1.50);
        assert_eq!(parsed[0].text, "First line");
        assert_eq!(parsed[1].time_sec, 5.50);
        assert_eq!(parsed[1].text, "♪");
    }
}

