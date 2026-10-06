use crate::commands::AppState;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let play_pause_i = MenuItem::with_id(app, "toggle_play", "Play / Pause", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let show_hide_i = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit Cremeplay", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &play_pause_i,
            &sep1,
            &show_hide_i,
            &sep2,
            &quit_i,
        ],
    )?;

    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Cremeplay - YouTube Music (Native Rust)")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "toggle_play" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.player.toggle_playback();
                    }
                }
                "show_hide" => {
                    if let Some(win) = app.get_webview_window("main") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = win.hide();
                        } else {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                }
                "quit" => {
                    crate::process_job::terminate_all_descendants();
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    let _ = tray;
    Ok(())
}
