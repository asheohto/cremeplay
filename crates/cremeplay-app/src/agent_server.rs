use crate::state::{ActiveView, AppState};
use cremeplay_core::innertube::TrackItem;
use rune::AppHandle;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Serialize)]
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
    Mute { value: Option<bool> },
    Search { query: String },
    View { name: String },
    ToggleDrawer,
    ToggleNowPlaying,
    ToggleSettings,
    PlayTrack { track: TrackItem },
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
                        let body_len = req_bytes.len() - hend;
                        if let Some(expected) = content_length {
                            if body_len >= expected {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }

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
                    "HTTP/1.1 {}\r\n\
                     Content-Type: application/json\r\n\
                     Access-Control-Allow-Origin: *\r\n\
                     Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                     Access-Control-Allow-Headers: Content-Type\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\r\n{}",
                    status_code,
                    response_body.len(),
                    response_body
                );

                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
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
                };
                let _ = tx.send(dto);
            });

            match rx.await {
                Ok(dto) => ("200 OK", serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string())),
                Err(_) => ("500 Internal Error", r#"{"error":"state fetch dropped"}"#.to_string()),
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

            let (tx, rx) = oneshot::channel();
            h.update(move |s, _ui_tx| {
                let result = apply_action(s, action);
                let _ = tx.send(result);
            });

            match rx.await {
                Ok(res) => ("200 OK", res),
                Err(_) => ("500 Internal Error", r#"{"error":"action dropped"}"#.to_string()),
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
        AgentAction::View { name } => {
            match name.to_lowercase().as_str() {
                "home" => s.navigate_to(ActiveView::Home),
                "search" => s.navigate_to(ActiveView::Search),
                "explore" => s.navigate_to(ActiveView::Explore),
                "library" => s.navigate_to(ActiveView::Library),
                _ => return format!(r#"{{"error":"unknown view: {}"}}"#, name),
            }
            format!(r#"{{"status":"ok","action":"view","active_view":"{}"}}"#, name)
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
        AgentAction::PlayTrack { track } => {
            s.play_track(track);
            r#"{"status":"ok","action":"play_track"}"#.to_string()
        }
    }
}
