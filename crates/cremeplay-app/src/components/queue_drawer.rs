use crate::icons;
use crate::state::{AppState, QueueDrawerTab};
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    // Header with tabs
    let is_queue = s.queue_drawer_tab == QueueDrawerTab::Queue;
    let is_lyrics = s.queue_drawer_tab == QueueDrawerTab::Lyrics;

    let tab_queue = el()
        .key("tab-drawer-queue")
        .h(34.0)
        .px(14.0)
        .radius(17.0)
        .center()
        .fill(if is_queue { t.band } else { rune::Color::TRANSPARENT })
        .border(1.0, if is_queue { t.border } else { rune::Color::TRANSPARENT })
        .child(
            text("Up next")
                .font(typo::LABEL)
                .color(if is_queue { t.brand_text } else { t.muted }),
        )
        .on_press(|s: &mut AppState| {
            s.queue_drawer_tab = QueueDrawerTab::Queue;
        });

    let tab_lyrics = el()
        .key("tab-drawer-lyrics")
        .h(34.0)
        .px(14.0)
        .radius(17.0)
        .center()
        .fill(if is_lyrics { t.band } else { rune::Color::TRANSPARENT })
        .border(1.0, if is_lyrics { t.border } else { rune::Color::TRANSPARENT })
        .child(
            text("Lyrics")
                .font(typo::LABEL)
                .color(if is_lyrics { t.brand_text } else { t.muted }),
        )
        .on_press(|s: &mut AppState| {
            s.queue_drawer_tab = QueueDrawerTab::Lyrics;
        });

    let close_btn = el()
        .key("close-queue")
        .size(32.0, 32.0)
        .radius(16.0)
        .center()
        .fill(t.hover)
        .child(icon(&icons::CLOSE, 16.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_queue_drawer = false;
        });

    let drawer_header = row()
        .w_full()
        .h(56.0)
        .px(16.0)
        .justify_between()
        .center_y()
        .child(
            row()
                .gap(8.0)
                .child(tab_queue)
                .child(tab_lyrics),
        )
        .child(close_btn);

    // Body content based on tab
    let drawer_body = if is_queue {
        let mut queue_list = col().w_full().gap(6.0);

        if s.queue.is_empty() {
            if let Some(current) = &s.current_track {
                let now_playing_card = row()
                    .w_full()
                    .h(52.0)
                    .radius(10.0)
                    .px(12.0)
                    .center_y()
                    .gap(12.0)
                    .fill(t.panel)
                    .border(1.0, t.brand)
                    .child(
                        el()
                            .size(36.0, 36.0)
                            .radius(6.0)
                            .fill(t.brand)
                            .center()
                            .child(icon(&icons::PLAY, 16.0).color(t.on_brand)),
                    )
                    .child(
                        col()
                            .gap(2.0)
                            .child(text(&current.title).font(typo::LABEL).color(t.heading))
                            .child(text(&current.artist).font(typo::CAPTION).color(t.muted)),
                    );

                queue_list = queue_list.child(now_playing_card);
            } else {
                queue_list = queue_list.child(
                    col()
                        .w_full()
                        .h(200.0)
                        .center()
                        .gap(8.0)
                        .child(icon(&icons::QUEUE, 32.0).color(t.muted))
                        .child(text("Queue is empty").font(typo::BODY).color(t.muted)),
                );
            }
        } else {
            let queue_summary = row()
                .w_full()
                .justify_between()
                .center_y()
                .pb(8.0)
                .child(
                    text(format!("{} tracks in queue", s.queue.len()))
                        .font(typo::CAPTION)
                        .color(t.muted),
                )
                .child(
                    el()
                        .key("btn-clear-queue")
                        .h(24.0)
                        .px(8.0)
                        .radius(4.0)
                        .fill(t.hover)
                        .center()
                        .child(text("Clear Up Next").font(typo::CAPTION).color(t.brand_text))
                        .on_press(|s: &mut AppState| {
                            if let Some(cur) = s.current_track.clone() {
                                s.queue = vec![cur];
                                s.queue_index = 0;
                            } else {
                                s.queue.clear();
                                s.queue_index = 0;
                            }
                        }),
                );
            queue_list = queue_list.child(queue_summary);

            for (idx, item) in s.queue.iter().enumerate() {
                let is_current = idx == s.queue_index;
                let bg = if is_current { t.band } else { t.hover };
                let border_col = if is_current { t.brand } else { rune::Color::TRANSPARENT };

                let track_item = item.clone();
                let play_click = row()
                    .grow(1.0)
                    .center_y()
                    .gap(10.0)
                    .child(
                        el()
                            .size(32.0, 32.0)
                            .radius(6.0)
                            .fill(t.band)
                            .center()
                            .child(icon(if is_current { &icons::PLAY } else { &icons::MUSIC_NOTE }, 16.0).color(if is_current { t.brand } else { t.muted })),
                    )
                    .child(
                        col()
                            .gap(2.0)
                            .child(text(&item.title).font(typo::LABEL).color(if is_current { t.brand_text } else { t.heading }))
                            .child(text(&item.artist).font(typo::CAPTION).color(t.muted)),
                    )
                    .on_press(move |s: &mut AppState| {
                        s.queue_index = idx;
                        s.play_track(track_item.clone());
                    });

                let track_add = item.clone();
                let add_btn = el()
                    .key(format!("queue-add-{}", idx))
                    .size(24.0, 24.0)
                    .radius(12.0)
                    .fill(rune::Color::TRANSPARENT)
                    .center()
                    .child(icon(&icons::NEW_PLAYLIST, 14.0).color(t.muted))
                    .on_press(move |s: &mut AppState| {
                        s.playlist_picker_track = Some(track_add.clone());
                        s.show_playlist_picker = true;
                    });

                let remove_btn = el()
                    .key(format!("queue-remove-{}", idx))
                    .size(24.0, 24.0)
                    .radius(12.0)
                    .fill(t.hover)
                    .center()
                    .child(icon(&icons::CLOSE, 12.0).color(t.muted))
                    .on_press(move |s: &mut AppState| {
                        if idx < s.queue.len() {
                            s.queue.remove(idx);
                            if s.queue_index >= idx && s.queue_index > 0 {
                                s.queue_index -= 1;
                            }
                        }
                    });

                let item_actions = row()
                    .center_y()
                    .gap(4.0)
                    .child(add_btn)
                    .child(remove_btn);

                let queue_card = row()
                    .key(format!("queue-item-{}", idx))
                    .w_full()
                    .h(48.0)
                    .radius(8.0)
                    .px(10.0)
                    .center_y()
                    .justify_between()
                    .fill(bg)
                    .border(1.0, border_col)
                    .child(play_click)
                    .child(item_actions);

                queue_list = queue_list.child(queue_card);
            }
        }

        col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(rune::ScrollSpec { stick_bottom: false })
            .padding(14.0)
            .child(queue_list)
    } else {
        // Lyrics tab
        let offset_sec = s.lyrics_offset_ms as f64 / 1000.0;
        let mut lyrics_list = col().w_full().gap(14.0).padding(16.0);

        if let Some(lyrics) = &s.lyrics {
            if !lyrics.lines.is_empty() {
                let active_idx = lyrics.lines.iter().enumerate().rposition(|(idx, line)| {
                    let t_start = line.time_sec + offset_sec;
                    let t_end = lyrics.lines.get(idx + 1).map(|next| next.time_sec + offset_sec).unwrap_or(t_start + 6.0);
                    s.current_time >= t_start && s.current_time < t_end
                });

                for (idx, line) in lyrics.lines.iter().enumerate() {
                    let adjusted_time = line.time_sec + offset_sec;
                    let is_active = Some(idx) == active_idx;
                    let line_text = line.text.clone();
                    let target_sec = adjusted_time;

                    let mut line_box = col().gap(2.0).child(
                        text(&line_text)
                            .font(typo::BODY)
                            .color(if is_active { t.brand } else { t.body }),
                    );
                    if let Some(rom) = &line.romanized {
                        line_box = line_box.child(
                            text(rom)
                                .font(typo::CAPTION)
                                .color(if is_active { t.brand_text } else { t.muted }),
                        );
                    }

                    lyrics_list = lyrics_list.child(
                        el()
                            .radius(6.0)
                            .px(8.0)
                            .py(4.0)
                            .fill(if is_active { t.band } else { rune::Color::TRANSPARENT })
                            .child(line_box)
                            .on_press(move |s: &mut AppState| {
                                s.current_time = target_sec;
                                s.player.seek(target_sec);
                            }),
                    );
                }
            } else if !lyrics.plain_lyrics.is_empty() {
                lyrics_list = lyrics_list.child(text(&lyrics.plain_lyrics).font(typo::BODY).color(t.body));
            }
        } else {
            lyrics_list = lyrics_list.child(
                col()
                    .w_full()
                    .h(200.0)
                    .center()
                    .gap(8.0)
                    .child(icon(&icons::MUSIC_NOTE, 32.0).color(t.muted))
                    .child(text("No lyrics found").font(typo::BODY).color(t.muted)),
            );
        }

        col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(rune::ScrollSpec { stick_bottom: false })
            .child(lyrics_list)
    };

    col()
        .key("queue-drawer")
        .w(320.0)
        .grow(1.0)
        .min_h(0.0)
        .fill(t.panel)
        .border(1.0, t.border)
        .child(drawer_header)
        .child(drawer_body)
}
