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

impl Default for TunaManager {
    fn default() -> Self {
        Self::new()
    }
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
        if enabled {
            info!("[Tuna] Enabled integration with OBS Tuna server (port {})", self.port);
            if let Some(payload) = self.last_payload.lock().unwrap().clone() {
                self.post_payload(payload);
            }
        } else {
            info!("[Tuna] Disabled OBS Tuna integration");
        }
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_track(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        cover_url: &str,
        video_id: &str,
        duration_secs: f64,
        current_time_secs: f64,
        is_playing: bool,
    ) {
        let status = if is_playing { "playing" } else { "paused" };
        let artists_vec = if artist.is_empty() {
            vec![]
        } else {
            artist.split(',').map(|s| s.trim().to_string()).collect()
        };

        let payload = TunaPayload {
            data: TunaData {
                title: title.to_string(),
                artists: artists_vec,
                status: status.to_string(),
                progress: (current_time_secs * 1000.0) as i64,
                duration: (duration_secs * 1000.0) as i64,
                cover: cover_url.to_string(),
                cover_url: cover_url.to_string(),
                album_url: "".to_string(),
                album: album.to_string(),
                url: format!("https://music.youtube.com/watch?v={}", video_id),
                tags: vec![],
            },
        };

        *self.last_payload.lock().unwrap() = Some(payload.clone());

        if *self.enabled.lock().unwrap() {
            self.post_payload(payload);
        }
    }

    pub fn update_playback_state(&self, is_playing: bool, current_time_secs: f64) {
        let mut last_guard = self.last_payload.lock().unwrap();
        if let Some(mut last) = last_guard.clone() {
            last.data.status = if is_playing {
                "playing".to_string()
            } else {
                "paused".to_string()
            };
            last.data.progress = (current_time_secs * 1000.0) as i64;
            *last_guard = Some(last.clone());
            if *self.enabled.lock().unwrap() {
                self.post_payload(last);
            }
        }
    }

    pub fn stop(&self) {
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

        tokio::spawn(async move {
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
