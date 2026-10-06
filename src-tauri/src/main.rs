#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
fn bring_existing_instance_to_foreground() -> bool {
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type HWND = *mut c_void;
    type BOOL = i32;
    type LPARAM = isize;
    type WNDENUMPROC = unsafe extern "system" fn(HWND, LPARAM) -> BOOL;

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SW_RESTORE: i32 = 9;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(lp_attributes: *mut c_void, b_initial_owner: BOOL, lp_name: *const u16) -> HANDLE;
        fn GetLastError() -> u32;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> HWND;
        fn ShowWindow(h_wnd: HWND, n_cmd_show: i32) -> BOOL;
        fn SetForegroundWindow(h_wnd: HWND) -> BOOL;
        fn EnumWindows(lp_enum_func: WNDENUMPROC, l_param: LPARAM) -> BOOL;
        fn GetWindowTextW(h_wnd: HWND, lp_string: *mut u16, n_max_count: i32) -> i32;
    }

    let mutex_name: Vec<u16> = "Global\\CremeplayAppSingleInstanceMutex\0".encode_utf16().collect();
    let title: Vec<u16> = "Cremeplay\0".encode_utf16().collect();

    unsafe {
        let handle = CreateMutexW(std::ptr::null_mut(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // Already running! Try direct window find first
            let mut target_hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

            // If not found by direct exact name, enumerate all top-level windows
            if target_hwnd.is_null() {
                unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                    let mut buf = [0u16; 512];
                    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), 512) };
                    if len > 0 {
                        let text = String::from_utf16_lossy(&buf[..len as usize]);
                        if text.contains("Cremeplay") {
                            unsafe {
                                let out_ptr = lparam as *mut HWND;
                                *out_ptr = hwnd;
                            }
                            return 0; // stop enumeration
                        }
                    }
                    1 // continue enumeration
                }

                EnumWindows(enum_proc, &mut target_hwnd as *mut _ as LPARAM);
            }

            if !target_hwnd.is_null() {
                ShowWindow(target_hwnd, SW_RESTORE);
                SetForegroundWindow(target_hwnd);
            }

            if !handle.is_null() {
                CloseHandle(handle);
            }
            return true;
        }

        let _ = handle;
    }

    false
}

fn main() {
    #[cfg(windows)]
    {
        if bring_existing_instance_to_foreground() {
            return;
        }
    }

    cremeplay_lib::run();
}
