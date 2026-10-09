# AGENTS.md — Cremeplay Architecture & Knowledge Base

This document transfers architectural context, constraints, low-memory guidelines, and hard-earned debugging lessons to future AI agents working on **Cremeplay**.

---

## 1. Project Mission & North Star: Zero Bloat, Ultra-Low Memory

Cremeplay is an ultra-fast, debloated YouTube Music desktop client built with **Tauri v2** and **Rust**.

### Memory Budget
* **Tray / Background**: **~50–60 MB RAM**
* **Open Window / Active Playback**: **~140–160 MB RAM**
* *Comparison*: Electron wrappers (Pear, YouTube Music Desktop) consume **450–850 MB+ RAM**.

### Rules for Any Future Code Changes
1. **Never introduce heavy frontend frameworks** (No React, Vue, Svelte, or Virtual-DOM). Vanilla TypeScript + Vite compiles to <100 KB JS and eliminates virtual DOM heap overhead.
2. **Never render unbounded DOM lists**. Virtualize or cap queues/search results to visible items.
3. **No JavaScript animation loops** (No GSAP, Framer Motion, or requestAnimationFrame intervals). Rely 100% on GPU-composited CSS properties (`transform`, `opacity`).
4. **Preserve the process trimming job**: Working set memory trimming in `process_job.rs` regularly purges inactive Chromium memory pools.
5. **Video is on-demand only**: Never play or render video streams in the background when the user is in "Song mode" (audio-only).

---

## 2. Architecture Overview

```
cremeplay/
├── frontend/                     # Vanilla TypeScript + Vite frontend
│   ├── index.html                # Single-page UI structure
│   ├── public/dict/              # Kuroshiro / Kuromoji dictionary files for Japanese
│   ├── src/
│   │   ├── main.ts               # Core UI controller, player events, state
│   │   ├── style.css             # Fluid styling, GPU transitions, custom themes
│   │   └── shims/                # Lightweight browser polyfills
│   └── vite.config.ts            # Vite bundler configuration
└── src-tauri/                    # Native Rust Backend (Tauri v2)
    ├── src/
    │   ├── main.rs / lib.rs      # App lifecycle, setup, command registration
    │   ├── commands.rs           # Tauri IPC handlers (`#[tauri::command]`)
    │   ├── innertube.rs          # Pure-Rust YouTube Music API client with SAPISID auth
    │   ├── lrclib.rs             # LRCLIB crowdsourced time-synced lyrics client
    │   ├── audio_player.rs       # Native WASAPI audio engine (rodio / symphonia)
    │   ├── config.rs             # Non-blocking JSON settings manager
    │   ├── process_job.rs        # Windows Job Object & Working Set trimmer
    │   ├── discord.rs            # Discord Rich Presence ("Listening" activity)
    │   ├── tuna.rs               # OBS Tuna local stream overlay server (port 1608)
    │   ├── sponsorblock.rs       # SponsorBlock segment auto-skip client
    │   └── tray.rs               # System tray icon and context menu
    ├── Cargo.toml                # Rust dependencies
    └── tauri.conf.json           # Tauri v2 window and bundle configuration
```

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
* Romanization engine supports Japanese (Kuroshiro/Kuromoji), Korean (Aromanize), Chinese (Pinyin), Cyrillic, Greek, and Indic.
* **Crucial**: English songs and non-foreign lines **must never be romanized**. Always verify `hasSubstantialNonLatin()` before attempting phonetic conversion.

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
