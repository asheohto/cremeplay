use crate::icons;
use crate::state::{AppState, NowPlayingTab};
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    // Header
    let collapse_btn = el()
        .key("btn-collapse-player")
        .size(38.0, 38.0)
        .radius(19.0)
        .center()
        .fill(t.hover)
        .border(1.0, t.border)
        .child(icon(&icons::CHEVRON_DOWN, 22.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_now_playing = false;
        });

    let tab_btn = |label: &str, tab: NowPlayingTab, current: NowPlayingTab| {
        let is_active = tab == current;
        el()
            .key(format!("np-tab-{}", label))
            .h(34.0)
            .px(16.0)
            .radius(17.0)
            .center()
            .fill(if is_active { t.brand } else { t.hover })
            .border(1.0, if is_active { t.brand } else { t.border })
            .child(
                text(label)
                    .font(typo::LABEL)
                    .color(if is_active { t.brand_text } else { t.heading }),
            )
            .on_press(move |s: &mut AppState| {
                s.now_playing_tab = tab;
            })
    };

    let tabs = row()
        .gap(10.0)
        .child(tab_btn("Cover", NowPlayingTab::Cover, s.now_playing_tab))
        .child(tab_btn("Up Next", NowPlayingTab::Queue, s.now_playing_tab))
        .child(tab_btn("Lyrics", NowPlayingTab::Lyrics, s.now_playing_tab));

    let header = row()
        .w_full()
        .h(68.0)
        .px(32.0)
        .justify_between()
        .center_y()
        .child(collapse_btn)
        .child(tabs)
        .child(el().size(38.0, 38.0));

    // Body based on tab
    let body = match s.now_playing_tab {
        NowPlayingTab::Cover => {
            let cover_artwork = if let Some(track) = &s.current_track {
                if let Some(img) = s.get_image(&track.artwork_url) {
                    el()
                        .size(320.0, 320.0)
                        .radius(20.0)
                        .border(1.0, t.border)
                        .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
                } else {
                    el()
                        .size(320.0, 320.0)
                        .radius(20.0)
                        .border(1.0, t.border)
                        .fill(t.band)
                        .center()
                        .child(icon(&icons::MUSIC_NOTE, 64.0).color(t.muted))
                }
            } else {
                el()
                    .size(320.0, 320.0)
                    .radius(20.0)
                    .border(1.0, t.border)
                    .fill(t.band)
                    .center()
                    .child(icon(&icons::MUSIC_NOTE, 64.0).color(t.muted))
            };

            let title_str = s.current_track.as_ref().map(|t| t.title.as_str()).unwrap_or("Not playing");
            let artist_str = s.current_track.as_ref().map(|t| t.artist.as_str()).unwrap_or("Pick a song to start");
            let is_liked = s.current_track.as_ref().map(|t| s.is_track_liked(&t.video_id)).unwrap_or(false);

            let np_like_btn = row()
                .key("np-like-btn")
                .h(38.0)
                .radius(19.0)
                .px(16.0)
                .center_y()
                .gap(8.0)
                .fill(if is_liked { t.band } else { t.hover })
                .border(1.0, if is_liked { t.brand } else { rune::Color::TRANSPARENT })
                .child(
                    icon(if is_liked { &icons::HEART_FILLED } else { &icons::HEART }, 18.0)
                        .color(if is_liked { t.brand } else { t.muted }),
                )
                .child(
                    text(if is_liked { "Favorited" } else { "Favorite" })
                        .font(typo::LABEL)
                        .color(if is_liked { t.brand_text } else { t.heading }),
                )
                .on_press(|s: &mut AppState| {
                    if let Some(track) = s.current_track.clone() {
                        s.toggle_like(track);
                    }
                });

            col()
                .center()
                .gap(24.0)
                .child(cover_artwork)
                .child(
                    col()
                        .center()
                        .gap(6.0)
                        .child(text(title_str).font(typo::HERO).color(t.heading))
                        .child(text(artist_str).font(typo::SUBTITLE).color(t.muted)),
                )
                .child(np_like_btn)
        }
        NowPlayingTab::Queue => {
            let mut queue_list = col().w(640.0).gap(8.0).py(20.0);

            if s.queue.is_empty() {
                if let Some(current) = &s.current_track {
                    let card = row()
                        .w_full()
                        .h(56.0)
                        .radius(10.0)
                        .px(16.0)
                        .center_y()
                        .gap(14.0)
                        .fill(t.band)
                        .border(1.0, t.brand)
                        .child(
                            el()
                                .size(40.0, 40.0)
                                .radius(8.0)
                                .fill(t.hover)
                                .center()
                                .child(icon(&icons::MUSIC_NOTE, 20.0).color(t.brand_text)),
                        )
                        .child(
                            col()
                                .gap(2.0)
                                .child(text(&current.title).font(typo::LABEL).color(t.brand_text))
                                .child(text(&current.artist).font(typo::CAPTION).color(t.muted)),
                        );
                    queue_list = queue_list.child(card);
                } else {
                    queue_list = queue_list.child(
                        col()
                            .center()
                            .gap(12.0)
                            .py(40.0)
                            .child(icon(&icons::QUEUE, 40.0).color(t.muted))
                            .child(text("Queue is empty").font(typo::TITLE).color(t.muted)),
                    );
                }
            } else {
                for (idx, item) in s.queue.iter().enumerate() {
                    let is_current = idx == s.queue_index;
                    let bg = if is_current { t.band } else { t.hover };
                    let border_col = if is_current { t.brand } else { rune::Color::TRANSPARENT };
                    let track_item = item.clone();

                    let card = row()
                        .key(format!("np-queue-item-{}", idx))
                        .w_full()
                        .h(52.0)
                        .radius(10.0)
                        .px(14.0)
                        .center_y()
                        .justify_between()
                        .fill(bg)
                        .border(1.0, border_col)
                        .child(
                            row()
                                .center_y()
                                .gap(12.0)
                                .child(
                                    el()
                                        .size(36.0, 36.0)
                                        .radius(6.0)
                                        .fill(t.band)
                                        .center()
                                        .child(icon(&icons::MUSIC_NOTE, 18.0).color(if is_current { t.brand_text } else { t.muted })),
                                )
                                .child(
                                    col()
                                        .gap(2.0)
                                        .child(text(&item.title).font(typo::LABEL).color(if is_current { t.brand_text } else { t.heading }))
                                        .child(text(&item.artist).font(typo::CAPTION).color(t.muted)),
                                ),
                        )
                        .on_press(move |s: &mut AppState| {
                            s.queue_index = idx;
                            s.play_track(track_item.clone());
                        });

                    queue_list = queue_list.child(card);
                }
            }

            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .center()
                .child(queue_list)
        }
        NowPlayingTab::Lyrics => {
            let offset_sec = s.lyrics_offset_ms as f64 / 1000.0;
            let mut lyrics_col = col().center().gap(18.0).py(40.0);
            if let Some(payload) = &s.lyrics {
                if !payload.lines.is_empty() {
                    for line in &payload.lines {
                        let line_time = line.time_sec + offset_sec;
                        let is_active = (s.current_time - line_time).abs() < 1.5;
                        let line_text = line.text.clone();
                        let target_sec = line_time;
                        lyrics_col = lyrics_col.child(
                            el()
                                .radius(8.0)
                                .px(16.0)
                                .py(6.0)
                                .child(
                                    text(&line_text)
                                        .font(if is_active { typo::HERO } else { typo::TITLE })
                                        .color(if is_active { t.brand } else { t.muted }),
                                )
                                .on_press(move |s: &mut AppState| {
                                    s.current_time = target_sec;
                                    s.player.seek(target_sec);
                                }),
                        );
                    }
                } else if !payload.plain_lyrics.is_empty() {
                    lyrics_col = lyrics_col.child(text(&payload.plain_lyrics).font(typo::BODY).color(t.body));
                }
            } else {
                lyrics_col = lyrics_col.child(text("No lyrics available").font(typo::TITLE).color(t.muted));
            }

            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .child(lyrics_col)
        }
    };

    col()
        .key("now-playing-overlay")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .fill(t.surface)
        .child(header)
        .child(
            col()
                .w_full()
                .h_full()
                .center()
                .child(body),
        )
}
