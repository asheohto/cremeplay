use crate::icons;
use crate::state::{AppState, SettingsTab};
use crate::theme::{all_themes, typo};
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    // Header
    let close_btn = el()
        .key("close-settings")
        .size(32.0, 32.0)
        .radius(16.0)
        .center()
        .fill(t.hover)
        .child(icon(&icons::CLOSE, 18.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_settings = false;
        });

    let header = row()
        .w_full()
        .h(48.0)
        .justify_between()
        .center_y()
        .child(text("Settings").font(typo::TITLE).color(t.heading))
        .child(close_btn);

    // Tabs
    let tab_item = |label: &str, tab: SettingsTab, current: SettingsTab| {
        let is_active = tab == current;
        el()
            .key(format!("set-tab-{}", label))
            .h(30.0)
            .px(12.0)
            .radius(6.0)
            .center()
            .fill(if is_active { t.hover } else { rune::Color::TRANSPARENT })
            .child(
                text(label)
                    .font(typo::LABEL)
                    .color(if is_active { t.brand } else { t.muted }),
            )
            .on_press(move |s: &mut AppState| {
                s.settings_tab = tab;
            })
    };

    let tabs = row()
        .w_full()
        .gap(8.0)
        .child(tab_item("General", SettingsTab::General, s.settings_tab))
        .child(tab_item("Account", SettingsTab::Account, s.settings_tab))
        .child(tab_item("Lyrics", SettingsTab::Lyrics, s.settings_tab))
        .child(tab_item("Integrations", SettingsTab::Integrations, s.settings_tab))
        .child(tab_item("Shortcuts", SettingsTab::Shortcuts, s.settings_tab))
        .child(tab_item("About", SettingsTab::About, s.settings_tab));

    let divider = || el().w_full().h(1.0).fill(t.border);

    // Body based on active tab
    let body = match s.settings_tab {
        SettingsTab::General => {
            // Theme selector grid: 2 columns matching original Cremeplay .theme-grid
            let theme_header = text("Color Themes").font(typo::LABEL).color(t.heading);
            let mut theme_grid = col().w_full().gap(8.0);
            let themes = all_themes();

            for chunk in themes.chunks(2) {
                let mut theme_row = row().w_full().gap(8.0);
                for theme in chunk {
                    let is_current = theme.id == s.theme.id;
                    let theme_id = theme.id;
                    let theme_card = row()
                        .key(format!("theme-card-{}", theme.id))
                        .grow(1.0)
                        .h(42.0)
                        .radius(12.0)
                        .fill(if is_current { theme.band } else { theme.surface })
                        .border(if is_current { 1.5 } else { 1.0 }, if is_current { theme.brand } else { theme.border })
                        .px(12.0)
                        .gap(10.0)
                        .center_y()
                        .child(
                            row()
                                .gap(4.0)
                                .child(el().size(12.0, 12.0).radius(6.0).fill(theme.brand))
                                .child(el().size(12.0, 12.0).radius(6.0).fill(theme.panel)),
                        )
                        .child(
                            text(theme.name)
                                .font(typo::LABEL)
                                .color(theme.heading)
                                .truncate(),
                        )
                        .on_press(move |s: &mut AppState| {
                            s.set_theme(theme_id);
                        });

                    theme_row = theme_row.child(theme_card);
                }
                theme_grid = theme_grid.child(theme_row);
            }

            // Toggles
            let toggle_row = |key: String, title: &str, is_on: bool, on_toggle: fn(&mut AppState)| {
                row()
                    .key(key)
                    .w_full()
                    .h(40.0)
                    .radius(10.0)
                    .px(12.0)
                    .justify_between()
                    .center_y()
                    .fill(t.hover)
                    .child(text(title).font(typo::BODY).color(t.heading))
                    .child(
                        row()
                            .size(42.0, 24.0)
                            .radius(12.0)
                            .fill(if is_on { t.brand } else { t.border })
                            .padding(3.0)
                            .center_y()
                            .child(
                                el()
                                    .size(18.0, 18.0)
                                    .radius(9.0)
                                    .fill(rune::Color::WHITE)
                                    .ml(if is_on { 18.0 } else { 0.0 }),
                            ),
                    )
                    .on_press(on_toggle)
            };

            let audio_dev_header = text("Audio Output Device").font(typo::LABEL).color(t.heading);
            let current_device = s.player.get_output_device();
            let mut dev_list = col().w_full().gap(6.0);

            let is_default = current_device.is_none();
            let default_btn = row()
                .key("audio-dev-default")
                .w_full()
                .h(36.0)
                .radius(8.0)
                .px(12.0)
                .justify_between()
                .center_y()
                .fill(if is_default { t.band } else { t.hover })
                .border(1.0, if is_default { t.brand } else { rune::Color::TRANSPARENT })
                .child(text("Default System Device").font(typo::BODY).color(if is_default { t.brand_text } else { t.heading }))
                .child(if is_default { text("Active").font(typo::CAPTION).color(t.brand) } else { text("").font(typo::CAPTION) })
                .on_press(|s: &mut AppState| {
                    s.set_audio_output_device(None);
                });
            dev_list = dev_list.child(default_btn);

            for dev in &s.audio_devices {
                let dev_name = dev.clone();
                let is_selected = current_device.as_deref() == Some(&dev_name);
                let btn = row()
                    .key(format!("audio-dev-{}", dev_name))
                    .w_full()
                    .h(36.0)
                    .radius(8.0)
                    .px(12.0)
                    .justify_between()
                    .center_y()
                    .fill(if is_selected { t.band } else { t.hover })
                    .border(1.0, if is_selected { t.brand } else { rune::Color::TRANSPARENT })
                    .child(text(&dev_name).font(typo::BODY).color(if is_selected { t.brand_text } else { t.heading }))
                    .child(if is_selected { text("Active").font(typo::CAPTION).color(t.brand) } else { text("").font(typo::CAPTION) })
                    .on_press(move |s: &mut AppState| {
                        s.set_audio_output_device(Some(dev_name.clone()));
                    });
                dev_list = dev_list.child(btn);
            }

            let cfg = s.config_mgr.get();
            col()
                .w_full()
                .gap(12.0)
                .child(theme_header)
                .child(theme_grid)
                .child(divider())
                .child(toggle_row("toggle-autoplay".to_string(), "Autoplay on startup", cfg.autoplay, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.autoplay = !cfg.autoplay;
                    s.config_mgr.save(cfg);
                }))
                .child(toggle_row("toggle-anim".to_string(), "Smooth UI animations", cfg.animations, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.animations = !cfg.animations;
                    s.config_mgr.save(cfg);
                }))
                .child(toggle_row("toggle-tray".to_string(), "Close to system tray", cfg.close_to_tray, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.close_to_tray = !cfg.close_to_tray;
                    s.config_mgr.save(cfg);
                }))
                .child(divider())
                .child(audio_dev_header)
                .child(dev_list)
        }
        SettingsTab::Account => {
            if let Some(acc) = &s.account {
                // Logged in view
                let avatar = if let Some(img) = s.get_image(&acc.avatar_url) {
                    el()
                        .size(48.0, 48.0)
                        .radius(24.0)
                        .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
                } else {
                    el()
                        .size(48.0, 48.0)
                        .radius(24.0)
                        .fill(t.brand)
                        .center()
                        .child(icon(&icons::SETTINGS, 24.0).color(t.on_brand))
                };

                let user_info = col()
                    .gap(4.0)
                    .child(text(&acc.name).font(typo::TITLE).color(t.heading))
                    .child(text("Connected to YouTube Music").font(typo::CAPTION).color(t.muted));

                let signout_btn = row()
                    .key("btn-signout")
                    .h(36.0)
                    .px(16.0)
                    .radius(18.0)
                    .fill(rune::color::hex("DE3730").unwrap_or(rune::Color { r: 0.87, g: 0.21, b: 0.19, a: 1.0 }))
                    .center()
                    .child(text("Sign Out").font(typo::LABEL).color(rune::Color::WHITE))
                    .on_press(|s: &mut AppState| {
                        s.logout();
                    });

                col()
                    .w_full()
                    .gap(16.0)
                    .child(
                        row()
                            .center_y()
                            .gap(14.0)
                            .child(avatar)
                            .child(user_info),
                    )
                    .child(divider())
                    .child(signout_btn)
            } else if s.is_polling_oauth {
                // OAuth Device Login in progress
                let code_str = s
                    .oauth_code_info
                    .as_ref()
                    .map(|i| i.user_code.clone())
                    .unwrap_or_else(|| "Loading…".to_string());

                let code_display = row()
                    .w_full()
                    .h(56.0)
                    .radius(12.0)
                    .fill(t.surface)
                    .border(1.0, t.border)
                    .center()
                    .child(
                        text(code_str)
                            .font(typo::HERO)
                            .color(t.brand_text),
                    );

                let copy_label = if s.oauth_copied { "✓ Code Copied" } else { "Copy Code" };
                let copy_btn = row()
                    .key("btn-oauth-copy")
                    .h(36.0)
                    .px(16.0)
                    .radius(18.0)
                    .fill(if s.oauth_copied { t.brand } else { t.hover })
                    .border(1.0, t.border)
                    .center()
                    .child(text(copy_label).font(typo::LABEL).color(if s.oauth_copied { t.on_brand } else { t.heading }))
                    .on_press(|s: &mut AppState| {
                        s.copy_oauth_code();
                    });

                let browser_btn = row()
                    .key("btn-oauth-browser")
                    .h(36.0)
                    .px(16.0)
                    .radius(18.0)
                    .fill(t.hover)
                    .border(1.0, t.border)
                    .center()
                    .child(text("Open Google Login").font(typo::LABEL).color(t.heading))
                    .on_press(|s: &mut AppState| {
                        s.open_oauth_url();
                    });

                let cancel_btn = row()
                    .key("btn-oauth-cancel")
                    .h(36.0)
                    .px(16.0)
                    .radius(18.0)
                    .fill(rune::Color::TRANSPARENT)
                    .border(1.0, t.border)
                    .center()
                    .child(text("Cancel").font(typo::LABEL).color(t.muted))
                    .on_press(|s: &mut AppState| {
                        s.cancel_oauth_login();
                    });

                let status_text = s
                    .oauth_status_msg
                    .as_deref()
                    .unwrap_or("Waiting for you to authorize in your browser…");

                col()
                    .w_full()
                    .gap(16.0)
                    .child(text("Google Account Sign-In").font(typo::TITLE).color(t.heading))
                    .child(
                        col()
                            .w_full()
                            .padding(18.0)
                            .radius(16.0)
                            .fill(t.panel)
                            .border(1.0, t.border)
                            .gap(14.0)
                            .child(text("Enter this code in your browser:").font(typo::LABEL).color(t.heading))
                            .child(code_display)
                            .child(
                                row()
                                    .gap(10.0)
                                    .center_y()
                                    .child(copy_btn)
                                    .child(browser_btn)
                                    .child(cancel_btn),
                            )
                            .child(
                                text(status_text)
                                    .font(typo::CAPTION)
                                    .color(t.brand),
                            ),
                    )
            } else {
                // Not logged in view: 1-Click Google Sign-In + Clipboard Sync + Cookie fallback
                let google_signin_btn = row()
                    .key("btn-google-device-login")
                    .w_full()
                    .h(48.0)
                    .radius(24.0)
                    .fill(t.brand)
                    .center()
                    .gap(10.0)
                    .child(icon(&icons::EXPLORE, 20.0).color(t.on_brand))
                    .child(text("✨ Sign In with Google (1-Click Popup)").font(typo::LABEL).color(t.on_brand))
                    .on_press(|s: &mut AppState| {
                        s.launch_auth_flow();
                    });

                let browser_btn = row()
                    .key("btn-open-browser-auth")
                    .h(38.0)
                    .px(16.0)
                    .radius(19.0)
                    .fill(t.hover)
                    .border(1.0, t.border)
                    .center()
                    .gap(8.0)
                    .child(text("Open in Browser").font(typo::LABEL).color(t.heading))
                    .on_press(|s: &mut AppState| {
                        s.open_auth_in_browser();
                    });

                let paste_btn = row()
                    .key("btn-paste-clipboard-auth")
                    .h(38.0)
                    .px(16.0)
                    .radius(19.0)
                    .fill(t.hover)
                    .border(1.0, t.border)
                    .center()
                    .gap(8.0)
                    .child(text("Paste from Clipboard").font(typo::LABEL).color(t.heading))
                    .on_press(|s: &mut AppState| {
                        s.paste_cookies_from_clipboard();
                    });

                let cookie_input = rune::text_input(&s.auth_cookie_edit, typo::CAPTION)
                    .key("cookie-input")
                    .single_line()
                    .w_full()
                    .h(38.0)
                    .radius(8.0)
                    .border(1.0, t.border)
                    .fill(t.surface)
                    .px(12.0)
                    .placeholder("Or paste SAPISID or cookie header here…", t.muted)
                    .on_edit(|s: &mut AppState| &mut s.auth_cookie_edit)
                    .on_submit(|s: &mut AppState, _| {
                        s.submit_cookies();
                    });

                let connect_btn = row()
                    .key("btn-connect-cookie")
                    .h(36.0)
                    .px(16.0)
                    .radius(18.0)
                    .fill(t.brand)
                    .center()
                    .child(text("Connect").font(typo::LABEL).color(t.on_brand))
                    .on_press(|s: &mut AppState| {
                        s.submit_cookies();
                    });

                col()
                    .w_full()
                    .gap(16.0)
                    .child(text("Sign in to YouTube Music").font(typo::TITLE).color(t.heading))
                    .child(
                        text("Sign in effortlessly to access your library, liked tracks, and personalized recommendations:")
                            .font(typo::BODY)
                            .color(t.muted),
                    )
                    .child(google_signin_btn)
                    .child(divider())
                    .child(
                        col()
                            .w_full()
                            .gap(10.0)
                            .child(text("Alternative: Browser Cookie Sync").font(typo::LABEL).color(t.heading))
                            .child(
                                text("If you are already logged into music.youtube.com in Chrome or Edge, click Paste from Clipboard:")
                                    .font(typo::CAPTION)
                                    .color(t.muted),
                            )
                            .child(
                                row()
                                    .gap(10.0)
                                    .child(paste_btn)
                                    .child(browser_btn),
                            )
                            .child(cookie_input)
                            .child(connect_btn),
                    )
            }

        }
        SettingsTab::Lyrics => {
            let cfg = s.config_mgr.get();
            let toggle_row = |key: String, title: &str, is_on: bool, on_toggle: fn(&mut AppState)| {
                row()
                    .key(key)
                    .w_full()
                    .h(40.0)
                    .radius(10.0)
                    .px(12.0)
                    .justify_between()
                    .center_y()
                    .fill(t.hover)
                    .child(text(title).font(typo::BODY).color(t.heading))
                    .child(
                        row()
                            .size(42.0, 24.0)
                            .radius(12.0)
                            .fill(if is_on { t.brand } else { t.border })
                            .padding(3.0)
                            .center_y()
                            .child(
                                el()
                                    .size(18.0, 18.0)
                                    .radius(9.0)
                                    .fill(rune::Color::WHITE)
                                    .ml(if is_on { 18.0 } else { 0.0 }),
                            ),
                    )
                    .on_press(on_toggle)
            };

            let preferred_source = cfg.lyrics_source.clone();
            let source_options = ["auto", "Better-Lyrics", "LRCLIB", "Genius"];
            let mut pref_pills = row().gap(6.0);
            for opt in &source_options {
                let is_sel = preferred_source.eq_ignore_ascii_case(opt);
                let opt_str = opt.to_string();
                let pill = el()
                    .key(format!("set-lyr-src-{}", opt))
                    .h(28.0)
                    .px(10.0)
                    .radius(14.0)
                    .center()
                    .fill(if is_sel { t.brand } else { t.surface })
                    .border(1.0, if is_sel { t.brand } else { t.border })
                    .child(
                        text(*opt)
                            .font(typo::CAPTION)
                            .color(if is_sel { t.brand_text } else { t.heading }),
                    )
                    .on_press(move |s: &mut AppState| {
                        let mut cfg = s.config_mgr.get();
                        cfg.lyrics_source = opt_str.clone();
                        s.lyrics_source = opt_str.clone();
                        s.config_mgr.save(cfg);
                    });
                pref_pills = pref_pills.child(pill);
            }

            let global_offset_ms = cfg.lyrics_offset_ms;
            let offset_row = row()
                .gap(6.0)
                .center_y()
                .child(
                    el()
                        .key("set-off-minus")
                        .h(26.0)
                        .px(8.0)
                        .radius(13.0)
                        .fill(t.surface)
                        .border(1.0, t.border)
                        .center()
                        .child(text("-200ms").font(typo::CAPTION).color(t.heading))
                        .on_press(|s: &mut AppState| {
                            let mut cfg = s.config_mgr.get();
                            cfg.lyrics_offset_ms -= 200;
                            s.lyrics_offset_ms = cfg.lyrics_offset_ms;
                            s.config_mgr.save(cfg);
                        }),
                )
                .child(
                    el()
                        .key("set-off-reset")
                        .h(26.0)
                        .px(10.0)
                        .radius(13.0)
                        .fill(if global_offset_ms != 0 { t.band } else { t.surface })
                        .border(1.0, if global_offset_ms != 0 { t.brand } else { t.border })
                        .center()
                        .child(
                            text(format!("{:+0}ms", global_offset_ms))
                                .font(typo::CAPTION)
                                .color(if global_offset_ms != 0 { t.brand_text } else { t.muted }),
                        )
                        .on_press(|s: &mut AppState| {
                            let mut cfg = s.config_mgr.get();
                            cfg.lyrics_offset_ms = 0;
                            s.lyrics_offset_ms = 0;
                            s.config_mgr.save(cfg);
                        }),
                )
                .child(
                    el()
                        .key("set-off-plus")
                        .h(26.0)
                        .px(8.0)
                        .radius(13.0)
                        .fill(t.surface)
                        .border(1.0, t.border)
                        .center()
                        .child(text("+200ms").font(typo::CAPTION).color(t.heading))
                        .on_press(|s: &mut AppState| {
                            let mut cfg = s.config_mgr.get();
                            cfg.lyrics_offset_ms += 200;
                            s.lyrics_offset_ms = cfg.lyrics_offset_ms;
                            s.config_mgr.save(cfg);
                        }),
                );

            let custom_count = cfg.custom_lyrics.len();

            col()
                .w_full()
                .gap(12.0)
                .child(text("Lyrics Settings").font(typo::TITLE).color(t.heading))
                .child(toggle_row("toggle-romanize".to_string(), "Romanize Foreign Lyrics", cfg.romanize_lyrics, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.romanize_lyrics = !cfg.romanize_lyrics;
                    s.config_mgr.save(cfg.clone());
                    s.apply_romanize_toggle(cfg.romanize_lyrics);
                }))
                .child(
                    row()
                        .w_full()
                        .h(48.0)
                        .radius(10.0)
                        .px(12.0)
                        .justify_between()
                        .center_y()
                        .fill(t.hover)
                        .child(text("Preferred Source").font(typo::BODY).color(t.heading))
                        .child(pref_pills),
                )
                .child(
                    row()
                        .w_full()
                        .h(44.0)
                        .radius(10.0)
                        .px(12.0)
                        .justify_between()
                        .center_y()
                        .fill(t.hover)
                        .child(text("Default Sync Offset").font(typo::BODY).color(t.heading))
                        .child(offset_row),
                )
                .child(
                    row()
                        .w_full()
                        .h(44.0)
                        .radius(10.0)
                        .px(12.0)
                        .justify_between()
                        .center_y()
                        .fill(t.hover)
                        .child(
                            col()
                                .gap(2.0)
                                .child(text("Custom Lyrics").font(typo::BODY).color(t.heading))
                                .child(text(format!("{} song(s) with custom lyrics", custom_count)).font(typo::CAPTION).color(t.muted)),
                        )
                        .child(
                            el()
                                .key("btn-clear-custom-lyrics")
                                .h(28.0)
                                .px(10.0)
                                .radius(14.0)
                                .fill(t.surface)
                                .border(1.0, t.border)
                                .center()
                                .child(text("Clear All").font(typo::CAPTION).color(t.muted))
                                .on_press(|s: &mut AppState| {
                                    let mut cfg = s.config_mgr.get();
                                    cfg.custom_lyrics.clear();
                                    s.config_mgr.save(cfg);
                                }),
                        ),
                )
        }
        SettingsTab::Integrations => {
            let cfg = s.config_mgr.get();
            let toggle_row = |key: String, title: &str, is_on: bool, on_toggle: fn(&mut AppState)| {
                row()
                    .key(key)
                    .w_full()
                    .h(40.0)
                    .radius(10.0)
                    .px(12.0)
                    .justify_between()
                    .center_y()
                    .fill(t.hover)
                    .child(text(title).font(typo::BODY).color(t.heading))
                    .child(
                        row()
                            .size(42.0, 24.0)
                            .radius(12.0)
                            .fill(if is_on { t.brand } else { t.border })
                            .padding(3.0)
                            .center_y()
                            .child(
                                el()
                                    .size(18.0, 18.0)
                                    .radius(9.0)
                                    .fill(rune::Color::WHITE)
                                    .ml(if is_on { 18.0 } else { 0.0 }),
                            ),
                    )
                    .on_press(on_toggle)
            };

            col()
                .w_full()
                .gap(12.0)
                .child(text("Integrations").font(typo::TITLE).color(t.heading))
                .child(toggle_row("toggle-discord".to_string(), "Discord Rich Presence", cfg.discord_rpc, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.discord_rpc = !cfg.discord_rpc;
                    s.discord.set_enabled(cfg.discord_rpc);
                    s.config_mgr.save(cfg);
                }))
                .child(toggle_row("toggle-sponsor".to_string(), "SponsorBlock Segment Auto-skip", cfg.sponsorblock, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.sponsorblock = !cfg.sponsorblock;
                    s.config_mgr.save(cfg);
                }))
                .child(toggle_row("toggle-tuna".to_string(), "OBS Tuna Overlay (port 1608)", cfg.tuna_obs, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.tuna_obs = !cfg.tuna_obs;
                    s.config_mgr.save(cfg);
                }))
        }
        SettingsTab::Shortcuts => {
            let shortcut_item = |key_badge: &str, desc: &str| {
                row()
                    .w_full()
                    .h(28.0)
                    .justify_between()
                    .center_y()
                    .child(text(desc).font(typo::BODY).color(t.heading))
                    .child(
                        el()
                            .h(22.0)
                            .px(8.0)
                            .radius(4.0)
                            .fill(t.hover)
                            .border(1.0, t.border)
                            .center()
                            .child(text(key_badge).font(typo::CAPTION).color(t.brand_text)),
                    )
            };

            col()
                .w_full()
                .gap(6.0)
                .child(text("Playback & Navigation Shortcuts").font(typo::LABEL).color(t.muted))
                .child(shortcut_item("Space", "Play / Pause"))
                .child(shortcut_item("N / P", "Next / Previous Track"))
                .child(shortcut_item("F", "Toggle Fullscreen Now Playing"))
                .child(shortcut_item("Q", "Toggle Queue & Lyrics Drawer"))
                .child(shortcut_item("M", "Mute / Unmute Audio"))
                .child(shortcut_item("L", "Like / Unlike Current Track"))
                .child(shortcut_item("S / R", "Shuffle / Cycle Repeat Mode"))
                .child(shortcut_item("← / →", "Seek Backward / Forward 5s"))
                .child(shortcut_item("↑ / ↓", "Volume Up / Down 5%"))
                .child(shortcut_item("/", "Quick Search Navigation"))
                .child(shortcut_item("?", "Open Shortcuts Guide"))
                .child(shortcut_item("Ctrl + ,", "Open Settings"))
                .child(shortcut_item("Esc", "Close Modals & Drawers"))
        }
        SettingsTab::About => {
            let cfg = s.config_mgr.get();
            let toggle_row = |key: String, title: &str, is_on: bool, on_toggle: fn(&mut AppState)| {
                row()
                    .key(key)
                    .w_full()
                    .h(40.0)
                    .radius(10.0)
                    .px(12.0)
                    .justify_between()
                    .center_y()
                    .fill(t.hover)
                    .child(text(title).font(typo::BODY).color(t.heading))
                    .child(
                        row()
                            .size(42.0, 24.0)
                            .radius(12.0)
                            .fill(if is_on { t.brand } else { t.border })
                            .padding(3.0)
                            .center_y()
                            .child(
                                el()
                                    .size(18.0, 18.0)
                                    .radius(9.0)
                                    .fill(rune::Color::WHITE)
                                    .ml(if is_on { 18.0 } else { 0.0 }),
                            ),
                    )
                    .on_press(on_toggle)
            };

            let update_button = if s.is_checking_update {
                el()
                    .key("btn-check-updates")
                    .h(32.0)
                    .px(14.0)
                    .radius(16.0)
                    .fill(t.hover)
                    .center()
                    .child(text("Checking for updates…").font(typo::CAPTION).color(t.muted))
            } else if let Some(info) = &s.update_info {
                if info.is_update_available {
                    el()
                        .key("btn-download-update")
                        .h(32.0)
                        .px(14.0)
                        .radius(16.0)
                        .fill(t.brand)
                        .center()
                        .child(text(format!("Download v{}", info.latest_version)).font(typo::LABEL).color(t.on_brand))
                        .on_press(|s: &mut AppState| {
                            s.open_update_url();
                        })
                } else {
                    el()
                        .key("btn-check-updates")
                        .h(32.0)
                        .px(14.0)
                        .radius(16.0)
                        .fill(t.hover)
                        .border(1.0, t.border)
                        .center()
                        .child(text("Check for Updates").font(typo::CAPTION).color(t.heading))
                        .on_press(|s: &mut AppState| {
                            s.check_for_updates(true);
                        })
                }
            } else {
                el()
                    .key("btn-check-updates")
                    .h(32.0)
                    .px(14.0)
                    .radius(16.0)
                    .fill(t.hover)
                    .border(1.0, t.border)
                    .center()
                    .child(text("Check for Updates").font(typo::CAPTION).color(t.heading))
                    .on_press(|s: &mut AppState| {
                        s.check_for_updates(true);
                    })
            };

            let status_text = s.update_status_message.as_deref().unwrap_or("Cremeplay is up to date");

            col()
                .w_full()
                .gap(12.0)
                .child(text("Cremeplay").font(typo::TITLE).color(t.heading))
                .child(text("Version 1.2.0 (Native Rune Webviewless Architecture)").font(typo::BODY).color(t.brand))
                .child(text("Zero bloat, ultra-low memory (~30 MB RAM) YouTube Music client in pure Rust & Rune.").font(typo::CAPTION).color(t.muted))
                .child(
                    row()
                        .w_full()
                        .h(46.0)
                        .radius(10.0)
                        .px(12.0)
                        .justify_between()
                        .center_y()
                        .fill(t.hover)
                        .child(
                            col()
                                .gap(2.0)
                                .child(text("Software Updates").font(typo::BODY).color(t.heading))
                                .child(text(status_text).font(typo::CAPTION).color(t.muted)),
                        )
                        .child(update_button),
                )
                .child(toggle_row("toggle-auto-update".to_string(), "Automatically Check on Startup", cfg.check_updates, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.check_updates = !cfg.check_updates;
                    s.config_mgr.save(cfg);
                }))
                .child(
                    row()
                        .gap(12.0)
                        .pt(8.0)
                        .child(
                            el()
                                .key("btn-open-github")
                                .h(30.0)
                                .px(12.0)
                                .radius(15.0)
                                .fill(t.surface)
                                .border(1.0, t.border)
                                .center()
                                .child(text("GitHub Repository").font(typo::CAPTION).color(t.heading))
                                .on_press(|_: &mut AppState| {
                                    #[cfg(target_os = "windows")]
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/C", "start", "https://github.com/asheohto/cremeplay"])
                                        .spawn();
                                }),
                        )
                        .child(
                            el()
                                .key("btn-open-issues")
                                .h(30.0)
                                .px(12.0)
                                .radius(15.0)
                                .fill(t.surface)
                                .border(1.0, t.border)
                                .center()
                                .child(text("Report an Issue").font(typo::CAPTION).color(t.muted))
                                .on_press(|_: &mut AppState| {
                                    #[cfg(target_os = "windows")]
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/C", "start", "https://github.com/asheohto/cremeplay/issues"])
                                        .spawn();
                                }),
                        ),
                )
        }
    };

    let modal = col()
        .key("settings-modal")
        .w(520.0)
        .radius(16.0)
        .fill(t.panel)
        .border(1.0, t.border)
        .padding(24.0)
        .gap(16.0)
        .child(header)
        .child(tabs)
        .child(divider())
        .child(body)
        .on_press(|_: &mut AppState| {
            // Absorb clicks inside modal
        });

    // Modal Backdrop overlay
    col()
        .key("settings-backdrop")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .fill(rune::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 })
        .child(modal)
        .on_press(|s: &mut AppState| {
            s.show_settings = false;
        })
}
