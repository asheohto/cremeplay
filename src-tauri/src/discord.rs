use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const DISCORD_CLIENT_ID: &str = "1177081335727267940";
const CREMEPLAY_ICON_URL: &str =
    "https://raw.githubusercontent.com/asheohto/cremeplay/main/frontend/public/cremeplay.png";

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

    pub fn get_last_song(&self) -> Option<SongPayload> {
        self.last_song.lock().unwrap().clone()
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

            let artist_display = if !song.artist.is_empty() {
                format!("by {}", song.artist)
            } else {
                "Cremeplay".to_string()
            };

            let mut act = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .status_display_type(activity::StatusDisplayType::Details)
                .name(details)
                .details(details)
                .state(&artist_display);

            let album_text = if !song.album.is_empty() {
                &song.album
            } else {
                "Cremeplay"
            };

            let large_img = if !song.artwork_url.is_empty() {
                &song.artwork_url
            } else {
                CREMEPLAY_ICON_URL
            };

            let assets = activity::Assets::new()
                .large_image(large_img)
                .large_text(album_text)
                .small_image(CREMEPLAY_ICON_URL)
                .small_text("Listening on Cremeplay");

            act = act.assets(assets);

            let mut buttons = Vec::new();
            let watch_url;
            if !song.video_id.is_empty() {
                watch_url = format!("https://music.youtube.com/watch?v={}", song.video_id);
                buttons.push(activity::Button::new("Listen on YouTube", &watch_url));
            }
            buttons.push(activity::Button::new(
                "Get Cremeplay",
                "https://github.com/asheohto/cremeplay",
            ));
            act = act.buttons(buttons);

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
