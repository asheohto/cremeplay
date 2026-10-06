//! Windows Process Memory Working Set Trimming
//! Traverses the Cremeplay process tree and calls K32EmptyWorkingSet to purge inactive pages.


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

