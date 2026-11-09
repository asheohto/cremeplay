use crate::state::AppState;
use rune::tray::{Tray, TrayEvent, TrayItem, TraySpec};
use rune::AppHandle;

#[cfg(target_os = "windows")]
use rune::tray::TrayIcon;

#[cfg(target_os = "windows")]
pub fn start(handle: AppHandle<AppState>) -> Option<Tray> {
    const LOGO_PNG: &[u8] = include_bytes!("../../../src-tauri/icons/128x128.png");
    let img = match image::load_from_memory(LOGO_PNG) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            log::warn!("[Tray] Failed to load logo png: {}", e);
            return None;
        }
    };
    let (w, h) = img.dimensions();
    let tray_icon = TrayIcon::rgba(w, h, img.into_raw());

    let menu = vec![
        TrayItem::action("play_pause", "Play / Pause"),
        TrayItem::action("next", "Next Track"),
        TrayItem::action("prev", "Previous Track"),
        TrayItem::Separator,
        TrayItem::action("show", "Open Cremeplay"),
        TrayItem::action("quit", "Quit Cremeplay"),
    ];

    let spec = TraySpec::new("cremeplay", "Cremeplay", tray_icon)
        .tooltip("Cremeplay")
        .menu(menu);

    Tray::new(spec, move |event| match event {
        TrayEvent::Activate { .. } => {
            handle.show();
            handle.focus();
        }
        TrayEvent::Menu(id) => match id.as_str() {
            "play_pause" => {
                handle.update(|s, _tx| {
                    s.is_playing = s.player.toggle_playback();
                });
            }
            "next" => {
                handle.update(|s, _tx| {
                    s.play_next();
                });
            }
            "prev" => {
                handle.update(|s, _tx| {
                    s.play_prev();
                });
            }
            "show" => {
                handle.show();
                handle.focus();
            }
            "quit" => {
                handle.quit();
            }
            _ => {}
        },
        _ => {}
    })
    .map_err(|e| {
        log::warn!("[Tray] Failed to initialize system tray: {}", e);
        e
    })
    .ok()
}

#[cfg(not(target_os = "windows"))]
pub fn start(_handle: AppHandle<AppState>) -> Option<Tray> {
    None
}
