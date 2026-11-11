use crate::icons;
use crate::state::AppState;
use crate::theme::typo;
use rune::{col, el, icon, row, text, El};

pub fn render(s: &AppState) -> El {
    let t = &s.theme;

    let close_btn = el()
        .key("close-rename-modal")
        .size(32.0, 32.0)
        .radius(16.0)
        .center()
        .fill(t.hover)
        .child(icon(&icons::CLOSE, 16.0).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_rename_playlist_modal = false;
            s.rename_playlist_id = None;
        });

    let header = row()
        .w_full()
        .h(40.0)
        .justify_between()
        .center_y()
        .child(text("Rename Playlist").font(typo::TITLE).color(t.heading))
        .child(close_btn);

    let input = rune::text_input(&s.rename_playlist_edit, typo::BODY)
        .key("rename-input")
        .single_line()
        .w_full()
        .h(42.0)
        .radius(8.0)
        .border(1.0, t.brand)
        .fill(t.surface)
        .px(12.0)
        .placeholder("Enter playlist title…", t.muted)
        .on_edit(|s: &mut AppState| &mut s.rename_playlist_edit)
        .on_submit(|s: &mut AppState, _| {
            if let Some(id) = s.rename_playlist_id.clone() {
                let title = s.rename_playlist_edit.text.clone();
                s.rename_custom_playlist(&id, title);
            }
            s.show_rename_playlist_modal = false;
            s.rename_playlist_id = None;
        });

    let cancel_btn = row()
        .key("btn-cancel-rename")
        .h(36.0)
        .px(16.0)
        .radius(18.0)
        .fill(t.hover)
        .border(1.0, t.border)
        .center()
        .child(text("Cancel").font(typo::LABEL).color(t.heading))
        .on_press(|s: &mut AppState| {
            s.show_rename_playlist_modal = false;
            s.rename_playlist_id = None;
        });

    let save_btn = row()
        .key("btn-save-rename")
        .h(36.0)
        .px(20.0)
        .radius(18.0)
        .fill(t.brand)
        .center()
        .child(text("Save").font(typo::LABEL).color(t.on_brand))
        .on_press(|s: &mut AppState| {
            if let Some(id) = s.rename_playlist_id.clone() {
                let title = s.rename_playlist_edit.text.clone();
                s.rename_custom_playlist(&id, title);
            }
            s.show_rename_playlist_modal = false;
            s.rename_playlist_id = None;
        });

    let btn_row = row()
        .w_full()
        .justify_end()
        .gap(10.0)
        .child(cancel_btn)
        .child(save_btn);

    let modal = col()
        .key("rename-modal-dialog")
        .w(360.0)
        .radius(16.0)
        .fill(t.panel)
        .border(1.0, t.border)
        .padding(20.0)
        .gap(16.0)
        .child(header)
        .child(input)
        .child(btn_row)
        .on_press(|_: &mut AppState| {});

    col()
        .key("rename-modal-backdrop")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .fill(rune::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 })
        .child(modal)
        .on_press(|s: &mut AppState| {
            s.show_rename_playlist_modal = false;
            s.rename_playlist_id = None;
        })
}
