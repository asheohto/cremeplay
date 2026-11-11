use crate::icons;
use crate::state::{AppState, SettingsTab};
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    let mut content = col()
        .key("library-content")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .gap(32.0)
        .padding(32.0);

    let on_brand = t.on_brand;
    let mut on_brand_subtle = t.on_brand;
    on_brand_subtle.a = 0.85;

    // 1. Header with Profile status
    let header_title = text("Your Library").font(typo::HERO).color(t.heading);
    let profile_badge = if let Some(acc) = &s.account {
        row()
            .h(38.0)
            .radius(19.0)
            .px(14.0)
            .center_y()
            .gap(10.0)
            .fill(t.panel)
            .border(1.0, t.border)
            .child(
                el()
                    .size(24.0, 24.0)
                    .radius(12.0)
                    .fill(t.brand)
                    .center()
                    .child(icon(&icons::SETTINGS, 14.0).color(on_brand)),
            )
            .child(text(&acc.name).font(typo::LABEL).color(t.heading))
    } else {
        row()
            .key("lib-signin-btn")
            .h(38.0)
            .radius(19.0)
            .px(16.0)
            .center_y()
            .gap(8.0)
            .fill(t.brand)
            .child(icon(&icons::HEART, 16.0).color(on_brand))
            .child(text("Connect Account").font(typo::LABEL).color(on_brand))
            .on_press(|s: &mut AppState| {
                s.show_settings = true;
                s.settings_tab = SettingsTab::Account;
            })
    };

    let header_row = row()
        .w_full()
        .justify_between()
        .center_y()
        .child(header_title)
        .child(profile_badge);

    content = content.child(header_row);

    // 2. Liked Music Hero Card
    let liked_hero = row()
        .key("lib-liked-hero")
        .w(480.0)
        .h(120.0)
        .radius(20.0)
        .fill(t.brand)
        .padding(18.0)
        .center_y()
        .justify_between()
        .child(
            row()
                .center_y()
                .gap(16.0)
                .child(
                    el()
                        .size(80.0, 80.0)
                        .radius(16.0)
                        .fill(rune::Color { r: on_brand.r, g: on_brand.g, b: on_brand.b, a: 0.15 })
                        .center()
                        .child(icon(&icons::HEART_FILLED, 36.0).color(on_brand)),
                )
                .child(
                    col()
                        .gap(4.0)
                        .child(text("Liked Music").font(typo::TITLE).color(on_brand))
                        .child(
                            text("Auto playlist from your library")
                                .font(typo::CAPTION)
                                .color(on_brand_subtle),
                        ),
                ),
        )
        .child(
            el()
                .size(48.0, 48.0)
                .radius(24.0)
                .fill(on_brand)
                .center()
                .child(icon(&icons::PLAY, 22.0).color(t.brand))
                .on_press(|s: &mut AppState| {
                    if !s.liked_tracks.is_empty() {
                        s.queue = s.liked_tracks.clone();
                        s.queue_index = 0;
                        let track = s.liked_tracks[0].clone();
                        s.play_track(track);
                    } else {
                        s.open_liked_music();
                    }
                }),
        )
        .on_press(|s: &mut AppState| {
            s.open_liked_music();
        });

    content = content.child(liked_hero);

    if !s.liked_tracks.is_empty() {
        let mut liked_list = col().w_full().gap(6.0);
        for (idx, trk) in s.liked_tracks.iter().enumerate() {
            let item = trk.clone();
            let track_add = trk.clone();
            let add_btn = el()
                .key(format!("lib-liked-add-{}", idx))
                .size(28.0, 28.0)
                .radius(14.0)
                .fill(rune::Color::TRANSPARENT)
                .center()
                .child(icon(&icons::NEW_PLAYLIST, 16.0).color(t.muted))
                .on_press(move |s: &mut AppState| {
                    s.playlist_picker_track = Some(track_add.clone());
                    s.show_playlist_picker = true;
                });

            let right_side = row()
                .center_y()
                .gap(8.0)
                .child(text(&trk.duration_text).font(typo::CAPTION).color(t.muted))
                .child(add_btn);

            let row_item = row()
                .key(format!("lib-liked-song-{}", idx))
                .w_full()
                .h(48.0)
                .radius(10.0)
                .px(12.0)
                .center_y()
                .justify_between()
                .fill(t.hover)
                .child(
                    row()
                        .center_y()
                        .gap(12.0)
                        .child(
                            el()
                                .size(32.0, 32.0)
                                .radius(6.0)
                                .fill(t.band)
                                .center()
                                .child(icon(&icons::MUSIC_NOTE, 16.0).color(t.brand_text)),
                        )
                        .child(
                            col()
                                .gap(2.0)
                                .child(text(&trk.title).font(typo::LABEL).color(t.heading))
                                .child(text(&trk.artist).font(typo::CAPTION).color(t.muted)),
                        ),
                )
                .child(right_side)
                .on_press(move |s: &mut AppState| {
                    s.play_track(item.clone());
                });
            liked_list = liked_list.child(row_item);
        }
        content = content.child(
            col()
                .w_full()
                .gap(10.0)
                .child(text("Favorite Tracks").font(typo::TITLE).color(t.heading))
                .child(liked_list),
        );
    }

    // 3. User Playlists Section
    let pl_heading = text("Playlists & Albums").font(typo::TITLE).color(t.heading);
    content = content.child(pl_heading);

    let mut pl_grid = row()
        .key("lib-playlists-grid")
        .w_full()
        .scroll_x()
        .gap(16.0);

    // "Create Playlist" card
    let create_card = col()
        .key("lib-create-pl-card")
        .w(160.0)
        .padding(8.0)
        .radius(16.0)
        .fill(t.hover)
        .border(1.0, t.border)
        .gap(6.0)
        .child(
            el()
                .size(144.0, 144.0)
                .radius(12.0)
                .fill(t.band)
                .center()
                .child(icon(&icons::NEW_PLAYLIST, 36.0).color(t.brand)),
        )
        .child(text("New Playlist").font(typo::LABEL).color(t.heading))
        .child(text("Create custom").font(typo::CAPTION).color(t.muted))
        .on_press(|s: &mut AppState| {
            let id = s.create_custom_playlist(String::new());
            s.open_custom_playlist(&id);
        });
    pl_grid = pl_grid.child(create_card);

    for (idx, pl) in s.custom_playlists.iter().enumerate() {
        let pl_thumb = if pl.tracks.len() >= 4 {
            let mut imgs = Vec::new();
            for trk in pl.tracks.iter().take(4) {
                if let Some(img) = s.get_image(&trk.artwork_url) {
                    imgs.push(img);
                }
            }
            if imgs.len() == 4 {
                col()
                    .size(144.0, 144.0)
                    .radius(12.0)
                    .clip_rounded(12.0)
                    .child(
                        row()
                            .size(144.0, 72.0)
                            .child(el().size(72.0, 72.0).pixels_fit(imgs[0].pixels(), rune::SurfaceFit::Fill))
                            .child(el().size(72.0, 72.0).pixels_fit(imgs[1].pixels(), rune::SurfaceFit::Fill))
                    )
                    .child(
                        row()
                            .size(144.0, 72.0)
                            .child(el().size(72.0, 72.0).pixels_fit(imgs[2].pixels(), rune::SurfaceFit::Fill))
                            .child(el().size(72.0, 72.0).pixels_fit(imgs[3].pixels(), rune::SurfaceFit::Fill))
                    )
            } else {
                let cover_url = pl
                    .cover_url
                    .as_deref()
                    .or_else(|| pl.tracks.first().map(|t| t.artwork_url.as_str()));
                if let Some(url) = cover_url && let Some(img) = s.get_image(url) {
                    el()
                        .size(144.0, 144.0)
                        .radius(12.0)
                        .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
                } else {
                    el()
                        .size(144.0, 144.0)
                        .radius(12.0)
                        .fill(t.band)
                        .center()
                        .child(icon(&icons::QUEUE, 36.0).color(t.brand))
                }
            }
        } else {
            let cover_url = pl
                .cover_url
                .as_deref()
                .or_else(|| pl.tracks.first().map(|t| t.artwork_url.as_str()));
            if let Some(url) = cover_url && let Some(img) = s.get_image(url) {
                el()
                    .size(144.0, 144.0)
                    .radius(12.0)
                    .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
            } else {
                el()
                    .size(144.0, 144.0)
                    .radius(12.0)
                    .fill(t.band)
                    .center()
                    .child(icon(&icons::QUEUE, 36.0).color(t.brand))
            }
        };

        let pl_id = pl.id.clone();
        let card = col()
            .key(format!("lib-custom-pl-card-{}", idx))
            .w(160.0)
            .min_w(0.0)
            .padding(8.0)
            .radius(16.0)
            .fill(t.panel)
            .border(1.0, t.border)
            .gap(6.0)
            .child(pl_thumb)
            .child(text(&pl.title).font(typo::LABEL).color(t.heading).max_lines(2).w_full())
            .child(text(format!("{} songs", pl.tracks.len())).font(typo::CAPTION).color(t.muted).truncate().w_full())
            .on_press(move |s: &mut AppState| {
                s.open_custom_playlist(&pl_id);
            });

        pl_grid = pl_grid.child(card);
    }

    for (idx, pl) in s.sidebar_playlists.iter().enumerate() {
        let pl_thumb = el()
            .size(144.0, 144.0)
            .radius(12.0)
            .fill(t.band)
            .center()
            .child(icon(&icons::QUEUE, 36.0).color(t.muted));

        let pl_id = pl.browse_id.clone();
        let card = col()
            .key(format!("lib-pl-card-{}", idx))
            .w(160.0)
            .min_w(0.0)
            .padding(8.0)
            .radius(16.0)
            .fill(t.panel)
            .border(1.0, t.border)
            .gap(6.0)
            .child(pl_thumb)
            .child(text(&pl.title).font(typo::LABEL).color(t.heading).max_lines(2).w_full())
            .child(text("Playlist").font(typo::CAPTION).color(t.muted).truncate().w_full())
            .on_press(move |s: &mut AppState| {
                s.load_playlist(&pl_id);
            });

        pl_grid = pl_grid.child(card);
    }

    content = content.child(pl_grid);

    content
}
