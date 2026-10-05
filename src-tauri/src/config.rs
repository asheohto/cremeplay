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
}

