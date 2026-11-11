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

    let is_custom = pl.browse_id.starts_with("cp-pl-");

    // 1. Playlist Header Banner
    let pl_thumb = if is_custom && pl.tracks.len() >= 4 {
        let mut imgs = Vec::new();
        for trk in pl.tracks.iter().take(4) {
            if let Some(img) = s.get_image(&trk.artwork_url) {
                imgs.push(img);
            }
        }
        if imgs.len() == 4 {
            col()
                .size(160.0, 160.0)
                .radius(16.0)
                .clip_rounded(16.0)
                .child(
                    row()
                        .size(160.0, 80.0)
                        .child(el().size(80.0, 80.0).pixels_fit(imgs[0].pixels(), rune::SurfaceFit::Fill))
                        .child(el().size(80.0, 80.0).pixels_fit(imgs[1].pixels(), rune::SurfaceFit::Fill))
                )
                .child(
                    row()
                        .size(160.0, 80.0)
                        .child(el().size(80.0, 80.0).pixels_fit(imgs[2].pixels(), rune::SurfaceFit::Fill))
                        .child(el().size(80.0, 80.0).pixels_fit(imgs[3].pixels(), rune::SurfaceFit::Fill))
                )
        } else if let Some(img) = s.get_image(&pl.artwork_url) {
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
        }
    } else if let Some(img) = s.get_image(&pl.artwork_url) {
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

    // Play All & Delete Buttons
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
        .child(icon(&icons::PLAY, 18.0).color(t.on_brand))
        .child(text("Play All").font(typo::LABEL).color(t.on_brand))
        .on_press(move |s: &mut AppState| {
            if let Some(track) = first_track.clone() {
                s.queue = pl_tracks_clone.clone();
                s.queue_index = 0;
                s.play_track(track);
            }
        });

    let (rename_btn, delete_btn) = if is_custom {
        let pl_id = pl.browse_id.clone();
        let pl_title = pl.title.clone();
        let pl_id_del = pl.browse_id.clone();
        let ren = row()
            .key("btn-rename-pl")
            .h(40.0)
            .px(16.0)
            .radius(20.0)
            .fill(t.hover)
            .border(1.0, t.border)
            .center_y()
            .gap(8.0)
            .child(icon(&icons::SETTINGS, 16.0).color(t.heading))
            .child(text("Rename").font(typo::LABEL).color(t.heading))
            .on_press(move |s: &mut AppState| {
                s.rename_playlist_id = Some(pl_id.clone());
                s.rename_playlist_edit = rune::TextEdit::new(pl_title.clone());
                s.show_rename_playlist_modal = true;
            });

        let del = row()
            .key("btn-delete-pl")
            .h(40.0)
            .px(16.0)
            .radius(20.0)
            .fill(t.hover)
            .border(1.0, t.border)
            .center_y()
            .gap(8.0)
            .child(icon(&icons::CLOSE, 16.0).color(t.muted))
            .child(text("Delete Playlist").font(typo::LABEL).color(t.heading))
            .on_press(move |s: &mut AppState| {
                s.delete_custom_playlist(&pl_id_del);
            });
        (Some(ren), Some(del))
    } else {
        (None, None)
    };

    let mut actions = row().gap(12.0).center_y().child(play_all_btn);
    if let Some(ren) = rename_btn {
        actions = actions.child(ren);
    }
    if let Some(del) = delete_btn {
        actions = actions.child(del);
    }

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
                .child(actions),
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

        let (move_up_btn, move_down_btn, remove_btn) = if is_custom {
            let pl_id = pl.browse_id.clone();
            let vid = track.video_id.clone();
            let pl_id_up = pl.browse_id.clone();
            let pl_id_down = pl.browse_id.clone();

            let up = if idx > 0 {
                Some(
                    el()
                        .key(format!("btn-up-track-{}", idx))
                        .size(24.0, 24.0)
                        .radius(12.0)
                        .center()
                        .fill(t.hover)
                        .child(icon(&icons::CHEVRON_UP, 14.0).color(t.muted))
                        .on_press(move |s: &mut AppState| {
                            s.reorder_custom_playlist_track(&pl_id_up, idx, idx - 1);
                        }),
                )
            } else {
                None
            };

            let down = if idx + 1 < pl.tracks.len() {
                Some(
                    el()
                        .key(format!("btn-down-track-{}", idx))
                        .size(24.0, 24.0)
                        .radius(12.0)
                        .center()
                        .fill(t.hover)
                        .child(icon(&icons::CHEVRON_DOWN, 14.0).color(t.muted))
                        .on_press(move |s: &mut AppState| {
                            s.reorder_custom_playlist_track(&pl_id_down, idx, idx + 1);
                        }),
                )
            } else {
                None
            };

            let rem = Some(
                el()
                    .key(format!("btn-rem-track-{}", idx))
                    .size(24.0, 24.0)
                    .radius(12.0)
                    .center()
                    .fill(t.hover)
                    .child(icon(&icons::CLOSE, 12.0).color(t.muted))
                    .on_press(move |s: &mut AppState| {
                        s.remove_track_from_custom_playlist(&pl_id, &vid);
                    }),
            );

            (up, down, rem)
        } else {
            (None, None, None)
        };

        let track_add_clone = track.clone();
        let add_btn = el()
            .key(format!("btn-add-pl-track-{}", idx))
            .size(28.0, 28.0)
            .radius(14.0)
            .center()
            .fill(rune::Color::TRANSPARENT)
            .child(icon(&icons::NEW_PLAYLIST, 16.0).color(t.muted))
            .on_press(move |s: &mut AppState| {
                s.playlist_picker_track = Some(track_add_clone.clone());
                s.show_playlist_picker = true;
            });

        let mut right_side = row()
            .center_y()
            .gap(8.0)
            .child(text(dur_str).font(typo::MONO_TIMER).color(t.muted))
            .child(add_btn);

        if let Some(btn) = move_up_btn {
            right_side = right_side.child(btn);
        }
        if let Some(btn) = move_down_btn {
            right_side = right_side.child(btn);
        }
        if let Some(btn) = remove_btn {
            right_side = right_side.child(btn);
        }

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
            .child(right_side)
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
