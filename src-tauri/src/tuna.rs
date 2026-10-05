use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunaData {
    pub title: String,
    pub artists: Vec<String>,
    pub status: String,
    pub progress: i64,
    pub duration: i64,
    pub cover: String,
    pub cover_url: String,
    pub album_url: String,
    pub album: String,
    pub url: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunaPayload {
    pub data: TunaData,
}

pub struct TunaManager {
    client: reqwest::Client,
    enabled: Arc<Mutex<bool>>,
    port: u16,
    last_payload: Arc<Mutex<Option<TunaPayload>>>,
}

impl TunaManager {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .build()
            .unwrap_or_default();

        Self {
            client,
            enabled: Arc::new(Mutex::new(true)),
            port: 1608,
            last_payload: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        {
            let mut en = self.enabled.lock().unwrap();
            *en = enabled;
        }
        info!("[Tuna] Integration set to enabled={}", enabled);
        if !enabled {
            self.send_stopped();
        } else if let Some(last) = self.last_payload.lock().unwrap().clone() {
            self.post_payload(last);
        }
    }

    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    pub fn update_track(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        artwork_url: &str,
        video_id: &str,
        duration_sec: f64,
        progress_sec: f64,
        is_playing: bool,
    ) {
        let status = if is_playing { "playing" } else { "stopped" };
        let progress_ms = (progress_sec * 1000.0).max(0.0) as i64;
        let duration_ms = (duration_sec * 1000.0).max(0.0) as i64;
        let yt_url = if video_id.is_empty() {
            String::new()
        } else {
            format!("https://music.youtube.com/watch?v={}", video_id)
        };

        let artists = if artist.is_empty() {
            vec![]
        } else {
            vec![artist.to_string()]
        };

        let payload = TunaPayload {
            data: TunaData {
                title: title.to_string(),
                artists,
                status: status.to_string(),
                progress: progress_ms,
                duration: duration_ms,
                cover: artwork_url.to_string(),
                cover_url: artwork_url.to_string(),
                album_url: artwork_url.to_string(),
                album: album.to_string(),
                url: yt_url,
                tags: vec![],
            },
        };

        *self.last_payload.lock().unwrap() = Some(payload.clone());

        if *self.enabled.lock().unwrap() {
            self.post_payload(payload);
        }
    }

    pub fn update_progress(&self, progress_sec: f64, is_playing: bool) {
        let mut last_guard = self.last_payload.lock().unwrap();
        if let Some(mut payload) = last_guard.clone() {
            payload.data.progress = (progress_sec * 1000.0).max(0.0) as i64;
            payload.data.status = if is_playing { "playing" } else { "stopped" }.to_string();
            *last_guard = Some(payload.clone());
            if *self.enabled.lock().unwrap() {
                self.post_payload(payload);
            }
        }
    }

    pub fn send_stopped(&self) {
        let mut last_guard = self.last_payload.lock().unwrap();
        if let Some(mut last) = last_guard.clone() {
            last.data.status = "stopped".to_string();
            *last_guard = Some(last.clone());
            if *self.enabled.lock().unwrap() {
                self.post_payload(last);
            }
        }
    }

    fn post_payload(&self, payload: TunaPayload) {
        let client = self.client.clone();
        let port = self.port;

        tauri::async_runtime::spawn(async move {
            let url = format!("http://127.0.0.1:{}/", port);
            match client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Headers", "*")
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    debug!("[Tuna] POST {} status: {}", url, resp.status());
                }
                Err(err) => {
                    debug!("[Tuna] OBS Tuna server not reachable at {}: {:?}", url, err);
                }
            }
        });
    }
}

