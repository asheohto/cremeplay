use crate::icons;
use crate::state::{ActiveView, AppState};
use crate::theme::typo;
use rune::{el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    // 0. Sidebar Toggle button
    let sidebar_toggle_btn = el()
        .key("sidebar-toggle-btn")
        .size(32.0, 32.0)
        .radius(16.0)
        .center()
        .fill(if s.sidebar_collapsed { t.band } else { rune::Color::TRANSPARENT })
        .child(
            icon(&icons::SIDEBAR_TOGGLE, 18.0)
                .color(if s.sidebar_collapsed { t.brand_text } else { t.heading }),
        )
        .help(if s.sidebar_collapsed { "Expand sidebar" } else { "Collapse sidebar" })
        .on_press(|s: &mut AppState| {
            s.toggle_sidebar();
        });

    // 1. Back button (32x32 circle)
    let can_go_back = !s.nav_history.is_empty();
    let back_btn = el()
        .key("nav-back")
        .size(32.0, 32.0)
        .radius(16.0)
        .center()
        .fill(if can_go_back { t.hover } else { rune::Color::TRANSPARENT })
        .border(1.0, if can_go_back { t.border } else { rune::Color::TRANSPARENT })
        .child(
            icon(&icons::NAV_BACK, 18.0)
                .color(if can_go_back { t.heading } else { t.muted }),
        )
        .on_press(|s: &mut AppState| {
            s.navigate_back();
        });

    // 2. Logo (Cremeplay app icon + Brand typography)
    let logo_img = if let Some(img) = s.logo_image.lock().unwrap().as_ref() {
        el()
            .size(32.0, 32.0)
            .radius(8.0)
            .pixels_fit(img.pixels(), rune::SurfaceFit::Fill)
    } else {
        el().size(32.0, 32.0).radius(8.0).fill(t.brand)
    };

    let logo_text = text("Cremeplay")
        .font(typo::TITLE)
        .color(t.heading);

    let logo = row()
        .key("logo-btn")
        .center_y()
        .gap(10.0)
        .child(logo_img)
        .child(logo_text)
        .on_press(|s: &mut AppState| {
            s.navigate_to(ActiveView::Home);
        });

    let left = row()
        .w(250.0)
        .center_y()
        .gap(8.0)
        .child(sidebar_toggle_btn)
        .child(back_btn)
        .child(logo);

    // 3. Search Bar (44px pill with Hugeicon search & clear)
    let search_icon_el = icon(&icons::SEARCH, 20.0).color(t.muted);

    let search_input_el = rune::text_input(&s.search_edit, typo::BODY)
        .key("header-search-input")
        .single_line()
        .w_full()
        .placeholder("Search songs, albums, artists", t.muted)
        .on_edit(|s: &mut AppState| &mut s.search_edit)
        .on_submit(|s: &mut AppState, _tx| {
            let q = s.search_edit.text.clone();
            s.trigger_search(q);
        });

    let mut search_bar = row()
        .key("search-bar")
        .w(520.0)
        .h(44.0)
        .radius(22.0)
        .fill(t.panel)
        .border(1.0, t.border)
        .px(16.0)
        .center_y()
        .gap(12.0)
        .child(search_icon_el)
        .child(search_input_el);

    if !s.search_edit.text.is_empty() {
        let clear_btn = el()
            .key("search-clear-btn")
            .size(24.0, 24.0)
            .radius(12.0)
            .center()
            .child(icon(&icons::SEARCH_CLEAR, 16.0).color(t.muted))
            .on_press(|s: &mut AppState| {
                s.search_edit.text.clear();
                s.search_suggestions.clear();
            });
        search_bar = search_bar.child(clear_btn);
    }

    let mut search_col = rune::col()
        .key("search-col")
        .w(520.0)
        .child(search_bar);

    if !s.search_suggestions.is_empty() && !s.search_edit.text.is_empty() {
        let mut sugg_list = rune::col()
            .key("search-suggestions-dropdown")
            .absolute(0.0, 50.0)
            .w(520.0)
            .radius(16.0)
            .fill(t.panel)
            .border(1.0, t.border)
            .padding(6.0)
            .gap(2.0);

        for (idx, suggestion) in s.search_suggestions.iter().take(6).enumerate() {
            let sugg = suggestion.clone();
            let item = row()
                .key(format!("sugg-item-{}", idx))
                .w_full()
                .h(38.0)
                .radius(10.0)
                .px(14.0)
                .center_y()
                .gap(12.0)
                .fill(t.hover)
                .child(icon(&icons::SEARCH, 16.0).color(t.muted))
                .child(text(&sugg).font(typo::BODY).color(t.heading))
                .on_press(move |s: &mut AppState| {
                    s.search_edit.text = sugg.clone();
                    s.search_suggestions.clear();
                    s.trigger_search(sugg.clone());
                });
            sugg_list = sugg_list.child(item);
        }

        search_col = search_col.child(sugg_list);
    }

    // 4. Right Controls (Theme, Settings, Sign in)
    let theme_btn = el()
        .key("theme-btn")
        .size(38.0, 38.0)
        .radius(19.0)
        .center()
        .fill(t.hover)
        .border(1.0, t.border)
        .child(icon(&icons::THEME, 20.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            let next_theme = match s.theme.id {
                "dark" => "european-vintage",
                "european-vintage" => "chinese-vintage",
                "chinese-vintage" => "matcha-forest",
                "matcha-forest" => "blueberry-berry",
                "blueberry-berry" => "sapphire-heritage",
                "sapphire-heritage" => "adaptive",
                _ => "dark",
            };
            s.set_theme(next_theme);
        });

    let settings_btn = el()
        .key("settings-btn")
        .size(38.0, 38.0)
        .radius(19.0)
        .center()
        .fill(t.hover)
        .border(1.0, t.border)
        .child(icon(&icons::SETTINGS, 20.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_settings = !s.show_settings;
        });

    let signin_btn = row()
        .key("signin-btn")
        .h(36.0)
        .radius(18.0)
        .px(18.0)
        .fill(t.brand)
        .center()
        .child(
            text(if let Some(acc) = &s.account {
                acc.name.as_str()
            } else {
                "Sign in"
            })
            .font(typo::LABEL)
            .color(t.brand_text),
        )
        .on_press(|s: &mut AppState| {
            if s.account.is_some() {
                s.show_settings = true;
                s.settings_tab = crate::state::SettingsTab::Account;
            } else {
                s.launch_auth_flow();
            }
        });

    let mut right = row()
        .justify_end()
        .center_y()
        .gap(10.0);

    if s.show_update_banner {
        let tag = s.update_info.as_ref().map(|u| u.latest_version.as_str()).unwrap_or("New");
        let update_pill = row()
            .key("header-update-badge")
            .h(32.0)
            .px(12.0)
            .radius(16.0)
            .center_y()
            .gap(6.0)
            .fill(t.band)
            .border(1.0, t.brand)
            .child(
                text(format!("Update v{}", tag))
                    .font(typo::CAPTION)
                    .color(t.brand_text),
            )
            .child(
                el()
                    .key("btn-header-dismiss-upd")
                    .size(16.0, 16.0)
                    .radius(8.0)
                    .center()
                    .child(icon(&icons::CLOSE, 10.0).color(t.muted))
                    .on_press(|s: &mut AppState| {
                        s.dismiss_update();
                    }),
            )
            .on_press(|s: &mut AppState| {
                s.open_update_url();
            });
        right = right.child(update_pill);
    }

    right = right
        .child(theme_btn)
        .child(settings_btn)
        .child(signin_btn);

    // Full 68px header bar
    row()
        .key("top-header")
        .w_full()
        .h(68.0)
        .fill(t.surface)
        .border(1.0, t.border)
        .px(24.0)
        .justify_between()
        .center_y()
        .child(left)
        .child(search_col)
        .child(right)
}
