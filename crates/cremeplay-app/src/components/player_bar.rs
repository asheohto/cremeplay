use crate::icons;
use crate::state::{AppState, RepeatMode};
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    // 1. Full-width top seekbar (2px / 4px hover)
    let progress = if s.duration > 0.0 {
        (s.current_time / s.duration).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    let seek_bar = row()
        .key("seek-container")
        .w_full()
        .h(6.0)
        .fill(t.border)
        .child(
            el()
                .w_frac(progress)
                .h(6.0)
                .fill(t.brand),
        )
        .on_drag(|s: &mut AppState, d: &mut rune::Drag, _tx: &mut rune::Tx| {
            if d.w > 0.0 && s.duration > 0.0 {
                let ratio = (d.x / d.w).clamp(0.0, 1.0) as f64;
                let target_time = ratio * s.duration;
                s.current_time = target_time;
                s.player.seek(target_time);
            }
        });

    // Format mm:ss
    let fmt_time = |secs: f64| {
        let total_sec = secs.max(0.0) as u64;
        let m = total_sec / 60;
        let s = total_sec % 60;
        format!("{}:{:02}", m, s)
    };

    let time_str = format!("{} / {}", fmt_time(s.current_time), fmt_time(s.duration));

    // 2. Left Playback Controls
    let shuffle_btn = el()
        .key("btn-shuffle")
        .size(36.0, 36.0)
        .radius(18.0)
        .center()
        .fill(if s.shuffle { t.hover } else { rune::Color::TRANSPARENT })
        .child(
            icon(&icons::SHUFFLE, 18.0)
                .color(if s.shuffle { t.brand } else { t.muted }),
        )
        .on_press(|s: &mut AppState| {
            s.shuffle = !s.shuffle;
        });

    let prev_btn = el()
        .key("btn-prev")
        .size(40.0, 40.0)
        .radius(20.0)
        .center()
        .fill(t.hover)
        .child(icon(&icons::PREV, 22.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.play_prev();
        });

    let play_btn = el()
        .key("btn-play")
        .size(52.0, 52.0)
        .radius(26.0)
        .fill(t.brand)
        .center()
        .child(
            icon(if s.is_playing { &icons::PAUSE } else { &icons::PLAY }, 26.0)
                .color(t.brand_text),
        )
        .on_press(|s: &mut AppState| {
            s.is_playing = s.player.toggle_playback();
        });

    let next_btn = el()
        .key("btn-next")
        .size(40.0, 40.0)
        .radius(20.0)
        .center()
        .fill(t.hover)
        .child(icon(&icons::NEXT, 22.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.play_next();
        });

    let repeat_btn = el()
        .key("btn-repeat")
        .size(36.0, 36.0)
        .radius(18.0)
        .center()
        .fill(if s.repeat_mode != RepeatMode::Off { t.hover } else { rune::Color::TRANSPARENT })
        .child(
            icon(&icons::REPEAT, 18.0)
                .color(if s.repeat_mode != RepeatMode::Off { t.brand } else { t.muted }),
        )
        .on_press(|s: &mut AppState| {
            s.repeat_mode = match s.repeat_mode {
                RepeatMode::Off => RepeatMode::All,
                RepeatMode::All => RepeatMode::One,
                RepeatMode::One => RepeatMode::Off,
            };
        });

    let time_label = text(time_str)
        .font(typo::MONO_TIMER)
        .color(t.muted);

    let left = row()
        .center_y()
        .gap(8.0)
        .child(shuffle_btn)
        .child(prev_btn)
        .child(play_btn)
        .child(next_btn)
        .child(repeat_btn)
        .child(time_label);

    // 3. Middle: Track Info & Artwork
    let thumb = if let Some(track) = &s.current_track {
        if let Some(img) = s.get_image(&track.artwork_url) {
            el()
                .size(48.0, 48.0)
                .radius(8.0)
                .border(1.0, t.border)
                .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
        } else {
            el()
                .size(48.0, 48.0)
                .radius(8.0)
                .border(1.0, t.border)
                .fill(t.band)
                .center()
                .child(icon(&icons::MUSIC_NOTE, 20.0).color(t.muted))
        }
    } else {
        el()
            .size(48.0, 48.0)
            .radius(8.0)
            .border(1.0, t.border)
            .fill(t.band)
            .center()
            .child(icon(&icons::MUSIC_NOTE, 20.0).color(t.muted))
    };

    let track_title = text(
        s.current_track
            .as_ref()
            .map(|t| t.title.as_str())
            .unwrap_or("Not playing"),
    )
    .font(typo::TITLE)
    .color(t.heading);

    let track_artist = text(
        s.current_track
            .as_ref()
            .map(|t| t.artist.as_str())
            .unwrap_or("Pick a song to start"),
    )
    .font(typo::CAPTION)
    .color(t.muted);

    let meta = col()
        .gap(2.0)
        .child(track_title)
        .child(track_artist);

    let is_liked = s
        .current_track
        .as_ref()
        .map(|track| s.is_track_liked(&track.video_id))
        .unwrap_or(false);

    let like_btn = el()
        .key("btn-like")
        .size(36.0, 36.0)
        .radius(18.0)
        .center()
        .fill(if is_liked { t.hover } else { rune::Color::TRANSPARENT })
        .child(
            icon(if is_liked { &icons::HEART_FILLED } else { &icons::HEART }, 20.0)
                .color(if is_liked { t.brand } else { t.muted }),
        )
        .on_press(|s: &mut AppState| {
            if let Some(track) = s.current_track.clone() {
                s.toggle_like(track);
            }
        });

    let mid = row()
        .key("bar-mid")
        .center_y()
        .gap(14.0)
        .px(10.0)
        .py(6.0)
        .radius(10.0)
        .fill(rune::Color::TRANSPARENT)
        .child(thumb)
        .child(meta)
        .child(like_btn)
        .on_press(|s: &mut AppState| {
            s.show_now_playing = !s.show_now_playing;
        });

    // 4. Right Controls: Volume & Drawer toggle
    let mute_btn = el()
        .key("btn-mute")
        .size(36.0, 36.0)
        .radius(18.0)
        .center()
        .child(
            icon(
                if s.is_muted || s.volume == 0.0 {
                    &icons::VOLUME_MUTE
                } else {
                    &icons::VOLUME_HIGH
                },
                20.0,
            )
            .color(t.heading),
        )
        .on_press(|s: &mut AppState| {
            if s.is_muted {
                s.is_muted = false;
                s.volume = s.prev_volume;
                s.player.set_volume(s.volume);
            } else {
                s.is_muted = true;
                s.prev_volume = s.volume;
                s.volume = 0.0;
                s.player.set_volume(0.0);
            }
        });

    // Volume Track (84px pill, clickable & draggable)
    let vol_track = row()
        .key("volume-slider")
        .w(84.0)
        .h(6.0)
        .radius(3.0)
        .fill(t.border)
        .child(
            el()
                .w_frac(s.volume)
                .h(6.0)
                .radius(3.0)
                .fill(t.brand),
        )
        .on_drag(|s: &mut AppState, d: &mut rune::Drag, _tx: &mut rune::Tx| {
            if d.w > 0.0 {
                let vol = (d.x / d.w).clamp(0.0, 1.0);
                s.volume = vol;
                s.is_muted = false;
                s.player.set_volume(vol);
            }
        });

    let queue_btn = el()
        .key("btn-queue")
        .size(38.0, 38.0)
        .radius(19.0)
        .center()
        .fill(if s.show_queue_drawer { t.hover } else { rune::Color::TRANSPARENT })
        .child(
            icon(&icons::QUEUE, 20.0)
                .color(if s.show_queue_drawer { t.brand } else { t.heading }),
        )
        .on_press(|s: &mut AppState| {
            s.show_queue_drawer = !s.show_queue_drawer;
        });

    let vol_pct = text(format!("{}%", (s.volume * 100.0).round() as u32))
        .font(typo::CAPTION)
        .color(t.muted);

    let right = row()
        .center_y()
        .gap(10.0)
        .child(mute_btn)
        .child(vol_track)
        .child(vol_pct)
        .child(queue_btn);

    // Full 76px bottom player bar
    let main_bar = row()
        .w_full()
        .h(72.0)
        .fill(t.panel)
        .border(1.0, t.border)
        .px(24.0)
        .justify_between()
        .center_y()
        .child(left)
        .child(mid)
        .child(right);

    col()
        .key("footer-bar")
        .w_full()
        .h(76.0)
        .child(seek_bar)
        .child(main_bar)
}
