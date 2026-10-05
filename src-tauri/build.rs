fn main() {
    slint_build::compile("ui/appwindow.slint").expect("Slint compilation failed");
    tauri_build::build();
}
