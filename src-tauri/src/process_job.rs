//! Windows Process Job & Audio Session Management
//! 1. Job Object: Bundles Cremeplay and all its child processes (WebView2 Browser, Renderer, GPU)
//!    into a unified job that automatically cleans up on exit.
//! 2. Working Set Trimming: Traverses the process tree to purge inactive memory pages.
//! 3. Audio Session Labeling: Identifies WASAPI audio sessions created by Cremeplay / WebView2
//!    and re-labels them to "Cremeplay" with the Cremeplay icon in Windows Volume Mixer.

#[cfg(windows)]
pub fn ensure_child_process_job() {
    use std::mem::size_of;
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type BOOL = i32;

    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x00002000;
    const JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK: u32 = 0x00001000;

    #[repr(C)]
    struct IO_COUNTERS {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
        io_info: IO_COUNTERS,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(lp_job_attributes: *mut c_void, lp_name: *const u16) -> HANDLE;
        fn SetInformationJobObject(
            h_job: HANDLE,
            job_object_info_class: u32,
            lp_job_object_info: *const c_void,
            cb_job_object_info_length: u32,
        ) -> BOOL;
        fn AssignProcessToJobObject(h_job: HANDLE, h_process: HANDLE) -> BOOL;
        fn OpenProcess(dw_desired_access: u32, b_inherit_handle: BOOL, dw_process_id: u32) -> HANDLE;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    static JOB_HANDLE: std::sync::atomic::AtomicPtr<c_void> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

    let mut job = JOB_HANDLE.load(std::sync::atomic::Ordering::Relaxed);
    if job.is_null() {
        unsafe {
            job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if !job.is_null() {
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.basic_limit_information.limit_flags =
                    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;

                let set_ok = SetInformationJobObject(
                    job,
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                    &info as *const _ as *const c_void,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );

                if set_ok != 0 {
                    JOB_HANDLE.store(job, std::sync::atomic::Ordering::Relaxed);
                } else {
                    let _ = CloseHandle(job);
                    job = std::ptr::null_mut();
                }
            }
        }
    }

    if !job.is_null() {
        const PROCESS_SET_QUOTA: u32 = 0x0100;
        const PROCESS_TERMINATE: u32 = 0x0001;
        let pids = get_descendant_pids();
        for pid in pids {
            unsafe {
                let h = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
                if !h.is_null() && h != (-1isize as HANDLE) {
                    let _ = AssignProcessToJobObject(job, h);
                    let _ = CloseHandle(h);
                }
            }
        }
    }
}

#[cfg(windows)]
pub fn get_descendant_pids() -> std::collections::HashSet<u32> {
    use std::mem::size_of;
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    #[repr(C)]
    struct PROCESSENTRY32W {
        dw_size: DWORD,
        cnt_usage: DWORD,
        th32_process_id: DWORD,
        th32_default_heap_id: usize,
        th32_module_id: DWORD,
        cnt_threads: DWORD,
        th32_parent_process_id: DWORD,
        pc_pri_class_base: i32,
        dw_flags: DWORD,
        sz_exe_file: [u16; 260],
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcessId() -> DWORD;
        fn CreateToolhelp32Snapshot(dw_flags: DWORD, th32_process_id: DWORD) -> HANDLE;
        fn Process32FirstW(h_snapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
        fn Process32NextW(h_snapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    let mut child_pids = std::collections::HashSet::new();

    unsafe {
        let current_pid = GetCurrentProcessId();
        child_pids.insert(current_pid);

        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if !snapshot.is_null() && snapshot != INVALID_HANDLE_VALUE {
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dw_size = size_of::<PROCESSENTRY32W>() as DWORD;

            let mut all_processes = Vec::new();
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    all_processes.push((entry.th32_process_id, entry.th32_parent_process_id));
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);

            let mut changed = true;
            while changed {
                changed = false;
                for &(pid, parent_pid) in &all_processes {
                    if child_pids.contains(&parent_pid) && !child_pids.contains(&pid) {
                        child_pids.insert(pid);
                        changed = true;
                    }
                }
            }
        }
    }

    child_pids
}

#[cfg(windows)]
pub fn trim_process_working_set() {
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
    const PROCESS_SET_QUOTA: DWORD = 0x0100;
    const PROCESS_QUERY_INFORMATION: DWORD = 0x0400;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn K32EmptyWorkingSet(h_process: HANDLE) -> BOOL;
        fn GetCurrentProcess() -> HANDLE;
        fn GetCurrentProcessId() -> DWORD;
        fn OpenProcess(dw_desired_access: DWORD, b_inherit_handle: BOOL, dw_process_id: DWORD) -> HANDLE;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    unsafe {
        let cur = GetCurrentProcess();
        let _ = K32EmptyWorkingSet(cur);

        let current_pid = GetCurrentProcessId();
        let child_pids = get_descendant_pids();

        for &pid in &child_pids {
            if pid != current_pid {
                let h_proc = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, 0, pid);
                if !h_proc.is_null() && h_proc != INVALID_HANDLE_VALUE {
                    let _ = K32EmptyWorkingSet(h_proc);
                    let _ = CloseHandle(h_proc);
                }
            }
        }
    }

    ensure_child_process_job();
    label_audio_sessions_as_cremeplay();
}

#[cfg(windows)]
pub fn label_audio_sessions_as_cremeplay() {
    use std::os::raw::c_void;
    type HRESULT = i32;

    #[repr(C)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    const CLSID_MMDEVICE_ENUMERATOR: GUID = GUID {
        data1: 0xBCDE0395,
        data2: 0xE52F,
        data3: 0x467C,
        data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
    };

    const IID_IMMDEVICE_ENUMERATOR: GUID = GUID {
        data1: 0xA95664D2,
        data2: 0x9614,
        data3: 0x4F35,
        data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
    };

    const IID_IAUDIO_SESSION_MANAGER2: GUID = GUID {
        data1: 0x77AA99A0,
        data2: 0x1BD6,
        data3: 0x484F,
        data4: [0x8B, 0xC7, 0x2C, 0x65, 0x4C, 0x9A, 0x9B, 0x6F],
    };

    const IID_IAUDIO_SESSION_CONTROL2: GUID = GUID {
        data1: 0xBFB7FF88,
        data2: 0x7239,
        data3: 0x4FC9,
        data4: [0x8F, 0xA2, 0x07, 0xC9, 0x50, 0xBE, 0x9C, 0x6D],
    };

    #[repr(C)]
    struct IMMDeviceEnumeratorVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        enum_audio_endpoints: *const c_void,
        get_default_audio_endpoint: unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    struct IMMDeviceVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        activate: unsafe extern "system" fn(*mut c_void, *const GUID, u32, *const c_void, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    struct IAudioSessionManager2Vtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_audio_session_control: *const c_void,
        get_simple_audio_volume: *const c_void,
        get_session_enumerator: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    struct IAudioSessionEnumeratorVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_count: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
        get_session: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    struct IAudioSessionControlVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_state: *const c_void,
        get_display_name: *const c_void,
        set_display_name: unsafe extern "system" fn(*mut c_void, *const u16, *const GUID) -> HRESULT,
        get_icon_path: *const c_void,
        set_icon_path: unsafe extern "system" fn(*mut c_void, *const u16, *const GUID) -> HRESULT,
    }

    #[repr(C)]
    struct IAudioSessionControl2Vtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_state: *const c_void,
        get_display_name: *const c_void,
        set_display_name: unsafe extern "system" fn(*mut c_void, *const u16, *const GUID) -> HRESULT,
        get_icon_path: *const c_void,
        set_icon_path: unsafe extern "system" fn(*mut c_void, *const u16, *const GUID) -> HRESULT,
        get_grouping_param: *const c_void,
        set_grouping_param: *const c_void,
        register_audio_session_notification: *const c_void,
        unregister_audio_session_notification: *const c_void,
        get_session_identifier: *const c_void,
        get_session_instance_identifier: *const c_void,
        get_process_id: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    }

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitialize(pv_reserved: *mut c_void) -> HRESULT;
        fn CoUninitialize();
        fn CoCreateInstance(
            rclsid: *const GUID,
            p_unk_outer: *mut c_void,
            dw_cls_context: u32,
            riid: *const GUID,
            ppv: *mut *mut c_void,
        ) -> HRESULT;
    }

    let pids = get_descendant_pids();
    if pids.is_empty() {
        return;
    }

    let display_name: Vec<u16> = "Cremeplay\0".encode_utf16().collect();
    let exe_icon: Vec<u16> = std::env::current_exe()
        .ok()
        .map(|p| format!("{},0\0", p.display()).encode_utf16().collect())
        .unwrap_or_else(|| "Cremeplay\0".encode_utf16().collect());

    unsafe {
        let _ = CoInitialize(std::ptr::null_mut());

        let mut enumerator: *mut c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_MMDEVICE_ENUMERATOR,
            std::ptr::null_mut(),
            1, // CLSCTX_INPROC_SERVER
            &IID_IMMDEVICE_ENUMERATOR,
            &mut enumerator,
        );

        if hr >= 0 && !enumerator.is_null() {
            let enum_vtbl = *(enumerator as *mut *mut IMMDeviceEnumeratorVtbl);
            let mut device: *mut c_void = std::ptr::null_mut();
            let hr_dev = ((*enum_vtbl).get_default_audio_endpoint)(enumerator, 0, 1, &mut device);

            if hr_dev >= 0 && !device.is_null() {
                let dev_vtbl = *(device as *mut *mut IMMDeviceVtbl);
                let mut session_mgr: *mut c_void = std::ptr::null_mut();
                let hr_mgr = ((*dev_vtbl).activate)(
                    device,
                    &IID_IAUDIO_SESSION_MANAGER2,
                    1, // CLSCTX_INPROC_SERVER
                    std::ptr::null(),
                    &mut session_mgr,
                );

                if hr_mgr >= 0 && !session_mgr.is_null() {
                    let mgr_vtbl = *(session_mgr as *mut *mut IAudioSessionManager2Vtbl);
                    let mut session_enum: *mut c_void = std::ptr::null_mut();
                    let hr_enum = ((*mgr_vtbl).get_session_enumerator)(session_mgr, &mut session_enum);

                    if hr_enum >= 0 && !session_enum.is_null() {
                        let sess_enum_vtbl = *(session_enum as *mut *mut IAudioSessionEnumeratorVtbl);
                        let mut count: i32 = 0;
                        let _ = ((*sess_enum_vtbl).get_count)(session_enum, &mut count);

                        for i in 0..count {
                            let mut ctrl: *mut c_void = std::ptr::null_mut();
                            let hr_ctrl = ((*sess_enum_vtbl).get_session)(session_enum, i, &mut ctrl);
                            if hr_ctrl >= 0 && !ctrl.is_null() {
                                let ctrl_vtbl = *(ctrl as *mut *mut IAudioSessionControlVtbl);
                                let mut ctrl2: *mut c_void = std::ptr::null_mut();
                                let hr_c2 = ((*ctrl_vtbl).query_interface)(ctrl, &IID_IAUDIO_SESSION_CONTROL2, &mut ctrl2);

                                if hr_c2 >= 0 && !ctrl2.is_null() {
                                    let ctrl2_vtbl = *(ctrl2 as *mut *mut IAudioSessionControl2Vtbl);
                                    let mut pid: u32 = 0;
                                    let hr_pid = ((*ctrl2_vtbl).get_process_id)(ctrl2, &mut pid);

                                    if hr_pid >= 0 && pids.contains(&pid) {
                                        let _ = ((*ctrl_vtbl).set_display_name)(ctrl, display_name.as_ptr(), std::ptr::null());
                                        let _ = ((*ctrl_vtbl).set_icon_path)(ctrl, exe_icon.as_ptr(), std::ptr::null());
                                    }

                                    let _ = ((*ctrl2_vtbl).release)(ctrl2);
                                }
                                let _ = ((*ctrl_vtbl).release)(ctrl);
                            }
                        }
                        let _ = ((*sess_enum_vtbl).release)(session_enum);
                    }
                    let _ = ((*mgr_vtbl).release)(session_mgr);
                }
                let _ = ((*dev_vtbl).release)(device);
            }
            let _ = ((*enum_vtbl).release)(enumerator);
        }

        CoUninitialize();
    }
}

#[cfg(windows)]
pub fn terminate_all_descendants() {
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
    const PROCESS_TERMINATE: DWORD = 0x0001;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(dw_desired_access: DWORD, b_inherit_handle: BOOL, dw_process_id: DWORD) -> HANDLE;
        fn TerminateProcess(h_process: HANDLE, u_exit_code: u32) -> BOOL;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
        fn GetCurrentProcessId() -> DWORD;
    }

    let cur_pid = unsafe { GetCurrentProcessId() };
    let pids = get_descendant_pids();
    for pid in pids {
        if pid != cur_pid {
            unsafe {
                let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
                if !h.is_null() && h != INVALID_HANDLE_VALUE {
                    let _ = TerminateProcess(h, 0);
                    let _ = CloseHandle(h);
                }
            }
        }
    }
}

#[cfg(not(windows))]
pub fn ensure_child_process_job() {}

#[cfg(not(windows))]
pub fn trim_process_working_set() {}

#[cfg(not(windows))]
pub fn label_audio_sessions_as_cremeplay() {}

#[cfg(not(windows))]
pub fn terminate_all_descendants() {}

