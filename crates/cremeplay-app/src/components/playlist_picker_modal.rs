use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    // Header
    let close_btn = el()
        .key("close-playlist-picker")
        .size(32.0, 32.0)
        .radius(16.0)
        .center()
        .fill(t.hover)
        .child(icon(&icons::CLOSE, 18.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_playlist_picker = false;
            s.playlist_picker_track = None;
        });

    let header = row()
        .w_full()
        .h(40.0)
        .justify_between()
        .center_y()
        .child(text("Add to Playlist").font(typo::TITLE).color(t.heading))
        .child(close_btn);

    // Track preview header
    let track_preview = if let Some(track) = &s.playlist_picker_track {
        let thumb = if let Some(img) = s.get_image(&track.artwork_url) {
            el()
                .size(40.0, 40.0)
                .radius(6.0)
                .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
        } else {
            el()
                .size(40.0, 40.0)
                .radius(6.0)
                .fill(t.hover)
                .center()
                .child(icon(&icons::MUSIC_NOTE, 18.0).color(t.muted))
        };

        row()
            .w_full()
            .gap(12.0)
            .center_y()
            .padding(8.0)
            .radius(8.0)
            .fill(t.band)
            .child(thumb)
            .child(
                col()
                    .gap(2.0)
                    .child(text(&track.title).font(typo::LABEL).color(t.heading))
                    .child(text(&track.artist).font(typo::CAPTION).color(t.muted)),
            )
    } else {
        el()
    };

    let divider = || el().w_full().h(1.0).fill(t.border);

    // Playlists list
    let target_video_id = s.playlist_picker_track.as_ref().map(|t| t.video_id.clone());

    let mut playlists_col = col().w_full().gap(6.0);

    if s.custom_playlists.is_empty() {
        playlists_col = playlists_col.child(
            el()
                .padding(12.0)
                .child(text("No custom playlists yet").font(typo::CAPTION).color(t.muted)),
        );
    } else {
        for pl in &s.custom_playlists {
            let pl_id = pl.id.clone();
            let is_already_added = target_video_id
                .as_ref()
                .map(|id| pl.tracks.iter().any(|t| &t.video_id == id))
                .unwrap_or(false);

            let status_badge = if is_already_added {
                text("Added")
                    .font(typo::CAPTION)
                    .color(t.brand)
            } else {
                text(format!("{} tracks", pl.tracks.len()))
                    .font(typo::CAPTION)
                    .color(t.muted)
            };

            let row_item = row()
                .key(format!("pick-pl-{}", pl.id))
                .w_full()
                .h(44.0)
                .px(12.0)
                .radius(8.0)
                .justify_between()
                .center_y()
                .fill(t.hover)
                .child(
                    text(&pl.title)
                        .font(typo::LABEL)
                        .color(if is_already_added { t.brand } else { t.heading }),
                )
                .child(status_badge)
                .on_press(move |s: &mut AppState| {
                    if let Some(track) = s.playlist_picker_track.clone() {
                        s.add_track_to_custom_playlist(&pl_id, track);
                    }
                    s.show_playlist_picker = false;
                    s.playlist_picker_track = None;
                });

            playlists_col = playlists_col.child(row_item);
        }
    }

    // New playlist button
    let new_pl_btn = row()
        .key("btn-picker-new-playlist")
        .w_full()
        .h(42.0)
        .px(12.0)
        .radius(8.0)
        .center_y()
        .gap(8.0)
        .fill(t.band)
        .child(icon(&icons::NEW_PLAYLIST, 18.0).color(t.brand))
        .child(text("New Playlist").font(typo::LABEL).color(t.brand))
        .on_press(|s: &mut AppState| {
            let count = s.custom_playlists.len() + 1;
            let id = s.create_custom_playlist(format!("Playlist #{}", count));
            if let Some(track) = s.playlist_picker_track.clone() {
                s.add_track_to_custom_playlist(&id, track);
            }
            s.show_playlist_picker = false;
            s.playlist_picker_track = None;
        });

    let modal = col()
        .key("playlist-picker-modal")
        .w(380.0)
        .radius(16.0)
        .fill(t.panel)
        .border(1.0, t.border)
        .padding(20.0)
        .gap(14.0)
        .child(header)
        .child(track_preview)
        .child(divider())
        .child(playlists_col)
        .child(divider())
        .child(new_pl_btn)
        .on_press(|_: &mut AppState| {
            // Absorb clicks inside dialog
        });

    // Backdrop
    col()
        .key("playlist-picker-backdrop")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .fill(rune::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 })
        .child(modal)
        .on_press(|s: &mut AppState| {
            s.show_playlist_picker = false;
            s.playlist_picker_track = None;
        })
}
