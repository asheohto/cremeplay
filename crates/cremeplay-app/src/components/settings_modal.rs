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
        .child(tab_item("About", SettingsTab::About, s.settings_tab));

    let divider = || el().w_full().h(1.0).fill(t.border);

    // Body based on active tab
    let body = match s.settings_tab {
        SettingsTab::General => {
            // Theme selector grid
            let theme_header = text("Color Themes").font(typo::LABEL).color(t.heading);
            let mut theme_grid = row().w_full().gap(8.0);

            for theme in all_themes().into_iter() {
                let is_current = theme.id == s.theme.id;
                let theme_id = theme.id;
                let theme_card = col()
                    .key(format!("theme-card-{}", theme.id))
                    .w(80.0)
                    .h(60.0)
                    .radius(8.0)
                    .fill(theme.surface)
                    .border(if is_current { 2.0 } else { 1.0 }, if is_current { theme.brand } else { theme.border })
                    .padding(6.0)
                    .justify_between()
                    .child(
                        row()
                            .gap(4.0)
                            .child(el().size(12.0, 12.0).radius(6.0).fill(theme.brand))
                            .child(el().size(12.0, 12.0).radius(6.0).fill(theme.panel)),
                    )
                    .child(
                        text(theme.name)
                            .font(typo::CAPTION)
                            .color(theme.heading),
                    )
                    .on_press(move |s: &mut AppState| {
                        s.set_theme(theme_id);
                    });

                theme_grid = theme_grid.child(theme_card);
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
                        .child(icon(&icons::SETTINGS, 24.0).color(t.brand_text))
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
            } else {
                // Not logged in view
                let cookie_input = rune::text_input(&s.auth_cookie_edit, typo::CAPTION)
                    .key("cookie-input")
                    .single_line()
                    .w_full()
                    .h(38.0)
                    .radius(8.0)
                    .border(1.0, t.border)
                    .fill(t.surface)
                    .px(12.0)
                    .placeholder("Paste SAPISID or cookie header here…", t.muted)
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
                    .child(text("Connect").font(typo::LABEL).color(t.brand_text))
                    .on_press(|s: &mut AppState| {
                        s.submit_cookies();
                    });

                let browser_btn = row()
                    .key("btn-open-browser-auth")
                    .h(36.0)
                    .px(16.0)
                    .radius(18.0)
                    .fill(t.hover)
                    .border(1.0, t.border)
                    .center()
                    .child(text("Open in Browser").font(typo::LABEL).color(t.heading))
                    .on_press(|_: &mut AppState| {
                        #[cfg(target_os = "windows")]
                        let _ = std::process::Command::new("cmd")
                            .args(["/C", "start", "https://accounts.google.com/ServiceLogin?ltmpl=music&service=youtube&continue=https%3A%2F%2Fmusic.youtube.com%2F"])
                            .spawn();
                    });

                col()
                    .w_full()
                    .gap(14.0)
                    .child(text("Sign in to YouTube Music").font(typo::TITLE).color(t.heading))
                    .child(text("Paste your account cookies below or log in via browser:").font(typo::BODY).color(t.muted))
                    .child(cookie_input)
                    .child(
                        row()
                            .gap(10.0)
                            .child(connect_btn)
                            .child(browser_btn),
                    )
                    .child(
                        col()
                            .gap(4.0)
                            .pt(6.0)
                            .child(text("How to connect:").font(typo::LABEL).color(t.heading))
                            .child(text("1. Open music.youtube.com in your browser & sign in.").font(typo::CAPTION).color(t.muted))
                            .child(text("2. Copy your Cookie header or SAPISID from DevTools or cookie extension.").font(typo::CAPTION).color(t.muted))
                            .child(text("3. Paste here and click Connect. Playlists & recommendations sync instantly.").font(typo::CAPTION).color(t.muted)),
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

            col()
                .w_full()
                .gap(12.0)
                .child(text("Lyrics Settings").font(typo::TITLE).color(t.heading))
                .child(toggle_row("toggle-romanize".to_string(), "Romanize Foreign Lyrics", cfg.romanize_lyrics, |s| {
                    let mut cfg = s.config_mgr.get();
                    cfg.romanize_lyrics = !cfg.romanize_lyrics;
                    s.config_mgr.save(cfg);
                }))
                .child(
                    row()
                        .w_full()
                        .h(40.0)
                        .radius(10.0)
                        .px(12.0)
                        .justify_between()
                        .center_y()
                        .fill(t.hover)
                        .child(text("Preferred Source").font(typo::BODY).color(t.heading))
                        .child(text("Auto (LRCLIB -> Better-Lyrics -> YT)").font(typo::CAPTION).color(t.brand)),
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
        SettingsTab::About => {
            col()
                .w_full()
                .gap(12.0)
                .child(text("Cremeplay").font(typo::TITLE).color(t.heading))
                .child(text("Version 1.2.0 (Native Rune Webviewless Edition)").font(typo::BODY).color(t.brand))
                .child(text("Zero bloat, ultra-low memory YouTube Music client in pure Rust & Rune.").font(typo::CAPTION).color(t.muted))
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
