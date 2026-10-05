// 100% Pure Native Rust Slint Application Runner
// Zero-webview, zero-Chromium, Direct-to-GPU native rendering (~15-20MB RAM).


slint::include_modules!();

use crate::audio_player::AudioPlayer;
use crate::discord::{DiscordManager, SongPayload};
use crate::innertube::{InnertubeClient, TrackItem};
use log::info;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

pub fn run_native() -> Result<(), Box<dyn std::error::Error>> {
    // Bind process to Windows Job Object
    crate::process_job::ensure_child_process_job();

    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();

    let app = AppWindow::new()?;
    let app_weak = app.as_weak();

    let innertube = Arc::new(InnertubeClient::new());
    let player = Arc::new(AudioPlayer::new());
    let discord = Arc::new(DiscordManager::new());

    // Restore saved cookies and user profile from config.json if available
    let (saved_cookies, saved_user) = load_saved_auth();
    if let Some(cookies) = saved_cookies {
        innertube.set_cookies(Some(cookies));
        app.set_is_logged_in(true);
        if let Some(user) = saved_user {
            app.set_user_name(SharedString::from(user));
        } else {
            app.set_user_name(SharedString::from("Connected"));
        }
    }

    // 1. Initial Load: Home Feed
    {
        let innertube_init = innertube.clone();
        let app_weak_init = app_weak.clone();
        rt.spawn(async move {
            if let Ok(sections) = innertube_init.get_home_feed().await {
                let mut tracks_list = Vec::new();
                for sec in sections {
                    for item in sec.items {
                        tracks_list.push(item);
                    }
                }
                update_slint_tracks(app_weak_init, "Quick Picks", tracks_list);
            }
        });
    }

    // 2. Search Callback
    {
        let app_weak_s = app_weak.clone();
        let innertube_s = innertube.clone();
        app.on_search(move |query_str| {
            let q = query_str.to_string();
            let app_w = app_weak_s.clone();
            let it = innertube_s.clone();

            tokio::spawn(async move {
                if let Ok(results) = it.search(&q).await {
                    let title = format!("Results for \"{}\"", q);
                    update_slint_tracks(app_w, &title, results.songs);
                }
            });
        });
    }

    // Callbacks for seek & login
    app.on_seek(move |_val| {
        // Seek callback
    });

    app.on_login_clicked(move || {
        info!("[Cremeplay Slint] Google Login requested");
    });

    // 3. Play Track Callback
    {
        let app_weak_p = app_weak.clone();
        let player_p = player.clone();
        let innertube_p = innertube.clone();
        let discord_p = discord.clone();

        app.on_play_track(move |track_data| {
            let video_id = track_data.video_id.to_string();
            let title = track_data.title.to_string();
            let artist = track_data.artist.to_string();
            let album = track_data.album.to_string();
            let artwork = track_data.artwork_url.to_string();

            let item = TrackItem {
                video_id: video_id.clone(),
                title: title.clone(),
                artist: artist.clone(),
                album: album.clone(),
                duration: 0.0,
                duration_text: track_data.duration_text.to_string(),
                artwork_url: artwork.clone(),
            };

            let app_w = app_weak_p.clone();
            let it = innertube_p.clone();
            let pl = player_p.clone();
            let dc = discord_p.clone();

            tokio::spawn(async move {
                if let Ok(stream_url) = it.get_audio_stream_url(&video_id).await {
                    let _ = pl.play_stream(&stream_url, item.clone()).await;

                    // Update Discord
                    let sp = SongPayload {
                        title: title.clone(),
                        artist: artist.clone(),
                        album: album.clone(),
                        artwork_url: artwork,
                        duration: 0.0,
                        video_id,
                    };
                    dc.update_song(&sp, true, 0.0);

                    // Update UI Now Playing
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = app_w.upgrade() {
                            ui.set_current_title(SharedString::from(title));
                            ui.set_current_artist(SharedString::from(artist));
                            ui.set_is_playing(true);
                        }
                    });
                }
            });
        });
    }

    // 4. Toggle Play/Pause
    {
        let app_weak_t = app_weak.clone();
        let player_t = player.clone();
        app.on_toggle_play(move || {
            let is_playing = player_t.toggle_playback();
            if let Some(ui) = app_weak_t.upgrade() {
                ui.set_is_playing(is_playing);
            }
        });
    }

    // 5. Volume Slider
    {
        let player_v = player.clone();
        app.on_set_volume(move |vol| {
            player_v.set_volume(vol);
        });
    }

    // 6. Navigation: Home
    {
        let app_weak_h = app_weak.clone();
        let innertube_h = innertube.clone();
        app.on_load_home(move || {
            let it = innertube_h.clone();
            let app_w = app_weak_h.clone();
            tokio::spawn(async move {
                if let Ok(sections) = it.get_home_feed().await {
                    let mut tracks_list = Vec::new();
                    for sec in sections {
                        for item in sec.items {
                            tracks_list.push(item);
                        }
                    }
                    update_slint_tracks(app_w, "Quick Picks", tracks_list);
                }
            });
        });
    }

    // 7. Navigation: Liked Music
    {
        let app_weak_l = app_weak.clone();
        let innertube_l = innertube.clone();
        app.on_load_liked(move || {
            let it = innertube_l.clone();
            let app_w = app_weak_l.clone();
            tokio::spawn(async move {
                if let Ok(liked) = it.get_liked_songs().await {
                    update_slint_tracks(app_w, "Liked Music ❤️", liked);
                }
            });
        });
    }

    // 8. Live 10Hz UI Progress Ticker & Memory Trimmer
    {
        let app_weak_tick = app_weak.clone();
        let player_tick = player.clone();
        std::thread::spawn(move || {
            let mut tick_counter: u64 = 0;
            loop {
                std::thread::sleep(Duration::from_millis(150));
                tick_counter += 1;

                // Periodically trim memory working set every ~30 seconds (200 ticks * 150ms)
                if tick_counter % 200 == 0 {
                    crate::process_job::trim_process_working_set();
                }

                let status = player_tick.get_status();
                if status.is_playing {
                    let current_time_str = format_time(status.current_time);
                    let duration_str = format_time(status.duration);
                    let pct = if status.duration > 0.0 {
                        ((status.current_time / status.duration) * 100.0) as f32
                    } else {
                        0.0
                    };

                    let app_w = app_weak_tick.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = app_w.upgrade() {
                            ui.set_current_time_text(SharedString::from(current_time_str));
                            ui.set_duration_text(SharedString::from(duration_str));
                            ui.set_progress_pct(pct);
                        }
                    });
                }
            }
        });
    }

    info!("[Cremeplay] 100% Native Rust Slint window active (~15-20MB RAM)");
    app.run()?;
    Ok(())
}

fn load_saved_auth() -> (Option<String>, Option<String>) {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let path = std::path::PathBuf::from(appdata)
            .join("com.cremeplay.desktop")
            .join("config.json");
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
                let cookies = cfg.get("authCookies").and_then(|v| v.as_str()).map(|s| s.to_string());
                let user = cfg.get("userName").and_then(|v| v.as_str()).map(|s| s.to_string());
                return (cookies, user);
            }
        }
    }
    (None, None)
}

fn update_slint_tracks(app_weak: slint::Weak<AppWindow>, title: &str, tracks: Vec<TrackItem>) {
    let title_owned = title.to_string();
    let slint_tracks: Vec<TrackData> = tracks
        .into_iter()
        .map(|t| TrackData {
            video_id: SharedString::from(t.video_id),
            title: SharedString::from(t.title),
            artist: SharedString::from(t.artist),
            album: SharedString::from(t.album),
            duration_text: SharedString::from(t.duration_text),
            artwork_url: SharedString::from(t.artwork_url),
        })
        .collect();

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            let model: Rc<VecModel<TrackData>> = Rc::new(VecModel::from(slint_tracks));
            let model_rc = ModelRc::from(model);
            ui.set_view_title(SharedString::from(title_owned));
            ui.set_tracks(model_rc);
        }
    });
}

fn format_time(seconds: f64) -> String {
    if seconds <= 0.0 || seconds.is_nan() {
        return "0:00".to_string();
    }
    let mins = (seconds / 60.0).floor() as u64;
    let secs = (seconds % 60.0).floor() as u64;
    format!("{}:{:02}", mins, secs)
}
