# AGENTS.md — Cremeplay Architecture & Knowledge Base

This document transfers architectural context, constraints, low-memory guidelines, and hard-earned debugging lessons to future AI agents working on **Cremeplay**.

---

## 1. Project Mission & North Star: Zero Bloat, Ultra-Low Memory

Cremeplay is an ultra-fast, debloated YouTube Music desktop client available in two architectures:
1. **Rune Webviewless Architecture** (`crates/`): 100% native Rust GPU UI rendering via Rune (`wgpu`, `taffy`, `parley`) with Rodio/Symphonia audio and zero Chromium/WebView2 overhead (**~25–35 MB RAM** active playback).
2. **Tauri v2 Desktop Architecture** (`src-tauri/` & `frontend/`): Vanilla TypeScript + Vite frontend running under WebView2 with process trimming (**~50 MB tray / ~140 MB open window**).

### Rune Memory & Performance Budget
* **RAM footprint**: **~25–35 MB RAM** (Zero WebView2, zero Chromium processes).
* **Launch latency**: **< 30 ms** first frame.
* **Frame rate**: Continuous 60 FPS GPU-accelerated rendering.

---

## 2. Architecture Overview

### A. Rune Webviewless Architecture (`crates/`)
```
cremeplay/
├── crates/
│   ├── cremeplay-core/           # Pure-Rust backend logic
│   │   ├── src/
│   │   │   ├── innertube.rs      # Innertube API + RustyPipe cipher & n-token resolution
│   │   │   ├── audio_player.rs   # Rodio + Symphonia native WASAPI audio streaming engine
│   │   │   ├── lrclib.rs         # LRCLIB synced lyrics client with bounded caching
│   │   │   ├── discord.rs        # Discord Rich Presence listening activity
│   │   │   ├── tuna.rs           # OBS Tuna local stream overlay server (port 1608)
│   │   │   ├── sponsorblock.rs   # SponsorBlock segment auto-skip client
│   │   │   └── config.rs         # Non-blocking JSON settings manager
│   │   └── Cargo.toml
│   └── cremeplay-app/            # Native Rune GPU Desktop Client
│       ├── src/
│       │   ├── main.rs           # AppSpec setup, 30fps poll ticker, keyboard hooks
│       │   ├── state.rs          # Centralized AppState, navigation, playback state
│       │   ├── theme.rs          # Matcha, dark, and light color tokens & typography
│       │   ├── icons.rs          # Hugeicons vector stroke paths
│       │   ├── agent_server.rs   # Embedded local agent server (127.0.0.1:9099)
│       │   └── components/       # Header, sidebar, player_bar, search, modals
│       └── Cargo.toml
```

### B. Tauri v2 Architecture (`src-tauri/` & `frontend/`)
```
cremeplay/
├── frontend/                     # Vanilla TypeScript + Vite frontend
│   ├── index.html                # Single-page UI structure
│   ├── public/dict/              # Kuroshiro / Kuromoji dictionary files for Japanese
│   ├── src/
│   │   ├── main.ts               # Core UI controller, player events, state
│   │   └── style.css             # Fluid styling, GPU transitions, custom themes
└── src-tauri/                    # Native Rust Backend (Tauri v2)
```

---

## 3. Live Agent Debugging & Testing

### A. Embedded Local Agent Server (`http://127.0.0.1:9099`)
Cremeplay embeds an asynchronous HTTP agent server in `cremeplay-app` powered by `AppHandle<AppState>`:
* `GET /api/ping`: Server health and version check.
* `GET /api/status`: Real-time player status, track metadata, queue length, active views, and modal flags.
* `POST /api/action`: Control actions executed safely on the GUI UI thread (`play`, `pause`, `toggle`, `next`, `prev`, `seek`, `volume`, `search`, `view`, `toggle_drawer`, `toggle_now_playing`, `toggle_settings`).

### B. Headless Rune Harness
Rune supports deterministic CLI testing flags:
* `cremeplay.exe --tree tree.json`: Dump full semantic UI layout and element hierarchy.
* `cremeplay.exe --snap snap.png --at <seconds> [--crop <key>]`: Pixel-perfect GPU snapshot.
* `cremeplay.exe --script script.json`: Automated headless user interaction replay (taps, drags, typing).

---

## 3. Critical Low-Memory Subsystems

### A. Windows Job Object & Working Set Trimming (`src-tauri/src/process_job.rs`)
* **Job Object**: Wraps the main process and all WebView2 child processes (Renderer, GPU, Network) under a single Windows Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. When Cremeplay closes, zero orphan processes linger.
* **Working Set Trim (`trim_process_working_set`)**: Every 20 seconds, the background thread runs `SetProcessWorkingSetSize` (`EmptyWorkingSet`) across all processes in the tree, flushing idle Chromium cache pages out of physical RAM.
* **Session Renaming**: Automatically labels the audio endpoint in Windows Volume Mixer and EarTrumpet as **Cremeplay** with the app icon instead of anonymous `msedgewebview2.exe`.

### B. On-Demand Video Switcher (Low Memory Mode)
* When playing in **Song Mode** (default):
  * The `#yt-player` iframe is hidden behind the cover artwork.
  * YouTube playback is kept at audio/default quality.
* When toggled to **Video Mode**:
  * Cover art fades out, `#yt-player` reveals, and responsive aspect ratio (`16:9`, `4:3`, or `9:16`) takes effect.
  * Quality is promoted to user's configured `videoQuality` (1080p, 720p, etc.).
* **Rule**: Do not decode or render video pipelines unless explicitly switched on by the user.

### C. Bounded Caching
* **Lyrics Cache (`lrclib.rs`)**: Wrapped in a bounded `BoundedCache` with maximum capacity (128 tracks) and FIFO eviction to prevent infinite memory expansion during multi-hour sessions.
* **Search / Queue Payload**: Keep IPC JSON payloads lean. Pass only needed fields across `invoke`.

---

## 4. Key Lessons, Gotchas & Debugging History

### 1. YouTube Embed Error 153 (`PLAYABILITY_ERROR_CODE_EMBEDDER_IDENTITY_MISSING_REFERRER`)
* **Problem**: YouTube embeds in Tauri WebView2 throw error 153 on some videos when loaded from `http://tauri.localhost`.
* **Root Cause**: YouTube's embed security checks require an embedder referrer identity.
* **Fix**: **MUST** retain `<meta name="referrer" content="unsafe-url" />` in `frontend/index.html`. Do NOT remove this tag.

### 2. Direct Stream Extraction vs IFrame Player
* YouTube's `youtubei/v1/player` endpoint for unauthenticated Android/Web clients often responds with `LOGIN_REQUIRED` or requires Proof-of-Origin (`poToken`).
* Cremeplay uses the official YouTube IFrame API (`window.YT.Player`) for resilient, unblocked playback across all tracks while managing custom playback state, volume, and seekbars from Cremeplay's UI.

### 3. Cross-Origin Iframe Security Boundary
* In WebView2, `http://tauri.localhost` and `https://www.youtube.com` are cross-origin.
* Accessing `iframe.contentDocument` or injecting styles inside the YouTube iframe throws a `SecurityError`. Do NOT attempt DOM scraping or CSS injection inside `iframe.contentDocument`.

### 4. Ambient Diffuser & Radial Gradients
* Avoid multi-layer radial gradient diffusers with high blurs (`filter: blur(40px)`) over flex containers without explicit relative positioning. They create CPU/GPU rendering overhead and ugly gradient washes across the window.
* Keep Now Playing backdrops simple with standard CSS backdrop blur on the scaled album art.

### 5. Multi-Language Romanization & English Exemption
* **Native Pure-Rust Romanizer (`crates/cremeplay-core/src/romanize.rs`)**:
  * Zero external dependencies, ultra-low memory.
  * Chinese Hanzi Pinyin binary-searched lookup (`hanzi_table.rs`, 1,411 common entries) with syllabic spacing.
  * Algorithmic decomposition of Korean Hangul syllables (Initial, Medial, Final).
  * Japanese Kana (Hiragana & Katakana) Hepburn mapping with digraphs, sokuon, and prolonged vowels.
  * Cyrillic (Russian, Ukrainian, Slavic) and Modern Greek with tonos accents.
  * In `frontend/`: Uses Kuroshiro/Kuromoji dictionary files for full Kanji disambiguation.
* **Crucial Rule**: English songs and non-foreign lines **must never be romanized**. Always verify `has_foreign_script()` / `hasSubstantialNonLatin()` before attempting phonetic conversion. Lines without non-Latin characters return `None` and are displayed in original form.

### 6. Settings Crash Prevention (Issue #4)
* When updating configuration in `src-tauri/src/commands.rs`, never block on disk I/O or background manager calls. Use async channels and debounced persistence in `ConfigManager`.

---

## 5. Build, Packaging & Release Guide

### Prerequisites
* Rust 1.85+ / 2024 edition
* Node.js 20+ & `pnpm`
* Windows 10/11 with WebView2 runtime

### Common Gotcha: `Access is denied (os error 5)`
If `pnpm build` or `cargo build --release` fails with:
```
error: failed to remove file `...target/release/cremeplay.exe`: Access is denied. (os error 5)
```
An existing instance of Cremeplay is running in the background or system tray.
Kill it before rebuilding:
```powershell
Stop-Process -Name "cremeplay" -Force -ErrorAction SilentlyContinue
```

### Production Build Commands
```powershell
# 1. Build frontend only:
pnpm build:frontend

# 2. Build full release binaries + NSIS installer:
pnpm build
```

### Artifact Outputs
* **NSIS Setup Installer**: `src-tauri/target/release/bundle/nsis/Cremeplay_<version>_x64-setup.exe`
* **Portable Executable**: Copy `src-tauri/target/release/cremeplay.exe` to `Cremeplay_<version>_x64-portable.exe`.

### GitHub Release Workflow
```powershell
# Commit and tag
git add .
git commit -m "feat: release description (closes #X)"
git push origin main
git tag -a vX.X -m "Release vX.X"
git push origin vX.X

# Create release with assets
gh release create vX.X `
  "src-tauri/target/release/bundle/nsis/Cremeplay_<version>_x64-setup.exe" `
  "src-tauri/target/release/Cremeplay_<version>_x64-portable.exe" `
  --title "Cremeplay vX.X" `
  --notes-file "release_notes.md"
```

---

## 6. Checklist for New Features

Before merging or releasing any new feature:
- [ ] **Memory Audit**: Does RAM remain under 60 MB (tray) and under 160 MB (window)?
- [ ] **No Unbounded DOM**: Are all list renders capped or paginated?
- [ ] **GPU-Only CSS Animations**: Are all transitions limited to `transform` and `opacity`?
- [ ] **Referrer Policy**: Is `<meta name="referrer" content="unsafe-url" />` intact?
- [ ] **Settings Non-blocking**: Are all settings writes debounced and non-blocking?
- [ ] **Clean Build**: Does `npm run build` in `frontend/` and `cargo check --manifest-path src-tauri/Cargo.toml` succeed with zero errors?

