use crate::audio_player::{AudioPlayer, PlayerStatus};
use crate::config::{AppConfig, ConfigManager};
use crate::discord::{DiscordManager, SongPayload};
use crate::innertube::{
    AccountProfile, HomeSection, InnertubeClient, PlaylistDetail, SearchResultPayload,
    SidebarPlaylist, TrackItem,
};
use crate::lrclib::{LrclibClient, LyricsPayload};
use crate::tuna::TunaManager;
use log::info;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

pub struct AppState {
    pub config_mgr: Arc<ConfigManager>,
    pub discord_mgr: Arc<DiscordManager>,
    pub tuna_mgr: Arc<TunaManager>,
    pub innertube: Arc<InnertubeClient>,
    pub lrclib: Arc<LrclibClient>,
    pub player: Arc<AudioPlayer>,
}

#[tauri::command]
pub async fn search_music(
    query: String,
    state: State<'_, AppState>,
) -> Result<SearchResultPayload, String> {
    info!("[Command] Search: {}", query);
    state.innertube.search(&query).await
}

#[tauri::command]
pub async fn get_user_playlists(
    state: State<'_, AppState>,
) -> Result<Vec<SidebarPlaylist>, String> {
    info!("[Command] Fetching user playlists");
    state.innertube.get_user_playlists().await
}

#[tauri::command]
pub async fn get_search_suggestions(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.innertube.get_search_suggestions(&query).await
}

#[tauri::command]
pub async fn get_home_feed(
    taste_seeds: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<Vec<HomeSection>, String> {
    info!("[Command] Fetching home feed");
    let mut combined_seeds = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(seeds) = taste_seeds {
        for s in seeds {
            let clean = s.trim().to_string();
            let lower = clean.to_lowercase();
            if !clean.is_empty() && lower != "unknown" && clean.len() >= 2 && !seen.contains(&lower) {
                seen.insert(lower);
                combined_seeds.push(clean);
            }
        }
    }

    let config_artists = state.config_mgr.get().taste_artists;
    for a in config_artists {
        let clean = a.trim().to_string();
        let lower = clean.to_lowercase();
        if !clean.is_empty() && lower != "unknown" && clean.len() >= 2 && !seen.contains(&lower) {
            seen.insert(lower);
            combined_seeds.push(clean);
        }
    }

    state.innertube.get_home_feed(&combined_seeds).await
}

#[tauri::command]
pub async fn get_explore_feed(state: State<'_, AppState>) -> Result<Vec<HomeSection>, String> {
    info!("[Command] Fetching explore feed");
    state.innertube.get_explore_feed().await
}

#[tauri::command]
pub async fn get_lyrics(
    video_id: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration: Option<f64>,
    state: State<'_, AppState>,
) -> Result<LyricsPayload, String> {
    info!(
        "[Command] Fetching lyrics for {} (title: {:?}, artist: {:?})",
        video_id, title, artist
    );

    // 0. Bounded In-Memory Cache check
    if let Some(cached) = state.lrclib.get_cached(&video_id) {
        return Ok(cached);
    }

    // 1. Tier 1: Try LRCLIB for crowdsourced time-synced lyrics
    if let (Some(t), Some(a)) = (title.as_deref(), artist.as_deref()) {
        if !t.trim().is_empty() {
            if let Some(payload) = state
                .lrclib
                .fetch_synced_lyrics(t, a, album.as_deref(), duration)
                .await
            {
                state.lrclib.set_cached(&video_id, payload.clone());
                return Ok(payload);
            }
        }
    }

    // 2. Tier 2: Try YouTube video/song Captions & ASR auto-generated captions
    if let Some(payload) = state.innertube.get_captions_lyrics(&video_id).await {
        state.lrclib.set_cached(&video_id, payload.clone());
        return Ok(payload);
    }

    // 3. Tier 3: Try YouTube Music Innertube plain lyrics
    if let Ok(plain) = state.innertube.get_lyrics(&video_id).await {
        if !plain.trim().is_empty()
            && !plain.starts_with("Lyrics are not available")
            && !plain.starts_with("No lyrics found")
        {
            let payload = LyricsPayload {
                synced: false,
                source: "YouTube Music".to_string(),
                instrumental: false,
                lines: Vec::new(),
                plain_lyrics: plain,
            };
            state.lrclib.set_cached(&video_id, payload.clone());
            return Ok(payload);
        }
    }

    // 4. Clean empty payload
    let empty_payload = LyricsPayload {
        synced: false,
        source: "None".to_string(),
        instrumental: false,
        lines: Vec::new(),
        plain_lyrics: "Lyrics are not available for this track.".to_string(),
    };
    state.lrclib.set_cached(&video_id, empty_payload.clone());
    Ok(empty_payload)
}

#[tauri::command]
pub async fn get_radio_queue(
    video_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TrackItem>, String> {
    info!("[Command] Fetching radio queue for {}", video_id);
    state.innertube.get_radio_queue(&video_id).await
}

#[tauri::command]
pub async fn get_playlist_or_album(
    browse_id: String,
    state: State<'_, AppState>,
) -> Result<PlaylistDetail, String> {
    info!("[Command] Fetching playlist or album for {}", browse_id);
    state.innertube.get_playlist_or_album(&browse_id).await
}

#[tauri::command]
pub async fn get_liked_songs(state: State<'_, AppState>) -> Result<Vec<TrackItem>, String> {
    info!("[Command] Fetching liked songs");
    state.innertube.get_liked_songs().await
}

#[tauri::command]
pub async fn get_account_profile(state: State<'_, AppState>) -> Result<AccountProfile, String> {
    Ok(state.innertube.get_account_profile().await)
}

#[tauri::command]
pub async fn play_track(
    app: AppHandle,
    track: TrackItem,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!("[Command] Play track: {} - {}", track.title, track.artist);

    // If native stream extraction succeeds, inform player; don't fail if YouTube blocks direct stream extraction
    if let Ok(stream_url) = state.innertube.get_audio_stream_url(&track.video_id).await {
        let _ = state.player.play_stream(&stream_url, track.clone()).await;
    }

    let song_payload = SongPayload {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        artwork_url: track.artwork_url.clone(),
        duration: track.duration,
        video_id: track.video_id.clone(),
    };
    state.discord_mgr.update_song(&song_payload, true, 0.0);
    state.tuna_mgr.update_track(
        &track.title,
        &track.artist,
        &track.album,
        &track.artwork_url,
        &track.video_id,
        track.duration,
        0.0,
        true,
    );

    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = format!("Cremeplay: {} - {}", track.title, track.artist);
        let _ = tray.set_tooltip(Some(tooltip));
    }

    state.config_mgr.record_artists_from_string(&track.artist);

    tauri::async_runtime::spawn(async move {
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
            crate::process_job::label_audio_sessions_as_cremeplay();
        }
    });

    Ok(())
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebPlaybackState {
    pub is_playing: bool,
    pub current_time: f64,
    pub duration: f64,
}

#[tauri::command]
pub fn on_song_change(
    app: AppHandle,
    payload: SongPayload,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!("[Observer] Song change: {} - {}", payload.title, payload.artist);
    state.discord_mgr.update_song(&payload, true, 0.0);
    state.tuna_mgr.update_track(
        &payload.title,
        &payload.artist,
        &payload.album,
        &payload.artwork_url,
        &payload.video_id,
        payload.duration,
        0.0,
        true,
    );

    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = format!("Cremeplay: {} - {}", payload.title, payload.artist);
        let _ = tray.set_tooltip(Some(tooltip));
    }

    state.config_mgr.record_artists_from_string(&payload.artist);

    tauri::async_runtime::spawn(async move {
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
            crate::process_job::label_audio_sessions_as_cremeplay();
        }
    });

    Ok(())
}

#[tauri::command]
pub fn on_playback_state_change(
    is_playing: bool,
    current_time: f64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.tuna_mgr.update_progress(current_time, is_playing);
    if is_playing {
        if let Some(song) = state.discord_mgr.get_last_song() {
            state.discord_mgr.update_song(&song, true, current_time);
        }
        tauri::async_runtime::spawn(async move {
            for _ in 0..5 {
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                crate::process_job::label_audio_sessions_as_cremeplay();
            }
        });
    } else {
        state.discord_mgr.clear_activity();
    }
    Ok(())
}

#[tauri::command]
pub fn update_playback_progress(
    title: String,
    artist: String,
    album: String,
    artwork_url: String,
    video_id: String,
    current_time: f64,
    duration: f64,
    is_playing: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.tuna_mgr.update_track(
        &title,
        &artist,
        &album,
        &artwork_url,
        &video_id,
        duration,
        current_time,
        is_playing,
    );
    Ok(())
}

#[tauri::command]
pub fn toggle_playback(state: State<'_, AppState>) -> Result<bool, String> {
    let is_playing = state.player.toggle_playback();
    let status = state.player.get_status();
    if let Some(track) = status.track {
        let song_payload = SongPayload {
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            artwork_url: track.artwork_url.clone(),
            duration: track.duration,
            video_id: track.video_id.clone(),
        };
        state
            .discord_mgr
            .update_song(&song_payload, is_playing, status.current_time);
        state.tuna_mgr.update_track(
            &track.title,
            &track.artist,
            &track.album,
            &track.artwork_url,
            &track.video_id,
            track.duration,
            status.current_time,
            is_playing,
        );
    } else {
        state.tuna_mgr.update_progress(status.current_time, is_playing);
    }
    Ok(is_playing)
}

#[tauri::command]
pub fn set_volume(volume: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.player.set_volume(volume);
    state.config_mgr.save_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn get_player_status(state: State<'_, AppState>) -> Result<PlayerStatus, String> {
    Ok(state.player.get_status())
}

#[tauri::command]
pub async fn update_settings(
    settings: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.config_mgr.save(settings.clone());
    let discord_mgr = state.discord_mgr.clone();
    let tuna_mgr = state.tuna_mgr.clone();
    let discord_rpc = settings.discord_rpc;
    let tuna_obs = settings.tuna_obs;

    tokio::task::spawn_blocking(move || {
        discord_mgr.set_enabled(discord_rpc);
        tuna_mgr.set_enabled(tuna_obs);
    });
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config_mgr.get())
}

/// Opens dedicated Google Login window
#[tauri::command]
pub async fn open_login_window(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("auth") {
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let login_url: tauri::Url =
        "https://accounts.google.com/ServiceLogin?ltmpl=music&service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F"
            .parse()
            .map_err(|e| format!("{:?}", e))?;

    let chrome_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.152 Safari/537.36";

    let win = WebviewWindowBuilder::new(&app, "auth", WebviewUrl::External(login_url))
        .title("Sign in - YouTube Music")
        .inner_size(540.0, 720.0)
        .center()
        .user_agent(chrome_ua)
        .build()
        .map_err(|e| e.to_string())?;

    // Ensure CloseRequested destroys the window immediately so 'X' button always exits
    let win_handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            info!("[Auth] Window close requested, destroying window");
            let _ = win_handle.destroy();
        }
    });

    let win_clone = win.clone();
    let app_handle = app.clone();
    let state_innertube = state.innertube.clone();
    let state_config = state.config_mgr.clone();

    // Poll for authentication cookies in the background
    tauri::async_runtime::spawn(async move {
        let target_url = match tauri::Url::parse("https://music.youtube.com") {
            Ok(u) => u,
            Err(_) => return,
        };

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            // Stop loop if the window was closed
            if win_clone.is_closable().is_err() || !win_clone.is_visible().unwrap_or(false) {
                info!("[Auth] Login window closed, stopping background watcher");
                break;
            }

            if let Ok(cookies) = win_clone.cookies_for_url(target_url.clone()) {
                let has_auth = cookies.iter().any(|c| {
                    let n = c.name();
                    n == "SAPISID" || n == "__Secure-3PAPISID" || n == "LOGIN_INFO"
                });

                if has_auth {
                    info!("[Auth] Detected authenticated YouTube Music cookies!");
                    let cookie_str = cookies
                        .iter()
                        .map(|c| format!("{}={}", c.name(), c.value()))
                        .collect::<Vec<_>>()
                        .join("; ");

                    state_innertube.set_cookies(Some(cookie_str.clone()));
                    let profile = state_innertube.get_account_profile().await;

                    let mut cfg = state_config.get();
                    cfg.auth_cookies = Some(cookie_str);
                    cfg.user_name = Some(profile.name.clone());
                    cfg.user_avatar = Some(profile.avatar_url.clone());
                    state_config.save(cfg);

                    let _ = app_handle.emit("auth_changed", &profile);
                    info!("[Auth] Login successful as: {}", profile.name);

                    let _ = win_clone.destroy();
                    break;
                }
            }
        }
    });

    Ok(())
}

/// Called when Google Sign-In completes and cookies are grabbed
#[tauri::command]
pub async fn on_login_complete(
    app: AppHandle,
    cookies: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!("[Auth] Google Sign-In complete, saving credentials");

    if let Some(win) = app.get_webview_window("auth") {
        let _ = win.destroy();
    }

    state.innertube.set_cookies(Some(cookies.clone()));
    let profile = state.innertube.get_account_profile().await;

    let mut cfg = state.config_mgr.get();
    cfg.auth_cookies = Some(cookies);
    cfg.user_name = Some(profile.name.clone());
    cfg.user_avatar = Some(profile.avatar_url.clone());
    state.config_mgr.save(cfg);

    let _ = app.emit("auth_changed", &profile);
    info!("[Auth] Logged in as: {}", profile.name);
    Ok(())
}

/// Allows manually setting cookies (e.g. from browser export)
#[tauri::command]
pub async fn set_auth_cookies(
    app: AppHandle,
    cookies: String,
    state: State<'_, AppState>,
) -> Result<AccountProfile, String> {
    info!("[Auth] Manual cookies set, verifying profile");
    state.innertube.set_cookies(Some(cookies.clone()));
    let profile = state.innertube.get_account_profile().await;

    let mut cfg = state.config_mgr.get();
    cfg.auth_cookies = Some(cookies);
    cfg.user_name = Some(profile.name.clone());
    cfg.user_avatar = Some(profile.avatar_url.clone());
    state.config_mgr.save(cfg);

    let _ = app.emit("auth_changed", &profile);
    info!("[Auth] Verified profile: {}", profile.name);
    Ok(profile)
}

#[tauri::command]
pub fn logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    info!("[Auth] Logging out");
    state.innertube.set_cookies(None);
    let mut cfg = state.config_mgr.get();
    cfg.auth_cookies = None;
    cfg.user_name = None;
    cfg.user_avatar = None;
    state.config_mgr.save(cfg);
    let _ = app.emit("auth_changed", ());
    Ok(())
}
