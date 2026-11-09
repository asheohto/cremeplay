// Cremeplay Global Windows Media Key Listener
// Listens for system hardware media keys (Play/Pause, Next Track, Prev Track, Stop)
// using zero-dependency Win32 RegisterHotKey so users can control music in the background.

#[cfg(windows)]
pub fn start(handle: rune::AppHandle<crate::state::AppState>) {
    std::thread::Builder::new()
        .name("cremeplay-media-keys".to_string())
        .spawn(move || {
            use std::os::raw::c_int;
            type HWND = *mut std::ffi::c_void;
            type UINT = u32;
            type WPARAM = usize;
            type LPARAM = isize;
            type BOOL = c_int;

            #[repr(C)]
            struct POINT {
                _x: i32,
                _y: i32,
            }

            #[repr(C)]
            struct MSG {
                _hwnd: HWND,
                message: UINT,
                w_param: WPARAM,
                _l_param: LPARAM,
                _time: u32,
                _pt: POINT,
            }

            #[link(name = "user32")]
            unsafe extern "system" {
                fn RegisterHotKey(hwnd: HWND, id: c_int, fs_modifiers: UINT, vk: UINT) -> BOOL;
                fn UnregisterHotKey(hwnd: HWND, id: c_int) -> BOOL;
                fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: UINT, max: UINT) -> BOOL;
            }

            const VK_MEDIA_NEXT_TRACK: UINT = 0xB0;
            const VK_MEDIA_PREV_TRACK: UINT = 0xB1;
            const VK_MEDIA_STOP: UINT = 0xB2;
            const VK_MEDIA_PLAY_PAUSE: UINT = 0xB3;
            const WM_HOTKEY: UINT = 0x0312;

            unsafe {
                // Register thread-level hotkeys (hwnd = null)
                let _ = RegisterHotKey(std::ptr::null_mut(), 101, 0, VK_MEDIA_PLAY_PAUSE);
                let _ = RegisterHotKey(std::ptr::null_mut(), 102, 0, VK_MEDIA_NEXT_TRACK);
                let _ = RegisterHotKey(std::ptr::null_mut(), 103, 0, VK_MEDIA_PREV_TRACK);
                let _ = RegisterHotKey(std::ptr::null_mut(), 104, 0, VK_MEDIA_STOP);

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    if msg.message == WM_HOTKEY {
                        match msg.w_param {
                            101 => {
                                handle.update(|s, _tx| {
                                    s.is_playing = s.player.toggle_playback();
                                });
                            }
                            102 => {
                                handle.update(|s, _tx| {
                                    s.play_next();
                                });
                            }
                            103 => {
                                handle.update(|s, _tx| {
                                    s.play_prev();
                                });
                            }
                            104 => {
                                handle.update(|s, _tx| {
                                    s.is_playing = false;
                                    s.player.pause();
                                });
                            }
                            _ => {}
                        }
                    }
                }

                UnregisterHotKey(std::ptr::null_mut(), 101);
                UnregisterHotKey(std::ptr::null_mut(), 102);
                UnregisterHotKey(std::ptr::null_mut(), 103);
                UnregisterHotKey(std::ptr::null_mut(), 104);
            }
        })
        .expect("Failed to spawn media keys listener thread");
}

#[cfg(not(windows))]
pub fn start(_handle: rune::AppHandle<crate::state::AppState>) {}
