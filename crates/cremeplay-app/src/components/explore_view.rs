use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    if s.is_loading_explore && s.explore_sections.is_empty() {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(12.0)
            .child(icon(&icons::EXPLORE, 36.0).color(t.brand))
            .child(text("Loading new releases & charts…").font(typo::BODY).color(t.muted));
    }

    let mut content = col()
        .key("explore-content")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .gap(32.0)
        .padding(32.0);

    // 1. Explore Header & Mood Chips
    let header_title = text("Explore").font(typo::HERO).color(t.heading);
    let mood_chips = [
        "New Releases",
        "Top Charts",
        "Moods & Genres",
        "Trending Hits",
        "Chill & Lofi",
        "Workout Energy",
        "Anime & J-Pop",
    ];

    let mut chip_row = row().gap(8.0);
    for chip in mood_chips {
        let chip_str = chip.to_string();
        chip_row = chip_row.child(
            row()
                .h(36.0)
                .px(16.0)
                .radius(18.0)
                .center()
                .fill(t.panel)
                .border(1.0, t.border)
                .child(text(chip).font(typo::LABEL).color(t.heading))
                .on_press(move |s: &mut AppState| {
                    s.trigger_search(chip_str.clone());
                }),
        );
    }

    let header_section = col()
        .w_full()
        .gap(16.0)
        .child(header_title)
        .child(chip_row);

    content = content.child(header_section);

    // 2. Sections from Explore feed
    for (s_idx, section) in s.explore_sections.iter().enumerate() {
        let section_title = text(&section.title)
            .font(typo::TITLE)
            .color(t.heading);

        let mut cards_row = row()
            .key(format!("explore-row-{}", s_idx))
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
                .key(format!("explore-card-{}-{}", s_idx, t_idx))
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
            .key(format!("explore-section-{}", s_idx))
            .w_full()
            .gap(14.0)
            .child(section_title)
            .child(cards_row);

        content = content.child(section_col);
    }

    content
}
