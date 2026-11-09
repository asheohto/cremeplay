use crate::theme::Theme;
use cremeplay_core::{
    innertube::{AccountProfile, ArtistDetail, HomeSection, InnertubeClient, PlaylistDetail, SearchResultPayload, SidebarPlaylist, TrackItem},
    lrclib::{LrclibClient, LyricsPayload},
    AudioPlayer, ConfigManager, DiscordManager, SponsorBlockClient, TunaManager,
};
use rune::{Image, ImageCache, TextEdit, Waker};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveView {
    Home,
    Explore,
    Library,
    Search,
    PlaylistDetail,
    ArtistDetail,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QueueDrawerTab {
    Queue,
    Lyrics,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NowPlayingTab {
    Cover,
    Queue,
    Lyrics,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    General,
    Account,
    Lyrics,
    Integrations,
    Shortcuts,
    About,
}

#[allow(dead_code)]
pub struct AppState {
    // Navigation
    pub active_view: ActiveView,
    pub search_query: String,
    pub search_edit: TextEdit,
    pub search_results: Option<SearchResultPayload>,
    pub is_searching: bool,
    pub search_suggestions: Vec<String>,
    pub last_suggestion_query: String,
    pub current_playlist: Option<PlaylistDetail>,
    pub is_loading_playlist: bool,
    pub current_artist: Option<ArtistDetail>,
    pub is_loading_artist: bool,
    pub nav_history: Vec<ActiveView>,

    // Player State
    pub current_track: Option<TrackItem>,
    pub is_playing: bool,
    pub current_time: f64,
    pub duration: f64,
    pub volume: f32,
    pub is_muted: bool,
    pub prev_volume: f32,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub queue: Vec<TrackItem>,
    pub queue_index: usize,

    // Lyrics State
    pub lyrics: Option<LyricsPayload>,
    pub available_lyrics: HashMap<String, LyricsPayload>,
    pub lyrics_source: String,
    pub lyrics_offset_ms: i32,
    pub active_lyric_index: Option<usize>,
    pub show_custom_lyrics_modal: bool,
    pub custom_lyrics_edit: TextEdit,

    // Drawer & Modals
    pub show_queue_drawer: bool,
    pub queue_drawer_tab: QueueDrawerTab,
    pub show_now_playing: bool,
    pub now_playing_tab: NowPlayingTab,
    pub show_settings: bool,
    pub settings_tab: SettingsTab,
    pub auth_cookie_edit: TextEdit,

    // Library & Feed
    pub home_sections: Vec<HomeSection>,
    pub is_loading_home: bool,
    pub explore_sections: Vec<HomeSection>,
    pub is_loading_explore: bool,
    pub sidebar_playlists: Vec<SidebarPlaylist>,
    pub liked_tracks: Vec<TrackItem>,
    pub account: Option<AccountProfile>,

    // Thread-safe async buffers
    pub pending_home_feed: Arc<Mutex<Option<Vec<HomeSection>>>>,
    pub pending_explore_feed: Arc<Mutex<Option<Vec<HomeSection>>>>,
    pub pending_search_results: Arc<Mutex<Option<SearchResultPayload>>>,
    pub pending_suggestions: Arc<Mutex<Option<Vec<String>>>>,
    pub pending_lyrics: Arc<Mutex<Option<(LyricsPayload, HashMap<String, LyricsPayload>)>>>,
    pub pending_playlists: Arc<Mutex<Option<Vec<SidebarPlaylist>>>>,
    pub pending_playlist_detail: Arc<Mutex<Option<PlaylistDetail>>>,
    pub pending_artist_detail: Arc<Mutex<Option<ArtistDetail>>>,
    pub pending_account: Arc<Mutex<Option<AccountProfile>>>,
    pub waker: Arc<Mutex<Option<Waker>>>,

    // Image Cache
    pub image_cache: Arc<Mutex<Option<ImageCache>>>,
    pub image_map: Arc<Mutex<HashMap<String, Image>>>,
    pub requested_images: Arc<Mutex<HashSet<String>>>,
    pub logo_image: Arc<Mutex<Option<Image>>>,

    pub sponsorblock: Arc<SponsorBlockClient>,
    pub sponsor_segments: Vec<(f64, f64)>,
    pub pending_segments: Arc<Mutex<Option<Vec<(f64, f64)>>>>,

    // Update State
    pub update_info: Option<cremeplay_core::UpdateInfo>,
    pub is_checking_update: bool,
    pub show_update_banner: bool,
    pub update_status_message: Option<String>,
    pub pending_update: Arc<Mutex<Option<Option<cremeplay_core::UpdateInfo>>>>,

    // Theming
    pub theme: Theme,
    pub dominant_color: Option<rune::Color>,
    pub pending_dominant: Arc<Mutex<Option<rune::Color>>>,

    // Core Managers
    pub player: Arc<AudioPlayer>,
    pub innertube: Arc<InnertubeClient>,
    pub lrclib: Arc<LrclibClient>,
    pub config_mgr: Arc<ConfigManager>,
    pub discord: Arc<DiscordManager>,
    pub tuna: Arc<TunaManager>,
}

impl AppState {
    pub fn new() -> Self {
        let config_mgr = Arc::new(ConfigManager::new());
        let cfg = config_mgr.get();
        let theme = Theme::get(&cfg.theme);

        let innertube = Arc::new(InnertubeClient::new());
        innertube.set_cookies(cfg.auth_cookies.clone());
        let lrclib = Arc::new(LrclibClient::new());
        let player = Arc::new(AudioPlayer::new());
        crate::agent_server::register_player(player.clone());
        let discord = Arc::new(DiscordManager::new());
        let tuna = Arc::new(TunaManager::new());
        let sponsorblock = Arc::new(SponsorBlockClient::new());
        let pending_segments = Arc::new(Mutex::new(None));

        player.set_volume(cfg.volume);

        Self {
            active_view: ActiveView::Home,
            search_query: String::new(),
            search_edit: TextEdit::default(),
            search_results: None,
            is_searching: false,
            search_suggestions: Vec::new(),
            last_suggestion_query: String::new(),
            current_playlist: None,
            is_loading_playlist: false,
            current_artist: None,
            is_loading_artist: false,
            nav_history: Vec::new(),

            current_track: None,
            is_playing: false,
            current_time: 0.0,
            duration: 0.0,
            volume: cfg.volume,
            is_muted: false,
            prev_volume: cfg.volume,
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            queue: Vec::new(),
            queue_index: 0,

            lyrics: None,
            available_lyrics: HashMap::new(),
            lyrics_source: cfg.lyrics_source.clone(),
            lyrics_offset_ms: cfg.lyrics_offset_ms,
            active_lyric_index: None,
            show_custom_lyrics_modal: false,
            custom_lyrics_edit: TextEdit::default(),

            show_queue_drawer: false,
            queue_drawer_tab: QueueDrawerTab::Queue,
            show_now_playing: false,
            now_playing_tab: NowPlayingTab::Cover,
            show_settings: false,
            settings_tab: SettingsTab::General,
            auth_cookie_edit: TextEdit::default(),

            home_sections: Vec::new(),
            is_loading_home: true,
            explore_sections: Vec::new(),
            is_loading_explore: true,
            sidebar_playlists: Vec::new(),
            liked_tracks: cfg.liked_tracks.clone(),
            account: if let Some(name) = &cfg.user_name {
                Some(AccountProfile {
                    name: name.clone(),
                    avatar_url: cfg.user_avatar.clone().unwrap_or_default(),
                    is_logged_in: true,
                })
            } else {
                None
            },

            pending_home_feed: Arc::new(Mutex::new(None)),
            pending_explore_feed: Arc::new(Mutex::new(None)),
            pending_search_results: Arc::new(Mutex::new(None)),
            pending_suggestions: Arc::new(Mutex::new(None)),
            pending_lyrics: Arc::new(Mutex::new(None)),
            pending_playlists: Arc::new(Mutex::new(None)),
            pending_playlist_detail: Arc::new(Mutex::new(None)),
            pending_artist_detail: Arc::new(Mutex::new(None)),
            pending_account: Arc::new(Mutex::new(None)),
            waker: Arc::new(Mutex::new(None)),

            image_cache: Arc::new(Mutex::new(None)),
            image_map: Arc::new(Mutex::new(HashMap::new())),
            requested_images: Arc::new(Mutex::new(HashSet::new())),
            logo_image: Arc::new(Mutex::new(None)),

            theme,
            dominant_color: None,
            pending_dominant: Arc::new(Mutex::new(None)),

            player,
            innertube,
            lrclib,
            config_mgr,
            discord,
            tuna,
            sponsorblock,
            sponsor_segments: Vec::new(),
            pending_segments,

            update_info: None,
            is_checking_update: false,
            show_update_banner: false,
            update_status_message: None,
            pending_update: Arc::new(Mutex::new(None)),
        }
    }

    pub fn navigate_to(&mut self, view: ActiveView) {
        if self.active_view != view {
            self.nav_history.push(self.active_view);
            self.active_view = view;
        }
    }

    pub fn navigate_back(&mut self) {
        if let Some(prev) = self.nav_history.pop() {
            self.active_view = prev;
        }
    }

    pub fn set_theme(&mut self, theme_id: &str) {
        let mut t = Theme::get(theme_id);
        if theme_id == "adaptive" {
            if let Some(dom) = self.dominant_color {
                t = t.with_tint(dom);
            }
        }
        self.theme = t;
        let mut cfg = self.config_mgr.get();
        cfg.theme = theme_id.to_string();
        self.config_mgr.save(cfg);
    }

    pub fn load_playlist(&mut self, browse_id: &str) {
        self.is_loading_playlist = true;
        self.current_playlist = None;
        self.navigate_to(ActiveView::PlaylistDetail);

        let innertube = self.innertube.clone();
        let pending = self.pending_playlist_detail.clone();
        let waker_arc = self.waker.clone();
        let id = browse_id.to_string();

        tokio::spawn(async move {
            if let Ok(detail) = innertube.get_playlist_or_album(&id).await {
                *pending.lock().unwrap() = Some(detail);
                if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                    waker();
                }
            }
        });
    }

    pub fn load_artist(&mut self, browse_id: &str) {
        self.is_loading_artist = true;
        self.current_artist = None;
        self.navigate_to(ActiveView::ArtistDetail);

        let innertube = self.innertube.clone();
        let pending = self.pending_artist_detail.clone();
        let waker_arc = self.waker.clone();
        let id = browse_id.to_string();

        tokio::spawn(async move {
            if let Ok(detail) = innertube.get_artist_detail(&id).await {
                *pending.lock().unwrap() = Some(detail);
                if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                    waker();
                }
            }
        });
    }

    pub fn play_track(&mut self, track: TrackItem) {
        self.current_track = Some(track.clone());
        self.duration = track.duration;
        self.current_time = 0.0;
        self.is_playing = true;
        self.sponsor_segments.clear();
        self.lyrics = None;
        self.available_lyrics.clear();
        self.show_custom_lyrics_modal = false;

        // Restore song-specific offset if previously saved
        if let Some(&off) = self.config_mgr.get().song_offsets.get(&track.video_id) {
            self.lyrics_offset_ms = off;
        } else {
            self.lyrics_offset_ms = self.config_mgr.get().lyrics_offset_ms;
        }

        // Spawn background task to extract dominant color from cover art for Adaptive Theme
        let artwork_url = track.artwork_url.clone();
        if !artwork_url.is_empty() {
            let pending_dom = self.pending_dominant.clone();
            let waker_dom = self.waker.clone();
            tokio::spawn(async move {
                if let Ok(resp) = reqwest::get(&artwork_url).await {
                    if let Ok(bytes) = resp.bytes().await {
                        if let Some(color) = extract_dominant_color(&bytes) {
                            *pending_dom.lock().unwrap() = Some(color);
                            if let Some(waker) = waker_dom.lock().unwrap().as_ref() {
                                waker();
                            }
                        }
                    }
                }
            });
        }

        let player = self.player.clone();
        let innertube = self.innertube.clone();
        let lrclib = self.lrclib.clone();
        let discord = self.discord.clone();
        let tuna = self.tuna.clone();
        let sponsorblock = self.sponsorblock.clone();
        let config_mgr = self.config_mgr.clone();
        let pending_segments = self.pending_segments.clone();
        let cfg_sb = self.config_mgr.get().sponsorblock;
        let pending_lyrics = self.pending_lyrics.clone();
        let waker_arc = self.waker.clone();
        let track_clone = track.clone();

        tokio::spawn(async move {
            // 1. Play stream
            eprintln!("[State] Resolving stream for video_id: {}", track_clone.video_id);
            match innertube.get_audio_stream_url(&track_clone.video_id).await {
                Ok(url) => {
                    eprintln!("[State] Got audio stream URL (length {}), sending to player...", url.len());
                    if let Err(e) = player.play_stream(&url, track_clone.clone()).await {
                        eprintln!("[State] Failed to play stream: {}", e);
                    } else {
                        eprintln!("[State] Playback started successfully!");
                    }
                }
                Err(e) => {
                    eprintln!("[State] Failed to get audio stream URL: {}", e);
                }
            }

            // 2. Discord RPC
            let song = cremeplay_core::discord::SongPayload {
                title: track_clone.title.clone(),
                artist: track_clone.artist.clone(),
                album: track_clone.album.clone(),
                artwork_url: track_clone.artwork_url.clone(),
                duration: track_clone.duration,
                video_id: track_clone.video_id.clone(),
            };
            discord.update_song(&song, true, 0.0);

            // 3. Tuna overlay
            tuna.update_track(&track_clone.title, &track_clone.artist, &track_clone.album, &track_clone.artwork_url, &track_clone.video_id, track_clone.duration, 0.0, true);

            // 4. Concurrent Multi-Source Synced Lyrics
            let pref_source = config_mgr.get().lyrics_source.clone();
            let custom_content = config_mgr.get().custom_lyrics.get(&track_clone.video_id).cloned();

            let (better_res, lrclib_res, captions_res, genius_res) = tokio::join!(
                lrclib.fetch_better_lyrics(&track_clone.title, &track_clone.artist, Some(&track_clone.album), Some(track_clone.duration), Some(&track_clone.video_id)),
                lrclib.fetch_synced_lyrics(&track_clone.title, &track_clone.artist, Some(&track_clone.album), Some(track_clone.duration)),
                innertube.get_captions_lyrics(&track_clone.video_id),
                lrclib.fetch_genius_lyrics(&track_clone.title, &track_clone.artist)
            );

            let mut all_sources = HashMap::new();
            if let Some(p) = better_res {
                all_sources.insert("Better-Lyrics".to_string(), p);
            }
            if let Some(p) = lrclib_res {
                all_sources.insert("LRCLIB".to_string(), p);
            }
            if let Some(p) = captions_res {
                all_sources.insert("YouTube Captions".to_string(), p);
            }
            if let Some(p) = genius_res {
                all_sources.insert("Genius".to_string(), p);
            }
            if let Some(custom) = custom_content {
                let p = cremeplay_core::lrclib::parse_custom_lyrics(&custom);
                all_sources.insert("Custom".to_string(), p);
            }

            if !all_sources.is_empty() {
                let should_rom = config_mgr.get().romanize_lyrics;
                if should_rom {
                    for p in all_sources.values_mut() {
                        cremeplay_core::romanize_payload(p);
                    }
                }

                let mut source_names: Vec<String> = all_sources.keys().cloned().collect();
                source_names.sort();
                eprintln!("[State] Resolved {} lyrics sources: {:?}", all_sources.len(), source_names);

                let picked_key = if all_sources.contains_key(&pref_source) {
                    pref_source
                } else if all_sources.contains_key("Better-Lyrics") {
                    "Better-Lyrics".to_string()
                } else if all_sources.contains_key("LRCLIB") {
                    "LRCLIB".to_string()
                } else if all_sources.contains_key("YouTube Captions") {
                    "YouTube Captions".to_string()
                } else if all_sources.contains_key("Custom") {
                    "Custom".to_string()
                } else {
                    source_names[0].clone()
                };

                if let Some(mut active_payload) = all_sources.get(&picked_key).cloned() {
                    active_payload.available_sources = source_names.clone();
                    *pending_lyrics.lock().unwrap() = Some((active_payload, all_sources));
                    if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                        waker();
                    }
                }
            }

            // 5. SponsorBlock auto-skip
            if cfg_sb {
                let segs = sponsorblock.get_segments(&track_clone.video_id).await;
                let ranges: Vec<(f64, f64)> = segs.into_iter().map(|s| s.segment).collect();
                *pending_segments.lock().unwrap() = Some(ranges);
                if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                    waker();
                }
            }
        });
    }

    pub fn select_lyrics_source(&mut self, source_name: &str) {
        if let Some(payload) = self.available_lyrics.get(source_name) {
            let mut p = payload.clone();
            if self.config_mgr.get().romanize_lyrics {
                cremeplay_core::romanize_payload(&mut p);
            }
            p.available_sources = self.available_lyrics.keys().cloned().collect();
            self.lyrics = Some(p);
            self.lyrics_source = source_name.to_string();
        }
    }

    pub fn apply_romanize_toggle(&mut self, enabled: bool) {
        if enabled {
            if let Some(lyrics) = &mut self.lyrics {
                cremeplay_core::romanize_payload(lyrics);
            }
            for payload in self.available_lyrics.values_mut() {
                cremeplay_core::romanize_payload(payload);
            }
        } else {
            if let Some(lyrics) = &mut self.lyrics {
                for line in &mut lyrics.lines {
                    line.romanized = None;
                }
            }
            for payload in self.available_lyrics.values_mut() {
                for line in &mut payload.lines {
                    line.romanized = None;
                }
            }
        }
    }

    pub fn adjust_lyrics_offset(&mut self, delta_ms: i32) {
        self.lyrics_offset_ms += delta_ms;
        if let Some(track) = &self.current_track {
            let mut cfg = self.config_mgr.get();
            cfg.song_offsets.insert(track.video_id.clone(), self.lyrics_offset_ms);
            self.config_mgr.save(cfg);
        }
    }

    pub fn reset_lyrics_offset(&mut self) {
        self.lyrics_offset_ms = 0;
        if let Some(track) = &self.current_track {
            let mut cfg = self.config_mgr.get();
            cfg.song_offsets.remove(&track.video_id);
            self.config_mgr.save(cfg);
        }
    }

    pub fn save_custom_lyrics(&mut self, content: String) {
        let clean = content.trim().to_string();
        if clean.is_empty() {
            return;
        }
        let mut parsed = cremeplay_core::lrclib::parse_custom_lyrics(&clean);
        if self.config_mgr.get().romanize_lyrics {
            cremeplay_core::romanize_payload(&mut parsed);
        }
        self.available_lyrics.insert("Custom".to_string(), parsed.clone());
        self.lyrics = Some(parsed);
        self.lyrics_source = "Custom".to_string();
        self.show_custom_lyrics_modal = false;

        if let Some(track) = &self.current_track {
            let mut cfg = self.config_mgr.get();
            cfg.custom_lyrics.insert(track.video_id.clone(), clean);
            self.config_mgr.save(cfg);
        }
    }

    pub fn check_for_updates(&mut self, _manual: bool) {
        self.is_checking_update = true;
        self.update_status_message = None;

        let pending = self.pending_update.clone();
        let waker_arc = self.waker.clone();
        let current_ver = env!("CARGO_PKG_VERSION").to_string();

        tokio::spawn(async move {
            let res = cremeplay_core::updater::check_for_updates(&current_ver).await;
            *pending.lock().unwrap() = Some(res);
            if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                waker();
            }
        });
    }

    pub fn dismiss_update(&mut self) {
        self.show_update_banner = false;
        if let Some(info) = &self.update_info {
            let mut cfg = self.config_mgr.get();
            cfg.dismissed_update = Some(info.latest_version.clone());
            self.config_mgr.save(cfg);
        }
    }

    pub fn open_update_url(&self) {
        if let Some(info) = &self.update_info {
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", &info.html_url])
                .spawn();
        }
    }

    pub fn trigger_search(&mut self, query: String) {
        if query.trim().is_empty() {
            return;
        }
        self.search_query = query.clone();
        self.search_suggestions.clear();
        self.last_suggestion_query = query.clone();
        self.is_searching = true;
        self.navigate_to(ActiveView::Search);

        let innertube = self.innertube.clone();
        let pending = self.pending_search_results.clone();
        let waker_arc = self.waker.clone();

        tokio::spawn(async move {
            if let Ok(results) = innertube.search(&query).await {
                *pending.lock().unwrap() = Some(results);
                if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                    waker();
                }
            }
        });
    }

    pub fn update_search_suggestions(&mut self) {
        let current_text = self.search_edit.text.trim().to_string();
        if current_text == self.last_suggestion_query {
            return;
        }
        self.last_suggestion_query = current_text.clone();
        if current_text.is_empty() {
            self.search_suggestions.clear();
            return;
        }

        let innertube = self.innertube.clone();
        let pending = self.pending_suggestions.clone();
        let waker_arc = self.waker.clone();

        tokio::spawn(async move {
            if let Ok(suggestions) = innertube.get_search_suggestions(&current_text).await {
                *pending.lock().unwrap() = Some(suggestions);
                if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                    waker();
                }
            }
        });
    }

    pub fn init_image_cache(&self, waker: Waker) {
        let cache = ImageCache::new(waker);
        const LOGO_PNG: &[u8] = include_bytes!("../../../src-tauri/icons/128x128.png");
        let logo = cache.load(LOGO_PNG.to_vec());
        *self.logo_image.lock().unwrap() = Some(logo);
        *self.image_cache.lock().unwrap() = Some(cache);
    }

    pub fn get_image(&self, url: &str) -> Option<Image> {
        if url.is_empty() {
            return None;
        }
        let map = self.image_map.lock().unwrap();
        if let Some(img) = map.get(url) {
            return Some(img.clone());
        }
        drop(map);

        let mut req = self.requested_images.lock().unwrap();
        if req.insert(url.to_string()) {
            let url_owned = url.to_string();
            let cache_lock = self.image_cache.clone();
            let map_arc = self.image_map.clone();
            let waker_arc = self.waker.clone();

            tokio::spawn(async move {
                if let Ok(resp) = reqwest::get(&url_owned).await {
                    if let Ok(bytes) = resp.bytes().await {
                        let cache_opt = cache_lock.lock().unwrap();
                        if let Some(cache) = cache_opt.as_ref() {
                            let img = cache.load(bytes.to_vec());
                            map_arc.lock().unwrap().insert(url_owned, img);
                            if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                                waker();
                            }
                        }
                    }
                }
            });
        }
        None
    }

    pub fn play_next(&mut self) {
        if !self.queue.is_empty() && self.queue_index + 1 < self.queue.len() {
            self.queue_index += 1;
            let track = self.queue[self.queue_index].clone();
            self.play_track(track);
        }
    }

    pub fn play_prev(&mut self) {
        if self.current_time > 3.0 {
            self.player.seek(0.0);
            self.current_time = 0.0;
        } else if !self.queue.is_empty() && self.queue_index > 0 {
            self.queue_index -= 1;
            let track = self.queue[self.queue_index].clone();
            self.play_track(track);
        }
    }

    pub fn is_track_liked(&self, video_id: &str) -> bool {
        self.liked_tracks.iter().any(|t| t.video_id == video_id)
    }

    pub fn toggle_like(&mut self, track: TrackItem) {
        if let Some(pos) = self.liked_tracks.iter().position(|t| t.video_id == track.video_id) {
            self.liked_tracks.remove(pos);
        } else {
            self.liked_tracks.push(track);
        }
        let mut cfg = self.config_mgr.get();
        cfg.liked_tracks = self.liked_tracks.clone();
        self.config_mgr.save(cfg);
    }

    pub fn submit_cookies(&mut self) {
        let cookie_str = self.auth_cookie_edit.text.trim().to_string();
        if !cookie_str.is_empty() {
            self.set_cookies(cookie_str);
            self.auth_cookie_edit.text.clear();
        }
    }

    pub fn set_cookies(&mut self, cookies: String) {
        let mut cookies_clean = cookies.trim().to_string();
        if cookies_clean.is_empty() {
            return;
        }

        // Strip leading "Cookie:" header if copied verbatim from DevTools
        if let Some(stripped) = cookies_clean.strip_prefix("Cookie:") {
            cookies_clean = stripped.trim().to_string();
        } else if let Some(stripped) = cookies_clean.strip_prefix("cookie:") {
            cookies_clean = stripped.trim().to_string();
        }

        // Handle JSON cookie array from browser extensions (e.g., EditThisCookie, Cookie-Editor)
        if cookies_clean.starts_with('[') && cookies_clean.ends_with(']') {
            if let Ok(json_arr) = serde_json::from_str::<Vec<serde_json::Value>>(&cookies_clean) {
                let parts: Vec<String> = json_arr
                    .iter()
                    .filter_map(|c| {
                        let name = c.get("name")?.as_str()?;
                        let value = c.get("value")?.as_str()?;
                        Some(format!("{}={}", name, value))
                    })
                    .collect();
                if !parts.is_empty() {
                    cookies_clean = parts.join("; ");
                }
            }
        }

        // If user pasted just a raw token without '=', treat as SAPISID
        if !cookies_clean.contains('=') {
            cookies_clean = format!("SAPISID={}; __Secure-3PAPISID={}", cookies_clean, cookies_clean);
        }

        self.innertube.set_cookies(Some(cookies_clean.clone()));
        let innertube = self.innertube.clone();
        let config_mgr = self.config_mgr.clone();
        let pending_home = self.pending_home_feed.clone();
        let pending_pl = self.pending_playlists.clone();
        let pending_acc = self.pending_account.clone();
        let waker_arc = self.waker.clone();

        tokio::spawn(async move {
            let profile = innertube.get_account_profile().await;
            let mut cfg = config_mgr.get();
            cfg.auth_cookies = Some(cookies_clean);
            cfg.user_name = Some(profile.name.clone());
            cfg.user_avatar = Some(profile.avatar_url.clone());
            config_mgr.save(cfg.clone());

            *pending_acc.lock().unwrap() = Some(profile);

            // Reload user's personalized home feed & playlists
            if let Ok(sections) = innertube.get_home_feed(&cfg.taste_artists).await {
                *pending_home.lock().unwrap() = Some(sections);
            }
            if let Ok(playlists) = innertube.get_user_playlists().await {
                *pending_pl.lock().unwrap() = Some(playlists);
            }

            if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                waker();
            }
        });
    }

    pub fn logout(&mut self) {
        self.innertube.set_cookies(None);
        self.account = None;
        let mut cfg = self.config_mgr.get();
        cfg.auth_cookies = None;
        cfg.user_name = None;
        cfg.user_avatar = None;
        self.config_mgr.save(cfg.clone());

        let innertube = self.innertube.clone();
        let pending_home = self.pending_home_feed.clone();
        let pending_pl = self.pending_playlists.clone();
        let waker_arc = self.waker.clone();

        tokio::spawn(async move {
            if let Ok(sections) = innertube.get_home_feed(&cfg.taste_artists).await {
                *pending_home.lock().unwrap() = Some(sections);
            }
            *pending_pl.lock().unwrap() = Some(Vec::new());
            if let Some(waker) = waker_arc.lock().unwrap().as_ref() {
                waker();
            }
        });
    }
}

pub fn extract_dominant_color(bytes: &[u8]) -> Option<rune::Color> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let step_x = (width / 16).max(1);
    let step_y = (height / 16).max(1);

    let mut best_color = None;
    let mut max_score = -1.0f32;

    for y in (0..height).step_by(step_y as usize) {
        for x in (0..width).step_by(step_x as usize) {
            let pixel = rgb.get_pixel(x, y);
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;

            let max_c = r.max(g).max(b);
            let min_c = r.min(g).min(b);
            let lum = (max_c + min_c) * 0.5;
            let sat = if max_c == min_c {
                0.0
            } else if lum < 0.5 {
                (max_c - min_c) / (max_c + min_c)
            } else {
                (max_c - min_c) / (2.0 - max_c - min_c)
            };

            // Avoid near-black, near-white, or completely desaturated grays
            if lum > 0.15 && lum < 0.85 && sat > 0.15 {
                let score = sat * (1.0 - (lum - 0.5).abs() * 2.0);
                if score > max_score {
                    max_score = score;
                    best_color = Some(rune::Color { r, g, b, a: 1.0 });
                }
            }
        }
    }

    best_color.or_else(|| {
        let mut r_sum = 0.0f32;
        let mut g_sum = 0.0f32;
        let mut b_sum = 0.0f32;
        let mut count = 0.0f32;
        for y in (0..height).step_by(step_y as usize) {
            for x in (0..width).step_by(step_x as usize) {
                let pixel = rgb.get_pixel(x, y);
                r_sum += pixel[0] as f32 / 255.0;
                g_sum += pixel[1] as f32 / 255.0;
                b_sum += pixel[2] as f32 / 255.0;
                count += 1.0;
            }
        }
        if count > 0.0 {
            Some(rune::Color {
                r: r_sum / count,
                g: g_sum / count,
                b: b_sum / count,
                a: 1.0,
            })
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dominant_color() {
        let mut img = image::RgbImage::new(10, 10);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([200, 50, 50]);
        }
        let mut bytes: Vec<u8> = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut bytes);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();

        let color = extract_dominant_color(&bytes).expect("Should extract color");
        assert!(color.r > 0.7);
        assert!(color.g < 0.3);
        assert!(color.b < 0.3);
    }
}
