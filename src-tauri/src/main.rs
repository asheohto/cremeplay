#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_slint = args.iter().any(|a| a == "--slint" || a == "--native")
        || std::env::var("CREMEPLAY_UI").map(|v| v.to_lowercase() == "slint").unwrap_or(false);

    if use_slint {
        if let Err(e) = cremeplay_lib::native_app::run_native() {
            eprintln!("[Cremeplay] Slint native GUI error: {e}");
            cremeplay_lib::run();
        }
    } else {
        // Default: The beautiful, polished Cremeplay experience with low-RAM optimizations
        cremeplay_lib::run();
    }
}
