use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const DISCORD_CLIENT_ID: &str = "1177081335727267940";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongPayload {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork_url: String,
    pub duration: f64,
    pub video_id: String,
}

pub struct DiscordManager {
    client: Arc<Mutex<Option<DiscordIpcClient>>>,
    enabled: Arc<Mutex<bool>>,
    last_song: Arc<Mutex<Option<SongPayload>>>,
}

impl DiscordManager {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            enabled: Arc::new(Mutex::new(true)),
            last_song: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut en = self.enabled.lock().unwrap();
        *en = enabled;
        if !enabled {
            self.clear_activity();
        } else if let Some(song) = self.last_song.lock().unwrap().clone() {
            self.update_song(&song, true, 0.0);
        }
    }

    fn ensure_connected(&self) -> bool {
        let mut client_lock = self.client.lock().unwrap();
        if client_lock.is_none() {
            let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID);
            if let Err(e) = client.connect() {
                debug!("[Discord] Failed to connect to Discord IPC: {:?}", e);
                return false;
            }
            info!("[Discord] Successfully connected to Discord RPC");
            *client_lock = Some(client);
            true
        } else {
            true
        }
    }

    pub fn update_song(&self, song: &SongPayload, is_playing: bool, current_time: f64) {
        *self.last_song.lock().unwrap() = Some(song.clone());

        if !*self.enabled.lock().unwrap() {
            return;
        }

        if !is_playing {
            self.clear_activity();
            return;
        }

        if !self.ensure_connected() {
            return;
        }

        let mut client_lock = self.client.lock().unwrap();
        if let Some(client) = client_lock.as_mut() {
            let details = if song.title.is_empty() {
                "Listening to Music"
            } else {
                &song.title
            };

            let state = if !song.artist.is_empty() {
                &song.artist
            } else {
                "YouTube Music"
            };

            let mut act = activity::Activity::new()
                .details(details)
                .state(state);

            // Large album image or fallback
            let assets = activity::Assets::new()
                .large_image(if !song.artwork_url.is_empty() {
                    &song.artwork_url
                } else {
                    "ytm_logo"
                })
                .large_text(if !song.album.is_empty() {
                    &song.album
                } else {
                    &song.title
                })
                .small_image("play")
                .small_text("Playing");

            act = act.assets(assets);

            // Calculate timestamps
            if song.duration > 0.0 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                let start_time = now - (current_time as i64);
                let end_time = start_time + (song.duration as i64);

                let timestamps = activity::Timestamps::new()
                    .start(start_time)
                    .end(end_time);

                act = act.timestamps(timestamps);
            }

            if let Err(e) = client.set_activity(act) {
                debug!("[Discord] Failed to set activity: {:?}", e);
                // Disconnected or socket error; reset client
                *client_lock = None;
            }
        }
    }

    pub fn clear_activity(&self) {
        let mut client_lock = self.client.lock().unwrap();
        if let Some(client) = client_lock.as_mut() {
            let _ = client.clear_activity();
        }
    }
}
