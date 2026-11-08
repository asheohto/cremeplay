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
        }
        NowPlayingTab::Queue => {
            col()
                .center()
                .child(text("Upcoming Tracks").font(typo::TITLE).color(t.heading))
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
