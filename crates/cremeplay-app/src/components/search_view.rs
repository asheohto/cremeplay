use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    if s.is_searching {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(14.0)
            .child(icon(&icons::SEARCH, 36.0).color(t.brand))
            .child(text("Searching YouTube Music…").font(typo::BODY).color(t.muted));
    }

    let results = match &s.search_results {
        Some(r) => r,
        None => {
            let chips = ["Trending Songs", "Lofi Beats", "Chill Acoustic", "Synthwave", "City Pop", "Anime OST", "Classical Focus"];
            let mut chip_row = row().gap(8.0);
            for chip in chips {
                let chip_str = chip.to_string();
                chip_row = chip_row.child(
                    row()
                        .h(34.0)
                        .px(16.0)
                        .radius(17.0)
                        .center()
                        .fill(t.panel)
                        .border(1.0, t.border)
                        .child(text(chip).font(typo::LABEL).color(t.heading))
                        .on_press(move |s: &mut AppState| {
                            s.trigger_search(chip_str.clone());
                        }),
                );
            }

            return col()
                .w_full()
                .h_full()
                .center()
                .gap(18.0)
                .child(icon(&icons::SEARCH, 48.0).color(t.brand))
                .child(text("Search songs, artists, or albums").font(typo::TITLE).color(t.heading))
                .child(text("Type in the search bar above or pick a mood:").font(typo::BODY).color(t.muted))
                .child(chip_row);
        }
    };

    let mut content = col()
        .key("search-results")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .gap(28.0)
        .padding(32.0);

    // Top Result if available
    if let Some(top) = &results.top_result {
        let top_title = text(&top.title).font(typo::TITLE).color(t.heading);
        let top_sub = text(&top.subtitle).font(typo::CAPTION).color(t.muted);
        let badge = row()
            .h(22.0)
            .radius(4.0)
            .px(8.0)
            .center()
            .fill(t.hover)
            .child(text(&top.result_type).font(typo::CAPTION).color(t.brand));

        let top_thumb = if let Some(img) = s.get_image(&top.artwork_url) {
            el()
                .size(72.0, 72.0)
                .radius(10.0)
                .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
        } else {
            el()
                .size(72.0, 72.0)
                .radius(10.0)
                .fill(t.band)
                .center()
                .child(icon(&icons::PLAY, 28.0).color(t.brand))
        };

        let mut top_card = row()
            .key("top-result-card")
            .w(440.0)
            .h(96.0)
            .radius(16.0)
            .fill(t.panel)
            .border(1.0, t.border)
            .padding(14.0)
            .center_y()
            .gap(16.0)
            .child(top_thumb)
            .child(
                col()
                    .gap(4.0)
                    .child(badge)
                    .child(top_title)
                    .child(top_sub),
            );

        if let Some(vid) = top.video_id.clone() {
            let track = cremeplay_core::innertube::TrackItem {
                video_id: vid,
                title: top.title.clone(),
                artist: top.subtitle.clone(),
                album: String::new(),
                duration: 0.0,
                duration_text: String::new(),
                artwork_url: top.artwork_url.clone(),
            };
            top_card = top_card.on_press(move |s: &mut AppState| {
                s.play_track(track.clone());
            });
        }

        content = content.child(
            col()
                .gap(12.0)
                .child(text("Top result").font(typo::TITLE).color(t.heading))
                .child(top_card),
        );
    }

    // Songs List
    if !results.songs.is_empty() {
        let songs_header = text("Songs").font(typo::TITLE).color(t.heading);
        let mut songs_col = col().w_full().gap(6.0);

        let all_songs = results.songs.clone();
        for (idx, song) in results.songs.iter().take(12).enumerate() {
            let song_title = text(&song.title).font(typo::LABEL).color(t.heading);
            let song_artist = text(&song.artist).font(typo::CAPTION).color(t.muted);
            let song_dur = text(&song.duration_text).font(typo::MONO_TIMER).color(t.muted);

            let song_thumb = if let Some(img) = s.get_image(&song.artwork_url) {
                el()
                    .size(40.0, 40.0)
                    .radius(8.0)
                    .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
            } else {
                el()
                    .size(40.0, 40.0)
                    .radius(8.0)
                    .fill(t.band)
                    .center()
                    .child(icon(&icons::MUSIC_NOTE, 20.0).color(t.muted))
            };

            let track_clone = song.clone();
            let all_songs_clone = all_songs.clone();
            let song_row = row()
                .key(format!("search-song-{}", idx))
                .w_full()
                .h(52.0)
                .radius(10.0)
                .px(14.0)
                .center_y()
                .justify_between()
                .fill(t.hover)
                .child(
                    row()
                        .center_y()
                        .gap(14.0)
                        .child(song_thumb)
                        .child(
                            col()
                                .gap(2.0)
                                .child(song_title)
                                .child(song_artist),
                        ),
                )
                .child(song_dur)
                .on_press(move |s: &mut AppState| {
                    s.queue = all_songs_clone.clone();
                    s.queue_index = idx;
                    s.play_track(track_clone.clone());
                });

            songs_col = songs_col.child(song_row);
        }

        content = content.child(
            col()
                .gap(14.0)
                .child(songs_header)
                .child(songs_col),
        );
    }

    // Albums Shelf
    if !results.albums.is_empty() {
        let albums_header = text("Albums").font(typo::TITLE).color(t.heading);
        let mut albums_row = row().w_full().gap(16.0);

        for (idx, album) in results.albums.iter().take(6).enumerate() {
            let thumb = if let Some(img) = s.get_image(&album.artwork_url) {
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

            let browse_id = album.browse_id.clone().unwrap_or_default();
            let card = col()
                .key(format!("search-album-{}", idx))
                .w(144.0)
                .padding(8.0)
                .radius(12.0)
                .fill(t.panel)
                .border(1.0, t.border)
                .gap(6.0)
                .child(thumb)
                .child(text(&album.title).font(typo::LABEL).color(t.heading))
                .child(text(&album.subtitle).font(typo::CAPTION).color(t.muted))
                .on_press(move |s: &mut AppState| {
                    if !browse_id.is_empty() {
                        s.load_playlist(&browse_id);
                    }
                });

            albums_row = albums_row.child(card);
        }

        content = content.child(
            col()
                .gap(14.0)
                .child(albums_header)
                .child(albums_row),
        );
    }

    // Featured Playlists Shelf
    if !results.playlists.is_empty() {
        let pl_header = text("Featured Playlists").font(typo::TITLE).color(t.heading);
        let mut pl_row = row().w_full().gap(16.0);

        for (idx, pl) in results.playlists.iter().take(6).enumerate() {
            let thumb = if let Some(img) = s.get_image(&pl.artwork_url) {
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

            let browse_id = pl.browse_id.clone().unwrap_or_default();
            let card = col()
                .key(format!("search-pl-{}", idx))
                .w(144.0)
                .padding(8.0)
                .radius(12.0)
                .fill(t.panel)
                .border(1.0, t.border)
                .gap(6.0)
                .child(thumb)
                .child(text(&pl.title).font(typo::LABEL).color(t.heading))
                .child(text(&pl.subtitle).font(typo::CAPTION).color(t.muted))
                .on_press(move |s: &mut AppState| {
                    if !browse_id.is_empty() {
                        s.load_playlist(&browse_id);
                    }
                });

            pl_row = pl_row.child(card);
        }

        content = content.child(
            col()
                .gap(14.0)
                .child(pl_header)
                .child(pl_row),
        );
    }

    // Artists Shelf
    if !results.artists.is_empty() {
        let artists_header = text("Artists").font(typo::TITLE).color(t.heading);
        let mut artists_row = row().w_full().gap(16.0);

        for (idx, artist) in results.artists.iter().take(6).enumerate() {
            let thumb = if let Some(img) = s.get_image(&artist.artwork_url) {
                el()
                    .size(100.0, 100.0)
                    .radius(50.0)
                    .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
            } else {
                el()
                    .size(100.0, 100.0)
                    .radius(50.0)
                    .fill(t.band)
                    .center()
                    .child(icon(&icons::MUSIC_NOTE, 32.0).color(t.brand))
            };

            let browse_id = artist.browse_id.clone().unwrap_or_default();
            let artist_name = artist.title.clone();
            let card = col()
                .key(format!("search-artist-{}", idx))
                .w(120.0)
                .padding(8.0)
                .radius(12.0)
                .center()
                .gap(8.0)
                .child(thumb)
                .child(text(&artist.title).font(typo::LABEL).color(t.heading))
                .child(text("Artist").font(typo::CAPTION).color(t.muted))
                .on_press(move |s: &mut AppState| {
                    if !browse_id.is_empty() {
                        s.load_artist(&browse_id);
                    } else {
                        s.trigger_search(artist_name.clone());
                    }
                });

            artists_row = artists_row.child(card);
        }

        content = content.child(
            col()
                .gap(14.0)
                .child(artists_header)
                .child(artists_row),
        );
    }

    content
}
