/**
 * YouTube Music Innertube API Client (Pure Rust)
 * Directly queries YouTube Music JSON APIs with full Google Account authentication support.
 */

use log::info;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackItem {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub duration_text: String,
    pub artwork_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSection {
    pub title: String,
    pub items: Vec<TrackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub name: String,
    pub avatar_url: String,
    pub is_logged_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub browse_id: String,
    pub title: String,
    pub description: String,
    pub subtitle: String,
    pub artwork_url: String,
    pub track_count: usize,
    pub tracks: Vec<TrackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidebarPlaylist {
    pub title: String,
    pub browse_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTopResult {
    pub title: String,
    pub subtitle: String,
    pub video_id: Option<String>,
    pub browse_id: Option<String>,
    pub artwork_url: String,
    pub result_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultPayload {
    pub top_result: Option<SearchTopResult>,
    pub songs: Vec<TrackItem>,
    pub library_tracks: Vec<TrackItem>,
}

pub struct InnertubeClient {
    client: reqwest::Client,
    cookies: Arc<Mutex<Option<String>>>,
}

impl InnertubeClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            ),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Self {
            client,
            cookies: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_cookies(&self, cookies: Option<String>) {
        *self.cookies.lock().unwrap() = cookies;
    }

    fn apply_auth_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let guard = self.cookies.lock().unwrap();
        if let Some(cookie_str) = guard.as_ref() {
            let mut r = req.header(COOKIE, cookie_str);

            if let Some(sapisid) = Self::extract_cookie(cookie_str, "SAPISID")
                .or_else(|| Self::extract_cookie(cookie_str, "__Secure-3PAPISID"))
            {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let origin = "https://music.youtube.com";
                let payload = format!("{} {} {}", now, sapisid, origin);
                let mut hasher = Sha1::new();
                hasher.update(payload.as_bytes());
                let hash = hasher.finalize();
                let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

                let auth_val = format!("SAPISIDHASH {}_{}", now, hash_hex);
                r = r
                    .header(AUTHORIZATION, auth_val)
                    .header("X-Origin", origin);
            }
            return r;
        }
        req
    }

    fn extract_cookie<'a>(cookies: &'a str, key: &str) -> Option<&'a str> {
        for part in cookies.split(';') {
            let trimmed = part.trim();
            if let Some(val) = trimmed.strip_prefix(key) {
                if let Some(eq_val) = val.strip_prefix('=') {
                    return Some(eq_val.trim());
                }
            }
        }
        None
    }

    pub async fn get_account_profile(&self) -> AccountProfile {
        let is_authed = self.cookies.lock().unwrap().is_some();
        if !is_authed {
            return AccountProfile {
                name: "Guest".to_string(),
                avatar_url: "".to_string(),
                is_logged_in: false,
            };
        }

        let url = "https://music.youtube.com/youtubei/v1/account/account_menu?prettyPrint=false";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            }
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        if let Ok(res) = req.send().await {
            if let Ok(val) = res.json::<Value>().await {
                let header = &val["actions"][0]["openPopupAction"]["popup"]["multiPageMenuRenderer"]["header"]["activeAccountHeaderRenderer"];
                let name = header["accountName"]["runs"][0]["text"].as_str().unwrap_or("You").to_string();
                let avatar = header["accountPhoto"]["thumbnails"].as_array().and_then(|t| t.last()).and_then(|t| t["url"].as_str()).unwrap_or("").to_string();

                return AccountProfile {
                    name,
                    avatar_url: avatar,
                    is_logged_in: true,
                };
            }
        }

        AccountProfile {
            name: "YouTube Music User".to_string(),
            avatar_url: "".to_string(),
            is_logged_in: true,
        }
    }

    pub async fn search_catalog(&self, query: &str) -> Result<(Option<SearchTopResult>, Vec<TrackItem>), String> {
        let url = "https://music.youtube.com/youtubei/v1/search?prettyPrint=false";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "query": query
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;

        let mut top_result = None;
        let mut songs = Vec::new();

        let sections = val["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]
            ["content"]["sectionListRenderer"]["contents"]
            .as_array();

        if let Some(sections) = sections {
            for section in sections {
                if let Some(card) = section.get("musicCardShelfRenderer") {
                    let title = card["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                    let subtitle = card["subtitle"]["runs"]
                        .as_array()
                        .map(|arr| arr.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join(""))
                        .unwrap_or_default();
                    let artwork_url = card["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                        .as_array()
                        .and_then(|arr| arr.last())
                        .and_then(|t| t["url"].as_str())
                        .unwrap_or("")
                        .to_string();
                    let video_id = card["onTap"]["watchEndpoint"]["videoId"].as_str().map(|s| s.to_string())
                        .or_else(|| {
                            card["buttons"].as_array()
                                .and_then(|btns| btns.get(0))
                                .and_then(|b| b["buttonRenderer"]["command"]["watchEndpoint"]["videoId"].as_str())
                                .map(|s| s.to_string())
                        });
                    let browse_id = card["onTap"]["browseEndpoint"]["browseId"].as_str().map(|s| s.to_string());
                    let result_type = if video_id.is_some() { "Song" } else if subtitle.to_lowercase().contains("artist") { "Artist" } else if subtitle.to_lowercase().contains("album") { "Album" } else { "Top Result" };
                    if !title.is_empty() {
                        top_result = Some(SearchTopResult {
                            title,
                            subtitle,
                            video_id,
                            browse_id,
                            artwork_url,
                            result_type: result_type.to_string(),
                        });
                    }
                }

                if let Some(items) = section["musicShelfRenderer"]["contents"].as_array() {
                    for item in items {
                        if let Some(track) = Self::parse_music_shelf_item(item) {
                            songs.push(track);
                        }
                    }
                }
                if let Some(items) = section["itemSectionRenderer"]["contents"].as_array() {
                    for item in items {
                        if let Some(track) = Self::parse_music_shelf_item(item) {
                            songs.push(track);
                        }
                    }
                }
            }
        }

        if songs.is_empty() {
            let direct_shelves = val["contents"]["sectionListRenderer"]["contents"].as_array();
            if let Some(shelves) = direct_shelves {
                for shelf in shelves {
                    if let Some(items) = shelf["musicShelfRenderer"]["contents"].as_array() {
                        for item in items {
                            if let Some(track) = Self::parse_music_shelf_item(item) {
                                songs.push(track);
                            }
                        }
                    }
                    if let Some(items) = shelf["itemSectionRenderer"]["contents"].as_array() {
                        for item in items {
                            if let Some(track) = Self::parse_music_shelf_item(item) {
                                songs.push(track);
                            }
                        }
                    }
                }
            }
        }

        Ok((top_result, songs))
    }

    pub async fn search_library(&self, query: &str) -> Result<Vec<TrackItem>, String> {
        let url = "https://music.youtube.com/youtubei/v1/search?prettyPrint=false";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "query": query,
            "params": "agIYBA%3D%3D"
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        let contents = val["contents"]["tabbedSearchResultsRenderer"]["tabs"][1]["tabRenderer"]
            ["content"]["sectionListRenderer"]["contents"]
            .as_array();

        if let Some(sections) = contents {
            for section in sections {
                if let Some(items) = section["musicShelfRenderer"]["contents"].as_array() {
                    for item in items {
                        if let Some(track) = Self::parse_music_shelf_item(item) {
                            results.push(track);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    pub async fn search(&self, query: &str) -> Result<SearchResultPayload, String> {
        let (catalog_res, library_res) = tokio::join!(
            self.search_catalog(query),
            self.search_library(query)
        );

        let (top_result, songs) = catalog_res.unwrap_or_default();
        let library_tracks = library_res.unwrap_or_default();

        info!(
            "[Innertube] Search for '{}' returned {} catalog songs and {} library songs",
            query,
            songs.len(),
            library_tracks.len()
        );

        Ok(SearchResultPayload {
            top_result,
            songs,
            library_tracks,
        })
    }

    pub async fn get_explore_feed(&self) -> Result<Vec<HomeSection>, String> {
        let url = "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false";

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "browseId": "FEmusic_explore"
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;
        let mut sections = Vec::new();

        let contents = val["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]
            ["tabRenderer"]["content"]["sectionListRenderer"]["contents"]
            .as_array();

        if let Some(contents) = contents {
            for section_val in contents {
                let carousel = &section_val["musicCarouselShelfRenderer"];
                let shelf = &section_val["musicShelfRenderer"];
                let target = if carousel.is_object() {
                    carousel
                } else if shelf.is_object() {
                    shelf
                } else {
                    continue;
                };

                let title = target["header"]["musicCarouselShelfBasicHeaderRenderer"]["title"]["runs"][0]["text"]
                    .as_str()
                    .or_else(|| target["header"]["musicShelfBasicHeaderRenderer"]["title"]["runs"][0]["text"].as_str())
                    .unwrap_or("Explore")
                    .to_string();

                let mut items = Vec::new();
                if let Some(shelf_items) = target["contents"].as_array() {
                    for item in shelf_items {
                        if let Some(track) = Self::parse_carousel_item(item) {
                            items.push(track);
                        }
                    }
                }

                if !items.is_empty() {
                    sections.push(HomeSection { title, items });
                }
            }
        }

        info!("[Innertube] Explore feed loaded {} sections", sections.len());
        Ok(sections)
    }

    pub async fn get_home_feed(&self) -> Result<Vec<HomeSection>, String> {
        let url = "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false";

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "browseId": "FEmusic_home"
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;
        let mut sections = Vec::new();

        let section_list = &val["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]
            ["tabRenderer"]["content"]["sectionListRenderer"];
        let contents = section_list["contents"].as_array();

        if let Some(contents) = contents {
            for section_val in contents {
                let carousel = &section_val["musicCarouselRenderer"].as_object().map(|_| &section_val["musicCarouselRenderer"]).unwrap_or(&section_val["musicCarouselShelfRenderer"]);
                if carousel.is_object() {
                    let header = &carousel["header"]["musicCarouselShelfBasicHeaderRenderer"];
                    let strapline = header["strapline"]["runs"][0]["text"].as_str().unwrap_or("").trim();
                    let main_title = header["title"]["runs"][0]["text"].as_str().unwrap_or("Quick Picks").trim();

                    let lower_main = main_title.to_lowercase();
                    let lower_strap = strapline.to_lowercase();

                    let title = if lower_main.contains("listen again") || lower_strap.contains("listen again") {
                        "Listen again".to_string()
                    } else if strapline.eq_ignore_ascii_case("similar to") {
                        format!("Similar to {}", main_title)
                    } else if lower_strap.contains("community") || lower_main.contains("community") {
                        "From the community".to_string()
                    } else if !strapline.is_empty() {
                        format!("{} • {}", strapline, main_title)
                    } else {
                        main_title.to_string()
                    };

                    let mut items = Vec::new();
                    if let Some(carousel_items) = carousel["contents"].as_array() {
                        for item in carousel_items {
                            if let Some(track) = Self::parse_carousel_item(item) {
                                items.push(track);
                            }
                        }
                    }

                    if title.to_lowercase().contains("community") {
                        items.retain(|t| t.video_id.starts_with("VL") || t.video_id.starts_with("PL") || t.artist.to_lowercase().contains("playlist"));
                    }

                    if !items.is_empty() {
                        sections.push(HomeSection { title, items });
                    }
                }
            }
        }

        // Fetch first continuation if available to get further shelves (Community, Indie, Discover, etc.)
        let continuation_token = section_list["continuations"]
            .as_array()
            .and_then(|arr| arr.get(0))
            .and_then(|c| c["nextContinuationData"]["continuation"].as_str())
            .map(|s| s.to_string());

        if let Some(token) = continuation_token {
            let cont_url = format!("https://music.youtube.com/youtubei/v1/browse?continuation={}&prettyPrint=false", token);
            let cont_body = json!({
                "context": {
                    "client": {
                        "clientName": "WEB_REMIX",
                        "clientVersion": "1.20260928.13.00",
                        "hl": "en",
                        "gl": "US"
                    }
                }
            });
            let cont_req = self.client.post(&cont_url).json(&cont_body);
            let cont_req = self.apply_auth_headers(cont_req);
            if let Ok(cont_res) = cont_req.send().await {
                if let Ok(cont_val) = cont_res.json::<Value>().await {
                    let cont_items = cont_val["continuationContents"]["sectionListContinuation"]["contents"].as_array();
                    if let Some(cont_items) = cont_items {
                        for section_val in cont_items {
                            let carousel = &section_val["musicCarouselShelfRenderer"];
                            if carousel.is_object() {
                                let header = &carousel["header"]["musicCarouselShelfBasicHeaderRenderer"];
                                let strapline = header["strapline"]["runs"][0]["text"].as_str().unwrap_or("").trim();
                                let main_title = header["title"]["runs"][0]["text"].as_str().unwrap_or("Quick Picks").trim();

                                let lower_main = main_title.to_lowercase();
                                let lower_strap = strapline.to_lowercase();

                                let title = if lower_main.contains("listen again") || lower_strap.contains("listen again") {
                                    "Listen again".to_string()
                                } else if strapline.eq_ignore_ascii_case("similar to") {
                                    format!("Similar to {}", main_title)
                                } else if lower_strap.contains("community") || lower_main.contains("community") {
                                    "From the community".to_string()
                                } else if !strapline.is_empty() {
                                    format!("{} • {}", strapline, main_title)
                                } else {
                                    main_title.to_string()
                                };

                                let mut items = Vec::new();
                                if let Some(carousel_items) = carousel["contents"].as_array() {
                                    for item in carousel_items {
                                        if let Some(track) = Self::parse_carousel_item(item) {
                                            items.push(track);
                                        }
                                    }
                                }

                                if title.to_lowercase().contains("community") {
                                    items.retain(|t| t.video_id.starts_with("VL") || t.video_id.starts_with("PL") || t.artist.to_lowercase().contains("playlist"));
                                }

                                if !items.is_empty() {
                                    sections.push(HomeSection { title, items });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Ensure "Daily Discover", "Similar to [Artist]", and "From the community" are present
        let has_discover = sections.iter().any(|s| s.title.to_lowercase().contains("discover"));
        let has_similar = sections.iter().any(|s| s.title.to_lowercase().contains("similar to"));

        // 1. Daily Discover section
        if !has_discover {
            if let Ok(res) = self.search("Daily Discover mix").await {
                if !res.songs.is_empty() {
                    let insert_pos = 1.min(sections.len());
                    sections.insert(insert_pos, HomeSection {
                        title: "Daily Discover".to_string(),
                        items: res.songs.into_iter().take(12).collect(),
                    });
                }
            }
        }

        // 2. Similar to (artist) section
        if !has_similar {
            let first_artist = sections.get(0).and_then(|s| s.items.get(0)).map(|t| t.artist.clone());
            if let Some(artist) = first_artist {
                if !artist.is_empty() && artist != "Unknown" {
                    if let Ok(res) = self.search(&format!("{} mix", artist)).await {
                        if !res.songs.is_empty() {
                            sections.push(HomeSection {
                                title: format!("Similar to {}", artist),
                                items: res.songs.into_iter().take(12).collect(),
                            });
                        }
                    }
                }
            }
        }

        // 3. "From the community" playlists section - MUST ONLY CONTAIN PLAYLISTS & BE PLACED BELOW "New releases"
        let mut community_items = Vec::new();
        if let Some(pos) = sections.iter().position(|s| s.title.to_lowercase().contains("community")) {
            let removed = sections.remove(pos);
            community_items.extend(removed.items);
        }

        if let Ok(playlists) = self.get_community_playlists().await {
            let mut seen_ids: std::collections::HashSet<String> = community_items.iter().map(|t| t.video_id.clone()).collect();
            for p in playlists {
                if !seen_ids.contains(&p.video_id) {
                    seen_ids.insert(p.video_id.clone());
                    community_items.push(p);
                }
            }
        }

        if !community_items.is_empty() {
            community_items.retain(|t| t.video_id.starts_with("VL") || t.video_id.starts_with("PL") || t.artist.to_lowercase().contains("playlist"));

            let community_section = HomeSection {
                title: "From the community".to_string(),
                items: community_items,
            };

            // Place directly below "New releases" (or "New albums & singles")
            let new_releases_idx = sections.iter().position(|s| {
                let lower = s.title.to_lowercase();
                lower.contains("new release") || lower.contains("new album") || lower.contains("singles")
            });

            if let Some(idx) = new_releases_idx {
                sections.insert(idx + 1, community_section);
            } else {
                let insert_idx = 1.min(sections.len());
                sections.insert(insert_idx, community_section);
            }
        }

        Ok(sections)
    }

    pub async fn get_community_playlists(&self) -> Result<Vec<TrackItem>, String> {
        let queries = [
            "community playlist",
            "trending playlist",
            "aesthetic playlist",
            "chill vibes playlist",
            "top hits playlist",
        ];

        let mut playlists = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for q in queries {
            if playlists.len() >= 48 {
                break;
            }

            let url = "https://music.youtube.com/youtubei/v1/search?prettyPrint=false";
            let body = json!({
                "context": {
                    "client": {
                        "clientName": "WEB_REMIX",
                        "clientVersion": "1.20240101.01.00",
                        "hl": "en",
                        "gl": "US"
                    }
                },
                "query": q
            });

            let req = self.client.post(url).json(&body);
            let req = self.apply_auth_headers(req);

            if let Ok(res) = req.send().await {
                if let Ok(val) = res.json::<Value>().await {
                    let sections = val["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]
                        ["content"]["sectionListRenderer"]["contents"]
                        .as_array();

                    if let Some(sections) = sections {
                        for section in sections {
                            let items = section["itemSectionRenderer"]["contents"].as_array()
                                .or_else(|| section["musicShelfRenderer"]["contents"].as_array());
                            if let Some(items) = items {
                                for item in items {
                                    let r = &item["musicResponsiveListItemRenderer"];
                                    if !r.is_object() {
                                        continue;
                                    }

                                    let browse_id = r["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
                                        .or_else(|| {
                                            r["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
                                        })
                                        .unwrap_or("");

                                    let subtitle = r["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
                                        .as_array()
                                        .map(|arr| arr.iter().filter_map(|x| x["text"].as_str()).collect::<Vec<_>>().join(""))
                                        .unwrap_or_default();

                                    let is_playlist = browse_id.starts_with("VL") || browse_id.starts_with("PL");

                                    if is_playlist && !browse_id.is_empty() && !seen_ids.contains(browse_id) {
                                        seen_ids.insert(browse_id.to_string());
                                        let title = r["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("Community Playlist").to_string();
                                        let artwork_url = r["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                                            .as_array()
                                            .and_then(|t| t.last())
                                            .and_then(|t| t["url"].as_str())
                                            .unwrap_or("")
                                            .to_string();

                                        playlists.push(TrackItem {
                                            video_id: browse_id.to_string(),
                                            title,
                                            artist: if subtitle.is_empty() { "Playlist".to_string() } else { subtitle },
                                            album: String::new(),
                                            duration: 0.0,
                                            duration_text: String::new(),
                                            artwork_url,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(playlists)
    }

    pub async fn get_liked_songs(&self) -> Result<Vec<TrackItem>, String> {
        // In YouTube Music Web, Liked Music is playlist VLLM
        if let Ok(detail) = self.get_playlist_or_album("VLLM").await {
            if !detail.tracks.is_empty() {
                info!("[Innertube] Loaded {} liked songs via VLLM", detail.tracks.len());
                return Ok(detail.tracks);
            }
        }

        // Fallback to FElikes
        if let Ok(detail) = self.get_playlist_or_album("FElikes").await {
            if !detail.tracks.is_empty() {
                info!("[Innertube] Loaded {} liked songs via FElikes", detail.tracks.len());
                return Ok(detail.tracks);
            }
        }

        Ok(Vec::new())
    }

    pub async fn get_user_playlists(&self) -> Result<Vec<SidebarPlaylist>, String> {
        let url = "https://music.youtube.com/youtubei/v1/guide?prettyPrint=false";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            }
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;
        let mut playlists = Vec::new();

        if let Some(items) = val["items"].as_array() {
            for section in items {
                if let Some(guide_items) = section["guideSectionRenderer"]["items"].as_array() {
                    for item in guide_items {
                        let entry = &item["guideEntryRenderer"];
                        let title = entry["formattedTitle"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                        let browse_id = entry["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str().unwrap_or("").to_string();
                        if !title.is_empty() && !browse_id.is_empty() {
                            if browse_id != "FEmusic_home" && browse_id != "FEmusic_explore" && browse_id != "FEmusic_library_landing" {
                                playlists.push(SidebarPlaylist { title, browse_id });
                            }
                        }
                    }
                }
            }
        }

        info!("[Innertube] Loaded {} sidebar playlists", playlists.len());
        Ok(playlists)
    }

    pub async fn get_audio_stream_url(&self, video_id: &str) -> Result<String, String> {
        let url = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";

        let body = json!({
            "context": {
                "client": {
                    "clientName": "ANDROID_MUSIC",
                    "clientVersion": "6.42.52",
                    "androidSdkVersion": 30,
                    "hl": "en",
                    "gl": "US"
                }
            },
            "videoId": video_id
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);

        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;

        let formats = val["streamingData"]["adaptiveFormats"]
            .as_array()
            .ok_or_else(|| "No streaming formats available".to_string())?;

        let mut best_audio_url = None;
        let mut best_bitrate = 0;

        for fmt in formats {
            let mime = fmt["mimeType"].as_str().unwrap_or("");
            if mime.starts_with("audio/") {
                if let Some(stream_url) = fmt["url"].as_str() {
                    let bitrate = fmt["bitrate"].as_i64().unwrap_or(0);
                    if bitrate > best_bitrate {
                        best_bitrate = bitrate;
                        best_audio_url = Some(stream_url.to_string());
                    }
                }
            }
        }

        if let Some(stream_url) = best_audio_url {
            info!(
                "[Innertube] Found audio stream for {} (bitrate: {} bps)",
                video_id, best_bitrate
            );
            return Ok(stream_url);
        }

        Err("Could not retrieve playable audio stream URL".to_string())
    }

    fn parse_music_shelf_item(item: &Value) -> Option<TrackItem> {
        let renderer = &item["musicResponsiveListItemRenderer"];
        let flex_columns = renderer["flexColumns"].as_array()?;

        let title = flex_columns
            .get(0)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
            .get(0)?["text"]
            .as_str()?
            .to_string();

        let video_id = renderer["playlistItemData"]["videoId"]
            .as_str()
            .or_else(|| {
                flex_columns.get(0)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
                    .get(0)?["navigationEndpoint"]["watchEndpoint"]["videoId"]
                    .as_str()
            })
            .unwrap_or("")
            .to_string();

        if video_id.is_empty() {
            return None;
        }

        let artist_runs = flex_columns
            .get(1)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
            .as_array()?;

        let artist = artist_runs
            .get(0)?["text"]
            .as_str()
            .unwrap_or("Unknown Artist")
            .to_string();

        let album = if artist_runs.len() >= 3 {
            artist_runs.get(2)?["text"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        let duration_text = flex_columns
            .get(1)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
            .as_array()
            .and_then(|r| r.last())
            .and_then(|r| r["text"].as_str())
            .unwrap_or("0:00")
            .to_string();

        let duration = Self::parse_duration_seconds(&duration_text);

        let artwork_url = renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
            .as_array()
            .and_then(|t| t.last())
            .and_then(|t| t["url"].as_str())
            .unwrap_or("")
            .to_string();

        Some(TrackItem {
            video_id,
            title,
            artist,
            album,
            duration,
            duration_text,
            artwork_url,
        })
    }

    fn parse_carousel_item(item: &Value) -> Option<TrackItem> {
        let two_row = &item["musicTwoRowItemRenderer"];
        if two_row.is_object() {
            let title = two_row["title"]["runs"][0]["text"].as_str()?.to_string();
            let video_id = two_row["navigationEndpoint"]["watchEndpoint"]["videoId"]
                .as_str()
                .or_else(|| {
                    two_row["thumbnailOverlay"]["musicItemThumbnailOverlayRenderer"]["content"]
                        ["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"]
                        .as_str()
                })
                .or_else(|| {
                    two_row["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
                })
                .or_else(|| {
                    two_row["thumbnailOverlay"]["musicItemThumbnailOverlayRenderer"]["content"]
                        ["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchPlaylistEndpoint"]["playlistId"]
                        .as_str()
                })
                .or_else(|| {
                    two_row["title"]["runs"][0]["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
                })
                .unwrap_or("")
                .to_string();

            if video_id.is_empty() {
                return None;
            }

            let artist = two_row["subtitle"]["runs"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join(""))
                .unwrap_or_else(|| "Unknown".to_string());

            let artwork_url = two_row["thumbnailRenderer"]["musicThumbnailRenderer"]["thumbnail"]
                ["thumbnails"]
                .as_array()
                .and_then(|t| t.last())
                .and_then(|t| t["url"].as_str())
                .unwrap_or("")
                .to_string();

            return Some(TrackItem {
                video_id,
                title,
                artist,
                album: String::new(),
                duration: 0.0,
                duration_text: String::new(),
                artwork_url,
            });
        }

        Self::parse_music_shelf_item(item)
    }

    pub async fn get_search_suggestions(&self, query: &str) -> Result<Vec<String>, String> {
        let url = reqwest::Url::parse_with_params(
            "https://suggestqueries.google.com/complete/search",
            &[("client", "firefox"), ("ds", "yt"), ("q", query)],
        )
        .map_err(|e| e.to_string())?;

        let res = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let val: Value = res.json().await.map_err(|e| e.to_string())?;
        let mut list = Vec::new();
        if let Some(arr) = val.get(1).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    list.push(s.to_string());
                }
            }
        }
        Ok(list)
    }

    pub async fn get_lyrics(&self, video_id: &str) -> Result<String, String> {
        let next_url = "https://music.youtube.com/youtubei/v1/next?prettyPrint=false";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "videoId": video_id
        });

        let req = self.client.post(next_url).json(&body);
        let req = self.apply_auth_headers(req);
        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;

        let tabs = val["contents"]["singleColumnMusicWatchNextResultsRenderer"]["tabbedRenderer"]
            ["watchNextTabbedResultsRenderer"]["tabs"]
            .as_array();

        let mut lyrics_browse_id = None;
        if let Some(tabs) = tabs {
            for tab in tabs {
                let browse_id = tab["tabRenderer"]["endpoint"]["browseEndpoint"]["browseId"].as_str();
                if let Some(bid) = browse_id {
                    if bid.starts_with("MPLY") {
                        lyrics_browse_id = Some(bid.to_string());
                        break;
                    }
                }
            }
        }

        let lyrics_id = match lyrics_browse_id {
            Some(id) => id,
            None => return Ok("No lyrics found for this song.".to_string()),
        };

        let browse_url = "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false";
        let browse_body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "browseId": lyrics_id
        });

        let req2 = self.client.post(browse_url).json(&browse_body);
        let req2 = self.apply_auth_headers(req2);
        let res2 = req2.send().await.map_err(|e| e.to_string())?;
        let val2: Value = res2.json().await.map_err(|e| e.to_string())?;

        let runs = val2["contents"]["sectionListRenderer"]["contents"][0]
            ["musicDescriptionShelfRenderer"]["description"]["runs"]
            .as_array();

        if let Some(runs) = runs {
            let mut lyrics = String::new();
            for run in runs {
                if let Some(text) = run["text"].as_str() {
                    lyrics.push_str(text);
                }
            }
            if !lyrics.is_empty() {
                return Ok(lyrics);
            }
        }

        Ok("Lyrics are not available for this track.".to_string())
    }

    pub async fn get_radio_queue(&self, video_id: &str) -> Result<Vec<TrackItem>, String> {
        let next_url = "https://music.youtube.com/youtubei/v1/next?prettyPrint=false";
        let playlist_id = format!("RDAMVM{}", video_id);
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260928.13.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "videoId": video_id,
            "playlistId": playlist_id,
            "params": "wAEB"
        });

        let req = self.client.post(next_url).json(&body);
        let req = self.apply_auth_headers(req);
        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;

        let queue_items = val["contents"]["singleColumnMusicWatchNextResultsRenderer"]["tabbedRenderer"]
            ["watchNextTabbedResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
            ["musicQueueRenderer"]["content"]["playlistPanelRenderer"]["contents"]
            .as_array();

        let mut tracks = Vec::new();
        if let Some(items) = queue_items {
            for item in items {
                if let Some(track) = Self::parse_playlist_panel_item(item) {
                    tracks.push(track);
                }
            }
        }

        Ok(tracks)
    }

    pub async fn get_playlist_or_album(&self, browse_id: &str) -> Result<PlaylistDetail, String> {
        let url = "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "browseId": browse_id
        });

        let req = self.client.post(url).json(&body);
        let req = self.apply_auth_headers(req);
        let res = req.send().await.map_err(|e| e.to_string())?;
        let val: Value = res.json().await.map_err(|e| e.to_string())?;

        let mut title = String::new();
        let mut subtitle = String::new();
        let mut description = String::new();
        let mut artwork_url = String::new();
        let mut tracks = Vec::new();

        if let Some(tc) = val["contents"]["twoColumnBrowseResultsRenderer"].as_object() {
            let header = tc.get("tabs")
                .and_then(|t| t.as_array())
                .and_then(|t| t.get(0))
                .and_then(|t0| t0["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array())
                .and_then(|c| c.get(0))
                .and_then(|s| s.get("musicResponsiveHeaderRenderer"));

            if let Some(h) = header {
                title = h["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                if let Some(runs) = h["subtitle"]["runs"].as_array() {
                    subtitle = runs.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("");
                }
                if let Some(desc_runs) = h["description"]["musicDescriptionShelfRenderer"]["description"]["runs"].as_array() {
                    description = desc_runs.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("");
                }
                artwork_url = h["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|th| th["url"].as_str())
                    .unwrap_or("")
                    .to_string();
            }

            let shelf_contents = tc.get("secondaryContents")
                .and_then(|sc| sc["sectionListRenderer"]["contents"].as_array())
                .and_then(|c| c.get(0))
                .and_then(|s| {
                    s.get("musicShelfRenderer")
                        .or_else(|| s.get("musicPlaylistShelfRenderer"))
                })
                .and_then(|s| s["contents"].as_array());

            if let Some(items) = shelf_contents {
                for item in items {
                    if let Some(track) = Self::parse_music_shelf_item(item) {
                        tracks.push(track);
                    }
                }
            }
        } else if let Some(sc) = val["contents"]["singleColumnBrowseResultsRenderer"].as_object() {
            let shelf_contents = sc.get("tabs")
                .and_then(|t| t.as_array())
                .and_then(|t| t.get(0))
                .and_then(|t0| t0["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array())
                .and_then(|c| c.get(0))
                .and_then(|s| {
                    s.get("musicPlaylistShelfRenderer")
                        .or_else(|| s.get("musicShelfRenderer"))
                })
                .and_then(|s| s["contents"].as_array());

            if let Some(items) = shelf_contents {
                for item in items {
                    if let Some(track) = Self::parse_music_shelf_item(item) {
                        tracks.push(track);
                    }
                }
            }
        }

        let track_count = tracks.len();
        Ok(PlaylistDetail {
            browse_id: browse_id.to_string(),
            title,
            description,
            subtitle,
            artwork_url,
            track_count,
            tracks,
        })
    }

    fn parse_playlist_panel_item(item: &Value) -> Option<TrackItem> {
        let renderer = &item["playlistPanelVideoRenderer"];
        if !renderer.is_object() {
            return None;
        }

        let video_id = renderer["videoId"].as_str()?.to_string();
        let title = renderer["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
        let artist = renderer["longBylineText"]["runs"][0]["text"]
            .as_str()
            .or_else(|| renderer["shortBylineText"]["runs"][0]["text"].as_str())
            .unwrap_or("Unknown Artist")
            .to_string();

        let album = renderer["longBylineText"]["runs"]
            .as_array()
            .and_then(|arr| if arr.len() >= 3 { arr.get(2) } else { None })
            .and_then(|r| r["text"].as_str())
            .unwrap_or("")
            .to_string();

        let duration_text = renderer["lengthText"]["runs"][0]["text"]
            .as_str()
            .unwrap_or("0:00")
            .to_string();

        let duration = Self::parse_duration_seconds(&duration_text);

        let artwork_url = renderer["thumbnail"]["thumbnails"]
            .as_array()
            .and_then(|t| t.last())
            .and_then(|t| t["url"].as_str())
            .unwrap_or("")
            .to_string();

        Some(TrackItem {
            video_id,
            title,
            artist,
            album,
            duration,
            duration_text,
            artwork_url,
        })
    }

    fn parse_duration_seconds(dur_str: &str) -> f64 {
        let parts: Vec<&str> = dur_str.split(':').collect();
        if parts.len() == 2 {
            let m: f64 = parts[0].parse().unwrap_or(0.0);
            let s: f64 = parts[1].parse().unwrap_or(0.0);
            m * 60.0 + s
        } else if parts.len() == 3 {
            let h: f64 = parts[0].parse().unwrap_or(0.0);
            let m: f64 = parts[1].parse().unwrap_or(0.0);
            let s: f64 = parts[2].parse().unwrap_or(0.0);
            h * 3600.0 + m * 60.0 + s
        } else {
            0.0
        }
    }
}
