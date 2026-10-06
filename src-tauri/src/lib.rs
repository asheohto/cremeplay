mod audio_player;
mod commands;
mod config;
mod discord;
mod innertube;
mod process_job;
mod sponsorblock;
mod tray;
mod tuna;

use audio_player::AudioPlayer;
use commands::{
    get_account_profile, get_explore_feed, get_home_feed, get_liked_songs, get_lyrics,
    get_playlist_or_album, get_player_status, get_radio_queue, get_search_suggestions,
    get_settings, get_user_playlists, logout, on_login_complete, on_playback_state_change,
    on_song_change, open_login_window, play_track, search_music, set_auth_cookies, set_volume,
    toggle_playback, update_playback_progress, update_settings, AppState,
};
use config::ConfigManager;
use discord::DiscordManager;
use innertube::InnertubeClient;
use log::info;
use std::sync::Arc;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    // Panic Hook: write critical crashes to logs/panic.txt for instant debugging
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("[PANIC] {}", info);
        eprintln!("{}", msg);
        log::error!("{}", msg);
        let log_dir = std::path::PathBuf::from(
            std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into()),
        )
        .join("com.cremeplay.desktop")
        .join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let _ = std::fs::write(log_dir.join("panic.txt"), &msg);
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .setup(|app| {
            let config_mgr = Arc::new(ConfigManager::new(app.handle()));
            let discord_mgr = Arc::new(DiscordManager::new());
            let tuna_mgr = Arc::new(tuna::TunaManager::new());
            let innertube = Arc::new(InnertubeClient::new());
            let player = Arc::new(AudioPlayer::new());

            let current_config = config_mgr.get();
            discord_mgr.set_enabled(current_config.discord_rpc);
            tuna_mgr.set_enabled(current_config.tuna_obs);

            // Restore saved cookies if logged in
            if let Some(cookies) = current_config.auth_cookies.clone() {
                innertube.set_cookies(Some(cookies));
            }

            // Start background progress ticker for live seekbar & SponsorBlock
            player.start_progress_ticker(app.handle().clone());

            // Memory Working Set Trimming: drops inactive Chromium buffer caches from all processes in the tree
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                process_job::trim_process_working_set();
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(20));
                loop {
                    interval.tick().await;
                    process_job::trim_process_working_set();
                }
            });

            app.manage(AppState {
                config_mgr: config_mgr.clone(),
                discord_mgr,
                tuna_mgr,
                innertube,
                player,
            });

            // Initialize System Tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                log::error!("[Tray] Failed to initialize tray: {:?}", e);
            }

            let mut win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Cremeplay")
                .inner_size(1280.0, 800.0)
                .min_inner_size(500.0, 400.0);

            if let Some(icon) = app.default_window_icon() {
                win_builder = win_builder.icon(icon.clone())?;
            }

            let win = win_builder.build()?;

            let config_for_close = config_mgr.clone();
            let win_handle = win.clone();
            let app_handle_exit = app.handle().clone();
            win.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if config_for_close.get().close_to_tray {
                        api.prevent_close();
                        let _ = win_handle.hide();
                    } else {
                        app_handle_exit.exit(0);
                    }
                }
            });

            info!("[Cremeplay] Native Cremeplay Experience loaded successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_music,
            get_search_suggestions,
            get_home_feed,
            get_explore_feed,
            get_liked_songs,
            get_account_profile,
            get_lyrics,
            get_radio_queue,
            get_playlist_or_album,
            get_user_playlists,
            play_track,
            toggle_playback,
            set_volume,
            get_player_status,
            get_settings,
            update_settings,
            open_login_window,
            on_login_complete,
            set_auth_cookies,
            logout,
            on_song_change,
            on_playback_state_change,
            update_playback_progress
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
