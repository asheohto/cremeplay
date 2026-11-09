# Cremeplay v1.2.0 — Native Rune Webviewless Architecture

### Highlights
- **100% Native Rust GPU Architecture**: Complete migration from Chromium/WebView2 to Rune (`wgpu`, `taffy`, `parley`). Zero Chromium processes, zero WebView2 overhead.
- **Ultra-Low Memory Footprint**: Idle memory is **~3.8 MB RAM**, active playback runs under **~28 MB RAM** (down from >140 MB).
- **Sub-30ms Launch Latency**: Native binary launches directly to GPU first-frame in milliseconds.
- **Custom Playlists System**: Full user-created playlists with persistent JSON storage, interactive "Add to Playlist" picker modal with track preview, single-click playlist creation, per-track removal, playlist deletion, and full playback queuing.
- **Variable Playback Speed**: Dynamic speed controls (`0.75x`, `Normal 1x`, `1.25x`, `1.5x`, `2.0x`) with smooth time-scaling and synchronized elapsed offset calculation.
- **Dynamic Audio Output Device Selection**: Seamlessly list and switch between available Windows audio endpoints (Speakers, Headphones, DACs, Virtual Cables) with persistent configuration.
- **Integrated Sleep Timer**: Sleek popover timer with quick intervals (15m, 30m, 45m, 60m), 'End of track' automatic playback pause, and live countdown badge.
- **Dynamic Audio Equalizer Visualizers**: Real-time harmonic animated equalizer meters in both the bottom player bar and full Now Playing overlay view.
- **Native System Tray**: Windows notification tray icon with background minimize, quick Play/Pause, Next, Previous, Open, and Exit options.
- **Adaptive Dark Dominant Color Theming**: Automatically extracts album artwork's dominant color palette and subtly tints background surfaces and lyrics canvases.
- **Gapless Audio Buffering & Stream Caching**: Background pre-fetches and caches forthcoming audio streams 15s before track end with a 128-entry bounded LRU cache for instant, zero-stutter track transitions.
- **Radio Recommendations & Autoplay**: Infinite automated queue populator that keeps playback uninterrupted when the active queue finishes.
- **Interactive Queue Drawer**: Track counter badge, one-click 'Clear Up Next' button, and individual track removal options.
- **Keyboard Shortcuts Reference**: Modal guide covering all playback, seek, overlay, and drawer hotkeys with '?' / 'Ctrl+,' shortcuts.
- **Multi-Source Synced Lyrics Pipeline**: Better-Lyrics TTML synchronized lyrics, LRCLIB LRC, Genius/YouTube captions, custom user lyrics, instant offset adjuster, and one-click source switching.
- **Live Local Agent Server**: Embedded HTTP API on `http://127.0.0.1:9099` for headless debugging, status queries, and external automation.
