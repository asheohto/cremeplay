//! Windows Job Object implementation to bind all child processes (including WebView2)
//! strictly to Cremeplay. When Cremeplay exits or terminates, the Windows kernel automatically
//! and cleanly terminates all child processes, preventing orphaned or scattered msedgewebview2 instances.

#[cfg(windows)]
pub fn ensure_child_process_job() {
    use std::mem::size_of;
    use std::os::raw::c_void;

    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x00002000;
    const JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK: DWORD = 0x00001000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;

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
        limit_flags: DWORD,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: DWORD,
        affinity: usize,
        priority_class: DWORD,
        scheduling_class: DWORD,
    }

    #[repr(C)]
    struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
        io_info: IO_COUNTERS,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_limit: usize,
        peak_job_memory_limit: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(lp_job_attributes: *mut c_void, lp_name: *const u16) -> HANDLE;
        fn SetInformationJobObject(
            h_job: HANDLE,
            job_object_info_class: u32,
            lp_job_object_info: *const c_void,
            cb_job_object_info_length: DWORD,
        ) -> BOOL;
        fn AssignProcessToJobObject(h_job: HANDLE, h_process: HANDLE) -> BOOL;
        fn GetCurrentProcess() -> HANDLE;
    }

    static JOB_HANDLE: std::sync::atomic::AtomicPtr<c_void> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

    unsafe {
        let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if !job.is_null() {
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.basic_limit_information.limit_flags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;

            let success = SetInformationJobObject(
                job,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                &info as *const _ as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD,
            );

            if success != 0 {
                let cur_proc = GetCurrentProcess();
                let assigned = AssignProcessToJobObject(job, cur_proc);
                if assigned != 0 {
                    log::info!("[JobObject] Successfully assigned Cremeplay process tree to Job Object with KILL_ON_JOB_CLOSE");
                }
            }

            // Keep the job handle open for the lifetime of Cremeplay.
            JOB_HANDLE.store(job, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[cfg(windows)]
pub fn trim_process_working_set() {
    use std::mem::size_of;
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
    const PROCESS_SET_QUOTA: DWORD = 0x0100;
    const PROCESS_QUERY_INFORMATION: DWORD = 0x0400;

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
        fn K32EmptyWorkingSet(h_process: HANDLE) -> BOOL;
        fn GetCurrentProcess() -> HANDLE;
        fn GetCurrentProcessId() -> DWORD;
        fn CreateToolhelp32Snapshot(dw_flags: DWORD, th32_process_id: DWORD) -> HANDLE;
        fn Process32FirstW(h_snapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
        fn Process32NextW(h_snapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
        fn OpenProcess(dw_desired_access: DWORD, b_inherit_handle: BOOL, dw_process_id: DWORD) -> HANDLE;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    unsafe {
        let cur = GetCurrentProcess();
        let _ = K32EmptyWorkingSet(cur);

        let current_pid = GetCurrentProcessId();
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if !snapshot.is_null() && snapshot != INVALID_HANDLE_VALUE {
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dw_size = size_of::<PROCESSENTRY32W>() as DWORD;

            let mut child_pids = std::collections::HashSet::new();
            child_pids.insert(current_pid);

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

            // Recursively identify all descendants in the process tree
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

            // Flush/empty working set for each child (WebView2 host, renderers, GPU processes)
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
    }
}

#[cfg(not(windows))]
pub fn ensure_child_process_job() {}

#[cfg(not(windows))]
pub fn trim_process_working_set() {}

