use crate::icons;
use crate::state::{ActiveView, AppState};
use crate::theme::typo;
use rune::{col, el, icon, row, text, El, Icon};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

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
                    .color(fg),
            )
            .on_press(move |s: &mut AppState| {
                s.navigate_to(view);
            })
    };

    let main_nav = col()
        .w_full()
        .gap(4.0)
        .child(nav_item("Home", &icons::HOME, ActiveView::Home, s.active_view))
        .child(nav_item("Explore", &icons::EXPLORE, ActiveView::Explore, s.active_view))
        .child(nav_item("Library", &icons::LIBRARY, ActiveView::Library, s.active_view));

    let divider = el()
        .w_full()
        .h(1.0)
        .fill(t.border);

    // Playlists section
    let new_pl_btn = row()
        .key("btn-new-pl")
        .w_full()
        .h(38.0)
        .radius(10.0)
        .px(12.0)
        .center_y()
        .gap(10.0)
        .fill(t.hover)
        .child(icon(&icons::NEW_PLAYLIST, 18.0).color(t.heading))
        .child(text("New playlist").font(typo::LABEL).color(t.heading))
        .on_press(|_: &mut AppState| {
            // New playlist handler
        });

    // Pinned Liked Music
    let liked_thumb = el()
        .size(34.0, 34.0)
        .radius(8.0)
        .fill(t.brand)
        .center()
        .child(icon(&icons::HEART_FILLED, 18.0).color(rune::Color::WHITE));

    let liked_info = col()
        .gap(2.0)
        .child(text("Liked Music").font(typo::LABEL).color(t.heading))
        .child(text("Auto playlist").font(typo::CAPTION).color(t.muted));

    let pinned_liked = row()
        .key("side-pinned-liked")
        .w_full()
        .h(48.0)
        .radius(10.0)
        .px(10.0)
        .center_y()
        .gap(12.0)
        .fill(if s.active_view == ActiveView::Library { t.hover } else { rune::Color::TRANSPARENT })
        .child(liked_thumb)
        .child(liked_info)
        .on_press(|s: &mut AppState| {
            s.navigate_to(ActiveView::Library);
        });

    let mut sidebar_col = col()
        .key("sidebar")
        .w(232.0)
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .fill(t.surface)
        .border(1.0, t.border)
        .padding(14.0)
        .gap(12.0)
        .child(main_nav)
        .child(divider)
        .child(new_pl_btn)
        .child(pinned_liked);

    // Render loaded playlists
    if !s.sidebar_playlists.is_empty() {
        let mut pl_col = col().w_full().gap(4.0);
        for pl in s.sidebar_playlists.iter().take(8) {
            let pl_thumb = el()
                .size(34.0, 34.0)
                .radius(8.0)
                .fill(t.band)
                .center()
                .child(icon(&icons::QUEUE, 16.0).color(t.muted));

            let pl_info = col()
                .gap(2.0)
                .child(text(&pl.title).font(typo::LABEL).color(t.heading))
                .child(text("Playlist").font(typo::CAPTION).color(t.muted));

            let pl_row = row()
                .key(format!("sidebar-pl-{}", pl.browse_id))
                .w_full()
                .h(48.0)
                .radius(10.0)
                .px(10.0)
                .center_y()
                .gap(12.0)
                .fill(rune::Color::TRANSPARENT)
                .child(pl_thumb)
                .child(pl_info);

            pl_col = pl_col.child(pl_row);
        }
        sidebar_col = sidebar_col.child(pl_col);
    }

    sidebar_col
}
