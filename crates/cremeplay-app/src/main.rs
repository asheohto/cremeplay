mod agent_server;
mod components;
pub mod icons;
mod state;
mod theme;

use components::{explore_view, header, home_view, library_view, now_playing, player_bar, playlist_view, queue_drawer, search_view, settings_modal, sidebar};
use state::{ActiveView, AppState};
use std::time::Duration;
use rune::{col, row, AppSpec, Cx, El, Key, KeyEvent, Tx, Waker};

fn view(s: &AppState, _: &mut Cx<AppState>) -> El {
    let t = &s.theme;

    // Center content according to active view
    let main_content = match s.active_view {
        ActiveView::Home => home_view::render(s),
        ActiveView::Search => search_view::render(s),
        ActiveView::Explore => explore_view::render(s),
        ActiveView::Library => library_view::render(s),
        ActiveView::PlaylistDetail => playlist_view::render(s),
    };

    let center_area = col()
        .key("main-center-area")
        .grow(1.0)
        .min_h(0.0)
        .fill(t.surface)
        .child(main_content);

    // Main middle row: Sidebar + Center + Queue Drawer (if open)
    let mut middle_row = row()
        .key("main-middle-row")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(sidebar::render(s))
        .child(center_area);

    if s.show_queue_drawer {
        middle_row = middle_row.child(queue_drawer::render(s));
    }

    // Root app layout
    let app_chrome = col()
        .key("app-chrome")
        .w_full()
        .h_full()
        .fill(t.surface)
        .child(header::render(s))
        .child(middle_row)
        .child(player_bar::render(s));

    let mut root = rune::el()
        .key("app-root")
        .w_full()
        .h_full()
        .child(app_chrome);

    // Overlays: Now Playing & Settings
    if s.show_now_playing {
        root = root.child(now_playing::render(s).absolute(0.0, 0.0).w_full().h_full());
    }

    if s.show_settings {
        root = root.child(settings_modal::render(s).absolute(0.0, 0.0).w_full().h_full());
    }

    root
}

/// Setup background fetching and ticker
fn setup(s: &mut AppState, waker: Waker) {
    *s.waker.lock().unwrap() = Some(waker.clone());
    s.init_image_cache(waker.clone());

    let innertube = s.innertube.clone();
    let config_mgr = s.config_mgr.clone();
    let pending_home = s.pending_home_feed.clone();
    let pending_explore = s.pending_explore_feed.clone();
    let pending_pl = s.pending_playlists.clone();
    let pending_acc = s.pending_account.clone();
    let waker_clone = waker.clone();

    tokio::spawn(async move {
        let cfg = config_mgr.get();
        let taste_artists = cfg.taste_artists.clone();

        // If authenticated, fetch profile
        if cfg.auth_cookies.is_some() {
            let profile = innertube.get_account_profile().await;
            if profile.is_logged_in {
                *pending_acc.lock().unwrap() = Some(profile);
                waker_clone();
            }
        }

        // Fetch home feed
        if let Ok(sections) = innertube.get_home_feed(&taste_artists).await {
            *pending_home.lock().unwrap() = Some(sections);
            waker_clone();
        }

        // Fetch explore feed
        if let Ok(sections) = innertube.get_explore_feed().await {
            *pending_explore.lock().unwrap() = Some(sections);
            waker_clone();
        }

        // Fetch sidebar playlists
        if let Ok(playlists) = innertube.get_user_playlists().await {
            *pending_pl.lock().unwrap() = Some(playlists);
            waker_clone();
        }
    });
}

/// Poll ticker: updates progress bar & audio state smoothly while playing
fn poll(s: &mut AppState, _: f64, tx: &mut Tx) -> bool {
    let mut state_changed = false;

    // Drain async home feed
    if let Some(sections) = s.pending_home_feed.lock().unwrap().take() {
        s.home_sections = sections;
        s.is_loading_home = false;
        state_changed = true;
    }

    // Drain async explore feed
    if let Some(sections) = s.pending_explore_feed.lock().unwrap().take() {
        s.explore_sections = sections;
        s.is_loading_explore = false;
        state_changed = true;
    }

    // Drain async search results
    if let Some(results) = s.pending_search_results.lock().unwrap().take() {
        s.search_results = Some(results);
        s.is_searching = false;
        state_changed = true;
    }

    // Drain async lyrics
    if let Some(lyrics) = s.pending_lyrics.lock().unwrap().take() {
        s.lyrics = Some(lyrics);
        state_changed = true;
    }

    // Drain async sponsor segments
    if let Some(segs) = s.pending_segments.lock().unwrap().take() {
        s.sponsor_segments = segs;
        state_changed = true;
    }

    // Drain playlists
    if let Some(playlists) = s.pending_playlists.lock().unwrap().take() {
        s.sidebar_playlists = playlists;
        state_changed = true;
    }

    // Drain playlist detail
    if let Some(detail) = s.pending_playlist_detail.lock().unwrap().take() {
        s.current_playlist = Some(detail);
        s.is_loading_playlist = false;
        state_changed = true;
    }

    // Drain account profile
    if let Some(profile) = s.pending_account.lock().unwrap().take() {
        s.account = Some(profile);
        state_changed = true;
    }

    let status = s.player.get_status();
    let prev_playing = s.is_playing;
    let prev_time = s.current_time;

    s.is_playing = status.is_playing;
    s.current_time = status.current_time;
    if status.duration > 0.0 {
        s.duration = status.duration;
    }

    // Auto-advance track on finish
    if s.is_playing && s.duration > 0.0 && s.current_time >= s.duration - 0.5 {
        match s.repeat_mode {
            crate::state::RepeatMode::One => {
                if let Some(track) = s.current_track.clone() {
                    s.play_track(track);
                }
            }
            _ => {
                s.play_next();
            }
        }
    }

    // Auto-skip SponsorBlock segments
    if s.is_playing && !s.sponsor_segments.is_empty() {
        for &(start, end) in &s.sponsor_segments {
            if s.current_time >= start && s.current_time < end {
                log::info!("[SponsorBlock] Auto-skipping segment {:.1}s -> {:.1}s", start, end);
                s.player.seek(end + 0.1);
                s.current_time = end + 0.1;
                state_changed = true;
                break;
            }
        }
    }

    // Tuna update
    if let Some(_track) = &s.current_track {
        s.tuna.update_playback_state(s.is_playing, s.current_time);
    }

    // Keep agent snapshot in sync for ultra-fast, zero-lock inspection
    crate::agent_server::update_snapshot(crate::agent_server::AppStatusDto {
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
    });

    // Wake 30 fps while playing for smooth seekbar, or 4 fps while paused for responsive updates
    if s.is_playing {
        tx.wake_after(Duration::from_millis(33));
        return true;
    } else {
        tx.wake_after(Duration::from_millis(250));
    }

    state_changed || prev_playing != s.is_playing || (prev_time - s.current_time).abs() > 0.5
}

/// Global key shortcut handler
fn on_key(s: &mut AppState, ev: &KeyEvent, tx: &mut Tx) -> bool {
    // If an overlay/modal is open, Escape closes it
    if ev.key == Key::Escape {
        if s.show_settings {
            s.show_settings = false;
            return true;
        }
        if s.show_now_playing {
            s.show_now_playing = false;
            return true;
        }
        if s.show_queue_drawer {
            s.show_queue_drawer = false;
            return true;
        }
    }

    // When text input is not focused, handle playback controls
    if tx.focused().is_none() {
        if let Key::Text(t) = &ev.key {
            match t.as_str() {
                " " => {
                    s.is_playing = s.player.toggle_playback();
                    return true;
                }
                "n" | "N" => {
                    s.play_next();
                    return true;
                }
                "p" | "P" => {
                    s.play_prev();
                    return true;
                }
                "m" | "M" => {
                    s.is_muted = !s.is_muted;
                    if s.is_muted {
                        s.prev_volume = s.volume;
                        s.volume = 0.0;
                        s.player.set_volume(0.0);
                    } else {
                        s.volume = if s.prev_volume > 0.0 { s.prev_volume } else { 0.7 };
                        s.player.set_volume(s.volume);
                    }
                    return true;
                }
                "q" | "Q" => {
                    s.show_queue_drawer = !s.show_queue_drawer;
                    return true;
                }
                "s" | "S" => {
                    s.shuffle = !s.shuffle;
                    return true;
                }
                "r" | "R" => {
                    s.repeat_mode = match s.repeat_mode {
                        crate::state::RepeatMode::Off => crate::state::RepeatMode::All,
                        crate::state::RepeatMode::All => crate::state::RepeatMode::One,
                        crate::state::RepeatMode::One => crate::state::RepeatMode::Off,
                    };
                    return true;
                }
                _ => {}
            }
        }
        match ev.key {
            Key::Command(',') => {
                s.show_settings = !s.show_settings;
                return true;
            }
            Key::Left => {
                let new_time = (s.current_time - 5.0).max(0.0);
                s.player.seek(new_time);
                s.current_time = new_time;
                return true;
            }
            Key::Right => {
                let new_time = (s.current_time + 5.0).min(s.duration);
                s.player.seek(new_time);
                s.current_time = new_time;
                return true;
            }
            Key::Up => {
                let new_vol = (s.volume + 0.05).min(1.0);
                s.volume = new_vol;
                s.is_muted = false;
                s.player.set_volume(new_vol);
                return true;
            }
            Key::Down => {
                let new_vol = (s.volume - 0.05).max(0.0);
                s.volume = new_vol;
                s.player.set_volume(new_vol);
                return true;
            }
            _ => {}
        }
    }

    false
}

#[cfg(windows)]
fn start_memory_trimmer() {
    tokio::spawn(async {
        use std::os::raw::c_void;
        type HANDLE = *mut c_void;
        type BOOL = i32;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn K32EmptyWorkingSet(h_process: HANDLE) -> BOOL;
            fn GetCurrentProcess() -> HANDLE;
        }

        // Wait 5 seconds after launch to allow initial WGPU shaders and textures to settle
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        loop {
            unsafe {
                let cur = GetCurrentProcess();
                let _ = K32EmptyWorkingSet(cur);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        }
    });
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");
    let _guard = rt.enter();

    #[cfg(windows)]
    start_memory_trimmer();

    let mut spec = AppSpec::new("Cremeplay", (1160.0, 780.0), AppState::new, view);
    spec.setup = Some(setup);
    spec.poll = Some(poll);
    spec.on_key = Some(on_key);

    let app = rune::App::new(spec);
    let handle = app.handle();
    agent_server::start(handle);

    app.run();
}
