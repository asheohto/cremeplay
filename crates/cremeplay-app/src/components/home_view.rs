use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    if s.is_loading_home && s.home_sections.is_empty() {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(12.0)
            .child(icon(&icons::MUSIC_NOTE, 36.0).color(t.brand))
            .child(text("Loading recommendations…").font(typo::BODY).color(t.muted));
    }

    if s.home_sections.is_empty() {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(14.0)
            .child(icon(&icons::EXPLORE, 48.0).color(t.muted))
            .child(text("Welcome to Cremeplay").font(typo::TITLE).color(t.heading))
            .child(text("Search for your favorite songs, or explore trending music.").font(typo::BODY).color(t.muted))
            .child(
                el()
                    .key("home-empty-explore-btn")
                    .h(38.0)
                    .px(20.0)
                    .radius(19.0)
                    .fill(t.brand)
                    .center()
                    .child(text("Explore Trending").font(typo::LABEL).color(t.brand_text))
                    .on_press(|s: &mut AppState| {
                        s.navigate_to(crate::state::ActiveView::Explore);
                    }),
            );
    }

    let mut content = col()
        .key("home-content")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .gap(36.0)
        .padding(32.0);

    for (s_idx, section) in s.home_sections.iter().enumerate() {
        let section_title = text(&section.title)
            .font(typo::TITLE)
            .color(t.heading);

        let mut cards_row = row()
            .key(format!("section-row-{}", s_idx))
            .w_full()
            .gap(16.0);

        for (t_idx, item) in section.items.iter().take(6).enumerate() {
            let card_thumb = if let Some(img) = s.get_image(&item.artwork_url) {
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
                    .child(icon(&icons::MUSIC_NOTE, 32.0).color(t.muted))
            };

            let card_title = text(&item.title)
                .font(typo::LABEL)
                .color(t.heading);

            let card_artist = text(&item.artist)
                .font(typo::CAPTION)
                .color(t.body);

            let track_clone = item.clone();
            let section_items = section.items.clone();
            let card = col()
                .key(format!("card-{}-{}", s_idx, t_idx))
                .w(160.0)
                .padding(8.0)
                .radius(16.0)
                .fill(t.panel)
                .border(1.0, t.border)
                .gap(6.0)
                .child(card_thumb)
                .child(card_title)
                .child(card_artist)
                .on_press(move |s: &mut AppState| {
                    s.queue = section_items.clone();
                    s.queue_index = t_idx;
                    s.play_track(track_clone.clone());
                });

            cards_row = cards_row.child(card);
        }

        let section_col = col()
            .key(format!("section-{}", s_idx))
            .w_full()
            .gap(14.0)
            .child(section_title)
            .child(cards_row);

        content = content.child(section_col);
    }

    content
}
