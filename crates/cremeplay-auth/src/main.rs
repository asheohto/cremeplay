#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dpi::{LogicalPosition, LogicalSize};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use wry::{Rect, WebViewBuilder};

const AUTH_URL: &str = "https://accounts.google.com/ServiceLogin?ltmpl=music&service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F";
const CHROME_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.152 Safari/537.36";

struct AuthApp {
    window: Option<Window>,
    webview: Option<wry::WebView>,
    last_poll: Instant,
    success: bool,
}

impl Default for AuthApp {
    fn default() -> Self {
        Self {
            window: None,
            webview: None,
            last_poll: Instant::now(),
            success: false,
        }
    }
}

impl ApplicationHandler for AuthApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attributes = Window::default_attributes();
        attributes.title = "Sign in - Cremeplay".to_string();
        attributes.inner_size = Some(LogicalSize::new(540, 720).into());
        attributes.resizable = true;

        let window = match event_loop.create_window(attributes) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[cremeplay-auth] Failed to create window: {:?}", e);
                std::process::exit(1);
            }
        };

        let webview = match WebViewBuilder::new()
            .with_user_agent(CHROME_UA)
            .with_url(AUTH_URL)
            .build_as_child(&window)
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[cremeplay-auth] Failed to create webview: {:?}", e);
                std::process::exit(1);
            }
        };

        self.window = Some(window);
        self.webview = Some(webview);
        self.last_poll = Instant::now();
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                if let (Some(window), Some(webview)) = (&self.window, &self.webview) {
                    let logical = size.to_logical::<u32>(window.scale_factor());
                    let _ = webview.set_bounds(Rect {
                        position: LogicalPosition::new(0, 0).into(),
                        size: LogicalSize::new(logical.width, logical.height).into(),
                    });
                }
            }
            WindowEvent::CloseRequested => {
                std::process::exit(0);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.success {
            return;
        }

        if self.last_poll.elapsed() < Duration::from_millis(800) {
            return;
        }
        self.last_poll = Instant::now();

        let webview = match &self.webview {
            Some(w) => w,
            None => return,
        };

        // Check cookies on music.youtube.com
        if let Ok(cookies) = webview.cookies_for_url("https://music.youtube.com") {
            let has_auth = cookies.iter().any(|c| {
                let n = c.name();
                n == "SAPISID" || n == "__Secure-3PAPISID" || n == "LOGIN_INFO"
            });

            if has_auth {
                self.success = true;
                let cookie_str = cookies
                    .iter()
                    .map(|c| format!("{}={}", c.name(), c.value()))
                    .collect::<Vec<_>>()
                    .join("; ");

                println!("[cremeplay-auth] Authenticated cookies detected!");

                // 1. Notify Cremeplay live agent server
                let payload = serde_json::json!({
                    "action": "set_cookies",
                    "cookies": cookie_str
                });

                let _ = std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_secs(3))
                        .build();

                    if let Ok(c) = client {
                        let _ = c.post("http://127.0.0.1:9099/api/action")
                            .json(&payload)
                            .send();
                    }

                    // 2. Also save to local config.json as fallback
                    if let Some(config_dir) = dirs::data_local_dir() {
                        let cfg_path = config_dir.join("com.cremeplay.desktop").join("config.json");
                        if let Ok(content) = std::fs::read_to_string(&cfg_path)
                            && let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                                json["auth_cookies"] = serde_json::Value::String(cookie_str);
                                let _ = std::fs::write(&cfg_path, json.to_string());
                        }
                    }

                    std::thread::sleep(Duration::from_millis(200));
                    std::process::exit(0);
                }).join();
            }
        }
    }
}

fn main() {
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("[cremeplay-auth] Failed to initialize event loop: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut app = AuthApp::default();
    let _ = event_loop.run_app(&mut app);
}
