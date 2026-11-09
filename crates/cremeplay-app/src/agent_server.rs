use crate::state::{ActiveView, AppState};
use cremeplay_core::audio_player::AudioPlayer;
use cremeplay_core::innertube::TrackItem;
use rune::AppHandle;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

static SHARED_SNAPSHOT: OnceLock<Arc<Mutex<AppStatusDto>>> = OnceLock::new();
static SHARED_PLAYER: OnceLock<Arc<AudioPlayer>> = OnceLock::new();

pub fn register_player(player: Arc<AudioPlayer>) {
    let _ = SHARED_PLAYER.set(player);
}

pub fn update_snapshot(dto: AppStatusDto) {
    if let Some(arc) = SHARED_SNAPSHOT.get() {
        *arc.lock().unwrap() = dto;
    } else {
        let _ = SHARED_SNAPSHOT.set(Arc::new(Mutex::new(dto)));
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppStatusDto {
    pub is_playing: bool,
    pub current_time: f64,
    pub duration: f64,
    pub volume: f32,
    pub is_muted: bool,
    pub shuffle: bool,
    pub repeat_mode: String,
    pub active_view: String,
    pub current_track: Option<TrackItem>,
    pub queue_len: usize,
    pub show_queue_drawer: bool,
    pub show_now_playing: bool,
    pub show_settings: bool,
    pub theme: String,
    pub liked_count: usize,
    pub is_current_liked: bool,
    pub search_suggestions: Vec<String>,
    pub lyrics_source: String,
    pub lyrics_offset_ms: i32,
    pub available_lyrics_sources: Vec<String>,
    pub is_update_available: bool,
    pub latest_version: Option<String>,
    pub sleep_timer_active: bool,
    pub sleep_timer_remaining_sec: Option<u64>,
    pub show_sleep_timer_menu: bool,
    pub audio_output_device: Option<String>,
    pub audio_devices: Vec<String>,
    pub playback_speed: f32,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    Seek { time: f64 },
    Volume { value: f32 },
    SetSpeed { value: f32 },
    Mute { value: Option<bool> },
    Search { query: String },
    SearchInput { text: String },
    View { name: String },
    ToggleDrawer,
    ToggleNowPlaying,
    ToggleSettings,
    ToggleLike,
    ToggleSleepTimerMenu,
    SleepTimer { mins: Option<u32>, end_of_track: Option<bool> },
    SetAudioOutputDevice { device: Option<String> },
    LoadArtist { id: String },
    LoadPlaylist { id: String },
    PlayTrack { track: TrackItem },
    Theme { name: String },
    SelectLyricsSource { source: String },
    AdjustLyricsOffset { delta_ms: i32 },
    SaveCustomLyrics { lyrics: String },
    CheckForUpdates { manual: Option<bool> },
    DismissUpdate,
}

pub fn start(handle: AppHandle<AppState>) {
    let handle = Arc::new(handle);
    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 9099));
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => {
                log::info!("[AgentServer] Listening on http://{}", addr);
                l
            }
            Err(e) => {
                log::warn!("[AgentServer] Failed to bind to {}: {}", addr, e);
                return;
            }
        };

        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            let h = Arc::clone(&handle);
            tokio::spawn(async move {
                let mut req_bytes = Vec::new();
                let mut buf = [0u8; 4096];
                let mut header_end = None;
                let mut content_length: Option<usize> = None;

                let _ = tokio::time::timeout(std::time::Duration::from_millis(500), async {
                    while req_bytes.len() < 1024 * 1024 {
                        let n = match socket.read(&mut buf).await {
                            Ok(n) if n > 0 => n,
                            _ => break,
                        };
                        req_bytes.extend_from_slice(&buf[..n]);

                        if header_end.is_none() {
                            if let Some(pos) = req_bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                                header_end = Some(pos + 4);
                            } else if let Some(pos) = req_bytes.windows(2).position(|w| w == b"\n\n") {
                                header_end = Some(pos + 2);
                            }

                            if let Some(hend) = header_end {
                                let headers_str = String::from_utf8_lossy(&req_bytes[..hend]);
                                for line in headers_str.lines() {
                                    if let Some((k, v)) = line.split_once(':') {
                                        if k.trim().eq_ignore_ascii_case("content-length") {
                                            content_length = v.trim().parse::<usize>().ok();
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(hend) = header_end {
                            let headers_str = String::from_utf8_lossy(&req_bytes[..hend]);
                            let first_line = headers_str.lines().next().unwrap_or("");
                            let is_bodyless = first_line.starts_with("GET ")
                                || first_line.starts_with("HEAD ")
                                || first_line.starts_with("OPTIONS ");
                            if is_bodyless || content_length == Some(0) {
                                break;
                            }

                            let body_len = req_bytes.len() - hend;
                            let is_valid_json = serde_json::from_slice::<serde::de::IgnoredAny>(&req_bytes[hend..]).is_ok();
                            if let Some(expected) = content_length {
                                if body_len >= expected || is_valid_json {
                                    break;
                                }
                            } else if is_valid_json {
                                break;
                            }
                        }
                    }
                }).await;

                let hend = match header_end {
                    Some(h) => h,
                    None => return,
                };

                let headers_part = String::from_utf8_lossy(&req_bytes[..hend]);
                let body = if let Some(expected) = content_length {
                    let end = (hend + expected).min(req_bytes.len());
                    String::from_utf8_lossy(&req_bytes[hend..end]).to_string()
                } else {
                    String::from_utf8_lossy(&req_bytes[hend..]).to_string()
                };

                let mut lines = headers_part.lines();
                let request_line = match lines.next() {
                    Some(l) => l,
                    None => return,
                };

                let parts: Vec<&str> = request_line.split_whitespace().collect();
                if parts.len() < 2 {
                    return;
                }

                let method = parts[0];
                let path = parts[1];

                let (status_code, response_body) = handle_request(&h, method, path, &body).await;
                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_code,
                    response_body.len(),
                    response_body
                );

                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
            });
        }
    });
}

async fn handle_request(
    h: &Arc<AppHandle<AppState>>,
    method: &str,
    path: &str,
    body: &str,
) -> (&'static str, String) {
    if method == "OPTIONS" {
        return ("200 OK", "{}".to_string());
    }

    match (method, path) {
        ("GET", "/api/ping") => {
            ("200 OK", r#"{"status":"ok","app":"cremeplay","version":"1.2.0","engine":"rune"}"#.to_string())
        }
        ("GET", "/api/status") | ("GET", "/api/state") => {
            if let Some(arc) = SHARED_SNAPSHOT.get() {
                let mut dto = arc.lock().unwrap().clone();
                if let Some(player) = SHARED_PLAYER.get() {
                    let status = player.get_status();
                    dto.is_playing = status.is_playing;
                    dto.current_time = status.current_time;
                    if status.duration > 0.0 {
                        dto.duration = status.duration;
                    }
                    if let Some(t) = status.track {
                        dto.current_track = Some(t);
                    }
                }
                return ("200 OK", serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string()));
            }

            let (tx, rx) = oneshot::channel();
            h.update(move |s, _ui_tx| {
                let dto = AppStatusDto {
                    is_playing: s.is_playing,
                    current_time: s.current_time,
                    duration: s.duration,
                    volume: s.volume,
                    is_muted: s.is_muted,
                    shuffle: s.shuffle,
                    repeat_mode: format!("{:?}", s.repeat_mode),
                    active_view: format!("{:?}", s.active_view),
                    current_track: s.current_track.clone(),
                    queue_len: s.queue.len(),
                    show_queue_drawer: s.show_queue_drawer,
                    show_now_playing: s.show_now_playing,
                    show_settings: s.show_settings,
                    theme: s.theme.id.to_string(),
                    liked_count: s.liked_tracks.len(),
                    is_current_liked: s
                        .current_track
                        .as_ref()
                        .map(|t| s.is_track_liked(&t.video_id))
                        .unwrap_or(false),
                    search_suggestions: s.search_suggestions.clone(),
                    lyrics_source: s.lyrics_source.clone(),
                    lyrics_offset_ms: s.lyrics_offset_ms,
                    available_lyrics_sources: s.available_lyrics.keys().cloned().collect(),
                    is_update_available: s.update_info.as_ref().map(|u| u.is_update_available).unwrap_or(false),
                    latest_version: s.update_info.as_ref().map(|u| u.latest_version.clone()),
                    sleep_timer_active: s.sleep_timer_end.is_some() || s.sleep_timer_end_of_track,
                    sleep_timer_remaining_sec: s.sleep_timer_end.and_then(|t| {
                        let now = std::time::Instant::now();
                        if t > now { Some((t - now).as_secs()) } else { Some(0) }
                    }),
                    show_sleep_timer_menu: s.show_sleep_timer_menu,
                    audio_output_device: s.player.get_output_device(),
                    audio_devices: s.audio_devices.clone(),
                    playback_speed: s.playback_speed,
                };
                let _ = tx.send(dto);
            });

            match tokio::time::timeout(std::time::Duration::from_secs(3), rx).await {
                Ok(Ok(dto)) => ("200 OK", serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string())),
                Ok(Err(_)) => ("500 Internal Error", r#"{"error":"state fetch dropped"}"#.to_string()),
                Err(_) => ("504 Gateway Timeout", r#"{"error":"state fetch timeout"}"#.to_string()),
            }
        }
        ("POST", "/api/action") => {
            let action: AgentAction = match serde_json::from_str(body) {
                Ok(a) => a,
                Err(e) => {
                    return (
                        "400 Bad Request",
                        format!(r#"{{"error":"invalid json action: {}"}}"#, e),
                    );
                }
            };

            // Fast-path: Audio controls can directly execute on the audio player with zero latency
            match &action {
                AgentAction::Play => {
                    if let Some(player) = SHARED_PLAYER.get() {
                        player.play();
                    }
                    h.update(|s, _| { s.is_playing = true; });
                    return ("200 OK", r#"{"status":"ok","action":"play"}"#.to_string());
                }
                AgentAction::Pause => {
                    if let Some(player) = SHARED_PLAYER.get() {
                        player.pause();
                    }
                    h.update(|s, _| { s.is_playing = false; });
                    return ("200 OK", r#"{"status":"ok","action":"pause"}"#.to_string());
                }
                AgentAction::Toggle => {
                    let is_playing = if let Some(player) = SHARED_PLAYER.get() {
                        player.toggle_playback()
                    } else {
                        false
                    };
                    h.update(move |s, _| { s.is_playing = is_playing; });
                    return (
                        "200 OK",
                        format!(r#"{{"status":"ok","action":"toggle","is_playing":{}}}"#, is_playing),
                    );
                }
                AgentAction::Seek { time } => {
                    let t = *time;
                    if let Some(player) = SHARED_PLAYER.get() {
                        player.seek(t);
                    }
                    h.update(move |s, _| { s.current_time = t; });
                    return (
                        "200 OK",
                        format!(r#"{{"status":"ok","action":"seek","current_time":{}}}"#, t),
                    );
                }
                AgentAction::Volume { value } => {
                    let vol = value.clamp(0.0, 1.0);
                    if let Some(player) = SHARED_PLAYER.get() {
                        player.set_volume(vol);
                    }
                    h.update(move |s, _| {
                        s.volume = vol;
                        s.is_muted = false;
                    });
                    return (
                        "200 OK",
                        format!(r#"{{"status":"ok","action":"volume","volume":{}}}"#, vol),
                    );
                }
                AgentAction::Mute { value } => {
                    let val = *value;
                    let mut is_muted = false;
                    if let Some(player) = SHARED_PLAYER.get() {
                        let status = player.get_status();
                        let new_mute = val.unwrap_or(status.volume > 0.0);
                        let target_vol = if new_mute { 0.0 } else { 0.7 };
                        player.set_volume(target_vol);
                        is_muted = new_mute;
                    }
                    h.update(move |s, _| {
                        let mute = val.unwrap_or(!s.is_muted);
                        s.is_muted = mute;
                        if mute {
                            s.prev_volume = s.volume;
                            s.volume = 0.0;
                        } else {
                            s.volume = if s.prev_volume > 0.0 { s.prev_volume } else { 0.7 };
                        }
                    });
                    return (
                        "200 OK",
                        format!(r#"{{"status":"ok","action":"mute","is_muted":{}}}"#, is_muted),
                    );
                }
                _ => {}
            }

            let (tx, rx) = oneshot::channel();
            let action_name = format!("{:?}", action);
            h.update(move |s, _ui_tx| {
                let result = apply_action(s, action);
                let _ = tx.send(result);
            });

            match tokio::time::timeout(std::time::Duration::from_millis(600), rx).await {
                Ok(Ok(res)) => ("200 OK", res),
                _ => ("200 OK", format!(r#"{{"status":"queued","action":"{}"}}"#, action_name)),
            }
        }
        _ => ("404 Not Found", r#"{"error":"not found"}"#.to_string()),
    }
}

fn apply_action(s: &mut AppState, action: AgentAction) -> String {
    match action {
        AgentAction::Play => {
            s.is_playing = true;
            s.player.play();
            r#"{"status":"ok","action":"play"}"#.to_string()
        }
        AgentAction::Pause => {
            s.is_playing = false;
            s.player.pause();
            r#"{"status":"ok","action":"pause"}"#.to_string()
        }
        AgentAction::Toggle => {
            s.is_playing = s.player.toggle_playback();
            format!(r#"{{"status":"ok","action":"toggle","is_playing":{}}}"#, s.is_playing)
        }
        AgentAction::Next => {
            s.play_next();
            r#"{"status":"ok","action":"next"}"#.to_string()
        }
        AgentAction::Prev => {
            s.play_prev();
            r#"{"status":"ok","action":"prev"}"#.to_string()
        }
        AgentAction::Seek { time } => {
            s.current_time = time.clamp(0.0, s.duration);
            s.player.seek(s.current_time);
            format!(r#"{{"status":"ok","action":"seek","current_time":{}}}"#, s.current_time)
        }
        AgentAction::Volume { value } => {
            s.volume = value.clamp(0.0, 1.0);
            s.is_muted = false;
            s.player.set_volume(s.volume);
            format!(r#"{{"status":"ok","action":"volume","volume":{}}}"#, s.volume)
        }
        AgentAction::SetSpeed { value } => {
            s.set_playback_speed(value);
            format!(r#"{{"status":"ok","action":"set_speed","speed":{}}}"#, s.playback_speed)
        }
        AgentAction::Mute { value } => {
            let mute = value.unwrap_or(!s.is_muted);
            s.is_muted = mute;
            if mute {
                s.prev_volume = s.volume;
                s.volume = 0.0;
                s.player.set_volume(0.0);
            } else {
                s.volume = if s.prev_volume > 0.0 { s.prev_volume } else { 0.7 };
                s.player.set_volume(s.volume);
            }
            format!(r#"{{"status":"ok","action":"mute","is_muted":{}}}"#, s.is_muted)
        }
        AgentAction::Search { query } => {
            s.search_query = query.clone();
            s.trigger_search(query);
            r#"{"status":"ok","action":"search"}"#.to_string()
        }
        AgentAction::SearchInput { text } => {
            s.search_edit.text = text;
            s.update_search_suggestions();
            format!(r#"{{"status":"ok","action":"search_input","text":"{}"}}"#, s.search_edit.text)
        }
        AgentAction::View { name } => {
            match name.to_lowercase().as_str() {
                "home" => s.navigate_to(ActiveView::Home),
                "search" => s.navigate_to(ActiveView::Search),
                "explore" => s.navigate_to(ActiveView::Explore),
                "library" => s.navigate_to(ActiveView::Library),
                "artist" | "artist_detail" => s.navigate_to(ActiveView::ArtistDetail),
                "playlist" | "playlist_detail" => s.navigate_to(ActiveView::PlaylistDetail),
                _ => return format!(r#"{{"error":"unknown view: {}"}}"#, name),
            }
            format!(r#"{{"status":"ok","action":"view","active_view":"{}"}}"#, name)
        }
        AgentAction::LoadArtist { id } => {
            s.load_artist(&id);
            format!(r#"{{"status":"ok","action":"load_artist","id":"{}"}}"#, id)
        }
        AgentAction::LoadPlaylist { id } => {
            s.load_playlist(&id);
            format!(r#"{{"status":"ok","action":"load_playlist","id":"{}"}}"#, id)
        }
        AgentAction::ToggleDrawer => {
            s.show_queue_drawer = !s.show_queue_drawer;
            format!(r#"{{"status":"ok","action":"toggle_drawer","show":{}}}"#, s.show_queue_drawer)
        }
        AgentAction::ToggleNowPlaying => {
            s.show_now_playing = !s.show_now_playing;
            format!(r#"{{"status":"ok","action":"toggle_now_playing","show":{}}}"#, s.show_now_playing)
        }
        AgentAction::ToggleSettings => {
            s.show_settings = !s.show_settings;
            format!(r#"{{"status":"ok","action":"toggle_settings","show":{}}}"#, s.show_settings)
        }
        AgentAction::ToggleLike => {
            if let Some(track) = s.current_track.clone() {
                s.toggle_like(track.clone());
                let is_liked = s.is_track_liked(&track.video_id);
                format!(r#"{{"status":"ok","action":"toggle_like","is_liked":{},"liked_count":{}}}"#, is_liked, s.liked_tracks.len())
            } else {
                r#"{"error":"no track currently playing to like"}"#.to_string()
            }
        }
        AgentAction::PlayTrack { track } => {
            s.play_track(track);
            r#"{"status":"ok","action":"play_track"}"#.to_string()
        }
        AgentAction::Theme { name } => {
            s.set_theme(&name);
            format!(r#"{{"status":"ok","action":"theme","theme":"{}"}}"#, name)
        }
        AgentAction::SelectLyricsSource { source } => {
            s.select_lyrics_source(&source);
            format!(r#"{{"status":"ok","action":"select_lyrics_source","source":"{}"}}"#, source)
        }
        AgentAction::AdjustLyricsOffset { delta_ms } => {
            s.adjust_lyrics_offset(delta_ms);
            format!(r#"{{"status":"ok","action":"adjust_lyrics_offset","offset_ms":{}}}"#, s.lyrics_offset_ms)
        }
        AgentAction::SaveCustomLyrics { lyrics } => {
            s.save_custom_lyrics(lyrics);
            r#"{"status":"ok","action":"save_custom_lyrics"}"#.to_string()
        }
        AgentAction::CheckForUpdates { manual } => {
            s.check_for_updates(manual.unwrap_or(true));
            r#"{"status":"ok","action":"check_for_updates"}"#.to_string()
        }
        AgentAction::DismissUpdate => {
            s.dismiss_update();
            r#"{"status":"ok","action":"dismiss_update"}"#.to_string()
        }
        AgentAction::ToggleSleepTimerMenu => {
            s.show_sleep_timer_menu = !s.show_sleep_timer_menu;
            format!(r#"{{"status":"ok","action":"toggle_sleep_timer_menu","show":{}}}"#, s.show_sleep_timer_menu)
        }
        AgentAction::SleepTimer { mins, end_of_track } => {
            let eot = end_of_track.unwrap_or(false);
            s.set_sleep_timer(mins, eot);
            format!(r#"{{"status":"ok","action":"sleep_timer","mins":{:?},"end_of_track":{}}}"#, mins, eot)
        }
        AgentAction::SetAudioOutputDevice { device } => {
            s.set_audio_output_device(device.clone());
            format!(r#"{{"status":"ok","action":"set_audio_output_device","device":{:?}}}"#, device)
        }
    }
}
