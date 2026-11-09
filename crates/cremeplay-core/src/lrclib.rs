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
    #[serde(default)]
    pub romanized: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsPayload {
    pub synced: bool,
    pub source: String, // "LRCLIB" | "Better-Lyrics" | "YouTube Captions" | "YouTube Music" | "Custom" | "None"
    pub instrumental: bool,
    pub lines: Vec<LyricLine>,
    pub plain_lyrics: String,
    #[serde(default)]
    pub available_sources: Vec<String>,
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
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
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
                    romanized: None,
                }],
                plain_lyrics: "Instrumental".to_string(),
                available_sources: vec!["LRCLIB".to_string()],
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
                    available_sources: vec!["LRCLIB".to_string()],
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
                    available_sources: vec!["LRCLIB".to_string()],
                });
            }
        }

        None
    }

    pub async fn fetch_better_lyrics(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_sec: Option<f64>,
        video_id: Option<&str>,
    ) -> Option<LyricsPayload> {
        let clean_title = clean_track_title(title);
        let clean_artist = clean_artist_name(artist);
        if clean_title.trim().is_empty() {
            return None;
        }

        // 1. Try official Better-Lyrics API with full track parameters
        let mut params = vec![
            ("s", clean_title.clone()),
            ("a", clean_artist.clone()),
        ];
        if let Some(alb) = album {
            if !alb.trim().is_empty() {
                params.push(("al", alb.trim().to_string()));
            }
        }
        if let Some(dur) = duration_sec {
            if dur > 1.0 {
                params.push(("d", dur.round().to_string()));
            }
        }

        if let Ok(url) = Url::parse_with_params("https://api.betterlyrics.org/getLyrics", &params) {
            if let Ok(resp) = self.client.get(url)
                .timeout(Duration::from_millis(3000))
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        if let Some(ttml) = body.get("ttml").and_then(|t| t.as_str()) {
                            let lines = parse_ttml(ttml);
                            if !lines.is_empty() {
                                let plain = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
                                return Some(LyricsPayload {
                                    synced: true,
                                    source: "Better-Lyrics".to_string(),
                                    instrumental: false,
                                    lines,
                                    plain_lyrics: plain,
                                    available_sources: vec!["Better-Lyrics".to_string()],
                                });
                            }
                        }
                    }
                }
            }
        }

        // 2. Unison crowdsourced Better-Lyrics fallback if video_id available
        if let Some(vid) = video_id {
            let unison_url = format!("https://unison.betterlyrics.org/lyrics?v={}", vid);
            if let Ok(resp) = self.client.get(&unison_url)
                .timeout(Duration::from_millis(2500))
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        if let Some(lrc) = body.get("lyrics").or_else(|| body.get("data").and_then(|d| d.get("lyrics"))).and_then(|l| l.as_str()) {
                            let lines = parse_lrc(lrc);
                            if !lines.is_empty() {
                                let plain = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
                                return Some(LyricsPayload {
                                    synced: true,
                                    source: "Better-Lyrics".to_string(),
                                    instrumental: false,
                                    lines,
                                    plain_lyrics: plain,
                                    available_sources: vec!["Better-Lyrics".to_string()],
                                });
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub async fn fetch_genius_lyrics(
        &self,
        title: &str,
        artist: &str,
    ) -> Option<LyricsPayload> {
        let clean_title = clean_track_title(title);
        let clean_artist = clean_artist_name(artist);
        if clean_title.trim().is_empty() {
            return None;
        }

        let query = format!("{} {}", clean_title, clean_artist);
        let url = Url::parse_with_params("https://genius.com/api/search/multi", &[("q", &query)]).ok()?;
        let resp = self.client.get(url)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_millis(3000))
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let body: serde_json::Value = resp.json().await.ok()?;
        let sections = body.get("response")?.get("sections")?.as_array()?;
        let mut song_url = None;

        for section in sections {
            if let Some(hits) = section.get("hits").and_then(|h| h.as_array()) {
                for hit in hits {
                    if hit.get("type").and_then(|t| t.as_str()) == Some("song") {
                        if let Some(url) = hit.get("result").and_then(|r| r.get("url")).and_then(|u| u.as_str()) {
                            song_url = Some(url.to_string());
                            break;
                        }
                    }
                }
            }
            if song_url.is_some() {
                break;
            }
        }

        let song_url = song_url?;
        let page_resp = self.client.get(&song_url)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_millis(3500))
            .send()
            .await
            .ok()?;

        if !page_resp.status().is_success() {
            return None;
        }

        let html_text = page_resp.text().await.ok()?;
        let plain_lyrics = scrape_genius_html(&html_text);
        if plain_lyrics.trim().is_empty() {
            return None;
        }

        Some(LyricsPayload {
            synced: false,
            source: "Genius".to_string(),
            instrumental: false,
            lines: Vec::new(),
            plain_lyrics,
            available_sources: vec!["Genius".to_string()],
        })
    }
}

/// Helper function to parse user-uploaded custom lyrics (.lrc or plain text)
pub fn parse_custom_lyrics(content: &str) -> LyricsPayload {
    let lines = parse_lrc(content);
    if !lines.is_empty() {
        let plain = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
        LyricsPayload {
            synced: true,
            source: "Custom".to_string(),
            instrumental: false,
            lines,
            plain_lyrics: plain,
            available_sources: vec!["Custom".to_string()],
        }
    } else {
        LyricsPayload {
            synced: false,
            source: "Custom".to_string(),
            instrumental: false,
            lines: Vec::new(),
            plain_lyrics: content.trim().to_string(),
            available_sources: vec!["Custom".to_string()],
        }
    }
}

/// Scrapes text inside data-lyrics-container attributes from Genius HTML
pub fn scrape_genius_html(html: &str) -> String {
    let mut lyrics = Vec::new();
    let mut cursor = 0;

    while let Some(start_pos) = html[cursor..].find("data-lyrics-container=\"true\"") {
        let abs_start = cursor + start_pos;
        // Find closing tag of this container
        if let Some(tag_close) = html[abs_start..].find('>') {
            let inner_start = abs_start + tag_close + 1;
            if let Some(div_end) = html[inner_start..].find("</div>") {
                let inner = &html[inner_start..inner_start + div_end];
                cursor = inner_start + div_end + 6;

                // Replace <br/> or <br> with newlines
                let with_newlines = inner
                    .replace("<br/>", "\n")
                    .replace("<br />", "\n")
                    .replace("<br>", "\n");

                let clean = strip_xml_tags_and_entities(&with_newlines);
                if !clean.trim().is_empty() {
                    lyrics.push(clean);
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    lyrics.join("\n\n")
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
                romanized: None,
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

pub fn parse_ttml(ttml_text: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    let mut offset_sec = 0.0;
    if let Some(off_idx) = ttml_text.find("lyricOffset=\"") {
        let val_start = off_idx + 13;
        if let Some(quote_end) = ttml_text[val_start..].find('"') {
            if let Ok(off) = ttml_text[val_start..val_start + quote_end].parse::<f64>() {
                offset_sec = off;
            }
        }
    }

    let mut cursor = 0;

    while let Some(p_start) = ttml_text[cursor..].find("<p ") {
        let abs_start = cursor + p_start;
        let Some(end_tag_rel) = ttml_text[abs_start..].find("</p>") else {
            break;
        };
        let abs_end = abs_start + end_tag_rel;
        let tag_and_content = &ttml_text[abs_start..abs_end];
        cursor = abs_end + 4;

        if let Some(begin_idx) = tag_and_content.find("begin=\"") {
            let val_start = begin_idx + 7;
            if let Some(quote_end) = tag_and_content[val_start..].find('"') {
                let time_str = &tag_and_content[val_start..val_start + quote_end];
                if let Some(time_sec) = parse_ttml_timestamp(time_str) {
                    if let Some(tag_close) = tag_and_content.find('>') {
                        let inner_xml = &tag_and_content[tag_close + 1..];
                        let clean_text = strip_xml_tags_and_entities(inner_xml);
                        let final_line = clean_lrc_line(&clean_text);
                        let adjusted = (time_sec + offset_sec).max(0.0);
                        lines.push(LyricLine {
                            time_sec: (adjusted * 100.0).round() / 100.0,
                            text: final_line,
                            romanized: None,
                        });
                    }
                }
            }
        }
    }

    lines.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

fn parse_ttml_timestamp(raw: &str) -> Option<f64> {
    let s = raw.trim().trim_end_matches('s');
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let h: f64 = parts[0].trim().parse().ok()?;
        let m: f64 = parts[1].trim().parse().ok()?;
        let sec: f64 = parts[2].trim().parse().ok()?;
        Some(h * 3600.0 + m * 60.0 + sec)
    } else if parts.len() == 2 {
        let m: f64 = parts[0].trim().parse().ok()?;
        let sec: f64 = parts[1].trim().parse().ok()?;
        Some(m * 60.0 + sec)
    } else if parts.len() == 1 {
        parts[0].trim().parse::<f64>().ok()
    } else {
        None
    }
}

fn strip_xml_tags_and_entities(raw: &str) -> String {
    let mut res = String::with_capacity(raw.len());
    let mut in_tag = false;

    for ch in raw.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            res.push(ch);
        }
    }

    let decoded = res
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    decoded.trim().to_string()
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

    #[test]
    fn test_parse_ttml() {
        let ttml = r#"
<tt xmlns="http://www.w3.org/ns/ttml">
  <head>
    <metadata>
      <audio lyricOffset="0.25"/>
    </metadata>
  </head>
  <body>
    <div>
      <p begin="00:01.000" end="00:04.000"><span>Hello</span> <span>world</span></p>
      <p begin="00:05.500" end="00:08.000">Second line of lyrics</p>
    </div>
  </body>
</tt>
"#;
        let parsed = parse_ttml(ttml);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].time_sec, 1.25);
        assert_eq!(parsed[0].text, "Hello world");
        assert_eq!(parsed[1].time_sec, 5.75);
        assert_eq!(parsed[1].text, "Second line of lyrics");
    }

    #[test]
    fn test_parse_custom_lyrics_synced_and_plain() {
        let lrc = "[00:02.50]Synced lyric\n[00:06.00]Another synced line";
        let parsed_lrc = parse_custom_lyrics(lrc);
        assert_eq!(parsed_lrc.lines.len(), 2);
        assert_eq!(parsed_lrc.lines[0].time_sec, 2.50);
        assert_eq!(parsed_lrc.lines[0].text, "Synced lyric");

        let plain = "Line 1\nLine 2\nLine 3";
        let parsed_plain = parse_custom_lyrics(plain);
        assert!(!parsed_plain.synced);
        assert_eq!(parsed_plain.lines.len(), 0);
        assert_eq!(parsed_plain.plain_lyrics, "Line 1\nLine 2\nLine 3");
    }
}

