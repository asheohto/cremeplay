/*!
 * YouTube Music Innertube API Client (Pure Rust)
 * Directly queries YouTube Music JSON APIs with full Google Account authentication support.
 */

use log::info;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
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

pub use rustypipe::client::OauthDeviceCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCodeInfo {
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u32,
    pub interval: u32,
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
    pub albums: Vec<SearchTopResult>,
    pub artists: Vec<SearchTopResult>,
    pub playlists: Vec<SearchTopResult>,
    pub library_tracks: Vec<TrackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSectionItem {
    pub title: String,
    pub subtitle: String,
    pub browse_id: Option<String>,
    pub video_id: Option<String>,
    pub artwork_url: String,
    pub year: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSection {
    pub title: String,
    pub items: Vec<ArtistSectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub browse_id: String,
    pub name: String,
    pub description: String,
    pub avatar_url: String,
    pub subscribers: String,
    pub top_songs: Vec<TrackItem>,
    pub sections: Vec<ArtistSection>,
}

#[derive(Debug, Clone)]
pub enum SearchItemVariant {
    Song(TrackItem),
    Album(SearchTopResult),
    Artist(SearchTopResult),
    Playlist(SearchTopResult),
}

pub struct InnertubeClient {
    client: reqwest::Client,
    cookies: Arc<Mutex<Option<String>>>,
    rustypipe: rustypipe::client::RustyPipe,
    stream_cache: Arc<Mutex<HashMap<String, (String, std::time::Instant)>>>,
}

impl Default for InnertubeClient {
    fn default() -> Self {
        Self::new()
    }
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

        let rustypipe = rustypipe::client::RustyPipe::builder()
            .build()
            .unwrap_or_default();

        Self {
            client,
            cookies: Arc::new(Mutex::new(None)),
            rustypipe,
            stream_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_cookies(&self, cookies: Option<String>) {
        *self.cookies.lock().unwrap() = cookies;
    }

    fn apply_auth_headers_raw(cookie_str: Option<&str>, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(cookie_str) = cookie_str {
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

    fn apply_auth_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let guard = self.cookies.lock().unwrap();
        Self::apply_auth_headers_raw(guard.as_deref(), req)
    }

    fn extract_cookie<'a>(cookies: &'a str, key: &str) -> Option<&'a str> {
        for part in cookies.split(';') {
            let trimmed = part.trim();
            if let Some(val) = trimmed.strip_prefix(key)
                && let Some(eq_val) = val.strip_prefix('=') {
                    return Some(eq_val.trim());
                }
        }
        None
    }

    pub async fn get_oauth_device_code(&self) -> Result<(OAuthCodeInfo, rustypipe::client::OauthDeviceCode), String> {
        let code = self.rustypipe.user_auth_get_code().await.map_err(|e| e.to_string())?;
        let info = OAuthCodeInfo {
            user_code: code.user_code.clone(),
            verification_url: code.verification_url.clone(),
            expires_in: code.expires_in,
            interval: code.interval,
        };
        Ok((info, code))
    }

    pub async fn check_oauth_login(&self, code: &rustypipe::client::OauthDeviceCode) -> Result<bool, String> {
        self.rustypipe.user_auth_login(code).await.map_err(|e| e.to_string())
    }

    pub async fn get_oauth_access_token(&self) -> Result<String, String> {
        self.rustypipe.user_auth_access_token().await.map_err(|e| e.to_string())
    }

    pub async fn get_profile_from_token(&self, access_token: &str) -> Result<AccountProfile, String> {
        let res = self.client
            .get("https://www.googleapis.com/oauth2/v3/userinfo")
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            let json: Value = res.json().await.map_err(|e| e.to_string())?;
            let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("YouTube User").to_string();
            let avatar = json.get("picture").and_then(|v| v.as_str()).unwrap_or("").to_string();
            return Ok(AccountProfile {
                name,
                avatar_url: avatar,
                is_logged_in: true,
            });
        }
        Err(format!("Status: {}", res.status()))
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

        if let Ok(res) = req.send().await
            && let Ok(val) = res.json::<Value>().await {
                let header = &val["actions"][0]["openPopupAction"]["popup"]["multiPageMenuRenderer"]["header"]["activeAccountHeaderRenderer"];
                let name = header["accountName"]["runs"][0]["text"].as_str().unwrap_or("You").to_string();
                let avatar = header["accountPhoto"]["thumbnails"].as_array().and_then(|t| t.last()).and_then(|t| t["url"].as_str()).unwrap_or("").to_string();

                return AccountProfile {
                    name,
                    avatar_url: avatar,
                    is_logged_in: true,
                };
            }

        AccountProfile {
            name: "YouTube Music User".to_string(),
            avatar_url: "".to_string(),
            is_logged_in: true,
        }
    }

    pub async fn search_catalog(
        &self,
        query: &str,
    ) -> Result<
        (
            Option<SearchTopResult>,
            Vec<TrackItem>,
            Vec<SearchTopResult>,
            Vec<SearchTopResult>,
            Vec<SearchTopResult>,
        ),
        String,
    > {
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
        let mut albums = Vec::new();
        let mut artists = Vec::new();
        let mut playlists = Vec::new();

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
                                .and_then(|btns| btns.first())
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

                let items = section["musicShelfRenderer"]["contents"].as_array()
                    .or_else(|| section["itemSectionRenderer"]["contents"].as_array());

                if let Some(items) = items {
                    for item in items {
                        if let Some(parsed) = Self::parse_search_item_variant(item) {
                            match parsed {
                                SearchItemVariant::Song(t) => songs.push(t),
                                SearchItemVariant::Album(a) => albums.push(a),
                                SearchItemVariant::Artist(ar) => artists.push(ar),
                                SearchItemVariant::Playlist(p) => playlists.push(p),
                            }
                        }
                    }
                }
            }
        }

        if songs.is_empty() {
            let direct_shelves = val["contents"]["sectionListRenderer"]["contents"].as_array();
            if let Some(shelves) = direct_shelves {
                for shelf in shelves {
                    let items = shelf["musicShelfRenderer"]["contents"].as_array()
                        .or_else(|| shelf["itemSectionRenderer"]["contents"].as_array());
                    if let Some(items) = items {
                        for item in items {
                            if let Some(parsed) = Self::parse_search_item_variant(item) {
                                match parsed {
                                    SearchItemVariant::Song(t) => songs.push(t),
                                    SearchItemVariant::Album(a) => albums.push(a),
                                    SearchItemVariant::Artist(ar) => artists.push(ar),
                                    SearchItemVariant::Playlist(p) => playlists.push(p),
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((top_result, songs, albums, artists, playlists))
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

        let (top_result, songs, albums, artists, playlists) = catalog_res.unwrap_or_else(|_| (None, Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        let library_tracks = library_res.unwrap_or_default();

        info!(
            "[Innertube] Search for '{}' returned {} songs, {} albums, {} artists, {} playlists",
            query,
            songs.len(),
            albums.len(),
            artists.len(),
            playlists.len()
        );

        Ok(SearchResultPayload {
            top_result,
            songs,
            albums,
            artists,
            playlists,
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

    pub async fn get_home_feed(&self, external_taste_seeds: &[String]) -> Result<Vec<HomeSection>, String> {
        let url = "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false";

        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260928.13.00",
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
            .and_then(|arr| arr.first())
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
            if let Ok(cont_res) = cont_req.send().await
                && let Ok(cont_val) = cont_res.json::<Value>().await {
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

        // Ensure "Daily Discover", "Similar to [Artist]", and "From the community" are present
        let has_discover = sections.iter().any(|s| s.title.to_lowercase().contains("discover"));
        let has_similar = sections.iter().any(|s| s.title.to_lowercase().contains("similar to"));

        // 1. Daily Discover section - PERSONALIZED TO USER TASTE
        if !has_discover {
            let discover_query = if let Some(top_seed) = external_taste_seeds.iter().find(|s| !s.trim().is_empty() && !s.eq_ignore_ascii_case("unknown")) {
                format!("{} discover mix", top_seed)
            } else {
                let first_artist = sections.first().and_then(|s| s.items.first()).map(|t| t.artist.clone());
                if let Some(art) = first_artist.filter(|a| !a.is_empty() && a != "Unknown") {
                    format!("{} discover mix", art)
                } else {
                    "Daily Discover mix".to_string()
                }
            };

            if let Ok(res) = self.search(&discover_query).await
                && !res.songs.is_empty() {
                    let insert_pos = 1.min(sections.len());
                    sections.insert(insert_pos, HomeSection {
                        title: "Daily Discover".to_string(),
                        items: res.songs.into_iter().take(12).collect(),
                    });
                }
        }

        // 2. Similar to (artist) section
        if !has_similar {
            let first_artist = sections.first().and_then(|s| s.items.first()).map(|t| t.artist.clone());
            if let Some(artist) = first_artist
                && !artist.is_empty() && artist != "Unknown"
                    && let Ok(res) = self.search(&format!("{} mix", artist)).await
                        && !res.songs.is_empty() {
                            sections.push(HomeSection {
                                title: format!("Similar to {}", artist),
                                items: res.songs.into_iter().take(12).collect(),
                            });
                        }
        }

        // 3. "From the community" playlists section - SUITED TO USER TASTE & PERSONALIZATION
        // Extract algorithmic seeds from the home feed shelves
        let mut home_extracted_seeds = Vec::new();
        for section in &sections {
            let lower_title = section.title.to_lowercase();
            if lower_title.starts_with("similar to ") {
                let artist = section.title["similar to ".len()..].trim().to_string();
                if !artist.is_empty() {
                    home_extracted_seeds.push(artist);
                }
            } else if lower_title.contains("listen again")
                || lower_title.contains("quick picks")
                || lower_title.contains("favorites")
                || lower_title.contains("mixed for you")
            {
                for item in section.items.iter().take(6) {
                    if !item.artist.is_empty() && !item.artist.eq_ignore_ascii_case("unknown") {
                        home_extracted_seeds.push(item.artist.clone());
                    }
                }
            }
        }

        let mut combined_seeds = Vec::new();
        let mut seen_seeds = std::collections::HashSet::new();

        // 1. External seeds (from user's recent plays and stored taste)
        for s in external_taste_seeds {
            let clean = s.trim().to_string();
            let lower = clean.to_lowercase();
            if !clean.is_empty() && lower != "unknown" && clean.len() >= 2 && !seen_seeds.contains(&lower) {
                seen_seeds.insert(lower);
                combined_seeds.push(clean);
            }
        }

        // 2. YouTube Music's algorithmic seeds from home feed
        for s in home_extracted_seeds {
            let clean = s.trim().to_string();
            let lower = clean.to_lowercase();
            if !clean.is_empty() && lower != "unknown" && clean.len() >= 2 && !seen_seeds.contains(&lower) {
                seen_seeds.insert(lower);
                combined_seeds.push(clean);
            }
        }

        // 3. If still empty and authenticated, sample from liked songs
        if combined_seeds.is_empty()
            && let Ok(liked) = self.get_liked_songs().await {
                for track in liked.iter().take(10) {
                    let clean = track.artist.trim().to_string();
                    let lower = clean.to_lowercase();
                    if !clean.is_empty() && lower != "unknown" && clean.len() >= 2 && !seen_seeds.contains(&lower) {
                        seen_seeds.insert(lower);
                        combined_seeds.push(clean);
                    }
                }
            }

        let mut community_items = Vec::new();
        if let Some(pos) = sections.iter().position(|s| s.title.to_lowercase().contains("community")) {
            let removed = sections.remove(pos);
            community_items.extend(removed.items);
        }

        if let Ok(playlists) = self.get_community_playlists(&combined_seeds).await {
            let mut seen_ids: std::collections::HashSet<String> = community_items.iter().map(|t| t.video_id.clone()).collect();
            for p in playlists {
                if !seen_ids.contains(&p.video_id) {
                    seen_ids.insert(p.video_id.clone());
                    community_items.push(p);
                }
            }
        }

        if !community_items.is_empty() {
            community_items.retain(|t| {
                let is_playlist_id = t.video_id.starts_with("VLPL")
                    || t.video_id.starts_with("PL")
                    || (t.video_id.starts_with("VL") && !t.video_id.starts_with("VLRD") && !t.video_id.starts_with("VLOLAK"));
                let is_community = !t.artist.to_lowercase().contains("youtube music");
                is_playlist_id && is_community
            });

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

        if sections.is_empty()
            && let Ok(explore) = self.get_explore_feed().await {
                sections = explore;
            }

        Ok(sections)
    }

    pub async fn get_community_playlists(&self, taste_seeds: &[String]) -> Result<Vec<TrackItem>, String> {
        let mut queries: Vec<String> = Vec::new();

        if !taste_seeds.is_empty() {
            for seed in taste_seeds.iter().take(4) {
                queries.push(format!("{} playlist", seed));
            }
            if let Some(top_seed) = taste_seeds.first() {
                queries.push(format!("{} vibe playlist", top_seed));
            }
            if queries.len() < 5 {
                queries.push("aesthetic vibes playlist".to_string());
            }
            if queries.len() < 6 {
                queries.push("chill vibes playlist".to_string());
            }
        } else {
            queries = vec![
                "community playlist".to_string(),
                "trending playlist".to_string(),
                "aesthetic playlist".to_string(),
                "chill vibes playlist".to_string(),
                "top hits playlist".to_string(),
            ];
        }

        let cookie_val = self.cookies.lock().unwrap().clone();

        let tasks = queries.iter().map(|q| {
            let client = self.client.clone();
            let c_val = cookie_val.clone();
            let query = q.clone();
            async move {
                Self::fetch_single_community_query(&client, c_val.as_deref(), &query).await
            }
        });

        let results_by_query = futures_util::future::join_all(tasks).await;

        let mut interleaved_playlists = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        let max_items = results_by_query.iter().map(|q| q.len()).max().unwrap_or(0);

        for i in 0..max_items {
            for query_items in &results_by_query {
                if i < query_items.len() {
                    let item = &query_items[i];
                    if !seen_ids.contains(&item.video_id) {
                        seen_ids.insert(item.video_id.clone());
                        interleaved_playlists.push(item.clone());
                        if interleaved_playlists.len() >= 48 {
                            return Ok(interleaved_playlists);
                        }
                    }
                }
            }
        }

        Ok(interleaved_playlists)
    }

    async fn fetch_single_community_query(
        client: &reqwest::Client,
        cookie_str: Option<&str>,
        query: &str,
    ) -> Vec<TrackItem> {
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

        let req = client.post(url).json(&body);
        let req = Self::apply_auth_headers_raw(cookie_str, req);

        let mut items_out = Vec::new();
        if let Ok(res) = req.send().await
            && let Ok(val) = res.json::<Value>().await {
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

                                let is_community = (browse_id.starts_with("VLPL")
                                    || browse_id.starts_with("PL")
                                    || (browse_id.starts_with("VL") && !browse_id.starts_with("VLRD") && !browse_id.starts_with("VLOLAK")))
                                    && !subtitle.to_lowercase().contains("youtube music");

                                if is_community && !browse_id.is_empty() {
                                    let title = r["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("Community Playlist").to_string();
                                    let artwork_url = r["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                                        .as_array()
                                        .and_then(|t| t.last())
                                        .and_then(|t| t["url"].as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    items_out.push(TrackItem {
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

        items_out
    }

    pub async fn get_liked_songs(&self) -> Result<Vec<TrackItem>, String> {
        // In YouTube Music Web, Liked Music is playlist VLLM
        if let Ok(detail) = self.get_playlist_or_album("VLLM").await
            && !detail.tracks.is_empty() {
                info!("[Innertube] Loaded {} liked songs via VLLM", detail.tracks.len());
                return Ok(detail.tracks);
            }

        // Fallback to FElikes
        if let Ok(detail) = self.get_playlist_or_album("FElikes").await
            && !detail.tracks.is_empty() {
                info!("[Innertube] Loaded {} liked songs via FElikes", detail.tracks.len());
                return Ok(detail.tracks);
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
                        if !title.is_empty() && !browse_id.is_empty()
                            && browse_id != "FEmusic_home" && browse_id != "FEmusic_explore" && browse_id != "FEmusic_library_landing" {
                                playlists.push(SidebarPlaylist { title, browse_id });
                            }
                    }
                }
            }
        }

        info!("[Innertube] Loaded {} sidebar playlists", playlists.len());
        Ok(playlists)
    }

    async fn verify_stream_playable(&self, url: &str) -> bool {
        let client = reqwest::Client::builder().build().unwrap_or_default();
        let req = client.get(url)
            .header("Range", "bytes=0-512")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_millis(2500));
        match req.send().await {
            Ok(res) => {
                let status = res.status();
                eprintln!("[VerifyStream] URL: {}... status: {}", &url[..url.len().min(40)], status);
                status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT
            }
            Err(e) => {
                eprintln!("[VerifyStream] Error: {}", e);
                false
            }
        }
    }

    pub fn cache_stream_url(&self, video_id: &str, url: String) {
        let mut cache = self.stream_cache.lock().unwrap();
        if cache.len() >= 128 {
            cache.clear();
        }
        cache.insert(video_id.to_string(), (url, std::time::Instant::now()));
    }

    pub async fn get_audio_stream_url(&self, video_id: &str) -> Result<String, String> {
        {
            let cache = self.stream_cache.lock().unwrap();
            if let Some((url, timestamp)) = cache.get(video_id)
                && timestamp.elapsed() < std::time::Duration::from_secs(7200) {
                    info!("[Innertube] Cache hit for audio stream: {}", video_id);
                    return Ok(url.clone());
                }
        }

        let res = self.get_audio_stream_url_uncached(video_id).await;
        if let Ok(ref url) = res {
            self.cache_stream_url(video_id, url.clone());
        }
        res
    }

    async fn get_audio_stream_url_uncached(&self, video_id: &str) -> Result<String, String> {
        // Strategy 1: RustyPipe (Full JS deobfuscation, n-sig decode, cipher resolver)
        if let Ok(player) = self.rustypipe.query().player(video_id).await {
            // Prefer mp4 / aac (best compatibility with rodio / symphonia)
            if let Some(stream) = player
                .audio_streams
                .iter()
                .filter(|s| s.mime.contains("mp4"))
                .max_by_key(|s| s.bitrate)
                && self.verify_stream_playable(&stream.url).await {
                    info!("[Innertube] RustyPipe extracted verified MP4 audio stream (bitrate: {})", stream.bitrate);
                    return Ok(stream.url.clone());
                }
            if let Some(stream) = player.audio_streams.iter().max_by_key(|s| s.bitrate)
                && self.verify_stream_playable(&stream.url).await {
                    info!("[Innertube] RustyPipe extracted verified audio stream (bitrate: {})", stream.bitrate);
                    return Ok(stream.url.clone());
                }
        }

        // Strategy 2: High-uptime Invidious direct proxy stream (itag 140 = 128k AAC / M4A)
        let inv_proxy_url = format!("https://yt.chocolatemoo53.com/latest_version?id={}&itag=140", video_id);
        if self.verify_stream_playable(&inv_proxy_url).await {
            info!("[Innertube] Invidious direct proxy stream verified for video {}", video_id);
            return Ok(inv_proxy_url);
        }

        // Strategy 3: ANDROID_VR client (bypasses Android PoToken & Web bot checks)
        let vr_url = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
        let vr_body = json!({
            "context": {
                "client": {
                    "clientName": "ANDROID_VR",
                    "clientVersion": "1.61.48",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "videoId": video_id
        });

        if let Ok(res) = self.client.post(vr_url).json(&vr_body).send().await
            && let Ok(val) = res.json::<Value>().await
                && let Some(formats) = val["streamingData"]["adaptiveFormats"].as_array() {
                    let mut audio_streams: Vec<&Value> = formats
                        .iter()
                        .filter(|f| {
                            f["mimeType"].as_str().unwrap_or("").starts_with("audio/")
                                && f["url"].is_string()
                        })
                        .collect();
                    audio_streams.sort_by_key(|f| f["bitrate"].as_i64().unwrap_or(0));
                    if let Some(best) = audio_streams.last()
                        && let Some(u) = best["url"].as_str()
                            && self.verify_stream_playable(u).await {
                                info!("[Innertube] ANDROID_VR extracted stream URL successfully");
                                return Ok(u.to_string());
                            }
                }

        // Strategy 4: Invidious API instances fallback
        let invidious_instances = [
            "https://inv.nadeko.net",
            "https://invidious.nerdvpn.de",
            "https://yt.artemislena.eu",
            "https://invidious.drgns.space",
            "https://invidious.projectsegfau.lt",
        ];

        for instance in invidious_instances {
            let inv_url = format!("{}/api/v1/videos/{}", instance, video_id);
            if let Ok(res) = self.client.get(&inv_url).timeout(std::time::Duration::from_secs(3)).send().await
                && let Ok(val) = res.json::<Value>().await
                    && let Some(adaptive_formats) = val["adaptiveFormats"].as_array() {
                        let audio_fmt = adaptive_formats.iter().find(|f| {
                            f["type"].as_str().map(|t| t.starts_with("audio/")).unwrap_or(false) && f["url"].is_string()
                        });
                        if let Some(fmt) = audio_fmt
                            && let Some(u) = fmt["url"].as_str()
                                && self.verify_stream_playable(u).await {
                                    info!("[Innertube] Invidious fallback succeeded from {}", instance);
                                    return Ok(u.to_string());
                                }
                    }
        }

        Err("Could not retrieve playable audio stream URL".to_string())
    }

    fn parse_search_item_variant(item: &Value) -> Option<SearchItemVariant> {
        let renderer = &item["musicResponsiveListItemRenderer"];
        if renderer.is_object() {
            let flex_columns = renderer["flexColumns"].as_array()?;
            let flex0 = flex_columns.first()?;
            let flex0_runs = flex0["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array()?;
            let title = flex0_runs.first()?["text"].as_str()?.to_string();

            let browse_id = flex0_runs.first()
                .and_then(|r| r["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str())
                .or_else(|| renderer["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str())
                .map(|s| s.to_string());

            let page_type = flex0_runs.first()
                .and_then(|r| {
                    r["navigationEndpoint"]["browseEndpoint"]["browseEndpointContextSupportedConfigs"]
                        ["browseEndpointContextMusicConfig"]["pageType"]
                        .as_str()
                })
                .unwrap_or("");

            let artwork_url = renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                .as_array()
                .and_then(|t| t.last())
                .and_then(|t| t["url"].as_str())
                .unwrap_or("")
                .to_string();

            let runs_col1 = flex_columns
                .get(1)
                .and_then(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array());

            let subtitle = runs_col1
                .map(|arr| arr.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join(""))
                .unwrap_or_default();

            let first_type_tag = runs_col1
                .and_then(|arr| arr.first())
                .and_then(|r| r["text"].as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if page_type == "MUSIC_PAGE_TYPE_ARTIST"
                || first_type_tag == "artist"
                || browse_id.as_deref().is_some_and(|b| b.starts_with("UC"))
            {
                return Some(SearchItemVariant::Artist(SearchTopResult {
                    title,
                    subtitle,
                    video_id: None,
                    browse_id,
                    artwork_url,
                    result_type: "Artist".to_string(),
                }));
            }

            if page_type == "MUSIC_PAGE_TYPE_ALBUM"
                || first_type_tag == "album"
                || first_type_tag == "single"
                || first_type_tag == "ep"
                || browse_id.as_deref().is_some_and(|b| b.starts_with("MPREb"))
            {
                return Some(SearchItemVariant::Album(SearchTopResult {
                    title,
                    subtitle,
                    video_id: None,
                    browse_id,
                    artwork_url,
                    result_type: "Album".to_string(),
                }));
            }

            if page_type == "MUSIC_PAGE_TYPE_PLAYLIST"
                || first_type_tag == "playlist"
                || browse_id
                    .as_deref()
                    .is_some_and(|b| b.starts_with("VL") || b.starts_with("PL"))
            {
                return Some(SearchItemVariant::Playlist(SearchTopResult {
                    title,
                    subtitle,
                    video_id: None,
                    browse_id,
                    artwork_url,
                    result_type: "Playlist".to_string(),
                }));
            }

            if let Some(track) = Self::parse_music_shelf_item(item) {
                return Some(SearchItemVariant::Song(track));
            }
        }

        let two_row = &item["musicTwoRowItemRenderer"];
        if two_row.is_object() {
            let title = two_row["title"]["runs"][0]["text"].as_str()?.to_string();
            let browse_id = two_row["navigationEndpoint"]["browseEndpoint"]["browseId"]
                .as_str()
                .map(|s| s.to_string());
            let page_type = two_row["navigationEndpoint"]["browseEndpoint"]
                ["browseEndpointContextSupportedConfigs"]["browseEndpointContextMusicConfig"]["pageType"]
                .as_str()
                .unwrap_or("");
            let subtitle = two_row["subtitle"]["runs"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join(""))
                .unwrap_or_default();
            let artwork_url = two_row["thumbnailRenderer"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                .as_array()
                .and_then(|arr| arr.last())
                .and_then(|t| t["url"].as_str())
                .unwrap_or("")
                .to_string();

            if page_type == "MUSIC_PAGE_TYPE_ARTIST" || subtitle.to_lowercase().contains("artist") {
                return Some(SearchItemVariant::Artist(SearchTopResult {
                    title,
                    subtitle,
                    video_id: None,
                    browse_id,
                    artwork_url,
                    result_type: "Artist".to_string(),
                }));
            }
            if page_type == "MUSIC_PAGE_TYPE_ALBUM" || subtitle.to_lowercase().contains("album") {
                return Some(SearchItemVariant::Album(SearchTopResult {
                    title,
                    subtitle,
                    video_id: None,
                    browse_id,
                    artwork_url,
                    result_type: "Album".to_string(),
                }));
            }
            if page_type == "MUSIC_PAGE_TYPE_PLAYLIST" || subtitle.to_lowercase().contains("playlist") {
                return Some(SearchItemVariant::Playlist(SearchTopResult {
                    title,
                    subtitle,
                    video_id: None,
                    browse_id,
                    artwork_url,
                    result_type: "Playlist".to_string(),
                }));
            }
            if let Some(track) = Self::parse_carousel_item(item) {
                return Some(SearchItemVariant::Song(track));
            }
        }

        None
    }

    fn parse_music_shelf_item(item: &Value) -> Option<TrackItem> {
        let renderer = &item["musicResponsiveListItemRenderer"];
        let flex_columns = renderer["flexColumns"].as_array()?;

        let title = flex_columns.first()?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
            .get(0)?["text"]
            .as_str()?
            .to_string();

        let video_id = renderer["playlistItemData"]["videoId"]
            .as_str()
            .or_else(|| {
                flex_columns.first()?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"]
                    .get(0)?["navigationEndpoint"]["watchEndpoint"]["videoId"]
                    .as_str()
            })
            .or_else(|| {
                renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"]
                    .as_str()
            })
            .or_else(|| {
                renderer["navigationEndpoint"]["watchEndpoint"]["videoId"]
                    .as_str()
            })
            .unwrap_or("")
            .to_string();

        if video_id.is_empty() {
            return None;
        }

        let artist_runs = flex_columns
            .get(1)
            .and_then(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array());

        let artist = artist_runs
            .and_then(|arr| arr.first())
            .and_then(|r| r["text"].as_str())
            .unwrap_or("")
            .to_string();

        let album = artist_runs
            .and_then(|arr| if arr.len() >= 3 { arr.get(2) } else { None })
            .and_then(|r| r["text"].as_str())
            .unwrap_or("")
            .to_string();

        let duration_text = renderer["fixedColumns"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"].as_array())
            .and_then(|arr| arr.first())
            .and_then(|r| r["text"].as_str())
            .or_else(|| {
                artist_runs
                    .and_then(|arr| arr.last())
                    .and_then(|r| r["text"].as_str())
            })
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
                if let Some(bid) = browse_id
                    && bid.starts_with("MPLY") {
                        lyrics_browse_id = Some(bid.to_string());
                        break;
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

    pub async fn get_captions_lyrics(&self, video_id: &str) -> Option<crate::lrclib::LyricsPayload> {
        let player_url = "https://www.youtube.com/youtubei/v1/player?key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";
        let body = json!({
            "context": {
                "client": {
                    "clientName": "ANDROID",
                    "clientVersion": "20.10.38"
                }
            },
            "videoId": video_id
        });

        let res = self
            .client
            .post(player_url)
            .header(
                USER_AGENT,
                "com.google.android.youtube/20.10.38 (Linux; U; Android 11)",
            )
            .json(&body)
            .send()
            .await
            .ok()?;

        let data: Value = res.json().await.ok()?;
        let caption_tracks = data["captions"]["playerCaptionsTracklistRenderer"]["captionTracks"].as_array()?;
        if caption_tracks.is_empty() {
            return None;
        }

        // Prioritize:
        // 1. Manual captions in English
        // 2. Any manual captions (non-ASR)
        // 3. ASR auto-generated English captions
        // 4. Any caption track
        let chosen_track = caption_tracks
            .iter()
            .find(|t| {
                let lang = t["languageCode"].as_str().unwrap_or("");
                let kind = t["kind"].as_str().unwrap_or("");
                kind != "asr" && (lang == "en" || lang.starts_with("en-"))
            })
            .or_else(|| {
                caption_tracks.iter().find(|t| {
                    let kind = t["kind"].as_str().unwrap_or("");
                    kind != "asr"
                })
            })
            .or_else(|| {
                caption_tracks.iter().find(|t| {
                    let lang = t["languageCode"].as_str().unwrap_or("");
                    lang == "en" || lang.starts_with("en-")
                })
            })
            .or_else(|| caption_tracks.first())?;

        let base_url = chosen_track["baseUrl"].as_str()?;
        let mut timedtext_url = base_url
            .replace("&fmt=srv3", "")
            .replace("&fmt=srv1", "")
            .replace("&fmt=srv2", "");
        if !timedtext_url.contains("&fmt=json3") {
            timedtext_url.push_str("&fmt=json3");
        }

        let tt_res = self.client.get(&timedtext_url).send().await.ok()?;
        let tt_data: Value = tt_res.json().await.ok()?;
        let events = tt_data["events"].as_array()?;

        struct CaptionCue {
            start_ms: f64,
            duration_ms: f64,
            text: String,
        }

        let mut raw_cues: Vec<CaptionCue> = Vec::new();
        for ev in events {
            let start_ms = ev["tStartMs"].as_f64().unwrap_or(0.0);
            let duration_ms = ev["dDurationMs"].as_f64().unwrap_or(1500.0);
            let mut line_text = String::new();
            if let Some(segs) = ev["segs"].as_array() {
                for seg in segs {
                    if let Some(txt) = seg["utf8"].as_str() {
                        line_text.push_str(txt);
                    }
                }
            }

            if let Some(cleaned) = Self::format_caption_line(&line_text)
                && !cleaned.is_empty() {
                    raw_cues.push(CaptionCue {
                        start_ms,
                        duration_ms,
                        text: cleaned,
                    });
                }
        }

        if raw_cues.is_empty() {
            return None;
        }

        raw_cues.sort_by(|a, b| a.start_ms.partial_cmp(&b.start_ms).unwrap_or(std::cmp::Ordering::Equal));

        // Group short/choppy cues into longer, natural sentence-by-sentence lyric lines
        let mut lines = Vec::new();
        let mut cur_sentence_words: Vec<String> = Vec::new();
        let mut cur_start_ms = 0.0;
        let mut cur_end_ms = 0.0;

        let flush_sentence = |words: &mut Vec<String>, start_ms: f64, lines: &mut Vec<crate::lrclib::LyricLine>| {
            if words.is_empty() {
                return;
            }
            let mut combined = words.join(" ");
            // Capitalize the first letter if lowercase
            if let Some(first_char) = combined.chars().next()
                && first_char.is_lowercase() {
                    let mut chars = combined.chars();
                    combined = format!("{}{}", chars.next().unwrap().to_uppercase(), chars.as_str());
                }
            lines.push(crate::lrclib::LyricLine {
                time_sec: (start_ms / 10.0).round() / 100.0,
                text: combined,
                romanized: None,
            });
            words.clear();
        };

        for cue in raw_cues {
            if cue.text == "♪" {
                flush_sentence(&mut cur_sentence_words, cur_start_ms, &mut lines);
                lines.push(crate::lrclib::LyricLine {
                    time_sec: (cue.start_ms / 10.0).round() / 100.0,
                    text: "♪".to_string(),
                    romanized: None,
                });
                cur_start_ms = cue.start_ms + cue.duration_ms;
                cur_end_ms = cur_start_ms;
                continue;
            }

            if cur_sentence_words.is_empty() {
                cur_start_ms = cue.start_ms;
                cur_end_ms = cue.start_ms + cue.duration_ms;
                cur_sentence_words.push(cue.text);
            } else {
                let last_word = cur_sentence_words.last().cloned().unwrap_or_default();
                let ends_with_punct = last_word.ends_with('.')
                    || last_word.ends_with('?')
                    || last_word.ends_with('!')
                    || last_word.ends_with(';')
                    || last_word.ends_with('。')
                    || last_word.ends_with('！')
                    || last_word.ends_with('？');

                let time_gap = cue.start_ms - cur_end_ms;
                let word_count = cur_sentence_words.len();

                // Break conditions:
                // 1. Prior phrase ended with sentence punctuation (. ! ?)
                // 2. Significant pause between vocal cues (> 1.2s)
                // 3. Moderate word count (>= 7 words) with a pause (> 0.55s) or comma
                // 4. Maximum word count reached (>= 12 words)
                let should_break = ends_with_punct
                    || time_gap > 1200.0
                    || (word_count >= 7 && (time_gap > 550.0 || last_word.ends_with(',')))
                    || word_count >= 12;

                if should_break {
                    flush_sentence(&mut cur_sentence_words, cur_start_ms, &mut lines);
                    cur_start_ms = cue.start_ms;
                    cur_end_ms = cue.start_ms + cue.duration_ms;
                    cur_sentence_words.push(cue.text);
                } else {
                    cur_end_ms = (cue.start_ms + cue.duration_ms).max(cur_end_ms);
                    cur_sentence_words.push(cue.text);
                }
            }
        }

        flush_sentence(&mut cur_sentence_words, cur_start_ms, &mut lines);

        if lines.is_empty() {
            return None;
        }

        lines.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap_or(std::cmp::Ordering::Equal));

        let plain_lyrics = lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Some(crate::lrclib::LyricsPayload {
            synced: true,
            source: "YouTube Captions".to_string(),
            instrumental: false,
            lines,
            plain_lyrics,
            available_sources: vec!["YouTube Captions".to_string()],
        })
    }

    fn format_caption_line(raw: &str) -> Option<String> {
        let unescaped = Self::unescape_xml_entities(raw.trim()).replace('\n', " ");
        let no_chevrons = unescaped
            .replace("&gt;&gt;&gt;", "")
            .replace("&gt;&gt;", "")
            .replace(">>>", "")
            .replace(">>", "");
        let mut trimmed = no_chevrons.trim();
        if trimmed.starts_with("- ") {
            trimmed = trimmed[2..].trim();
        } else if trimmed.starts_with('-') {
            trimmed = trimmed[1..].trim();
        }
        if trimmed.is_empty() {
            return None;
        }

        let lower = trimmed.to_lowercase();
        // Standalone [music] / (music) / [musique] etc. -> music note symbol
        if lower == "[music]"
            || lower == "(music)"
            || lower == "[musique]"
            || lower == "(musique)"
            || lower == "[música]"
            || lower == "(música)"
            || lower == "[♪]"
            || lower == "(♪)"
            || lower == "♪"
        {
            return Some("♪".to_string());
        }

        // When [music] appears in the same sync lyric line with other text, remove it
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
        let final_text = collapsed.trim();
        if final_text.is_empty() {
            Some("♪".to_string())
        } else {
            Some(final_text.to_string())
        }
    }

    fn unescape_xml_entities(input: &str) -> String {
        input
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&quot;", "\"")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
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
                .and_then(|t| t.first())
                .and_then(|t0| t0["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array())
                .and_then(|c| c.first())
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
                .and_then(|c| c.first())
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
                .and_then(|t| t.first())
                .and_then(|t0| t0["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array())
                .and_then(|c| c.first())
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

        // Check top-level header fallback if not populated
        if title.is_empty() {
            let top_header = val.get("header")
                .and_then(|h| {
                    h.get("musicResponsiveHeaderRenderer")
                        .or_else(|| h.get("musicDetailHeaderRenderer"))
                        .or_else(|| h.get("musicEditablePlaylistDetailHeaderRenderer").and_then(|e| e.get("header").and_then(|h2| h2.get("musicResponsiveHeaderRenderer"))))
                });

            if let Some(h) = top_header {
                title = h["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                if let Some(runs) = h["subtitle"]["runs"].as_array() {
                    subtitle = runs.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("");
                }
                if let Some(desc_runs) = h["description"]["musicDescriptionShelfRenderer"]["description"]["runs"].as_array() {
                    description = desc_runs.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("");
                }
                if artwork_url.is_empty() {
                    artwork_url = h["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                        .as_array()
                        .and_then(|arr| arr.last())
                        .and_then(|th| th["url"].as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
        }

        // Post-process tracks to inherit album / artist defaults if missing
        let default_artist = if !subtitle.is_empty() {
            subtitle.split('•').next().map(|s| s.trim()).unwrap_or(&subtitle).to_string()
        } else {
            String::new()
        };

        for track in &mut tracks {
            if (track.artist.is_empty() || track.artist == "Unknown Artist")
                && !default_artist.is_empty() {
                    track.artist = default_artist.clone();
                }
            if track.album.is_empty() && !title.is_empty() {
                track.album = title.clone();
            }
            if track.artwork_url.is_empty() && !artwork_url.is_empty() {
                track.artwork_url = artwork_url.clone();
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

    pub async fn get_artist_detail(&self, browse_id: &str) -> Result<ArtistDetail, String> {
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

        let mut name = String::new();
        let mut description = String::new();
        let mut avatar_url = String::new();
        let mut subscribers = String::new();
        let mut top_songs = Vec::new();
        let mut sections = Vec::new();

        // Header
        let header = val.get("header")
            .and_then(|h| h.get("musicImmersiveHeaderRenderer").or_else(|| h.get("musicVisualHeaderRenderer")));

        if let Some(h) = header {
            name = h["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
            if let Some(desc) = h["description"]["runs"].as_array() {
                description = desc.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("");
            }
            if let Some(sub_text) = h["subscriptionButton"]["subscribeButtonRenderer"]["subscriberCountText"]["runs"][0]["text"].as_str() {
                subscribers = sub_text.to_string();
            } else if let Some(sub_text) = h["subscriberCountText"]["runs"][0]["text"].as_str() {
                subscribers = sub_text.to_string();
            }
            avatar_url = h["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                .as_array()
                .and_then(|arr| arr.last())
                .and_then(|th| th["url"].as_str())
                .unwrap_or("")
                .to_string();
        }

        // Tabs & Shelves
        let shelves = val["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array();
        if let Some(shelves) = shelves {
            for shelf in shelves {
                // Top songs / Songs shelf
                if let Some(music_shelf) = shelf.get("musicShelfRenderer") {
                    let shelf_title = music_shelf["title"]["runs"][0]["text"].as_str().unwrap_or("Top Songs").to_string();
                    let items = music_shelf["contents"].as_array();
                    if let Some(items) = items {
                        let mut shelf_tracks = Vec::new();
                        for item in items {
                            if let Some(track) = Self::parse_music_shelf_item(item) {
                                shelf_tracks.push(track);
                            }
                        }
                        if shelf_title.to_lowercase().contains("song") && top_songs.is_empty() {
                            top_songs = shelf_tracks;
                        } else if !shelf_tracks.is_empty() {
                            let items: Vec<ArtistSectionItem> = shelf_tracks.into_iter().map(|t| ArtistSectionItem {
                                title: t.title,
                                subtitle: t.artist,
                                browse_id: None,
                                video_id: Some(t.video_id),
                                artwork_url: t.artwork_url,
                                year: None,
                            }).collect();
                            sections.push(ArtistSection {
                                title: shelf_title,
                                items,
                            });
                        }
                    }
                }

                // Carousel shelf (Albums, Singles, Related, etc.)
                if let Some(carousel) = shelf.get("musicCarouselShelfRenderer") {
                    let title = carousel["header"]["musicCarouselShelfBasicHeaderRenderer"]["title"]["runs"][0]["text"].as_str().unwrap_or("Releases").to_string();
                    let mut section_items = Vec::new();
                    if let Some(items) = carousel["contents"].as_array() {
                        for item in items {
                            let two_row = &item["musicTwoRowItemRenderer"];
                            if two_row.is_object() {
                                let item_title = two_row["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                                let browse_id = two_row["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str().map(|s| s.to_string());
                                let video_id = two_row["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str().map(|s| s.to_string());
                                let artwork_url = two_row["thumbnailRenderer"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"].as_array().and_then(|arr| arr.last()).and_then(|t| t["url"].as_str()).unwrap_or("").to_string();
                                let subtitle_runs = two_row["subtitle"]["runs"].as_array();
                                let subtitle = subtitle_runs.map(|arr| arr.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default();
                                let year = subtitle_runs.and_then(|arr| arr.last()).and_then(|r| r["text"].as_str()).map(|s| s.to_string());

                                if !item_title.is_empty() {
                                    section_items.push(ArtistSectionItem {
                                        title: item_title,
                                        subtitle,
                                        browse_id,
                                        video_id,
                                        artwork_url,
                                        year,
                                    });
                                }
                            }
                        }
                    }
                    if !section_items.is_empty() {
                        sections.push(ArtistSection {
                            title,
                            items: section_items,
                        });
                    }
                }
            }
        }

        Ok(ArtistDetail {
            browse_id: browse_id.to_string(),
            name,
            description,
            avatar_url,
            subscribers,
            top_songs,
            sections,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_audio_stream() {
        let client = InnertubeClient::new();
        let res = client.get_audio_stream_url("MV_3Dpw-BRY").await;
        println!("Stream result on MV_3Dpw-BRY: {:?}", res);
        assert!(res.is_ok(), "Expected stream URL, got: {:?}", res);
    }
}

