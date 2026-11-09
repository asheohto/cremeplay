#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("../../src-tauri/icons/icon.ico");
    res.set("FileDescription", "Cremeplay Music Player");
    res.set("ProductName", "Cremeplay");
    res.set("OriginalFilename", "cremeplay.exe");
    let _ = res.compile();
}

#[cfg(not(windows))]
fn main() {}
