use crate::icons;
use crate::state::{ActiveView, AppState};
use crate::theme::typo;
use rune::{col, el, icon, row, text, El, Icon};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;
    let is_collapsed = s.sidebar_collapsed;

    // Helper for navigation item (44px pill with Hugeicon)
    let nav_item = |label: &'static str, ic: &'static Icon, view: ActiveView, current: ActiveView| {
        let is_active = current == view;
        let bg = if is_active {
            t.band
        } else {
            rune::Color::TRANSPARENT
        };
        let fg = if is_active {
            t.brand_text
        } else {
            t.body
        };

        if is_collapsed {
            // Icon rail button (44x44 circular centered pill)
            el()
                .key(format!("nav-item-collapsed-{}", label))
                .size(44.0, 44.0)
                .radius(22.0)
                .center()
                .fill(bg)
                .border(1.0, if is_active { t.border } else { rune::Color::TRANSPARENT })
                .child(icon(ic, 22.0).color(if is_active { t.brand_text } else { t.muted }))
                .on_press(move |s: &mut AppState| {
                    s.navigate_to(view);
                })
        } else {
            // Expanded 44px pill with label
            row()
                .key(format!("nav-item-{}", label))
                .w_full()
                .h(44.0)
                .radius(22.0)
                .px(16.0)
                .center_y()
                .gap(16.0)
                .fill(bg)
                .border(1.0, if is_active { t.border } else { rune::Color::TRANSPARENT })
                .child(icon(ic, 22.0).color(if is_active { t.brand_text } else { t.muted }))
                .child(
                    text(label)
                        .font(if is_active { typo::LABEL } else { typo::BODY })
                        .color(fg)
                        .truncate()
                        .grow(1.0)
                        .min_w(0.0),
                )
                .on_press(move |s: &mut AppState| {
                    s.navigate_to(view);
                })
        }
    };

    let main_nav = if is_collapsed {
        col()
            .w_full()
            .items_center()
            .gap(6.0)
            .child(nav_item("Home", &icons::HOME, ActiveView::Home, s.active_view))
            .child(nav_item("Explore", &icons::EXPLORE, ActiveView::Explore, s.active_view))
            .child(nav_item("Library", &icons::LIBRARY, ActiveView::Library, s.active_view))
    } else {
        col()
            .w_full()
            .gap(4.0)
            .child(nav_item("Home", &icons::HOME, ActiveView::Home, s.active_view))
            .child(nav_item("Explore", &icons::EXPLORE, ActiveView::Explore, s.active_view))
            .child(nav_item("Library", &icons::LIBRARY, ActiveView::Library, s.active_view))
    };

    let divider = el()
        .w_full()
        .h(1.0)
        .fill(t.border);

    // Playlists section: New Playlist Button
    let new_pl_btn = if is_collapsed {
        el()
            .key("btn-new-pl-collapsed")
            .size(44.0, 44.0)
            .radius(22.0)
            .fill(t.hover)
            .center()
            .child(icon(&icons::NEW_PLAYLIST, 18.0).color(t.heading))
            .on_press(|s: &mut AppState| {
                let id = s.create_custom_playlist(String::new());
                s.open_custom_playlist(&id);
            })
    } else {
        row()
            .key("btn-new-pl")
            .w_full()
            .h(38.0)
            .radius(10.0)
            .px(12.0)
            .center_y()
            .gap(10.0)
            .fill(t.hover)
            .child(icon(&icons::NEW_PLAYLIST, 18.0).color(t.heading))
            .child(
                text("New playlist")
                    .font(typo::LABEL)
                    .color(t.heading)
                    .truncate()
                    .grow(1.0)
                    .min_w(0.0),
            )
            .on_press(|s: &mut AppState| {
                let id = s.create_custom_playlist(String::new());
                s.open_custom_playlist(&id);
            })
    };

    // Pinned Liked Music
    let is_liked_active = s.active_view == ActiveView::PlaylistDetail
        && s.current_playlist.as_ref().map(|p| p.browse_id == "LM").unwrap_or(false);

    let liked_thumb = el()
        .size(34.0, 34.0)
        .radius(8.0)
        .fill(t.brand)
        .center()
        .child(icon(&icons::HEART_FILLED, 18.0).color(rune::Color::WHITE));

    let pinned_liked = if is_collapsed {
        el()
            .key("side-pinned-liked-collapsed")
            .size(44.0, 44.0)
            .radius(22.0)
            .fill(if is_liked_active { t.band } else { rune::Color::TRANSPARENT })
            .center()
            .child(liked_thumb)
            .on_press(|s: &mut AppState| {
                s.open_liked_music();
            })
    } else {
        let liked_info = col()
            .grow(1.0)
            .min_w(0.0)
            .gap(2.0)
            .child(
                text("Liked Music")
                    .font(typo::LABEL)
                    .color(t.heading)
                    .truncate()
                    .w_full(),
            )
            .child(
                text("Auto playlist")
                    .font(typo::CAPTION)
                    .color(t.muted)
                    .truncate()
                    .w_full(),
            );

        row()
            .key("side-pinned-liked")
            .w_full()
            .h(48.0)
            .radius(10.0)
            .px(10.0)
            .center_y()
            .gap(12.0)
            .fill(if is_liked_active { t.band } else { rune::Color::TRANSPARENT })
            .child(liked_thumb)
            .child(liked_info)
            .on_press(|s: &mut AppState| {
                s.open_liked_music();
            })
    };

    let sidebar_width = if is_collapsed { 72.0 } else { 232.0 };
    let sidebar_padding = if is_collapsed { 10.0 } else { 14.0 };

    let mut sidebar_col = col()
        .key("sidebar")
        .w(sidebar_width)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .fill(t.surface)
        .border(1.0, t.border)
        .padding(sidebar_padding)
        .gap(12.0)
        .child(main_nav)
        .child(divider)
        .child(new_pl_btn)
        .child(pinned_liked);

    // Render custom user playlists
    if !s.custom_playlists.is_empty() {
        let mut custom_pl_col = col().w_full().gap(4.0);
        if is_collapsed {
            custom_pl_col = custom_pl_col.items_center();
        }

        for pl in &s.custom_playlists {
            let pl_id = pl.id.clone();
            let is_active = s.current_playlist.as_ref().map(|p| p.browse_id == pl_id).unwrap_or(false)
                && s.active_view == ActiveView::PlaylistDetail;

            let pl_icon = if pl.tracks.len() >= 4 {
                let mut imgs = Vec::new();
                for trk in pl.tracks.iter().take(4) {
                    if let Some(img) = s.get_image(&trk.artwork_url) {
                        imgs.push(img);
                    }
                }
                if imgs.len() == 4 {
                    col()
                        .size(34.0, 34.0)
                        .radius(8.0)
                        .clip_rounded(8.0)
                        .child(
                            row()
                                .size(34.0, 17.0)
                                .child(el().size(17.0, 17.0).pixels_fit(imgs[0].pixels(), rune::SurfaceFit::Fill))
                                .child(el().size(17.0, 17.0).pixels_fit(imgs[1].pixels(), rune::SurfaceFit::Fill))
                        )
                        .child(
                            row()
                                .size(34.0, 17.0)
                                .child(el().size(17.0, 17.0).pixels_fit(imgs[2].pixels(), rune::SurfaceFit::Fill))
                                .child(el().size(17.0, 17.0).pixels_fit(imgs[3].pixels(), rune::SurfaceFit::Fill))
                        )
                } else {
                    let cover_url = pl
                        .cover_url
                        .as_deref()
                        .or_else(|| pl.tracks.first().map(|t| t.artwork_url.as_str()));
                    if let Some(url) = cover_url && let Some(img) = s.get_image(url) {
                        el()
                            .size(34.0, 34.0)
                            .radius(8.0)
                            .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
                    } else {
                        el()
                            .size(34.0, 34.0)
                            .radius(8.0)
                            .fill(if is_active { t.brand } else { t.band })
                            .center()
                            .child(icon(&icons::QUEUE, 18.0).color(if is_active { t.brand_text } else { t.muted }))
                    }
                }
            } else {
                let cover_url = pl
                    .cover_url
                    .as_deref()
                    .or_else(|| pl.tracks.first().map(|t| t.artwork_url.as_str()));
                if let Some(url) = cover_url && let Some(img) = s.get_image(url) {
                    el()
                        .size(34.0, 34.0)
                        .radius(8.0)
                        .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
                } else {
                    el()
                        .size(34.0, 34.0)
                        .radius(8.0)
                        .fill(if is_active { t.brand } else { t.band })
                        .center()
                        .child(icon(&icons::QUEUE, 18.0).color(if is_active { t.brand_text } else { t.muted }))
                }
            };

            let pl_item = if is_collapsed {
                let pl_id_press = pl_id.clone();
                el()
                    .key(format!("side-custom-pl-col-{}", pl.id))
                    .size(44.0, 44.0)
                    .radius(10.0)
                    .fill(if is_active { t.hover } else { rune::Color::TRANSPARENT })
                    .center()
                    .child(pl_icon)
                    .on_press(move |s: &mut AppState| {
                        s.open_custom_playlist(&pl_id_press);
                    })
            } else {
                let pl_meta = col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(2.0)
                    .child(
                        text(&pl.title)
                            .font(typo::LABEL)
                            .color(if is_active { t.brand } else { t.heading })
                            .truncate()
                            .w_full(),
                    )
                    .child(
                        text(format!("{} songs", pl.tracks.len()))
                            .font(typo::CAPTION)
                            .color(t.muted)
                            .truncate()
                            .w_full(),
                    );

                let pl_id_press = pl_id.clone();
                row()
                    .key(format!("side-custom-pl-{}", pl.id))
                    .w_full()
                    .h(48.0)
                    .radius(10.0)
                    .px(10.0)
                    .center_y()
                    .gap(12.0)
                    .fill(if is_active { t.hover } else { rune::Color::TRANSPARENT })
                    .child(pl_icon)
                    .child(pl_meta)
                    .on_press(move |s: &mut AppState| {
                        s.open_custom_playlist(&pl_id_press);
                    })
            };

            custom_pl_col = custom_pl_col.child(pl_item);
        }
        sidebar_col = sidebar_col.child(custom_pl_col);
    }

    // Render loaded playlists
    if !s.sidebar_playlists.is_empty() {
        let mut pl_col = col().w_full().gap(4.0);
        if is_collapsed {
            pl_col = pl_col.items_center();
        }

        for pl in s.sidebar_playlists.iter().take(8) {
            let pl_thumb = el()
                .size(34.0, 34.0)
                .radius(8.0)
                .fill(t.band)
                .center()
                .child(icon(&icons::QUEUE, 16.0).color(t.muted));

            let pl_id = pl.browse_id.clone();
            let is_selected = s
                .current_playlist
                .as_ref()
                .map(|p| p.browse_id == pl.browse_id)
                .unwrap_or(false);

            let pl_item = if is_collapsed {
                let pl_id_press = pl_id.clone();
                el()
                    .key(format!("sidebar-pl-col-{}", pl.browse_id))
                    .size(44.0, 44.0)
                    .radius(10.0)
                    .fill(if is_selected { t.hover } else { rune::Color::TRANSPARENT })
                    .center()
                    .child(pl_thumb)
                    .on_press(move |s: &mut AppState| {
                        s.load_playlist(&pl_id_press);
                    })
            } else {
                let pl_info = col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(2.0)
                    .child(
                        text(&pl.title)
                            .font(typo::LABEL)
                            .color(t.heading)
                            .truncate()
                            .w_full(),
                    )
                    .child(
                        text("Playlist")
                            .font(typo::CAPTION)
                            .color(t.muted)
                            .truncate()
                            .w_full(),
                    );

                let pl_id_press = pl_id.clone();
                row()
                    .key(format!("sidebar-pl-{}", pl.browse_id))
                    .w_full()
                    .h(48.0)
                    .radius(10.0)
                    .px(10.0)
                    .center_y()
                    .gap(12.0)
                    .fill(if is_selected { t.hover } else { rune::Color::TRANSPARENT })
                    .child(pl_thumb)
                    .child(pl_info)
                    .on_press(move |s: &mut AppState| {
                        s.load_playlist(&pl_id_press);
                    })
            };

            pl_col = pl_col.child(pl_item);
        }
        sidebar_col = sidebar_col.child(pl_col);
    }

    sidebar_col
}
