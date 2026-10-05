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
    use std::os::raw::c_void;
    type HANDLE = *mut c_void;
    type BOOL = i32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn K32EmptyWorkingSet(h_process: HANDLE) -> BOOL;
        fn GetCurrentProcess() -> HANDLE;
    }

    unsafe {
        let cur = GetCurrentProcess();
        let _ = K32EmptyWorkingSet(cur);
    }
}

#[cfg(not(windows))]
pub fn ensure_child_process_job() {}

#[cfg(not(windows))]
pub fn trim_process_working_set() {}

