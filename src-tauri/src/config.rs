use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub adblock: bool,
    pub sponsorblock: bool,
    pub discord_rpc: bool,
    #[serde(default = "default_true")]
    pub tuna_obs: bool,
    pub media_controls: bool,
    pub close_to_tray: bool,
    pub auth_cookies: Option<String>,
    pub user_name: Option<String>,
    pub user_avatar: Option<String>,
    #[serde(default)]
    pub taste_artists: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            adblock: true,
            sponsorblock: true,
            discord_rpc: true,
            tuna_obs: true,
            media_controls: true,
            close_to_tray: true,
            auth_cookies: None,
            user_name: None,
            user_avatar: None,
            taste_artists: Vec::new(),
        }
    }
}

pub struct ConfigManager {
    pub config: Arc<Mutex<AppConfig>>,
    path: PathBuf,
}

impl ConfigManager {
    pub fn new(app: &AppHandle) -> Self {
        let config_dir = app
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let _ = fs::create_dir_all(&config_dir);
        let path = config_dir.join("config.json");

        let loaded = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str::<AppConfig>(&data).ok())
                .unwrap_or_default()
        } else {
            AppConfig::default()
        };

        Self {
            config: Arc::new(Mutex::new(loaded)),
            path,
        }
    }

    pub fn get(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn save(&self, new_config: AppConfig) {
        if let Ok(mut cfg) = self.config.lock() {
            *cfg = new_config.clone();
            if let Ok(data) = serde_json::to_string_pretty(&new_config) {
                let _ = fs::write(&self.path, data);
            }
        }
    }

    pub fn record_artists_from_string(&self, artist_str: &str) {
        let clean = artist_str.trim();
        if clean.is_empty() || clean.eq_ignore_ascii_case("unknown") {
            return;
        }

        let lower = clean.to_lowercase();
        let main = if let Some(idx) = lower.find(" feat.") {
            &clean[..idx]
        } else if let Some(idx) = lower.find(" ft.") {
            &clean[..idx]
        } else {
            clean
        };

        let parts: Vec<&str> = main.split(&[',', '&'][..]).map(|s| s.trim()).filter(|s| s.len() >= 2).collect();
        if parts.is_empty() {
            return;
        }

        let updated = {
            let mut cfg = self.config.lock().unwrap();
            for part in parts {
                if part.eq_ignore_ascii_case("unknown") {
                    continue;
                }
                cfg.taste_artists.retain(|a| !a.eq_ignore_ascii_case(part));
                cfg.taste_artists.insert(0, part.to_string());
            }
            cfg.taste_artists.truncate(25);
            cfg.clone()
        };
        self.save(updated);
    }
}

