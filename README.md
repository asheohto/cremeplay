<div align="center">

  <img src="assets/cremeplay.png" alt="Cremeplay Logo" width="140" />

  # Cremeplay

  Ultra-lightweight, debloated YouTube Music desktop client powered by Tauri v2 and Rust.

  [![Release](https://img.shields.io/github/v/release/asheohto/cremeplay?style=for-the-badge&logo=github&color=E4544B)](https://github.com/asheohto/cremeplay/releases/latest)
  [![License](https://img.shields.io/badge/License-MIT-2ecc71?style=for-the-badge)](LICENSE)
  [![Support on Ko-fi](https://img.shields.io/badge/Ko--fi-omoretti-FF5E5B?style=for-the-badge&logo=kofi&logoColor=white)](https://ko-fi.com/omoretti)

</div>

---

## Support

<a href='https://ko-fi.com/omoretti' target='_blank'><img height='36' style='border:0px;height:36px;' src='https://storage.ko-fi.com/cdn/kofi5.png' border='0' alt='Buy Me a Coffee at ko-fi.com' /></a>

If Cremeplay helps you enjoy music without Chromium overhead, you can support the project on [Ko-fi](https://ko-fi.com/omoretti).

---

## About
<img width="636" height="415" alt="image" src="https://github.com/user-attachments/assets/2bb34b37-802a-4cc1-a666-c67626e7e948" />
<img width="636" height="415" alt="image" src="https://github.com/user-attachments/assets/76e72bde-3df2-4146-92bb-06ca5cdb873a" />
<img width="420" height="415" alt="image" src="https://github.com/user-attachments/assets/185bd902-f50a-4ac0-b210-199386eb3785" />


Most YouTube Music desktop players run on Electron or heavy web wrappers. They consume 400 MB to 800 MB+ of RAM, render unnecessary video decoders and Canvas layers when you just want audio, and accumulate memory leaks over extended playback sessions.

**Cremeplay** is an ultra-lightweight, high-performance desktop music player built with **Tauri v2** and **Rust**. It replaces the Chromium audio pipeline with a native Rust audio engine powered by `rodio` and Symphonia, routing raw audio directly to Windows WASAPI with a minimal private working set of ~40–60 MB RAM.

It interfaces directly with YouTube Music's Innertube API via asynchronous Rust services, providing authentic music discovery, radio recommendations, and library management without Google Polymer overhead or background tracking bloat.

---

## Performance Comparison

| Metric | Pear Desktop (Electron) | Webview Wrappers | **Cremeplay (Tauri v2 + Rust)** |
| :--- | :--- | :--- | :--- |
| **Audio Pipeline** | Chromium Web Audio | Chromium Web Audio | **Native Rust `rodio` (WASAPI)** |
| **API Client** | Google Polymer Web App | Google Polymer Web App | **Direct Rust Innertube JSON API** |
| **Video Decoding** | Yes (Buffers 1080p/720p video in RAM) | Yes (Buffers video stream in RAM) | **Zero (Streams pure raw audio tracks)** |
| **Memory Footprint** | ~400 MB – 800 MB RAM | ~250 MB – 350 MB RAM | **~40 MB – 60 MB RAM** |
| **Startup Speed** | ~3.5s | ~1.5s | **< 400ms** |
| **Binary Size** | ~200 MB+ | ~90 MB | **~30 MB (Self-contained)** |

---

## Features

- **Sub-60 MB Memory Footprint**: Minimal private working set using native Rust audio streaming and a featherweight UI shell.
- **Native Rust Audio Engine**: Raw audio decoded via Symphonia and streamed straight to Windows WASAPI with zero video decoding overhead.
- **Authentic YouTube Music Algorithm**: Complete feed exploration including *Listen Again*, *Daily Discover*, *From the Community* user playlists, and *Similar To* artists. Song selections queue smart "Up Next" tracks following YouTube's live music algorithm.
- **OBS Tuna Integration**: Built-in broadcast server on `http://127.0.0.1:1608/` serving live track metadata and album art for streaming overlays and desktop lyrics tools like [LyricReme](https://github.com/asheohto/lyricreme).
- **Dynamic Album Art Theme**: Extracts vibrant dominant tones directly from the current album cover art to ambiently tint the interface in real time, alongside handcrafted dark themes.
- **Native SponsorBlock**: Automatically skips non-music intros, skits, chatter, and video outros before frames are rendered.
- **Discord Rich Presence**: Displays live song title, artist, album art, elapsed time, and duration directly to your Discord profile.
- **Playlist & Library Power**: Collage covers for custom playlists without artwork, track shuffling, link copying, search filtering, and Google account login support with secure persistent session cookies.
- **System Tray & Media Shortcuts**: Full Windows media keys support (Play/Pause, Next, Previous) and unobtrusive minimize-to-tray background playback.

---

## Getting Started

### 1. Download

1. Go to the [Releases](https://github.com/asheohto/cremeplay/releases) page.
2. Download the latest `cremeplay.exe`.
3. Run `cremeplay.exe` — standalone, zero installer required, works out of the box.

### 2. (Optional) Connect OBS Tuna or LyricReme

Cremeplay automatically runs a lightweight Tuna server on `http://127.0.0.1:1608/`. Open [LyricReme](https://github.com/asheohto/lyricreme) or your OBS Tuna overlay to enjoy synchronized lyrics and live stream widgets with zero setup.

---

## Building from Source

Requires Windows 10/11, [Rust](https://rustup.rs/) (MSVC toolchain), and [Node.js](https://nodejs.org/) (v20+ with pnpm):

```powershell
# Clone the repository
git clone https://github.com/asheohto/cremeplay.git
cd cremeplay

# Install frontend dependencies
pnpm install

# Run live-reloading development build
pnpm dev

# Build standalone production bundle
pnpm build
```

The compiled release binary is output to:
`.\src-tauri\target\release\cremeplay.exe`

---

## Integrations

| Integration | Details | Default Port / State |
| :--- | :--- | :--- |
| **OBS Tuna** | Real-time track metadata and cover art server for OBS and desktop widgets | `http://127.0.0.1:1608/` |
| **Discord RPC** | Shows current track, artist, album art, and live elapsed timer on Discord | Enabled |
| **SponsorBlock** | Skips non-music intros, interludes, and chatter automatically | Enabled |
| **WASAPI Audio** | Low-latency direct sound output via `rodio` | Enabled |

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` | Play / Pause |
| `Media Play / Pause` | Global system Play / Pause |
| `Media Next` | Skip to next track |
| `Media Previous` | Skip to previous track |
| `Esc` | Clear search / Close modal views |

---

## License

MIT - see [LICENSE](LICENSE).

---

## Support

- Issues: https://github.com/asheohto/cremeplay/issues
- Ko-fi: https://ko-fi.com/omoretti

---

## Tags

youtube music desktop, lightweight youtube music, youtube music tauri, rust music player, rodio audio, debloated youtube music, youtube music client, discord rich presence music, obs tuna youtube music, sponsorblock music, desktop music player, windows youtube music, low ram music player, lyricreme companion, tauri v2, wasapi audio
