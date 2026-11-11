use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    if s.is_loading_artist && s.current_artist.is_none() {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(12.0)
            .child(icon(&icons::MUSIC_NOTE, 36.0).color(t.brand))
            .child(text("Loading artist profile…").font(typo::BODY).color(t.muted));
    }

    let artist = match &s.current_artist {
        Some(a) => a,
        None => {
            return col()
                .w_full()
                .h_full()
                .center()
                .gap(12.0)
                .child(icon(&icons::MUSIC_NOTE, 40.0).color(t.muted))
                .child(text("No artist selected").font(typo::TITLE).color(t.muted));
        }
    };

    let mut content = col()
        .key("artist-content")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .gap(32.0)
        .padding(32.0);

    // 1. Artist Hero Banner
    let avatar = if let Some(img) = s.get_image(&artist.avatar_url) {
        el()
            .size(160.0, 160.0)
            .radius(80.0)
            .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
    } else {
        el()
            .size(160.0, 160.0)
            .radius(80.0)
            .fill(t.band)
            .center()
            .child(icon(&icons::MUSIC_NOTE, 48.0).color(t.brand))
    };

    let name_text = text(&artist.name).font(typo::HERO).color(t.heading);
    let subs_text = text(if !artist.subscribers.is_empty() {
        format!("Artist • {}", artist.subscribers)
    } else {
        "Artist".to_string()
    })
    .font(typo::BODY)
    .color(t.muted);

    // Play Top Songs Button
    let top_songs_clone = artist.top_songs.clone();
    let first_track = artist.top_songs.first().cloned();
    let play_btn = row()
        .key("btn-artist-play")
        .h(40.0)
        .px(20.0)
        .radius(20.0)
        .fill(t.brand)
        .center_y()
        .gap(8.0)
        .child(icon(&icons::PLAY, 18.0).color(t.on_brand))
        .child(text("Play Top Songs").font(typo::LABEL).color(t.on_brand))
        .on_press(move |s: &mut AppState| {
            if let Some(track) = first_track.clone() {
                s.queue = top_songs_clone.clone();
                s.queue_index = 0;
                s.play_track(track);
            }
        });

    let hero_banner = row()
        .w_full()
        .gap(28.0)
        .center_y()
        .child(avatar)
        .child(
            col()
                .gap(10.0)
                .child(name_text)
                .child(subs_text)
                .child(play_btn),
        );

    content = content.child(hero_banner);

    // 2. Top Songs List
    if !artist.top_songs.is_empty() {
        let top_header = text("Top Songs").font(typo::TITLE).color(t.heading);
        let mut songs_col = col().w_full().gap(4.0);
        let all_songs = artist.top_songs.clone();

        for (idx, track) in artist.top_songs.iter().take(10).enumerate() {
            let is_current = s
                .current_track
                .as_ref()
                .map(|c| c.video_id == track.video_id)
                .unwrap_or(false);

            let bg = if is_current { t.band } else { rune::Color::TRANSPARENT };
            let track_clone = track.clone();
            let queue_clone = all_songs.clone();

            let thumb = if let Some(img) = s.get_image(&track.artwork_url) {
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
                    .child(icon(&icons::MUSIC_NOTE, 18.0).color(t.muted))
            };

            let track_add_clone = track.clone();
            let add_btn = el()
                .key(format!("artist-song-add-{}", idx))
                .size(28.0, 28.0)
                .radius(14.0)
                .center()
                .fill(rune::Color::TRANSPARENT)
                .child(icon(&icons::NEW_PLAYLIST, 16.0).color(t.muted))
                .on_press(move |s: &mut AppState| {
                    s.playlist_picker_track = Some(track_add_clone.clone());
                    s.show_playlist_picker = true;
                });

            let song_right = row()
                .center_y()
                .gap(8.0)
                .child(
                    text(&track.duration_text)
                        .font(typo::MONO_TIMER)
                        .color(t.muted),
                )
                .child(add_btn);

            let row_item = row()
                .key(format!("artist-song-{}", idx))
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
                        .gap(16.0)
                        .child(
                            text(format!("{}", idx + 1))
                                .font(typo::MONO_TIMER)
                                .color(t.muted),
                        )
                        .child(thumb)
                        .child(
                            col()
                                .gap(2.0)
                                .child(text(&track.title).font(typo::LABEL).color(t.heading))
                                .child(text(&track.album).font(typo::CAPTION).color(t.muted)),
                        ),
                )
                .child(song_right)
                .on_press(move |s: &mut AppState| {
                    s.queue = queue_clone.clone();
                    s.queue_index = idx;
                    s.play_track(track_clone.clone());
                });

            songs_col = songs_col.child(row_item);
        }

        content = content.child(
            col()
                .gap(12.0)
                .child(top_header)
                .child(songs_col),
        );
    }

    // 3. Sections: Albums, Singles, etc.
    for (sec_idx, sec) in artist.sections.iter().enumerate() {
        if sec.items.is_empty() {
            continue;
        }

        let sec_header = text(&sec.title).font(typo::TITLE).color(t.heading);
        let mut shelf = row().w_full().gap(16.0);

        for (item_idx, item) in sec.items.iter().take(6).enumerate() {
            let thumb = if let Some(img) = s.get_image(&item.artwork_url) {
                el()
                    .size(130.0, 130.0)
                    .radius(10.0)
                    .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
            } else {
                el()
                    .size(130.0, 130.0)
                    .radius(10.0)
                    .fill(t.band)
                    .center()
                    .child(icon(&icons::QUEUE, 32.0).color(t.muted))
            };

            let browse_id = item.browse_id.clone().unwrap_or_default();
            let card = col()
                .key(format!("art-sec-{}-{}", sec_idx, item_idx))
                .w(144.0)
                .min_w(0.0)
                .padding(8.0)
                .radius(12.0)
                .fill(t.panel)
                .border(1.0, t.border)
                .gap(6.0)
                .child(thumb)
                .child(text(&item.title).font(typo::LABEL).color(t.heading).max_lines(2).w_full())
                .child(text(&item.subtitle).font(typo::CAPTION).color(t.muted).truncate().w_full())
                .on_press(move |s: &mut AppState| {
                    if !browse_id.is_empty() {
                        s.load_playlist(&browse_id);
                    }
                });

            shelf = shelf.child(card);
        }

        content = content.child(
            col()
                .gap(12.0)
                .child(sec_header)
                .child(shelf),
        );
    }

    content
}
