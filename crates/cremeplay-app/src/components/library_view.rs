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

    let white = rune::Color::WHITE;
    let white_subtle = rune::Color { r: 1.0, g: 1.0, b: 1.0, a: 0.85 };

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
                    .child(icon(&icons::SETTINGS, 14.0).color(white)),
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
            .child(icon(&icons::HEART, 16.0).color(white))
            .child(text("Connect Account").font(typo::LABEL).color(white))
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
                        .fill(rune::Color { r: 1.0, g: 1.0, b: 1.0, a: 0.15 })
                        .center()
                        .child(icon(&icons::HEART_FILLED, 36.0).color(white)),
                )
                .child(
                    col()
                        .gap(4.0)
                        .child(text("Liked Music").font(typo::TITLE).color(white))
                        .child(
                            text("Auto playlist from your library")
                                .font(typo::CAPTION)
                                .color(white_subtle),
                        ),
                ),
        )
        .child(
            el()
                .size(48.0, 48.0)
                .radius(24.0)
                .fill(rune::Color::WHITE)
                .center()
                .child(icon(&icons::PLAY, 22.0).color(t.brand)),
        )
        .on_press(|s: &mut AppState| {
            s.load_playlist("LM");
        });

    content = content.child(liked_hero);

    // 3. User Playlists Section
    let pl_heading = text("Playlists & Albums").font(typo::TITLE).color(t.heading);
    content = content.child(pl_heading);

    if s.sidebar_playlists.is_empty() {
        let empty_box = col()
            .w_full()
            .h(220.0)
            .center()
            .gap(12.0)
            .child(icon(&icons::QUEUE, 44.0).color(t.muted))
            .child(text("No playlists saved yet").font(typo::TITLE).color(t.heading))
            .child(
                text("Connect your YouTube Music account in Settings to sync your personal playlists")
                    .font(typo::BODY)
                    .color(t.muted),
            );

        content = content.child(empty_box);
    } else {
        let mut pl_grid = row()
            .key("lib-playlists-grid")
            .w_full()
            .gap(16.0);

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
                .padding(8.0)
                .radius(16.0)
                .fill(t.panel)
                .border(1.0, t.border)
                .gap(6.0)
                .child(pl_thumb)
                .child(text(&pl.title).font(typo::LABEL).color(t.heading))
                .child(text("Playlist").font(typo::CAPTION).color(t.muted))
                .on_press(move |s: &mut AppState| {
                    s.load_playlist(&pl_id);
                });

            pl_grid = pl_grid.child(card);
        }

        content = content.child(pl_grid);
    }

    content
}
