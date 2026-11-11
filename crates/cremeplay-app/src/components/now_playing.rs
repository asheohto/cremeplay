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
                    .color(if is_active { t.on_brand } else { t.heading }),
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

            let np_visualizer = if s.is_playing {
                let t_sec = s.current_time;
                let b1 = 6.0 + 14.0 * ((t_sec * 6.0).sin().abs() as f32);
                let b2 = 8.0 + 18.0 * ((t_sec * 9.0 + 0.9).sin().abs() as f32);
                let b3 = 10.0 + 22.0 * ((t_sec * 12.0 + 1.8).sin().abs() as f32);
                let b4 = 12.0 + 24.0 * ((t_sec * 15.0 + 2.7).sin().abs() as f32);
                let b5 = 10.0 + 22.0 * ((t_sec * 11.0 + 3.6).sin().abs() as f32);
                let b6 = 8.0 + 18.0 * ((t_sec * 8.0 + 4.5).sin().abs() as f32);
                let b7 = 6.0 + 14.0 * ((t_sec * 5.0 + 5.4).sin().abs() as f32);
                row()
                    .gap(4.0)
                    .center_y()
                    .h(38.0)
                    .px(12.0)
                    .radius(19.0)
                    .fill(t.hover)
                    .child(el().w(3.0).h(b1).radius(1.5).fill(t.brand))
                    .child(el().w(3.0).h(b2).radius(1.5).fill(t.brand))
                    .child(el().w(3.0).h(b3).radius(1.5).fill(t.brand))
                    .child(el().w(3.0).h(b4).radius(1.5).fill(t.brand))
                    .child(el().w(3.0).h(b5).radius(1.5).fill(t.brand))
                    .child(el().w(3.0).h(b6).radius(1.5).fill(t.brand))
                    .child(el().w(3.0).h(b7).radius(1.5).fill(t.brand))
            } else {
                row()
                    .gap(4.0)
                    .center_y()
                    .h(38.0)
                    .px(12.0)
                    .radius(19.0)
                    .fill(t.hover)
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
                    .child(el().w(3.0).h(4.0).radius(1.5).fill(t.muted))
            };

            let np_add_btn = row()
                .key("np-add-btn")
                .h(38.0)
                .radius(19.0)
                .px(16.0)
                .center_y()
                .gap(8.0)
                .fill(t.hover)
                .border(1.0, t.border)
                .child(icon(&icons::NEW_PLAYLIST, 18.0).color(t.heading))
                .child(text("Add to Playlist").font(typo::LABEL).color(t.heading))
                .on_press(|s: &mut AppState| {
                    if let Some(track) = s.current_track.clone() {
                        s.playlist_picker_track = Some(track);
                        s.show_playlist_picker = true;
                    }
                });

            let actions_row = row()
                .gap(12.0)
                .center_y()
                .child(np_like_btn)
                .child(np_add_btn)
                .child(np_visualizer);

            let speeds: [f32; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];
            let mut speed_row = row().gap(8.0).center_y();
            for &spd in &speeds {
                let is_cur = (s.playback_speed - spd).abs() < 0.01;
                let label = if (spd - 1.0).abs() < 0.01 { "Normal (1x)".to_string() } else { format!("{}x", spd) };
                let pill = el()
                    .key(format!("spd-{}", spd))
                    .h(28.0)
                    .px(12.0)
                    .radius(14.0)
                    .center()
                    .fill(if is_cur { t.brand } else { t.hover })
                    .border(1.0, if is_cur { t.brand } else { t.border })
                    .child(
                        text(label)
                            .font(typo::CAPTION)
                            .color(if is_cur { t.on_brand } else { t.heading }),
                    )
                    .on_press(move |s: &mut AppState| {
                        s.set_playback_speed(spd);
                    });
                speed_row = speed_row.child(pill);
            }

            col()
                .center()
                .gap(20.0)
                .child(cover_artwork)
                .child(
                    col()
                        .center()
                        .gap(6.0)
                        .child(text(title_str).font(typo::HERO).color(t.heading))
                        .child(text(artist_str).font(typo::SUBTITLE).color(t.muted)),
                )
                .child(actions_row)
                .child(speed_row)
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

                    let track_add = item.clone();
                    let add_btn = el()
                        .key(format!("np-queue-add-{}", idx))
                        .size(28.0, 28.0)
                        .radius(14.0)
                        .fill(rune::Color::TRANSPARENT)
                        .center()
                        .child(icon(&icons::NEW_PLAYLIST, 16.0).color(t.muted))
                        .on_press(move |s: &mut AppState| {
                            s.playlist_picker_track = Some(track_add.clone());
                            s.show_playlist_picker = true;
                        });

                    let remove_btn = el()
                        .key(format!("np-queue-rem-{}", idx))
                        .size(28.0, 28.0)
                        .radius(14.0)
                        .fill(t.hover)
                        .center()
                        .child(icon(&icons::CLOSE, 14.0).color(t.muted))
                        .on_press(move |s: &mut AppState| {
                            if idx < s.queue.len() {
                                s.queue.remove(idx);
                                if s.queue_index >= idx && s.queue_index > 0 {
                                    s.queue_index -= 1;
                                }
                            }
                        });

                    let np_actions = row()
                        .center_y()
                        .gap(6.0)
                        .child(add_btn)
                        .child(remove_btn);

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
                                .grow(1.0)
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
                                )
                                .on_press(move |s: &mut AppState| {
                                    s.queue_index = idx;
                                    s.play_track(track_item.clone());
                                }),
                        )
                        .child(np_actions);

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
            let active_source = s.lyrics.as_ref().map(|l| l.source.as_str()).unwrap_or("None");

            // Source Switcher Pills
            let mut source_pills = row().gap(6.0).center_y();
            let mut sources: Vec<String> = s.available_lyrics.keys().cloned().collect();
            sources.sort();
            if sources.is_empty() && s.lyrics.is_some() {
                sources.push(active_source.to_string());
            }

            for src in &sources {
                let is_current = src == active_source;
                let src_clone = src.clone();
                let pill = el()
                    .key(format!("np-lyric-src-{}", src))
                    .h(26.0)
                    .px(10.0)
                    .radius(13.0)
                    .center()
                    .fill(if is_current { t.brand } else { t.hover })
                    .border(1.0, if is_current { t.brand } else { t.border })
                    .child(
                        text(src)
                            .font(typo::CAPTION)
                            .color(if is_current { t.on_brand } else { t.heading }),
                    )
                    .on_press(move |s: &mut AppState| {
                        s.select_lyrics_source(&src_clone);
                    });
                source_pills = source_pills.child(pill);
            }

            let custom_lrc_btn = el()
                .key("btn-toggle-custom-lrc")
                .h(26.0)
                .px(10.0)
                .radius(13.0)
                .center()
                .fill(if s.show_custom_lyrics_modal { t.brand } else { t.hover })
                .border(1.0, t.border)
                .child(
                    text("+ Custom LRC")
                        .font(typo::CAPTION)
                        .color(if s.show_custom_lyrics_modal { t.on_brand } else { t.muted }),
                )
                .on_press(|s: &mut AppState| {
                    s.show_custom_lyrics_modal = !s.show_custom_lyrics_modal;
                });
            source_pills = source_pills.child(custom_lrc_btn);

            // Offset controls
            let offset_label = format!("{:+0.1}s", offset_sec);
            let offset_ctrls = row()
                .gap(4.0)
                .center_y()
                .child(
                    el()
                        .key("np-offset-minus")
                        .h(24.0)
                        .px(8.0)
                        .radius(12.0)
                        .fill(t.hover)
                        .border(1.0, t.border)
                        .center()
                        .child(text("-0.2s").font(typo::CAPTION).color(t.heading))
                        .on_press(|s: &mut AppState| {
                            s.adjust_lyrics_offset(-200);
                        }),
                )
                .child(
                    el()
                        .key("np-offset-display")
                        .h(24.0)
                        .px(8.0)
                        .radius(12.0)
                        .fill(if s.lyrics_offset_ms != 0 { t.band } else { rune::Color::TRANSPARENT })
                        .border(1.0, if s.lyrics_offset_ms != 0 { t.brand } else { rune::Color::TRANSPARENT })
                        .center()
                        .child(
                            text(format!("Sync: {}", offset_label))
                                .font(typo::CAPTION)
                                .color(if s.lyrics_offset_ms != 0 { t.brand_text } else { t.muted }),
                        )
                        .on_press(|s: &mut AppState| {
                            s.reset_lyrics_offset();
                        }),
                )
                .child(
                    el()
                        .key("np-offset-plus")
                        .h(24.0)
                        .px(8.0)
                        .radius(12.0)
                        .fill(t.hover)
                        .border(1.0, t.border)
                        .center()
                        .child(text("+0.2s").font(typo::CAPTION).color(t.heading))
                        .on_press(|s: &mut AppState| {
                            s.adjust_lyrics_offset(200);
                        }),
                );

            let lyrics_bar = row()
                .w_full()
                .max_w(680.0)
                .h(36.0)
                .justify_between()
                .center_y()
                .px(12.0)
                .child(source_pills)
                .child(offset_ctrls);

            // Custom lyrics modal if active
            let custom_box = if s.show_custom_lyrics_modal {
                let input = rune::text_input(&s.custom_lyrics_edit, typo::CAPTION)
                    .key("custom-lrc-input")
                    .w_full()
                    .h(80.0)
                    .radius(8.0)
                    .border(1.0, t.brand)
                    .fill(t.surface)
                    .px(10.0)
                    .py(6.0)
                    .placeholder("Paste LRC timestamps here, e.g. [00:12.34] Lyrics line...", t.muted)
                    .on_edit(|s: &mut AppState| &mut s.custom_lyrics_edit);

                let save_btn = el()
                    .key("btn-save-custom-lrc")
                    .h(30.0)
                    .px(14.0)
                    .radius(15.0)
                    .fill(t.brand)
                    .center()
                    .child(text("Save & Apply").font(typo::LABEL).color(t.on_brand))
                    .on_press(|s: &mut AppState| {
                        let text = s.custom_lyrics_edit.text.clone();
                        s.save_custom_lyrics(text);
                    });

                let cancel_btn = el()
                    .key("btn-cancel-custom-lrc")
                    .h(30.0)
                    .px(14.0)
                    .radius(15.0)
                    .fill(t.hover)
                    .border(1.0, t.border)
                    .center()
                    .child(text("Cancel").font(typo::LABEL).color(t.heading))
                    .on_press(|s: &mut AppState| {
                        s.show_custom_lyrics_modal = false;
                    });

                col()
                    .w_full()
                    .max_w(680.0)
                    .px(12.0)
                    .py(12.0)
                    .radius(10.0)
                    .fill(t.panel)
                    .border(1.0, t.border)
                    .gap(8.0)
                    .child(text("Upload / Custom LRC Lyrics").font(typo::LABEL).color(t.heading))
                    .child(input)
                    .child(
                        row()
                            .gap(8.0)
                            .justify_end()
                            .child(cancel_btn)
                            .child(save_btn),
                    )
            } else {
                col()
            };

            let mut lyrics_col = col().center().gap(18.0).py(20.0);
            if let Some(payload) = &s.lyrics {
                if !payload.lines.is_empty() {
                    let active_idx = payload.lines.iter().enumerate().rposition(|(idx, line)| {
                        let t_start = line.time_sec + offset_sec;
                        let t_end = payload.lines.get(idx + 1).map(|next| next.time_sec + offset_sec).unwrap_or(t_start + 6.0);
                        s.current_time >= t_start && s.current_time < t_end
                    });

                    for (idx, line) in payload.lines.iter().enumerate() {
                        let line_time = line.time_sec + offset_sec;
                        let is_active = Some(idx) == active_idx;
                        let line_text = line.text.clone();
                        let target_sec = line_time;

                        let mut line_box = col()
                            .center()
                            .gap(3.0)
                            .child(
                                text(&line_text)
                                    .font(if is_active { typo::HERO } else { typo::TITLE })
                                    .color(if is_active { t.brand } else { t.muted }),
                            );

                        if let Some(rom) = &line.romanized {
                            line_box = line_box.child(
                                text(rom)
                                    .font(typo::CAPTION)
                                    .color(if is_active { t.brand_text } else { t.muted }),
                            );
                        }

                        lyrics_col = lyrics_col.child(
                            el()
                                .radius(8.0)
                                .px(16.0)
                                .py(6.0)
                                .child(line_box)
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
                .center()
                .child(lyrics_bar)
                .child(custom_box)
                .child(
                    col()
                        .w_full()
                        .grow(1.0)
                        .min_h(0.0)
                        .scroll(rune::ScrollSpec { stick_bottom: false })
                        .child(lyrics_col),
                )
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
