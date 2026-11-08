use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    if s.is_loading_playlist && s.current_playlist.is_none() {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(12.0)
            .child(icon(&icons::QUEUE, 36.0).color(t.brand))
            .child(text("Loading playlist…").font(typo::BODY).color(t.muted));
    }

    let pl = match &s.current_playlist {
        Some(p) => p,
        None => {
            return col()
                .w_full()
                .h_full()
                .center()
                .gap(12.0)
                .child(icon(&icons::QUEUE, 40.0).color(t.muted))
                .child(text("No playlist selected").font(typo::TITLE).color(t.muted));
        }
    };

    // 1. Playlist Header Banner
    let pl_thumb = if let Some(img) = s.get_image(&pl.artwork_url) {
        el()
            .size(160.0, 160.0)
            .radius(16.0)
            .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
    } else {
        el()
            .size(160.0, 160.0)
            .radius(16.0)
            .fill(t.band)
            .center()
            .child(icon(&icons::QUEUE, 48.0).color(t.muted))
    };

    let title_text = text(&pl.title).font(typo::HERO).color(t.heading);
    let meta_text = text(format!("{} • {} songs", pl.subtitle, pl.track_count))
        .font(typo::BODY)
        .color(t.muted);

    // Play All Button
    let pl_tracks_clone = pl.tracks.clone();
    let first_track = pl.tracks.first().cloned();
    let play_all_btn = row()
        .key("btn-play-all")
        .h(40.0)
        .px(20.0)
        .radius(20.0)
        .fill(t.brand)
        .center_y()
        .gap(8.0)
        .child(icon(&icons::PLAY, 18.0).color(t.brand_text))
        .child(text("Play All").font(typo::LABEL).color(t.brand_text))
        .on_press(move |s: &mut AppState| {
            if let Some(track) = first_track.clone() {
                s.queue = pl_tracks_clone.clone();
                s.queue_index = 0;
                s.play_track(track);
            }
        });

    let header_banner = row()
        .w_full()
        .gap(24.0)
        .center_y()
        .child(pl_thumb)
        .child(
            col()
                .gap(8.0)
                .child(title_text)
                .child(meta_text)
                .child(play_all_btn),
        );

    // 2. Track List
    let mut tracks_col = col().w_full().gap(4.0);
    let all_tracks = pl.tracks.clone();

    for (idx, track) in pl.tracks.iter().enumerate() {
        let is_current = s
            .current_track
            .as_ref()
            .map(|c| c.video_id == track.video_id)
            .unwrap_or(false);

        let bg = if is_current { t.band } else { rune::Color::TRANSPARENT };
        let track_clone = track.clone();
        let queue_clone = all_tracks.clone();

        let track_idx = text(format!("{}", idx + 1))
            .font(typo::MONO_TIMER)
            .color(if is_current { t.brand } else { t.muted });

        let track_thumb = if let Some(img) = s.get_image(&track.artwork_url) {
            el()
                .size(40.0, 40.0)
                .radius(6.0)
                .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
        } else {
            el()
                .size(40.0, 40.0)
                .radius(6.0)
                .fill(t.band)
                .center()
                .child(icon(&icons::MUSIC_NOTE, 16.0).color(t.muted))
        };

        let total_sec = track.duration as u64;
        let dur_str = format!("{}:{:02}", total_sec / 60, total_sec % 60);

        let track_row = row()
            .key(format!("pl-track-{}", idx))
            .w_full()
            .h(52.0)
            .radius(10.0)
            .px(12.0)
            .center_y()
            .justify_between()
            .fill(bg)
            .child(
                row()
                    .center_y()
                    .gap(14.0)
                    .child(row().w(28.0).center().child(track_idx))
                    .child(track_thumb)
                    .child(
                        col()
                            .gap(2.0)
                            .child(text(&track.title).font(typo::LABEL).color(t.heading))
                            .child(text(&track.artist).font(typo::CAPTION).color(t.muted)),
                    ),
            )
            .child(text(dur_str).font(typo::MONO_TIMER).color(t.muted))
            .on_press(move |s: &mut AppState| {
                s.queue = queue_clone.clone();
                s.queue_index = idx;
                s.play_track(track_clone.clone());
            });

        tracks_col = tracks_col.child(track_row);
    }

    col()
        .key("playlist-detail-view")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .padding(32.0)
        .gap(32.0)
        .child(header_banner)
        .child(tracks_col)
}
