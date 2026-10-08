import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createRomanizer } from 'lyric-romanizer';

declare global {
  interface Window {
    onYouTubeIframeAPIReady?: () => void;
    YT?: any;
  }
}

const NON_LATIN_REGEX = /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uac00-\ud7af\u0400-\u04ff\u0600-\u06ff\u0e00-\u0e7f]/;

const lyricRomanizer = createRomanizer({
  japaneseDictPath: '/dict',
  engines: {
    chinese: async (line) => {
      const { pinyin } = await import('pinyin-pro');
      return pinyin(line, { toneType: 'symbol', type: 'string', nonZh: 'consecutive' });
    },
  },
});

function cleanRomanizedText(str: string): string {
  if (!str) return '';
  return str
    .replace(/\s+/g, ' ')
    .replace(/\s+([.,!?:;~、。！？)\]\}])/g, '$1')
    .replace(/([(\[{])\s+/g, '$1')
    .replace(/([.,!?:;])(?=[A-Za-z0-9])/g, '$1 ')
    .trim();
}

function sanitizeLyricText(text: string): string {
  if (!text) return '';
  const trimmed = text.trim();
  if (/^(\[|\()\s*(music|musique|música|♪)\s*(\]|\))$/i.test(trimmed) || trimmed === '♪') {
    return '♪';
  }
  const removed = trimmed
    .replace(/(\[|\()\s*(music|musique|música)\s*(\]|\))/gi, '')
    .replace(/\s{2,}/g, ' ')
    .trim();
  return removed || '♪';
}

const romanizedLyricsCache = new Map<string, { synced?: (string | null)[]; plain?: string }>();


interface TrackItem {
  videoId: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  durationText: string;
  artworkUrl: string;
}

interface HomeSection {
  title: string;
  items: TrackItem[];
}

interface AccountProfile {
  name: string;
  avatarUrl: string;
  isLoggedIn: boolean;
}

interface PlaylistDetail {
  browseId: string;
  title: string;
  description: string;
  subtitle: string;
  artworkUrl: string;
  trackCount: number;
  tracks: TrackItem[];
}

interface SearchTopResult {
  title: string;
  subtitle: string;
  videoId?: string | null;
  browseId?: string | null;
  artworkUrl: string;
  resultType: string;
}

interface SearchResultPayload {
  topResult?: SearchTopResult | null;
  songs: TrackItem[];
  libraryTracks: TrackItem[];
}

interface SidebarPlaylist {
  title: string;
  browseId: string;
}

interface PlayerStatus {
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  track: TrackItem | null;
}

interface LyricLine {
  timeSec: number;
  text: string;
}

interface LyricsPayload {
  synced: boolean;
  source: string;
  instrumental: boolean;
  lines: LyricLine[];
  plainLyrics: string;
}

interface AppConfig {
  adblock: boolean;
  sponsorblock: boolean;
  discordRpc: boolean;
  tunaObs: boolean;
  mediaControls: boolean;
  closeToTray: boolean;
  autoplay?: boolean;
  volume?: number;
  animations?: boolean;
  romanizeLyrics?: boolean;
  romanizeMode?: 'off' | 'subtext' | 'main' | 'only';
  enableVideoSwitcher?: boolean;
  videoQuality?: string;
  authCookies?: string | null;
  userName?: string | null;
  userAvatar?: string | null;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const root = document.documentElement;
const view = $('view-container'), searchInput = $<HTMLInputElement>('search-input'), searchClearBtn = $('search-clear-btn'), searchSuggestions = $('search-suggestions');
const iconPlay = $('icon-play'), iconPause = $('icon-pause');
const thumb = $<HTMLImageElement>('player-thumb'), pTitle = $('player-title'), pArtist = $('player-artist');
const tCur = $('time-current'), tDur = $('time-duration'), fill = $('progress-bar'), seek = $('seek-container');
const vol = $<HTMLInputElement>('volume-slider'), queueDrawer = $('queue-drawer'), queueList = $('queue-list');
const navs = [$('nav-home'), $('nav-explore'), $('nav-liked')];
const sidePlaylistsList = $('side-playlists-list');
const sidePinnedLiked = $('side-pinned-liked');
const btnNewPlaylist = $('btn-new-playlist');

// Now Playing Overlay elements
const npOverlay = $('now-playing-overlay');
const npBackdrop = $('np-backdrop');
const npArt = $<HTMLImageElement>('np-artwork');
const npTitle = $('np-title'), npArtist = $('np-artist');
const npProgress = $('np-progress'), npSeek = $('np-seek');
const npTCur = $('np-time-current'), npTDur = $('np-time-duration');
const npBtnPlay = $('np-btn-play'), npIconPlay = $('np-icon-play'), npIconPause = $('np-icon-pause');
const npBtnPrev = $('np-btn-prev'), npBtnNext = $('np-btn-next');
const npBtnLike = $('np-btn-like'), npBtnQueue = $('np-btn-queue');
const btnCollapse = $('btn-collapse-player');

// Cover Video Switcher elements
const npModeSwitcherContainer = $('np-mode-switcher-container');
const npModeSong = $('np-mode-song');
const npModeVideo = $('np-mode-video');
const npCoverContainer = document.querySelector('.np-cover-container') as HTMLElement | null;
let isVideoMode = false;

// Tabs & Views
const npTabCoverBtn = $('np-tab-cover-btn'), npTabQueueBtn = $('np-tab-queue-btn'), npTabLyricsBtn = $('np-tab-lyrics-btn');
const npViewCover = $('np-view-cover'), npViewQueue = $('np-view-queue'), npViewLyrics = $('np-view-lyrics');
const npQueueList = $('np-queue-list'), npLyricsText = $('np-lyrics-text');

const tabDrawerQueue = $('tab-drawer-queue'), tabDrawerLyrics = $('tab-drawer-lyrics');
const drawerLyricsContainer = $('drawer-lyrics-container'), drawerLyricsText = $('drawer-lyrics-text');

// Controls
const btnShuffle = $('btn-shuffle'), npBtnShuffle = $('np-btn-shuffle');
const btnRepeat = $('btn-repeat'), npBtnRepeat = $('np-btn-repeat');
const repeatBadge = $('repeat-badge'), npRepeatBadge = $('np-repeat-badge');

// Context Menu
const contextMenu = $('track-context-menu');
const menuStartRadio = $('menu-start-radio');
const menuPlayNext = $('menu-play-next');
const menuAddQueue = $('menu-add-queue');
const menuCopyLink = $('menu-copy-link');

let current: TrackItem | null = null;
let home: HomeSection[] = [];
let queue: TrackItem[] = [];
let originalQueue: TrackItem[] = [];
let qIndex = -1;
let duration = 0;
let lastVol = 1;
let isShuffled = false;
let repeatMode: 'off' | 'all' | 'one' = 'off';
let activeOverlayTab: 'cover' | 'queue' | 'lyrics' = 'cover';
let activeDrawerTab: 'queue' | 'lyrics' = 'queue';
let contextTrack: TrackItem | null = null;
let userPlaylists: SidebarPlaylist[] = [];
let currentSearchPayload: SearchResultPayload | null = null;
let activeSearchTab: 'catalog' | 'library' = 'catalog';

let ytPlayer: any = null;
let ytReady = false;
let pendingTrack: TrackItem | null = null;
let pendingResumeSec = 0;
let lastPersistSec = 0;
let searchDebounceTimer: any = null;
let selectedSuggestionIndex = -1;
let searchSuggestionsList: string[] = [];
let lastTunaUpdateSec = 0;
let currentLyricsPayload: LyricsPayload | null = null;
let currentLyricsTrackId = '';
let activeLyricLineIdx = -1;
let userLyricsScrollPauseUntil = 0;

let appSettings: AppConfig = {
  adblock: true,
  sponsorblock: true,
  discordRpc: true,
  tunaObs: true,
  mediaControls: true,
  closeToTray: true,
  autoplay: false,
  volume: 1.0,
  animations: true,
  romanizeLyrics: true,
  romanizeMode: 'subtext',
  enableVideoSwitcher: false,
  videoQuality: 'auto',
};

function updateVideoSwitcherVisibility() {
  if (npModeSwitcherContainer) {
    npModeSwitcherContainer.classList.toggle('hidden', !appSettings.enableVideoSwitcher);
  }
  if (!appSettings.enableVideoSwitcher && isVideoMode) {
    setVideoMode(false);
  }
}

function detectVideoAspectRatio(): number {
  try {
    const iframe = document.getElementById('yt-player') as HTMLIFrameElement;
    if (iframe && iframe.contentDocument) {
      const vid = iframe.contentDocument.querySelector('video') as HTMLVideoElement;
      if (vid && vid.videoWidth && vid.videoHeight && vid.videoWidth > 0 && vid.videoHeight > 0) {
        return vid.videoWidth / vid.videoHeight;
      }
    }
  } catch {}

  // Check if current track is a vertical Short by title
  if (current && (current.title.toLowerCase().includes('#shorts') || current.title.toLowerCase().includes('/shorts'))) {
    return 9 / 16;
  }

  return 16 / 9;
}

function updateCoverContainerLayout() {
  const container = (npCoverContainer || document.getElementById('np-cover-container')) as HTMLElement | null;
  if (!container) return;

  if (!isVideoMode) {
    container.classList.remove('video-active');
    container.style.aspectRatio = '1 / 1';
    container.style.width = 'min(340px, 50vw)';
    container.style.maxWidth = 'min(340px, 50vw)';
    container.style.height = 'min(340px, 50vw)';
    return;
  }

  container.classList.add('video-active');
  const ratio = detectVideoAspectRatio();
  container.style.aspectRatio = `${ratio}`;

  if (ratio >= 1.5) {
    // 16:9 or 21:9 Widescreen
    container.style.width = 'min(480px, 92vw)';
    container.style.maxWidth = 'min(480px, 92vw)';
    container.style.height = 'auto';
  } else if (ratio >= 1.1) {
    // 4:3 Standard
    container.style.width = 'min(400px, 80vw)';
    container.style.maxWidth = 'min(400px, 80vw)';
    container.style.height = 'auto';
  } else if (ratio <= 0.8) {
    // 9:16 Vertical
    container.style.width = 'min(240px, 46vw)';
    container.style.maxWidth = 'min(240px, 46vw)';
    container.style.height = 'auto';
  } else {
    // Near 1:1
    container.style.width = 'min(340px, 50vw)';
    container.style.maxWidth = 'min(340px, 50vw)';
    container.style.height = 'auto';
  }
}

function setVideoMode(video: boolean) {
  isVideoMode = video;
  if (npModeSong) npModeSong.classList.toggle('active', !video);
  if (npModeVideo) npModeVideo.classList.toggle('active', video);

  updateCoverContainerLayout();

  if (ytPlayer && ytReady) {
    try {
      if (video) {
        const targetQ = appSettings.videoQuality || 'auto';
        if (ytPlayer.setPlaybackQuality) ytPlayer.setPlaybackQuality(targetQ);
      } else {
        if (ytPlayer.setPlaybackQuality) ytPlayer.setPlaybackQuality('default');
      }
    } catch {}
  }
}

let saveSettingsDebounceTimer: any = null;

async function loadSettings() {
  try {
    const cfg = await invoke<AppConfig>('get_settings');
    if (cfg) {
      appSettings = cfg;
      const setAutoplay = $<HTMLInputElement>('set-autoplay');
      const setAnimations = $<HTMLInputElement>('set-animations');
      const setVideoSwitcher = $<HTMLInputElement>('set-video-switcher');
      const setVideoQuality = $<HTMLSelectElement>('set-video-quality');
      const setRomanizeMode = $<HTMLSelectElement>('set-romanize-mode');
      const setDiscord = $<HTMLInputElement>('set-discord');
      const setSponsor = $<HTMLInputElement>('set-sponsor');
      const setTuna = $<HTMLInputElement>('set-tuna');
      const setTray = $<HTMLInputElement>('set-tray');
      if (setAutoplay) setAutoplay.checked = !!cfg.autoplay;
      if (setAnimations) setAnimations.checked = cfg.animations !== false;
      if (setVideoSwitcher) setVideoSwitcher.checked = !!cfg.enableVideoSwitcher;
      if (setVideoQuality) setVideoQuality.value = cfg.videoQuality || 'auto';
      appSettings.videoQuality = cfg.videoQuality || 'auto';

      const effectiveMode: 'off' | 'subtext' | 'main' | 'only' =
        cfg.romanizeMode ? (cfg.romanizeMode as any) : (cfg.romanizeLyrics === false ? 'off' : 'subtext');
      if (setRomanizeMode) setRomanizeMode.value = effectiveMode;
      appSettings.romanizeMode = effectiveMode;
      appSettings.romanizeLyrics = effectiveMode !== 'off';
      updateVideoSwitcherVisibility();

      if (setDiscord) setDiscord.checked = !!cfg.discordRpc;
      if (setSponsor) setSponsor.checked = !!cfg.sponsorblock;
      if (setTuna) setTuna.checked = !!cfg.tunaObs;
      if (setTray) setTray.checked = !!cfg.closeToTray;

      root.classList.toggle('reduce-motion', cfg.animations === false);

      const savedVolStr = localStorage.getItem('cremeplay_volume');
      const effectiveVol = savedVolStr !== null ? parseFloat(savedVolStr) : (typeof cfg.volume === 'number' ? cfg.volume : 1.0);
      if (vol) vol.value = String(effectiveVol);
      lastVol = effectiveVol > 0 ? effectiveVol : 1;
      invoke('set_volume', { volume: effectiveVol }).catch(() => {});

      if (cfg.romanizeLyrics !== false && cfg.romanizeMode !== 'off') {
        lyricRomanizer.warmup(['japanese', 'chinese', 'korean']).catch(() => {});
      }
    }
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function saveSettings() {
  const setAutoplay = $<HTMLInputElement>('set-autoplay');
  const setAnimations = $<HTMLInputElement>('set-animations');
  const setVideoSwitcher = $<HTMLInputElement>('set-video-switcher');
  const setVideoQuality = $<HTMLSelectElement>('set-video-quality');
  const setRomanizeMode = $<HTMLSelectElement>('set-romanize-mode');
  const setDiscord = $<HTMLInputElement>('set-discord');
  const setSponsor = $<HTMLInputElement>('set-sponsor');
  const setTuna = $<HTMLInputElement>('set-tuna');
  const setTray = $<HTMLInputElement>('set-tray');

  if (setAutoplay) appSettings.autoplay = setAutoplay.checked;
  if (setAnimations) {
    appSettings.animations = setAnimations.checked;
    root.classList.toggle('reduce-motion', !setAnimations.checked);
  }
  if (setVideoSwitcher) {
    appSettings.enableVideoSwitcher = setVideoSwitcher.checked;
    updateVideoSwitcherVisibility();
  }
  if (setVideoQuality) {
    appSettings.videoQuality = setVideoQuality.value;
    if (isVideoMode && ytPlayer && ytReady && ytPlayer.setPlaybackQuality) {
      try {
        ytPlayer.setPlaybackQuality(setVideoQuality.value);
      } catch {}
    }
  }
  if (setRomanizeMode) {
    const prevMode = appSettings.romanizeMode;
    const newMode = setRomanizeMode.value as 'off' | 'subtext' | 'main' | 'only';
    appSettings.romanizeMode = newMode;
    appSettings.romanizeLyrics = newMode !== 'off';
    if (prevMode !== newMode && currentLyricsPayload) {
      renderLyrics(currentLyricsPayload);
      updateSyncedLyricsProgress(ytPlayer?.getCurrentTime?.() || 0, true);
    }
  }
  if (setDiscord) appSettings.discordRpc = setDiscord.checked;
  if (setSponsor) appSettings.sponsorblock = setSponsor.checked;
  if (setTuna) appSettings.tunaObs = setTuna.checked;
  if (setTray) appSettings.closeToTray = setTray.checked;

  if (saveSettingsDebounceTimer) {
    clearTimeout(saveSettingsDebounceTimer);
  }
  saveSettingsDebounceTimer = setTimeout(async () => {
    try {
      await invoke('update_settings', { settings: appSettings });
    } catch (e) {
      console.error('Failed to update settings:', e);
    }
  }, 100);
}

function syncTunaProgress(force = false) {
  if (!current) {
    invoke('on_playback_state_change', { isPlaying: false, currentTime: 0 }).catch(() => {});
    return;
  }
  const cur = ytPlayer && ytReady && ytPlayer.getCurrentTime ? ytPlayer.getCurrentTime() : 0;
  const dur = duration || (ytPlayer && ytReady && ytPlayer.getDuration ? ytPlayer.getDuration() : 0);
  const isPlaying = ytPlayer && ytReady && ytPlayer.getPlayerState ? ytPlayer.getPlayerState() === 1 : false;

  if (force || Math.abs(cur - lastTunaUpdateSec) >= 1.5) {
    lastTunaUpdateSec = cur;
    invoke('update_playback_progress', {
      title: current.title,
      artist: current.artist,
      album: current.album || '',
      artworkUrl: current.artworkUrl,
      videoId: current.videoId,
      currentTime: cur,
      duration: dur,
      isPlaying,
    }).catch(() => {});
  }
}

interface PersistedPlaybackState {
  current: TrackItem | null;
  queue: TrackItem[];
  originalQueue: TrackItem[];
  qIndex: number;
  currentTime: number;
  isShuffled: boolean;
  repeatMode: 'off' | 'all' | 'one';
}

function savePlaybackState(currentTime = 0) {
  if (!queue || !queue.length) return;
  try {
    const state: PersistedPlaybackState = {
      current,
      queue,
      originalQueue: originalQueue.length ? originalQueue : queue,
      qIndex: Math.max(0, qIndex),
      currentTime: Math.floor(currentTime),
      isShuffled,
      repeatMode,
    };
    localStorage.setItem('cremeplay_resumed_queue', JSON.stringify(state));
  } catch (err) {
    console.warn('Failed to save resumed queue:', err);
  }
}

function restorePlaybackState(): boolean {
  try {
    const raw = localStorage.getItem('cremeplay_resumed_queue');
    if (!raw) return false;
    const state: PersistedPlaybackState = JSON.parse(raw);
    if (!state || !state.queue || !state.queue.length) return false;

    queue = state.queue;
    originalQueue = state.originalQueue?.length ? state.originalQueue : [...state.queue];
    qIndex = state.qIndex >= 0 && state.qIndex < queue.length ? state.qIndex : 0;
    current = state.current || queue[qIndex];
    isShuffled = !!state.isShuffled;
    repeatMode = state.repeatMode || 'off';

    btnShuffle?.classList.toggle('active', isShuffled);
    npBtnShuffle?.classList.toggle('active', isShuffled);
    btnRepeat?.classList.toggle('active', repeatMode !== 'off');
    npBtnRepeat?.classList.toggle('active', repeatMode !== 'off');
    repeatBadge?.classList.toggle('hidden', repeatMode !== 'one');
    npRepeatBadge?.classList.toggle('hidden', repeatMode !== 'one');

    if (current) {
      showNow(current);
      fetchLyrics(current);
      if (state.currentTime > 0) {
        tCur.textContent = fmt(state.currentTime);
        if (npTCur) npTCur.textContent = fmt(state.currentTime);
        if (current.duration > 0) {
          const pct = `${Math.min(100, (state.currentTime / current.duration) * 100)}%`;
          fill.style.width = pct;
          if (npProgress) npProgress.style.width = pct;
        }
      }
      pendingTrack = current;
      pendingResumeSec = state.currentTime || 0;
    }

    renderQueue();
    renderOverlayQueue();
    return true;
  } catch (err) {
    console.warn('Failed to restore previous queue:', err);
    return false;
  }
}

const PLACEHOLDER = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Crect width='160' height='160' fill='%231a1a1a'/%3E%3C/svg%3E";
const PLAY_SVG = '<svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor"><polygon points="6 4 20 12 6 20 6 4"/></svg>';

const esc = (s: unknown) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const art = (u: string) => (u && /^https?:\/\//.test(u) ? esc(u) : PLACEHOLDER);
const fmt = (s: number) => (!s || isNaN(s) || s < 0 ? '0:00' : `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`);
const msg = (t: string) => { view.innerHTML = `<div class="msg">${esc(t)}</div>`; };
const setActive = (el: HTMLElement | null) => navs.forEach((n) => n.classList.toggle('active', n === el));

function initYTPlayer() {
  // Listen for YouTube iframe internal postMessages for adState tracking
  window.addEventListener('message', (e) => {
    try {
      if (typeof e.data === 'string') {
        const parsed = JSON.parse(e.data);
        if (parsed.event === 'infoDelivery' && parsed.info) {
          // If YouTube signals an ad is active (adState > 0), force skip to end of ad segment
          if (parsed.info.adState && parsed.info.adState > 0) {
            if (ytPlayer && ytReady && ytPlayer.getCurrentTime && ytPlayer.getDuration) {
              const dur = ytPlayer.getDuration();
              if (dur > 0) ytPlayer.seekTo(dur, true);
            }
          }
        }
      }
    } catch { }
  });

  // Zero-overhead DOM sentinel: auto-skips and mutes interstitial ads if present
  setInterval(() => {
    try {
      const iframe = document.getElementById('yt-player') as HTMLIFrameElement;
      if (!iframe || !iframe.contentDocument) return;
      const doc = iframe.contentDocument;

      // Ensure iframe retains unsafe-url referrer policy
      if (iframe.getAttribute('referrerpolicy') !== 'unsafe-url') {
        iframe.setAttribute('referrerpolicy', 'unsafe-url');
      }

      // Inject persistent CSS to strip all YouTube watermarks, titles, share buttons, play/pause bezel, and suggestions
      if (!doc.getElementById('cremeplay-clean-embed')) {
        const cleanStyle = doc.createElement('style');
        cleanStyle.id = 'cremeplay-clean-embed';
        cleanStyle.textContent = `
          .ytp-chrome-top,
          .ytp-title,
          .ytp-title-text,
          .ytp-title-link,
          .ytp-title-channel,
          .ytp-watermark,
          .ytp-youtube-button,
          a.ytp-watermark,
          .ytp-share-button,
          .ytp-button-share,
          .ytp-share-panel,
          .ytp-bezel,
          .ytp-bezel-text,
          .ytp-bezel-text-wrapper,
          .ytp-bezel-icon,
          .ytp-large-play-button,
          .ytp-large-play-button-bg,
          .ytp-pause-overlay,
          .ytp-pause-overlay-container,
          .ytp-suggestions,
          .ytp-ce-element,
          .ytp-ce-covering-overlay,
          .ytp-ce-expanding-overlay,
          .ytp-cards-teaser,
          .ytp-show-cards-title,
          .ytp-contextmenu,
          .ytp-gradient-top,
          .ytp-gradient-bottom {
            display: none !important;
            opacity: 0 !important;
            visibility: hidden !important;
            pointer-events: none !important;
          }
        `;
        doc.head ? doc.head.appendChild(cleanStyle) : doc.body.appendChild(cleanStyle);
      }

      // Proactively remove any watermark, title or suggestions elements from the DOM
      const watermarks = doc.querySelectorAll('.ytp-watermark, .ytp-youtube-button, .ytp-chrome-top, .ytp-bezel, .ytp-pause-overlay, .ytp-share-button, .ytp-large-play-button');
      watermarks.forEach((el) => el.remove());

      // Cosmetic cleanup: remove banner and overlay elements
      const adOverlays = doc.querySelectorAll('.video-ads, .ytp-ad-module, .ytp-ad-overlay-container, .ytp-ad-image-overlay, .ytp-ad-text-overlay');
      adOverlays.forEach((el) => el.remove());

      // Auto-click any skip ad button immediately
      const skipBtn = doc.querySelector('.ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-skip-ad-button, .ytp-ad-preview-container') as HTMLElement;
      if (skipBtn) skipBtn.click();

      // Fast-forward video ad if in ad state
      const adShowing = doc.querySelector('.ad-showing, .ad-interrupting');
      const vid = doc.querySelector('video') as HTMLVideoElement;
      if (adShowing && vid) {
        vid.muted = true;
        vid.playbackRate = 16.0;
        if (isFinite(vid.duration) && vid.duration > 0) {
          vid.currentTime = vid.duration;
        }
      }
    } catch { }
  }, 250);

  window.onYouTubeIframeAPIReady = () => {
    try {
      ytPlayer = new window.YT.Player('yt-player', {
        height: '100%',
        width: '100%',
        playerVars: {
          autoplay: 1,
          controls: 0,
          disablekb: 1,
          fs: 0,
          playsinline: 1,
          rel: 0,
          iv_load_policy: 3,
          modestbranding: 1,
        },
        events: {
          onReady: () => {
            ytReady = true;
            // Restore volume on YT player
            const savedVolStr = localStorage.getItem('cremeplay_volume');
            const effectiveVol = savedVolStr !== null ? parseFloat(savedVolStr) : (typeof appSettings.volume === 'number' ? appSettings.volume : 1.0);
            if (vol) vol.value = String(effectiveVol);
            try {
              if (ytPlayer && ytPlayer.setVolume) {
                ytPlayer.setVolume(Math.round(effectiveVol * 100));
              }
            } catch {}

            if (pendingTrack) {
              const t = pendingTrack;
              const startSec = pendingResumeSec;
              pendingTrack = null;
              pendingResumeSec = 0;
              if (appSettings.autoplay) {
                play(t, queue.length ? queue : [t], qIndex >= 0 ? qIndex : 0, false, startSec);
              } else {
                cue(t, queue.length ? queue : [t], qIndex >= 0 ? qIndex : 0, startSec);
              }
            }
          },
          onStateChange: (event: any) => {
            if (event.data === 1) {
              setPlay(true);
              fallbackAttempts = 0;
              if (isVideoMode) {
                updateCoverContainerLayout();
              }
            } else if (event.data === 2) {
              setPlay(false);
            } else if (event.data === 0) {
              handleTrackEnd();
            }
          },
          onError: async (e: any) => {
            console.warn('[Cremeplay Player] YouTube error code:', e.data);
            if (current) {
              await handlePlaybackError(current, e.data);
            }
          },
        },
      });
    } catch (err) {
      console.error('[Cremeplay Player] Failed to init YT Player:', err);
    }
  };

  const tag = document.createElement('script');
  tag.src = 'https://www.youtube.com/iframe_api';
  document.head.appendChild(tag);
}

const failedVideoIds = new Set<string>();
let fallbackAttempts = 0;

async function handlePlaybackError(track: TrackItem, code: number) {
  console.warn(`[Cremeplay Player] Playback error code ${code} on: ${track.title} (${track.videoId})`);

  if (failedVideoIds.has(track.videoId)) {
    fallbackAttempts = 0;
    showToast(`"${track.title}" is unavailable. Skipping...`);
    if (qIndex + 1 < queue.length || repeatMode === 'all') {
      step(1);
    } else {
      setPlay(false);
    }
    return;
  }

  failedVideoIds.add(track.videoId);

  // If this specific upload has embedding disabled (101, 150, 152, 153), attempt auto-resolving alternate upload
  if (fallbackAttempts < 2) {
    fallbackAttempts++;
    try {
      showToast(`Playback restricted for this upload. Searching alternate stream...`);
      const searchRes = await invoke<SearchResultPayload>('search_music', { query: `${track.title} ${track.artist}` });
      const candidates = [...(searchRes.songs || []), ...(searchRes.libraryTracks || [])];
      const alt = candidates.find((c) => c.videoId && c.videoId !== track.videoId && !failedVideoIds.has(c.videoId));

      if (alt) {
        console.log(`[Cremeplay Player] Switching to alternative upload: ${alt.title} (${alt.videoId})`);
        const updatedTrack: TrackItem = {
          ...track,
          videoId: alt.videoId,
        };
        if (queue[qIndex]) {
          queue[qIndex] = updatedTrack;
        }
        current = updatedTrack;
        if (ytPlayer && ytReady && ytPlayer.loadVideoById) {
          ytPlayer.loadVideoById({
            videoId: alt.videoId,
            startSeconds: 0,
            suggestedQuality: isVideoMode ? 'auto' : 'default',
          });
          ytPlayer.playVideo();
          return;
        }
      }
    } catch (err) {
      console.warn('Fallback search failed:', err);
    }
  }

  fallbackAttempts = 0;
  showToast(`"${track.title}" is unavailable for playback. Skipping...`);
  if (qIndex + 1 < queue.length || repeatMode === 'all') {
    step(1);
  } else {
    setPlay(false);
  }
}

function handleTrackEnd() {
  if (repeatMode === 'one' && current) {
    if (ytPlayer && ytReady && ytPlayer.seekTo) {
      ytPlayer.seekTo(0);
      ytPlayer.playVideo();
      setPlay(true);
      return;
    }
  }

  if (qIndex + 1 < queue.length) {
    step(1);
  } else if (repeatMode === 'all' && queue.length > 0) {
    play(queue[0], queue, 0, true);
  } else if (current) {
    // Endless radio: auto-fetch more related songs when queue is exhausted!
    invoke<TrackItem[]>('get_radio_queue', { videoId: current.videoId })
      .then((more) => {
        if (more && more.length > 1) {
          queue = more;
          originalQueue = [...more];
          qIndex = 0;
          play(queue[1] || queue[0], queue, queue.length > 1 ? 1 : 0, true);
        } else {
          setPlay(false);
        }
      })
      .catch(() => setPlay(false));
  } else {
    setPlay(false);
  }
}

async function init() {
  restorePlaybackState();
  initYTPlayer();
  bind();
  loadSettings();
  listen<PlayerStatus>('player_status', (e) => onStatus(e.payload));
  listen('auth_changed', async () => {
    await refreshAuth();
    await loadSidebarPlaylists();
    await loadHome();
  });

  // Smooth continuous playback progress ticker
  setInterval(() => {
    if (ytPlayer && ytReady && ytPlayer.getPlayerState && ytPlayer.getPlayerState() === 1) {
      const cur = ytPlayer.getCurrentTime() || 0;
      const dur = ytPlayer.getDuration() || duration;
      if (dur > 0) {
        duration = dur;
        const curFormatted = fmt(cur);
        const durFormatted = fmt(dur);
        const pct = `${Math.min(100, (cur / dur) * 100)}%`;

        tCur.textContent = curFormatted;
        tDur.textContent = durFormatted;
        fill.style.width = pct;

        if (npTCur) npTCur.textContent = curFormatted;
        if (npTDur) npTDur.textContent = durFormatted;
        if (npProgress) npProgress.style.width = pct;

        if (Math.abs(cur - lastPersistSec) >= 4) {
          lastPersistSec = cur;
          savePlaybackState(cur);
        }
      }
      syncTunaProgress();
      updateSyncedLyricsProgress(cur);
    }
  }, 250);

  setupLyricsScrollInteractions();

  window.addEventListener('beforeunload', () => {
    savePlaybackState(ytPlayer?.getCurrentTime?.() || 0);
  });

  await refreshAuth();
  await loadSidebarPlaylists();
  await loadHome();
}

function bind() {
  let logoRotation = 0;
  const logoBtn = $('logo-btn');
  const logoImg = document.querySelector<HTMLImageElement>('.logo-img');

  const spinLogo = () => {
    logoRotation += 360;
    if (logoImg) {
      logoImg.style.transform = `rotate(${logoRotation}deg)`;
    }
  };

  const goHome = () => {
    spinLogo();
    setActive(navs[0]);
    document.querySelectorAll('.side-playlist-item').forEach((el) => el.classList.remove('active'));
    setChip(0);
    renderHome(home);
  };
  if (logoBtn) logoBtn.onclick = goHome;
  navs[0].onclick = goHome;
  navs[1].onclick = loadExplore;
  navs[2].onclick = loadLibrary;

  if (sidePinnedLiked) {
    sidePinnedLiked.onclick = () => {
      document.querySelectorAll('.side-playlist-item').forEach((el) => el.classList.remove('active'));
      sidePinnedLiked.classList.add('active');
      loadLibrary();
    };
  }

  if (btnNewPlaylist) {
    btnNewPlaylist.onclick = () => {
      alert('Create playlist is coming soon. Use YouTube Music to manage new playlists.');
    };
  }

  const chips = [...document.querySelectorAll<HTMLElement>('.chip')];
  chips.forEach((c, i) => (c.onclick = () => {
    setChip(i);
    setActive(navs[0]);
    document.querySelectorAll('.side-playlist-item').forEach((el) => el.classList.remove('active'));
    i === 0 ? renderHome(home) : search(`${c.textContent} music`);
  }));

  // Search input events & Autocomplete suggestions
  searchInput.addEventListener('input', () => {
    const q = searchInput.value.trim();
    if (searchClearBtn) searchClearBtn.classList.toggle('hidden', !q);
    clearTimeout(searchDebounceTimer);
    if (!q) {
      hideSuggestions();
      return;
    }
    searchDebounceTimer = setTimeout(() => fetchSuggestions(q), 180);
  });

  searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      moveSuggestion(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      moveSuggestion(-1);
    } else if (e.key === 'Enter') {
      if (selectedSuggestionIndex >= 0 && searchSuggestionsList[selectedSuggestionIndex]) {
        const selected = searchSuggestionsList[selectedSuggestionIndex];
        searchInput.value = selected;
        hideSuggestions();
        search(selected);
      } else {
        const q = searchInput.value.trim();
        if (q) {
          hideSuggestions();
          search(q);
        }
      }
    } else if (e.key === 'Escape') {
      hideSuggestions();
    }
  });

  if (searchClearBtn) {
    searchClearBtn.onclick = () => {
      searchInput.value = '';
      searchClearBtn.classList.add('hidden');
      hideSuggestions();
      searchInput.focus();
    };
  }

  // Hide suggestions on outside click
  document.addEventListener('click', (e) => {
    if (!searchInput.contains(e.target as Node) && !searchSuggestions?.contains(e.target as Node)) {
      hideSuggestions();
    }
    if (contextMenu && !contextMenu.contains(e.target as Node)) {
      hideContextMenu();
    }
  });

  // Playback toggling
  const togglePlay = () => {
    if (!ytPlayer || !ytReady) return;
    try {
      const state = ytPlayer.getPlayerState ? ytPlayer.getPlayerState() : -1;
      if (state === 1) {
        ytPlayer.pauseVideo();
        setPlay(false);
      } else {
        ytPlayer.playVideo();
        setPlay(true);
      }
    } catch (err) {
      console.error(err);
    }
  };

  $('btn-play').onclick = togglePlay;
  if (npBtnPlay) npBtnPlay.onclick = togglePlay;

  $('btn-next').onclick = () => step(1);
  $('btn-prev').onclick = () => step(-1);
  if (npBtnNext) npBtnNext.onclick = () => step(1);
  if (npBtnPrev) npBtnPrev.onclick = () => step(-1);

  // Shuffle & Repeat handlers
  const onToggleShuffle = () => toggleShuffle();
  if (btnShuffle) btnShuffle.onclick = onToggleShuffle;
  if (npBtnShuffle) npBtnShuffle.onclick = onToggleShuffle;

  const onCycleRepeat = () => cycleRepeat();
  if (btnRepeat) btnRepeat.onclick = onCycleRepeat;
  if (npBtnRepeat) npBtnRepeat.onclick = onCycleRepeat;

  // Bottom bar seek
  seek.onclick = (e) => handleSeek(e, seek);
  if (npSeek) npSeek.onclick = (e) => handleSeek(e, npSeek);

  vol.oninput = () => {
    const v = parseFloat(vol.value);
    if (v > 0) lastVol = v;
    try { localStorage.setItem('cremeplay_volume', String(v)); } catch {}
    appSettings.volume = v;
    if (ytPlayer && ytReady && ytPlayer.setVolume) {
      ytPlayer.setVolume(Math.round(v * 100));
    }
    invoke('set_volume', { volume: v }).catch(() => { });
  };

  $('btn-mute').onclick = () => {
    const cur = parseFloat(vol.value);
    const next = cur > 0 ? 0 : lastVol;
    vol.value = String(next);
    try { localStorage.setItem('cremeplay_volume', String(next)); } catch {}
    appSettings.volume = next;
    if (ytPlayer && ytReady && ytPlayer.setVolume) {
      ytPlayer.setVolume(Math.round(next * 100));
    }
    invoke('set_volume', { volume: next }).catch(() => { });
  };

  // Like buttons
  const toggleLike = () => {
    $('btn-like').classList.toggle('liked');
    if (npBtnLike) npBtnLike.classList.toggle('liked');
  };
  $('btn-like').onclick = toggleLike;
  if (npBtnLike) npBtnLike.onclick = toggleLike;

  // Queue drawer toggle
  $('btn-queue').onclick = () => queueDrawer.classList.toggle('open');
  if (npBtnQueue) npBtnQueue.onclick = () => queueDrawer.classList.toggle('open');
  $('close-queue').onclick = () => queueDrawer.classList.remove('open');

  // Drawer tabs (Queue / Lyrics)
  if (tabDrawerQueue) tabDrawerQueue.onclick = () => setDrawerTab('queue');
  if (tabDrawerLyrics) tabDrawerLyrics.onclick = () => setDrawerTab('lyrics');

  // Now Playing Overlay tabs
  if (npTabCoverBtn) npTabCoverBtn.onclick = () => setOverlayTab('cover');
  if (npTabQueueBtn) npTabQueueBtn.onclick = () => setOverlayTab('queue');
  if (npTabLyricsBtn) npTabLyricsBtn.onclick = () => setOverlayTab('lyrics');

  // Open Full Album Cover overlay
  const openNowPlaying = () => {
    if (npOverlay) npOverlay.classList.remove('hidden');
    renderOverlayQueue();
  };
  const closeNowPlaying = () => {
    if (npOverlay) npOverlay.classList.add('hidden');
  };

  const barMid = $('bar-mid');
  if (barMid) barMid.onclick = openNowPlaying;
  thumb.onclick = openNowPlaying;
  pTitle.onclick = openNowPlaying;
  pArtist.onclick = openNowPlaying;

  const playerBar = document.querySelector('.bar');
  if (playerBar) {
    playerBar.addEventListener('click', (e) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;
      if (target.closest('button, input, .seek, .seek-bar, .ctl, .icon-btn, #volume-slider, #btn-like, #btn-mute')) return;
      openNowPlaying();
    });
  }

  if (npModeSong) npModeSong.onclick = () => setVideoMode(false);
  if (npModeVideo) npModeVideo.onclick = () => setVideoMode(true);

  if (btnCollapse) btnCollapse.onclick = closeNowPlaying;

  // Global Keyboard Shortcuts
  window.addEventListener('keydown', (e) => {
    // If user is typing in an input or textarea, don't hijack media keys
    const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
    if (tag === 'input' || tag === 'textarea') {
      if (e.key === 'Escape') {
        hideSuggestions();
      }
      return;
    }

    if (e.key === ' ' || e.code === 'Space') {
      e.preventDefault();
      togglePlay();
    } else if (e.key === 'k' || e.key === 'K') {
      e.preventDefault();
      togglePlay();
    } else if (e.key === 'j' || e.key === 'J' || e.key === 'ArrowLeft') {
      e.preventDefault();
      seekOffset(-5);
    } else if (e.key === 'l' || e.key === 'L' || e.key === 'ArrowRight') {
      e.preventDefault();
      seekOffset(5);
    } else if ((e.shiftKey && e.key === 'N') || e.key === '>') {
      e.preventDefault();
      step(1);
    } else if ((e.shiftKey && e.key === 'P') || e.key === '<') {
      e.preventDefault();
      step(-1);
    } else if (e.key === 'm' || e.key === 'M') {
      e.preventDefault();
      $('btn-mute').click();
    } else if (e.key === 'f' || e.key === 'F') {
      e.preventDefault();
      if (npOverlay?.classList.contains('hidden')) {
        openNowPlaying();
      } else {
        closeNowPlaying();
      }
    } else if (e.key === 'Escape') {
      closeNowPlaying();
      queueDrawer.classList.remove('open');
      $('settings-backdrop').classList.add('hidden');
      hideContextMenu();
      hideSuggestions();
    }
  });

  // Context Menu Actions
  if (menuStartRadio) {
    menuStartRadio.onclick = () => {
      if (contextTrack) {
        startRadio(contextTrack);
        hideContextMenu();
      }
    };
  }
  if (menuPlayNext) {
    menuPlayNext.onclick = () => {
      if (contextTrack) {
        queue.splice(qIndex + 1, 0, contextTrack);
        renderQueue();
        renderOverlayQueue();
        hideContextMenu();
      }
    };
  }
  if (menuAddQueue) {
    menuAddQueue.onclick = () => {
      if (contextTrack) {
        queue.push(contextTrack);
        renderQueue();
        renderOverlayQueue();
        hideContextMenu();
      }
    };
  }
  if (menuCopyLink) {
    menuCopyLink.onclick = () => {
      if (contextTrack) {
        const link = `https://music.youtube.com/watch?v=${contextTrack.videoId}`;
        navigator.clipboard.writeText(link).catch(() => { });
        hideContextMenu();
      }
    };
  }

  // Auth & Settings
  $('btn-login').onclick = () => invoke('open_login_window').catch((e) => console.error(e));
  $('btn-logout').onclick = async () => { try { await invoke('logout'); await refreshAuth(); await loadHome(); } catch (e) { console.error(e); } };

  const modal = $('settings-backdrop');
  $('settings-btn').onclick = () => {
    loadSettings();
    renderThemeSelector();
    modal.classList.remove('hidden');
  };
  $('close-settings').onclick = () => modal.classList.add('hidden');
  modal.onclick = (e) => { if (e.target === modal) modal.classList.add('hidden'); };

  const setAutoplay = $<HTMLInputElement>('set-autoplay');
  if (setAutoplay) setAutoplay.onchange = saveSettings;
  const setAnimations = $<HTMLInputElement>('set-animations');
  if (setAnimations) setAnimations.onchange = saveSettings;
  const setVideoSwitcher = $<HTMLInputElement>('set-video-switcher');
  if (setVideoSwitcher) setVideoSwitcher.onchange = saveSettings;
  const setVideoQuality = $<HTMLSelectElement>('set-video-quality');
  if (setVideoQuality) setVideoQuality.onchange = saveSettings;
  const setRomanizeMode = $<HTMLSelectElement>('set-romanize-mode');
  if (setRomanizeMode) setRomanizeMode.onchange = saveSettings;
  const setDiscord = $<HTMLInputElement>('set-discord');
  if (setDiscord) setDiscord.onchange = saveSettings;
  const setSponsor = $<HTMLInputElement>('set-sponsor');
  if (setSponsor) setSponsor.onchange = saveSettings;
  const setTuna = $<HTMLInputElement>('set-tuna');
  if (setTuna) setTuna.onchange = saveSettings;
  const setTray = $<HTMLInputElement>('set-tray');
  if (setTray) setTray.onchange = saveSettings;

  const btnSaveCookie = $('btn-save-cookie');
  if (btnSaveCookie) {
    btnSaveCookie.onclick = async () => {
      const cookieInput = $<HTMLTextAreaElement>('cookie-input');
      const val = cookieInput?.value.trim();
      if (!val) return;
      try {
        await invoke('set_auth_cookies', { cookies: val });
        modal.classList.add('hidden');
        cookieInput.value = '';
        await refreshAuth();
        await loadHome();
      } catch (e) {
        alert('Failed to set cookies: ' + e);
      }
    };
  }
}

// Search Suggestions Autocomplete
async function fetchSuggestions(q: string) {
  try {
    const list = await invoke<string[]>('get_search_suggestions', { query: q });
    if (!list || !list.length) {
      hideSuggestions();
      return;
    }
    searchSuggestionsList = list.slice(0, 8);
    selectedSuggestionIndex = -1;
    renderSuggestions(searchSuggestionsList);
  } catch {
    hideSuggestions();
  }
}

function renderSuggestions(list: string[]) {
  if (!searchSuggestions) return;
  searchSuggestions.innerHTML = '';
  list.forEach((sug, idx) => {
    const item = document.createElement('div');
    item.className = 'suggestion-item';
    item.innerHTML = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/></svg><span>${esc(sug)}</span>`;
    item.onclick = () => {
      searchInput.value = sug;
      hideSuggestions();
      search(sug);
    };
    searchSuggestions.appendChild(item);
  });
  searchSuggestions.classList.remove('hidden');
}

function moveSuggestion(delta: number) {
  if (!searchSuggestionsList.length) return;
  selectedSuggestionIndex = (selectedSuggestionIndex + delta + searchSuggestionsList.length) % searchSuggestionsList.length;
  const items = searchSuggestions.querySelectorAll('.suggestion-item');
  items.forEach((it, idx) => {
    it.classList.toggle('selected', idx === selectedSuggestionIndex);
  });
  if (searchSuggestionsList[selectedSuggestionIndex]) {
    searchInput.value = searchSuggestionsList[selectedSuggestionIndex];
  }
}

function hideSuggestions() {
  if (searchSuggestions) searchSuggestions.classList.add('hidden');
  selectedSuggestionIndex = -1;
  searchSuggestionsList = [];
}

// Context Menu
function showContextMenu(e: MouseEvent, track: TrackItem) {
  e.preventDefault();
  e.stopPropagation();
  contextTrack = track;
  if (!contextMenu) return;

  contextMenu.style.left = `${Math.min(window.innerWidth - 200, e.clientX)}px`;
  contextMenu.style.top = `${Math.min(window.innerHeight - 180, e.clientY)}px`;
  contextMenu.classList.remove('hidden');
}

function hideContextMenu() {
  if (contextMenu) contextMenu.classList.add('hidden');
  contextTrack = null;
}

// Tab Switching
function setOverlayTab(tab: 'cover' | 'queue' | 'lyrics') {
  activeOverlayTab = tab;
  npTabCoverBtn?.classList.toggle('active', tab === 'cover');
  npTabQueueBtn?.classList.toggle('active', tab === 'queue');
  npTabLyricsBtn?.classList.toggle('active', tab === 'lyrics');

  npViewCover?.classList.toggle('hidden', tab !== 'cover');
  npViewQueue?.classList.toggle('hidden', tab !== 'queue');
  npViewLyrics?.classList.toggle('hidden', tab !== 'lyrics');

  if (tab === 'queue') renderOverlayQueue();
  if (tab === 'lyrics') {
    setTimeout(() => updateSyncedLyricsProgress(ytPlayer?.getCurrentTime?.() || 0, true), 40);
  }
}

function setDrawerTab(tab: 'queue' | 'lyrics') {
  activeDrawerTab = tab;
  tabDrawerQueue?.classList.toggle('active', tab === 'queue');
  tabDrawerLyrics?.classList.toggle('active', tab === 'lyrics');

  queueList?.classList.toggle('hidden', tab !== 'queue');
  drawerLyricsContainer?.classList.toggle('hidden', tab !== 'lyrics');
  if (tab === 'lyrics') {
    setTimeout(() => updateSyncedLyricsProgress(ytPlayer?.getCurrentTime?.() || 0, true), 40);
  }
}

// Playback Modes (Shuffle & Repeat)
function toggleShuffle() {
  isShuffled = !isShuffled;
  btnShuffle?.classList.toggle('active', isShuffled);
  npBtnShuffle?.classList.toggle('active', isShuffled);

  if (isShuffled) {
    if (queue.length > qIndex + 1) {
      const past = queue.slice(0, qIndex + 1);
      const remaining = queue.slice(qIndex + 1);
      // Fisher-Yates shuffle remaining
      for (let i = remaining.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [remaining[i], remaining[j]] = [remaining[j], remaining[i]];
      }
      queue = [...past, ...remaining];
    }
  } else {
    // Restore original ordering
    if (originalQueue.length && current) {
      const newIdx = originalQueue.findIndex((t) => t.videoId === current?.videoId);
      queue = [...originalQueue];
      qIndex = newIdx >= 0 ? newIdx : qIndex;
    }
  }
  renderQueue();
  renderOverlayQueue();
  savePlaybackState(ytPlayer?.getCurrentTime?.() || 0);
}

function cycleRepeat() {
  if (repeatMode === 'off') {
    repeatMode = 'all';
  } else if (repeatMode === 'all') {
    repeatMode = 'one';
  } else {
    repeatMode = 'off';
  }

  const isAll = repeatMode === 'all';
  const isOne = repeatMode === 'one';

  btnRepeat?.classList.toggle('active', isAll || isOne);
  npBtnRepeat?.classList.toggle('active', isAll || isOne);

  repeatBadge?.classList.toggle('hidden', !isOne);
  npRepeatBadge?.classList.toggle('hidden', !isOne);
  savePlaybackState(ytPlayer?.getCurrentTime?.() || 0);
}

function seekOffset(deltaSec: number) {
  if (!ytPlayer || !ytReady || duration <= 0) return;
  const cur = ytPlayer.getCurrentTime() || 0;
  const target = Math.max(0, Math.min(duration, cur + deltaSec));
  ytPlayer.seekTo(target, true);
  syncTunaProgress(true);
  updateSyncedLyricsProgress(target, true);
}

function handleSeek(e: MouseEvent, targetElem: HTMLElement) {
  if (duration <= 0) return;
  const r = targetElem.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
  const targetTime = pct * duration;

  fill.style.width = `${pct * 100}%`;
  if (npProgress) npProgress.style.width = `${pct * 100}%`;

  const timeFormatted = fmt(targetTime);
  tCur.textContent = timeFormatted;
  if (npTCur) npTCur.textContent = timeFormatted;

  if (ytPlayer && ytReady && ytPlayer.seekTo) {
    ytPlayer.seekTo(targetTime, true);
    syncTunaProgress(true);
    updateSyncedLyricsProgress(targetTime, true);
  }
}

function setChip(i: number) {
  document.querySelectorAll('.chip').forEach((c, j) => c.classList.toggle('active', i === j));
}

async function refreshAuth() {
  let p: AccountProfile | null = null;
  try { p = await invoke<AccountProfile>('get_account_profile'); } catch { /* signed out */ }
  const on = !!p?.isLoggedIn;
  $('btn-login').classList.toggle('hidden', on);
  $('user-profile').classList.toggle('hidden', !on);
  if (on && p) {
    $('user-name').textContent = p.name || 'Account';
    if (p.avatarUrl) $<HTMLImageElement>('user-avatar').src = p.avatarUrl;
  }
}

function recordTasteArtist(artistName?: string) {
  if (!artistName) return;
  const clean = artistName.trim();
  if (clean.length < 2 || clean.toLowerCase() === 'unknown') return;

  const parts = clean.split(/[,&]|\s+feat\.|\s+ft\./i).map(s => s.trim()).filter(s => s.length >= 2 && s.toLowerCase() !== 'unknown');
  try {
    let stored: string[] = JSON.parse(localStorage.getItem('cremeplay_taste_artists') || '[]');
    if (!Array.isArray(stored)) stored = [];
    for (const p of parts) {
      const lower = p.toLowerCase();
      stored = stored.filter(x => x.toLowerCase() !== lower);
      stored.unshift(p);
    }
    if (stored.length > 25) stored = stored.slice(0, 25);
    localStorage.setItem('cremeplay_taste_artists', JSON.stringify(stored));
  } catch {}
}

function getTasteSeeds(): string[] {
  const seeds: string[] = [];
  const seen = new Set<string>();

  const add = (name?: string) => {
    if (!name) return;
    const clean = name.trim();
    const lower = clean.toLowerCase();
    if (clean.length >= 2 && lower !== 'unknown' && !seen.has(lower)) {
      seen.add(lower);
      seeds.push(clean);
    }
  };

  // 1. Stored taste artists from recent playback
  try {
    const stored = JSON.parse(localStorage.getItem('cremeplay_taste_artists') || '[]');
    if (Array.isArray(stored)) {
      stored.forEach(add);
    }
  } catch {}

  // 2. Currently playing / resumed queue artists
  if (current?.artist) add(current.artist);
  try {
    const rawQueue = localStorage.getItem('cremeplay_resumed_queue');
    if (rawQueue) {
      const parsed = JSON.parse(rawQueue);
      if (parsed.queue && Array.isArray(parsed.queue)) {
        parsed.queue.slice(0, 10).forEach((t: TrackItem) => {
          if (t.artist) add(t.artist);
        });
      }
    }
  } catch {}

  return seeds.slice(0, 10);
}

async function loadHome() {
  msg('Loading…');
  try {
    const tasteSeeds = getTasteSeeds();
    home = await invoke<HomeSection[]>('get_home_feed', { tasteSeeds });
    renderHome(home);
  } catch {
    msg('Search above to find songs, albums and artists.');
  }
}

async function loadSidebarPlaylists() {
  if (!sidePlaylistsList) return;
  try {
    const remote = await invoke<SidebarPlaylist[]>('get_user_playlists').catch(() => []);
    let local: Array<{ title: string; browseId: string; artworkUrl?: string }> = [];
    try {
      local = JSON.parse(localStorage.getItem('cremeplay_saved_playlists') || '[]');
    } catch {}

    const map = new Map<string, SidebarPlaylist>();
    (remote || []).forEach(p => map.set(p.browseId, p));
    local.forEach(p => {
      if (!map.has(p.browseId)) {
        map.set(p.browseId, { title: p.title, browseId: p.browseId, artworkUrl: p.artworkUrl || '', trackCount: 0 });
      }
    });

    userPlaylists = Array.from(map.values());
    sidePlaylistsList.innerHTML = '';
    if (!userPlaylists || !userPlaylists.length) {
      return;
    }
    userPlaylists.forEach((pl) => {
      const item = document.createElement('div');
      item.className = 'side-playlist-item';
      item.title = pl.title;
      item.innerHTML = `
        <div class="side-pl-thumb">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 18V5l12-2v13" />
            <circle cx="6" cy="18" r="3" />
            <circle cx="18" cy="16" r="3" />
          </svg>
        </div>
        <div class="side-pl-info">
          <div class="side-pl-title">${esc(pl.title)}</div>
          <div class="side-pl-sub">Playlist</div>
        </div>
      `;
      item.onclick = () => {
        document.querySelectorAll('.side-playlist-item').forEach((el) => el.classList.remove('active'));
        navs.forEach((n) => n.classList.remove('active'));
        item.classList.add('active');
        openPlaylist(pl.browseId);
      };
      sidePlaylistsList.appendChild(item);
    });
  } catch (err) {
    console.warn('Failed to load user playlists for sidebar:', err);
  }
}

function showToast(text: string) {
  let toast = document.querySelector('.toast-notice') as HTMLElement;
  if (!toast) {
    toast = document.createElement('div');
    toast.className = 'toast-notice';
    document.body.appendChild(toast);
  }
  toast.textContent = text;
  toast.classList.add('show');
  setTimeout(() => toast.classList.remove('show'), 2500);
}

async function playPlaylist(browseId: string) {
  try {
    const detail = await invoke<PlaylistDetail>('get_playlist_or_album', { browseId });
    if (detail && detail.tracks && detail.tracks.length > 0) {
      play(detail.tracks[0], detail.tracks, 0);
    }
  } catch (err) {
    console.warn('Failed to play playlist directly:', err);
  }
}

async function openPlaylist(browseId: string) {
  msg('Loading playlist…');
  try {
    const detail = await invoke<PlaylistDetail>('get_playlist_or_album', { browseId });
    if (!detail) {
      msg('Playlist not found.');
      return;
    }
    view.innerHTML = '';
    const header = document.createElement('div');
    header.className = 'playlist-header';
    const isAlbum = detail.browseId.startsWith('MPRE');

    // Collage cover if playlist does not have custom artwork
    let coverHtml = '';
    const hasCustomArtwork = detail.artworkUrl &&
      !detail.artworkUrl.includes('skeleton') &&
      !detail.artworkUrl.includes('data:image/gif') &&
      detail.artworkUrl !== PLACEHOLDER;

    if (hasCustomArtwork) {
      coverHtml = `<img class="playlist-cover" src="${art(detail.artworkUrl)}" alt="" />`;
    } else {
      const validCovers = (detail.tracks || [])
        .map(t => t.artworkUrl)
        .filter(u => u && !u.includes('data:image/gif') && u !== PLACEHOLDER);

      if (validCovers.length >= 4) {
        coverHtml = `
          <div class="playlist-cover collage">
            <img src="${art(validCovers[0])}" alt="" />
            <img src="${art(validCovers[1])}" alt="" />
            <img src="${art(validCovers[2])}" alt="" />
            <img src="${art(validCovers[3])}" alt="" />
          </div>
        `;
      } else if (validCovers.length > 0) {
        coverHtml = `<img class="playlist-cover" src="${art(validCovers[0])}" alt="" />`;
      } else {
        coverHtml = `<div class="playlist-cover empty-cover"><svg width="56" height="56" viewBox="0 0 24 24" fill="currentColor"><path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/></svg></div>`;
      }
    }

    header.innerHTML = `
      ${coverHtml}
      <div class="playlist-info">
        <span class="playlist-type">${isAlbum ? 'Album' : 'Playlist'}</span>
        <h1 class="playlist-title">${esc(detail.title || 'Untitled Playlist')}</h1>
        <div class="playlist-subtitle">${esc(detail.subtitle || '')}${detail.subtitle ? ' • ' : ''}${detail.tracks.length} songs</div>
        <div class="playlist-actions">
          <button id="btn-play-playlist" class="btn-primary" title="Play all tracks">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><polygon points="6 4 20 12 6 20 6 4"/></svg>
            Play All
          </button>
          <button id="btn-shuffle-playlist" class="btn-secondary" title="Shuffle play">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 3 21 3 21 8"/><line x1="4" y1="20" x2="21" y2="3"/><polyline points="21 16 21 21 16 21"/><line x1="15" y1="15" x2="21" y2="21"/><line x1="4" y1="4" x2="9" y2="9"/></svg>
            Shuffle
          </button>
          <button id="btn-copy-link" class="btn-secondary" title="Copy playlist link">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
            Copy Link
          </button>
          <button id="btn-save-playlist" class="btn-secondary" title="Save to library">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"/></svg>
            Save
          </button>
          <button id="btn-edit-playlist" class="btn-secondary" title="Edit playlist title">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>
            Edit
          </button>
        </div>
      </div>
    `;
    view.appendChild(header);

    const btnPlayPl = header.querySelector('#btn-play-playlist') as HTMLElement;
    if (btnPlayPl && detail.tracks.length > 0) {
      btnPlayPl.onclick = () => play(detail.tracks[0], detail.tracks, 0);
    }

    const btnShufflePl = header.querySelector('#btn-shuffle-playlist') as HTMLElement;
    if (btnShufflePl && detail.tracks.length > 0) {
      btnShufflePl.onclick = () => {
        const shuffled = [...detail.tracks];
        for (let i = shuffled.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
        }
        play(shuffled[0], shuffled, 0);
        showToast('Playing playlist shuffled!');
      };
    }

    const btnCopyLink = header.querySelector('#btn-copy-link') as HTMLElement;
    if (btnCopyLink) {
      btnCopyLink.onclick = () => {
        const cleanId = detail.browseId.replace(/^VL/, '');
        const shareUrl = `https://music.youtube.com/playlist?list=${cleanId}`;
        navigator.clipboard.writeText(shareUrl).then(() => {
          showToast('Copied playlist link to clipboard!');
        }).catch(() => {
          showToast(`Link: ${shareUrl}`);
        });
      };
    }

    const btnSavePl = header.querySelector('#btn-save-playlist') as HTMLElement;
    if (btnSavePl) {
      btnSavePl.onclick = () => {
        try {
          const saved: Array<{ title: string; browseId: string; artworkUrl?: string }> = JSON.parse(localStorage.getItem('cremeplay_saved_playlists') || '[]');
          if (!saved.some(p => p.browseId === detail.browseId)) {
            saved.push({ title: detail.title || 'Saved Playlist', browseId: detail.browseId, artworkUrl: detail.artworkUrl });
            localStorage.setItem('cremeplay_saved_playlists', JSON.stringify(saved));
            loadSidebarPlaylists();
          }
          showToast('Saved playlist to library!');
        } catch {
          showToast('Saved playlist!');
        }
      };
    }

    const btnEditPl = header.querySelector('#btn-edit-playlist') as HTMLElement;
    if (btnEditPl) {
      btnEditPl.onclick = () => {
        const titleEl = header.querySelector('.playlist-title') as HTMLElement;
        const currentTitle = titleEl ? titleEl.textContent || '' : detail.title;
        const newTitle = prompt('Edit playlist name:', currentTitle);
        if (newTitle && newTitle.trim() && newTitle.trim() !== currentTitle) {
          detail.title = newTitle.trim();
          if (titleEl) titleEl.textContent = newTitle.trim();
          showToast('Playlist renamed!');
        }
      };
    }

    renderList(detail.tracks);
  } catch (err) {
    msg(`Failed to load playlist: ${err}`);
  }
}

async function loadExplore() {
  setActive(navs[1]);
  document.querySelectorAll('.side-playlist-item').forEach((el) => el.classList.remove('active'));
  msg('Loading Explore feed…');
  try {
    const exploreSections = await invoke<HomeSection[]>('get_explore_feed');
    if (exploreSections && exploreSections.length) {
      renderExplore(exploreSections);
    } else {
      search('Top hits');
    }
  } catch (e) {
    console.error('Failed to load explore feed:', e);
    search('Top hits');
  }
}

function createCard(t: TrackItem, items: TrackItem[], index: number, isAlgorithmicRadio = false): HTMLElement {
  const c = document.createElement('div');
  const isArtist = (t.artist || '').toLowerCase() === 'artist'
    || (t.album || '').toLowerCase() === 'artist'
    || (t.durationText || '').toLowerCase() === 'artist';

  const isPlaylist = t.videoId.startsWith('VL') || t.videoId.startsWith('PL') || (t.artist || '').toLowerCase().includes('playlist');

  const isLandscapeHint = !isPlaylist && !isArtist && (
    /hqdefault|mqdefault|sddefault|maxresdefault|watch\?v=/.test(t.artworkUrl || '')
    || (/=w(\d+)-h(\d+)/.test(t.artworkUrl || '') && (() => {
      const m = (t.artworkUrl || '').match(/=w(\d+)-h(\d+)/);
      return m ? Number(m[1]) > Number(m[2]) * 1.25 : false;
    })())
  );

  if (isArtist) {
    c.className = 'card is-artist';
  } else if (isLandscapeHint) {
    c.className = 'card ratio-16-9';
  } else {
    c.className = 'card ratio-1-1';
  }

  let thumbMarkup = `<img src="${art(t.artworkUrl)}" alt="" loading="lazy">`;
  if (isPlaylist && (!t.artworkUrl || t.artworkUrl.includes('data:image/gif') || t.artworkUrl === PLACEHOLDER)) {
    thumbMarkup = `<div class="empty-cover" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:var(--panel)"><svg width="36" height="36" viewBox="0 0 24 24" fill="currentColor"><path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/></svg></div>`;
  }

  c.innerHTML = `
    <div class="thumb">
      ${thumbMarkup}
      <div class="pbtn">${PLAY_SVG}</div>
    </div>
    <div class="c1" title="${esc(t.title)}">${esc(t.title)}</div>
    <div class="c2">${esc(t.artist || '')}</div>
  `;

  if (!isArtist && !isPlaylist) {
    const img = c.querySelector('img');
    if (img) {
      img.onload = () => {
        if (img.naturalWidth > img.naturalHeight * 1.25) {
          c.classList.remove('ratio-1-1');
          c.classList.add('ratio-16-9');
        } else {
          c.classList.remove('ratio-16-9');
          c.classList.add('ratio-1-1');
        }
      };
    }
  }

  const pbtn = c.querySelector('.pbtn') as HTMLElement;
  if (pbtn) {
    pbtn.onclick = (e) => {
      e.stopPropagation();
      if (isPlaylist) {
        playPlaylist(t.videoId);
      } else if (isAlgorithmicRadio) {
        startRadio(t);
      } else {
        play(t, items, index >= 0 ? index : 0);
      }
    };
  }

  c.onclick = () => {
    if (isPlaylist) {
      openPlaylist(t.videoId);
    } else if (isAlgorithmicRadio) {
      startRadio(t);
    } else {
      play(t, items, index >= 0 ? index : 0);
    }
  };
  c.oncontextmenu = (e) => showContextMenu(e, t);
  return c;
}

function renderExplore(sections: HomeSection[]) {
  if (!sections?.length) return msg('Explore feed is currently empty.');
  view.innerHTML = '';

  sections.forEach((sec) => {
    if (!sec.items || !sec.items.length) return;
    const lowerTitle = sec.title.toLowerCase();
    const isNewReleases = lowerTitle.includes('new album') || lowerTitle.includes('singles');
    const isTopSongs = lowerTitle.includes('top song') || lowerTitle.includes('chart') || lowerTitle.includes('ranking');

    const el = document.createElement('section');
    el.className = 'shelf';

    if (isTopSongs) {
      el.innerHTML = `<h2>${esc(sec.title)}</h2>`;
      renderRankedList(sec.items, el);
    } else {
      el.innerHTML = `
        <div class="shelf-header">
          <h2>${esc(sec.title)}</h2>
          <div class="shelf-arrows">
            <button class="icon-btn shelf-scroll-prev" title="Previous">‹</button>
            <button class="icon-btn shelf-scroll-next" title="Next">›</button>
          </div>
        </div>
        <div class="shelf-grid-2row"></div>
      `;
      const shelfContainer = el.querySelector('.shelf-grid-2row') as HTMLElement;
      const btnPrev = el.querySelector('.shelf-scroll-prev') as HTMLElement;
      const btnNext = el.querySelector('.shelf-scroll-next') as HTMLElement;
      if (btnPrev && shelfContainer) btnPrev.onclick = () => shelfContainer.scrollBy({ left: -520, behavior: 'smooth' });
      if (btnNext && shelfContainer) btnNext.onclick = () => shelfContainer.scrollBy({ left: 520, behavior: 'smooth' });

      const isSingleRow = isNewReleases || sec.items.length <= 4;
      const row1 = document.createElement('div');
      row1.className = 'shelf-row';
      const row2 = document.createElement('div');
      row2.className = 'shelf-row';

      sec.items.forEach((t, i) => {
        const isPl = t.videoId.startsWith('VL') || t.videoId.startsWith('PL');
        const card = createCard(t, sec.items, i, !isPl);
        if (isSingleRow) {
          row1.appendChild(card);
        } else {
          if (i % 2 === 0) row1.appendChild(card);
          else row2.appendChild(card);
        }
      });

      shelfContainer.appendChild(row1);
      if (!isSingleRow && row2.children.length > 0) {
        shelfContainer.appendChild(row2);
      }
    }

    view.appendChild(el);
  });
}

function renderRankedList(tracks: TrackItem[], host: HTMLElement = view) {
  if (!tracks?.length) {
    host.insertAdjacentHTML('beforeend', '<div class="msg">No songs found.</div>');
    return;
  }
  const list = document.createElement('div');
  list.className = 'list';
  tracks.forEach((t, i) => {
    const rank = i + 1;
    const isTop = rank <= 3;
    const r = document.createElement('div');
    r.className = 'row-item' + (current?.videoId === t.videoId ? ' now' : '');
    r.innerHTML = `
      <div class="rank-badge ${isTop ? 'top-rank' : ''}">${rank}</div>
      <img src="${art(t.artworkUrl)}" alt="" loading="lazy">
      <div class="rinfo">
        <div class="r1">${esc(t.title)}</div>
        <div class="r2">${esc(t.artist)}${t.album ? ' • ' + esc(t.album) : ''}</div>
      </div>
      <span class="rdur">${esc(t.durationText || fmt(t.duration))}</span>
    `;
    r.onclick = () => play(t, tracks, i);
    r.oncontextmenu = (e) => showContextMenu(e, t);
    list.appendChild(r);
  });
  host.appendChild(list);
}

async function loadLibrary() {
  setActive(navs[2]);
  document.querySelectorAll('.side-playlist-item').forEach((el) => el.classList.remove('active'));
  if (sidePinnedLiked) sidePinnedLiked.classList.add('active');
  msg('Loading your library…');
  const hero = (sub: string) => `<div class="hero"><div class="hero-cover"><svg width="64" height="64" viewBox="0 0 24 24" fill="#fff"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54z"/></svg></div><div><h1>Liked Music</h1><p>${esc(sub)}</p></div></div>`;
  try {
    const tracks = await invoke<TrackItem[]>('get_liked_songs');
    if (!tracks?.length) {
      view.innerHTML = hero('Sign in to sync your saved songs and playlists') + '<div class="msg">Songs you like while signed in will appear here. In the meantime, explore trending releases above.</div>';
      return;
    }
    view.innerHTML = hero(`${tracks.length} songs`);
    renderList(tracks);
  } catch {
    view.innerHTML = hero('Sign in to access your personal library') + '<div class="msg">Connect your YouTube account from the top right to sync your liked songs and playlists.</div>';
  }
}

function renderHome(sections: HomeSection[]) {
  if (!sections?.length) return msg('Search above to find songs, albums and artists.');
  view.innerHTML = '';

  // Position "From the community" directly below "New releases"
  const commIdx = sections.findIndex(s => /community/i.test(s.title));
  if (commIdx !== -1) {
    const commSection = sections.splice(commIdx, 1)[0];
    const newRelIdx = sections.findIndex(s => /new\s+release|new\s+album|singles/i.test(s.title));
    if (newRelIdx !== -1) {
      sections.splice(newRelIdx + 1, 0, commSection);
    } else {
      sections.splice(Math.min(1, sections.length), 0, commSection);
    }
  }

  sections.forEach((sec) => {
    if (!sec.items || !sec.items.length) return;

    let title = sec.title;
    if (/listen\s+again/i.test(title)) {
      title = 'Listen again';
    }

    const isCommunityShelf = /community/i.test(title);

    // From the community should only show playlists!
    let items = sec.items;
    if (isCommunityShelf) {
      items = items.filter(t => t.videoId.startsWith('VL') || t.videoId.startsWith('PL') || (t.artist || '').toLowerCase().includes('playlist'));
      if (!items.length) return;
    }

    const el = document.createElement('section');
    el.className = 'shelf';
    el.innerHTML = `
      <div class="shelf-header">
        <h2>${esc(title)}</h2>
        <div class="shelf-arrows">
          <button class="icon-btn shelf-scroll-prev" title="Previous">‹</button>
          <button class="icon-btn shelf-scroll-next" title="Next">›</button>
        </div>
      </div>
      <div class="shelf-grid-2row"></div>
    `;

    const shelfContainer = el.querySelector('.shelf-grid-2row') as HTMLElement;
    const btnPrev = el.querySelector('.shelf-scroll-prev') as HTMLElement;
    const btnNext = el.querySelector('.shelf-scroll-next') as HTMLElement;

    if (btnPrev && shelfContainer) {
      btnPrev.onclick = () => shelfContainer.scrollBy({ left: -520, behavior: 'smooth' });
    }
    if (btnNext && shelfContainer) {
      btnNext.onclick = () => shelfContainer.scrollBy({ left: 520, behavior: 'smooth' });
    }

    const isSingleRow = items.length <= 4;
    const row1 = document.createElement('div');
    row1.className = 'shelf-row';
    const row2 = document.createElement('div');
    row2.className = 'shelf-row';

    items.forEach((t, i) => {
      const isPl = isCommunityShelf || t.videoId.startsWith('VL') || t.videoId.startsWith('PL');
      const card = createCard(t, items, i, !isPl);
      if (isSingleRow) {
        row1.appendChild(card);
      } else {
        if (i % 2 === 0) {
          row1.appendChild(card);
        } else {
          row2.appendChild(card);
        }
      }
    });

    shelfContainer.appendChild(row1);
    if (!isSingleRow && row2.children.length > 0) {
      shelfContainer.appendChild(row2);
    }

    view.appendChild(el);
  });
}

function renderList(tracks: TrackItem[], host: HTMLElement = view) {
      if (!tracks?.length) {
        host.insertAdjacentHTML('beforeend', '<div class="msg">No songs found.</div>');
        return;
      }
      const list = document.createElement('div');
      list.className = 'list';
      tracks.forEach((t, i) => {
        const r = document.createElement('div');
        r.className = 'row-item' + (current?.videoId === t.videoId ? ' now' : '');
        r.innerHTML = `<img src="${art(t.artworkUrl)}" alt="" loading="lazy"><div class="rinfo"><div class="r1">${esc(t.title)}</div><div class="r2">${esc(t.artist)}${t.album ? ' • ' + esc(t.album) : ''}</div></div><span class="rdur">${esc(t.durationText || fmt(t.duration))}</span>`;
        r.onclick = () => play(t, tracks, i);
        r.oncontextmenu = (e) => showContextMenu(e, t);
        list.appendChild(r);
      });
      host.appendChild(list);
    }

async function search(q: string) {
      msg(`Searching for “${q}”…`);
      try {
        currentSearchPayload = await invoke<SearchResultPayload>('search_music', { query: q });
        if (
          !currentSearchPayload ||
          (!currentSearchPayload.songs.length &&
            !currentSearchPayload.libraryTracks.length &&
            !currentSearchPayload.topResult)
        ) {
          msg(`No results found for “${q}”. Try another search.`);
          return;
        }
        activeSearchTab = 'catalog';
        renderSearchResults(q);
      } catch (e) {
        msg(`Search failed: ${e}`);
      }
    }

function renderSearchResults(q: string) {
      if (!currentSearchPayload) return;
      view.innerHTML = '';

      const header = document.createElement('div');
      header.className = 'search-header';
      const songsCount = currentSearchPayload.songs.length;
      const libCount = currentSearchPayload.libraryTracks.length;

      header.innerHTML = `
    <div class="shelf" style="margin-bottom: 0;">
      <h2>Results for “${esc(q)}”</h2>
    </div>
    <div class="search-tabs">
      <button class="search-tab ${activeSearchTab === 'catalog' ? 'active' : ''}" id="tab-search-catalog">
        YT Music (${songsCount})
      </button>
      <button class="search-tab ${activeSearchTab === 'library' ? 'active' : ''}" id="tab-search-library">
        Library (${libCount})
      </button>
    </div>
  `;
      view.appendChild(header);

      const tabCatalog = header.querySelector('#tab-search-catalog') as HTMLElement;
      const tabLib = header.querySelector('#tab-search-library') as HTMLElement;

      tabCatalog.onclick = () => {
        activeSearchTab = 'catalog';
        renderSearchResults(q);
      };
      tabLib.onclick = () => {
        activeSearchTab = 'library';
        renderSearchResults(q);
      };

      const resultsContainer = document.createElement('div');
      resultsContainer.className = 'search-content-area';
      view.appendChild(resultsContainer);

      if (activeSearchTab === 'catalog') {
        // Render Top Result card if present
        if (currentSearchPayload.topResult) {
          const top = currentSearchPayload.topResult;
          const topCard = document.createElement('div');
          topCard.className = 'top-result-card';
          const typeLabel = (top.resultType || 'Top Result').toUpperCase();
          topCard.innerHTML = `
        <div class="top-result-thumb">
          <img src="${art(top.artworkUrl)}" alt="" loading="lazy">
        </div>
        <div class="top-result-info">
          <span class="top-result-badge">Top Result • ${esc(typeLabel)}</span>
          <div class="top-result-title" title="${esc(top.title)}">${esc(top.title)}</div>
          <div class="top-result-sub">${esc(top.subtitle)}</div>
        </div>
        <div class="top-result-play-btn" title="Play">
          ${PLAY_SVG}
        </div>
      `;
          topCard.onclick = () => {
            if (top.videoId) {
              const t: TrackItem = {
                videoId: top.videoId,
                title: top.title,
                artist: top.subtitle,
                album: '',
                duration: 0,
                durationText: '',
                artworkUrl: top.artworkUrl,
              };
              play(t, currentSearchPayload?.songs || [t], 0);
            } else if (top.browseId) {
              openPlaylist(top.browseId);
            }
          };
          resultsContainer.appendChild(topCard);
        }

        if (currentSearchPayload.songs.length) {
          const sec = document.createElement('div');
          sec.className = 'shelf';
          sec.innerHTML = `<h2>Songs</h2>`;
          renderList(currentSearchPayload.songs, sec);
          resultsContainer.appendChild(sec);
        } else if (!currentSearchPayload.topResult) {
          resultsContainer.innerHTML = `<div class="msg">No catalog songs found for “${esc(q)}”.</div>`;
        }
      } else {
        // Library tab
        if (currentSearchPayload.libraryTracks.length) {
          const sec = document.createElement('div');
          sec.className = 'shelf';
          sec.innerHTML = `<h2>Songs in your Library</h2>`;
          renderList(currentSearchPayload.libraryTracks, sec);
          resultsContainer.appendChild(sec);
        } else {
          resultsContainer.innerHTML = `<div class="msg">No tracks found in your library for “${esc(q)}”.</div>`;
        }
      }
    }

async function play(t: TrackItem, list: TrackItem[] = [t], idx = 0, skipRadio = false, startSec = 0) {
      queue = list;
      originalQueue = [...list];
      qIndex = idx;
      fallbackAttempts = 0;
      showNow(t);
      setPlay(true);
      recordTasteArtist(t.artist);
      renderQueue();
      renderOverlayQueue();
      savePlaybackState(startSec);
      document.title = `${t.title} • ${t.artist} - Cremeplay`;

      if (ytPlayer && ytReady && ytPlayer.loadVideoById) {
        try {
          if (startSec > 0) {
            ytPlayer.loadVideoById({
              videoId: t.videoId,
              startSeconds: startSec,
              suggestedQuality: isVideoMode ? 'auto' : 'default',
            });
          } else {
            ytPlayer.loadVideoById({
              videoId: t.videoId,
              suggestedQuality: isVideoMode ? 'auto' : 'default',
            });
          }
          ytPlayer.playVideo();
        } catch (err) {
          console.error('Play error:', err);
        }
      } else {
        pendingTrack = t;
        pendingResumeSec = startSec;
      }

      // Fetch synced lyrics in background
      fetchLyrics(t);

      // Notify Rust backend for Discord RPC and System Tray tooltip
      try {
        await invoke('play_track', { track: t });
      } catch (e) {
        console.warn('Backend notification notice:', e);
      }
    }

async function cue(t: TrackItem, list: TrackItem[] = [t], idx = 0, startSec = 0) {
      queue = list;
      originalQueue = [...list];
      qIndex = idx;
      current = t;
      showNow(t);
      setPlay(false);
      renderQueue();
      renderOverlayQueue();
      document.title = `${t.title} • ${t.artist} - Cremeplay`;

      if (startSec > 0) {
        currentTime = startSec;
        if (tCur) tCur.textContent = fmt(startSec);
        if (npTCur) npTCur.textContent = fmt(startSec);
        if (t.duration > 0) {
          const pct = `${Math.min(100, (startSec / t.duration) * 100)}%`;
          if (fill) fill.style.width = pct;
          if (npProgress) npProgress.style.width = pct;
        }
      }

      if (ytPlayer && ytReady && ytPlayer.cueVideoById) {
        try {
          ytPlayer.cueVideoById({
            videoId: t.videoId,
            startSeconds: startSec,
            suggestedQuality: isVideoMode ? 'auto' : 'default',
          });
        } catch (err) {
          console.error('Cue error:', err);
        }
      }

      fetchLyrics(t);
    }

async function startRadio(t: TrackItem) {
      // 1. Play song immediately
      play(t, [t], 0, true);

      // 2. Concurrently fetch YouTube algorithmic radio queue (RDAMVM<videoId>)
      try {
        const radioItems = await invoke<TrackItem[]>('get_radio_queue', { videoId: t.videoId });
        if (radioItems && radioItems.length > 0) {
          const fullQueue = [...radioItems];
          const existIdx = fullQueue.findIndex((x) => x.videoId === t.videoId);
          if (existIdx > 0) {
            const [curr] = fullQueue.splice(existIdx, 1);
            fullQueue.unshift(curr);
          } else if (existIdx === -1) {
            fullQueue.unshift(t);
          }
          queue = fullQueue;
          originalQueue = [...fullQueue];
          qIndex = 0;
          renderQueue();
          renderOverlayQueue();
          savePlaybackState(ytPlayer?.getCurrentTime?.() || 0);
        }
      } catch (err) {
        console.warn('Radio queue fetch notice:', err);
      }
    }

async function fetchLyrics(track: TrackItem) {
      const videoId = track.videoId;
      currentLyricsTrackId = videoId;
      currentLyricsPayload = null;
      activeLyricLineIdx = -1;

      const loadingHtml = '<div class="lyrics-loading-state">Searching lyrics…</div>';
      if (npLyricsText) npLyricsText.innerHTML = loadingHtml;
      if (drawerLyricsText) drawerLyricsText.innerHTML = loadingHtml;

      try {
        const payload = await invoke<LyricsPayload>('get_lyrics', {
          videoId: track.videoId,
          title: track.title,
          artist: track.artist,
          album: track.album || null,
          duration: track.duration || null,
        });

        if (current?.videoId !== videoId || currentLyricsTrackId !== videoId) {
          return;
        }

        currentLyricsPayload = payload;
        renderLyrics(payload);
        updateSyncedLyricsProgress(ytPlayer?.getCurrentTime?.() || 0, true);
      } catch (err) {
        if (current?.videoId !== videoId) return;
        const fallbackHtml = '<div class="lyrics-empty-state">Lyrics not available for this track.</div>';
        if (npLyricsText) npLyricsText.innerHTML = fallbackHtml;
        if (drawerLyricsText) drawerLyricsText.innerHTML = fallbackHtml;
      }
    }

function hasSubstantialNonLatin(texts: string[]): boolean {
  const combined = texts.join('');
  let nonLatinCount = 0;
  let letterCount = 0;
  for (const ch of combined) {
    if (NON_LATIN_REGEX.test(ch)) {
      nonLatinCount++;
    }
    if (/\p{L}/u.test(ch)) {
      letterCount++;
    }
  }
  if (nonLatinCount === 0) return false;
  // If fewer than 4 non-latin chars in the entire song, it's just decorative/bracketed credit
  if (nonLatinCount < 4) return false;
  // If non-latin is less than 6% of total letters, it's predominantly an English/Latin song
  if (letterCount > 0 && (nonLatinCount / letterCount) < 0.06) return false;
  return true;
}

function renderLyrics(payload: LyricsPayload) {
      const badgeSource = payload.synced
        ? `Synced • ${payload.source}`
        : (payload.source !== 'None' ? `Plain • ${payload.source}` : '');

      const badgeHtml = badgeSource
        ? `<div class="lyrics-header"><span class="lyrics-source-badge">${esc(badgeSource)}</span></div>`
        : '';

      const cacheKey = currentLyricsTrackId || (payload.lines?.[0]?.text ?? payload.plainLyrics?.slice(0, 30) ?? '');
      const cached = romanizedLyricsCache.get(cacheKey);

      const romanizeMode: 'off' | 'subtext' | 'main' | 'only' =
        appSettings.romanizeMode ? appSettings.romanizeMode : (appSettings.romanizeLyrics ? 'subtext' : 'off');

      if (payload.synced && payload.lines && payload.lines.length > 0) {
        // Sanitize lyric lines: standalone [music] -> ♪, inline [music] removed
        payload.lines.forEach((l) => {
          l.text = sanitizeLyricText(l.text);
        });

        const rawTexts = payload.lines.map((l) => l.text);
        const hasNonLatin = hasSubstantialNonLatin(rawTexts);
        const shouldRomanize = romanizeMode !== 'off' && hasNonLatin;

        const linesHtml = payload.lines
          .map((l, i) => {
            const romaji = shouldRomanize && cached?.synced ? cached.synced[i] : null;
            const subHtml = romaji ? `<span class="lyric-romaji">${esc(romaji)}</span>` : '';
            return `<div class="lyric-line" data-idx="${i}" data-time="${l.timeSec}"><span class="lyric-main">${esc(l.text)}</span>${subHtml}</div>`;
          })
          .join('');

        const content = `${badgeHtml}<div class="lyrics-synced-list mode-${romanizeMode}"><div class="lyrics-dynamic-box" aria-hidden="true"></div>${linesHtml}</div>`;
        if (npLyricsText) npLyricsText.innerHTML = content;
        if (drawerLyricsText) drawerLyricsText.innerHTML = content;

        const attachSeek = (container: HTMLElement | null) => {
          if (!container) return;
          container.querySelectorAll('.lyric-line').forEach((lineEl) => {
            (lineEl as HTMLElement).onclick = (e) => {
              e.stopPropagation();
              const time = parseFloat(lineEl.getAttribute('data-time') || '0');
              if (ytPlayer && ytReady && ytPlayer.seekTo) {
                ytPlayer.seekTo(time, true);
                updateSyncedLyricsProgress(time, true);
              }
            };
          });
        };

        attachSeek(npLyricsText);
        attachSeek(drawerLyricsText);

        if (shouldRomanize && (!cached || !cached.synced)) {
          const activeTrackId = currentLyricsTrackId;
          lyricRomanizer
            .romanizeLines(rawTexts)
            .then((res) => {
              if (currentLyricsTrackId !== activeTrackId) return;
              const romanizedLines = res.lines.map((r, i) => {
                const original = rawTexts[i] || '';
                // If line does not have non-Latin characters (English line), DO NOT romanize
                if (!NON_LATIN_REGEX.test(original)) {
                  return null;
                }
                const cleaned = cleanRomanizedText(r);
                return cleaned && cleaned.toLowerCase() !== original.trim().toLowerCase() ? cleaned : null;
              });
              const existing = romanizedLyricsCache.get(cacheKey) || {};
              existing.synced = romanizedLines;
              romanizedLyricsCache.set(cacheKey, existing);

              if (appSettings.romanizeMode === 'off') return;

              const containers = [npLyricsText, drawerLyricsText];
              containers.forEach((container) => {
                if (!container) return;
                container.querySelectorAll<HTMLElement>('.lyric-line').forEach((lineEl) => {
                  const idx = parseInt(lineEl.getAttribute('data-idx') || '-1', 10);
                  if (idx >= 0 && idx < romanizedLines.length && romanizedLines[idx]) {
                    let romajiEl = lineEl.querySelector<HTMLElement>('.lyric-romaji');
                    if (!romajiEl) {
                      romajiEl = document.createElement('span');
                      romajiEl.className = 'lyric-romaji';
                      lineEl.appendChild(romajiEl);
                    }
                    romajiEl.textContent = romanizedLines[idx]!;
                  }
                });
              });

              const activeEl = npLyricsText?.querySelector('.lyric-line.active') as HTMLElement | null;
              if (activeEl && npLyricsText) updateDynamicLyricsBox(npLyricsText, activeEl);
              const drawerActiveEl = drawerLyricsText?.querySelector('.lyric-line.active') as HTMLElement | null;
              if (drawerActiveEl && drawerLyricsText) updateDynamicLyricsBox(drawerLyricsText, drawerActiveEl);
            })
            .catch((err) => {
              console.warn('Lyric romanization failed:', err);
            });
        }
      } else if (
        payload.plainLyrics &&
        payload.plainLyrics.trim().length > 0 &&
        !payload.plainLyrics.startsWith('Lyrics are not available') &&
        !payload.plainLyrics.startsWith('No lyrics found')
      ) {
        const rawPlain = payload.plainLyrics;
        const plainLines = rawPlain.split('\n');
        const hasNonLatin = hasSubstantialNonLatin(plainLines);
        const shouldRomanize = romanizeMode !== 'off' && hasNonLatin;

        let displayPlain = rawPlain;
        if (shouldRomanize && cached?.plain) {
          displayPlain = cached.plain;
        }

        const content = `${badgeHtml}<div class="lyrics-plain-text">${esc(displayPlain)}</div>`;
        if (npLyricsText) npLyricsText.innerHTML = content;
        if (drawerLyricsText) drawerLyricsText.innerHTML = content;

        if (shouldRomanize && (!cached || !cached.plain)) {
          const activeTrackId = currentLyricsTrackId;
          lyricRomanizer
            .romanizeLines(plainLines)
            .then((res) => {
              if (currentLyricsTrackId !== activeTrackId) return;
              const formattedPlain = plainLines
                .map((line, i) => {
                  const cleanedLine = sanitizeLyricText(line);
                  // If line is English, keep it original without romanization
                  if (!NON_LATIN_REGEX.test(cleanedLine)) {
                    return cleanedLine;
                  }
                  const r = cleanRomanizedText(res.lines[i] || '');
                  const hasR = r && r.toLowerCase() !== cleanedLine.trim().toLowerCase();
                  if (!hasR || romanizeMode === 'off') {
                    return cleanedLine;
                  }
                  if (romanizeMode === 'only') {
                    return r;
                  }
                  if (romanizeMode === 'main') {
                    return `${r}\n  ↳ ${cleanedLine}`;
                  }
                  return `${cleanedLine}\n  ↳ ${r}`;
                })
                .join('\n');
              const existing = romanizedLyricsCache.get(cacheKey) || {};
              existing.plain = formattedPlain;
              romanizedLyricsCache.set(cacheKey, existing);

              if (appSettings.romanizeMode === 'off') return;

              const plainEls = [
                npLyricsText?.querySelector<HTMLElement>('.lyrics-plain-text'),
                drawerLyricsText?.querySelector<HTMLElement>('.lyrics-plain-text'),
              ];
              plainEls.forEach((el) => {
                if (el) el.textContent = formattedPlain;
              });
            })
            .catch((err) => {
              console.warn('Plain lyrics romanization failed:', err);
            });
        }
      } else {
        const content = '<div class="lyrics-empty-state">Lyrics not available for this track.</div>';
        if (npLyricsText) npLyricsText.innerHTML = content;
        if (drawerLyricsText) drawerLyricsText.innerHTML = content;
      }
    }

function updateDynamicLyricsBox(container: HTMLElement, activeEl: HTMLElement | null) {
      const box = container.querySelector('.lyrics-dynamic-box') as HTMLElement | null;
      if (!box) return;

      if (!activeEl) {
        box.classList.remove('visible');
        return;
      }

      const offsetTop = activeEl.offsetTop;
      const offsetLeft = activeEl.offsetLeft;
      const width = activeEl.offsetWidth;
      const height = activeEl.offsetHeight;

      if (width > 0 && height > 0) {
        box.style.transform = `translate3d(${offsetLeft}px, ${offsetTop}px, 0)`;
        box.style.width = `${width}px`;
        box.style.height = `${height}px`;
        // Morph smoothly: capsule pill for single line, rounded card for multi-line
        box.style.borderRadius = height <= 54 ? '9999px' : '16px';
        box.classList.add('visible');
      } else {
        box.classList.remove('visible');
      }
    }

function updateSyncedLyricsProgress(currentTime: number, forceCenter = false) {
      if (!currentLyricsPayload || !currentLyricsPayload.synced || !currentLyricsPayload.lines.length) {
        return;
      }

      const lines = currentLyricsPayload.lines;
      const searchTime = currentTime + 0.15;

      let targetIdx = -1;
      for (let i = 0; i < lines.length; i++) {
        if (lines[i].timeSec <= searchTime) {
          targetIdx = i;
        } else {
          break;
        }
      }

      if (targetIdx !== activeLyricLineIdx || forceCenter) {
        activeLyricLineIdx = targetIdx;

        const pairs = [
          { textEl: npLyricsText, scrollEl: npViewLyrics?.querySelector('.lyrics-view') as HTMLElement | null },
          { textEl: drawerLyricsText, scrollEl: drawerLyricsContainer },
        ];

        pairs.forEach(({ textEl, scrollEl }) => {
          if (!textEl) return;
          const allLines = textEl.querySelectorAll('.lyric-line');
          allLines.forEach((el, idx) => {
            if (idx === targetIdx) {
              el.classList.add('active');
              el.classList.remove('past');
            } else if (idx < targetIdx) {
              el.classList.remove('active');
              el.classList.add('past');
            } else {
              el.classList.remove('active', 'past');
            }
          });

          const activeEl = targetIdx >= 0 ? (allLines[targetIdx] as HTMLElement) : null;
          updateDynamicLyricsBox(textEl, activeEl);

          if (scrollEl && activeEl) {
            const isPaused = Date.now() < userLyricsScrollPauseUntil && !forceCenter;
            if (!isPaused) {
              const containerHeight = scrollEl.clientHeight;
              const targetOffset = Math.max(40, containerHeight * 0.38);
              const targetScrollTop = activeEl.offsetTop - targetOffset + (activeEl.offsetHeight / 2);

              scrollEl.scrollTo({
                top: Math.max(0, targetScrollTop),
                behavior: forceCenter ? 'auto' : 'smooth',
              });
            }
          }
        });
      }
    }

function setupLyricsScrollInteractions() {
      const onUserScroll = () => {
        userLyricsScrollPauseUntil = Date.now() + 4000;
      };

      const npScroll = npViewLyrics?.querySelector('.lyrics-view');
      if (npScroll) {
        npScroll.addEventListener('wheel', onUserScroll, { passive: true });
        npScroll.addEventListener('touchstart', onUserScroll, { passive: true });
        npScroll.addEventListener('touchmove', onUserScroll, { passive: true });
      }

      if (drawerLyricsContainer) {
        drawerLyricsContainer.addEventListener('wheel', onUserScroll, { passive: true });
        drawerLyricsContainer.addEventListener('touchstart', onUserScroll, { passive: true });
        drawerLyricsContainer.addEventListener('touchmove', onUserScroll, { passive: true });
      }

      window.addEventListener('resize', () => {
        if (activeLyricLineIdx >= 0) {
          if (npLyricsText) {
            const el = npLyricsText.querySelectorAll('.lyric-line')?.[activeLyricLineIdx] as HTMLElement;
            if (el) updateDynamicLyricsBox(npLyricsText, el);
          }
          if (drawerLyricsText) {
            const el = drawerLyricsText.querySelectorAll('.lyric-line')?.[activeLyricLineIdx] as HTMLElement;
            if (el) updateDynamicLyricsBox(drawerLyricsText, el);
          }
        }
      });
    }

function step(d: number) {
      const n = qIndex + d;
      if (n >= 0 && n < queue.length) {
        play(queue[n], queue, n, true);
      } else if (repeatMode === 'all' && queue.length > 0) {
        play(queue[0], queue, 0, true);
      }
    }

function renderQueue() {
      queueList.innerHTML = '';
      if (!queue.length) {
        queueList.innerHTML = '<div class="msg">Your queue is empty.</div>';
        return;
      }
      renderList(queue, queueList);
      queueList.querySelectorAll('.row-item')[qIndex]?.classList.add('now');
    }

function renderOverlayQueue() {
      if (!npQueueList) return;
      npQueueList.innerHTML = '';
      if (!queue.length) {
        npQueueList.innerHTML = '<div class="msg">Your queue is empty.</div>';
        return;
      }
      renderList(queue, npQueueList);
      npQueueList.querySelectorAll('.row-item')[qIndex]?.classList.add('now');
    }

function onStatus(s: PlayerStatus) {
      if (s.track && current?.videoId !== s.track.videoId) showNow(s.track);
      if (s.isPlaying && s.currentTime > 0) {
        updateSyncedLyricsProgress(s.currentTime);
      }
    }

function showNow(t: TrackItem) {
      current = t;
      pTitle.textContent = t.title;
      pArtist.textContent = t.artist;
      thumb.src = art(t.artworkUrl);
      tDur.textContent = t.durationText || fmt(t.duration);
      fill.style.width = '0%';

      // Update Now Playing overlay
      if (npTitle) npTitle.textContent = t.title;
      if (npArtist) npArtist.textContent = t.artist;
      if (npArt) npArt.src = art(t.artworkUrl);
      if (npTDur) npTDur.textContent = t.durationText || fmt(t.duration);
      if (npProgress) npProgress.style.width = '0%';
      if (npBackdrop && t.artworkUrl) {
        npBackdrop.style.backgroundImage = `url('${esc(t.artworkUrl)}')`;
      }

      // If Adaptive Album theme is active, extract and apply artwork colors
      if (root.getAttribute('data-theme') === 'adaptive') {
        updateAdaptiveThemeFromArtwork(t.artworkUrl);
      }

      if (isVideoMode) {
        updateCoverContainerLayout();
      }
    }

function setPlay(on: boolean) {
      iconPlay.classList.toggle('hidden', on);
      iconPause.classList.toggle('hidden', !on);
      if (npIconPlay && npIconPause) {
        npIconPlay.classList.toggle('hidden', on);
        npIconPause.classList.toggle('hidden', !on);
      }
      if (!on) {
        document.title = 'Cremeplay';
      }
      syncTunaProgress(true);
    }

// Theme Handling & Palettes
interface ThemeOption {
    id: string;
    name: string;
    desc: string;
    swatches: string[];
  }

const THEMES: ThemeOption[] = [
    { id: 'light', name: 'Creme Light', desc: 'Coral & Cream', swatches: ['#F7F7F5', '#E4544B', '#1C1917'] },
    { id: 'dark', name: 'Vintage Olive', desc: 'Forest & Gold', swatches: ['#1F2319', '#7C7436', '#F2EAA7'] },
    { id: 'sapphire-heritage', name: 'Sapphire Slate', desc: 'Royal Blue & Slate', swatches: ['#2A313D', '#4B5DC3', '#FDD95F'] },
    { id: 'crimson-silk', name: 'Plum Silk', desc: 'Plum & Lavender', swatches: ['#FFEDD5', '#4A152D', '#8E93CB'] },
    { id: 'banana-goji', name: 'Banana Goji', desc: 'Banana & Red Berry', swatches: ['#F9E199', '#B61524', '#FFFDF5'] },
    { id: 'adaptive', name: 'Adaptive Art', desc: 'Matches Album Art', swatches: ['#E4544B', '#4B5DC3', '#FDD95F', '#54927D'] },
    { id: 'european-vintage', name: 'Burgundy Gold', desc: 'Burgundy & Gold', swatches: ['#602238', '#F4D864', '#EFE9D3'] },
    { id: 'chinese-vintage', name: 'Terracotta', desc: 'Terracotta & Ochre', swatches: ['#BD433E', '#DC9960', '#EBC799'] },
    { id: 'matcha-forest', name: 'Matcha Forest', desc: 'Sage & Coffee', swatches: ['#CBDD89', '#54927D', '#453832'] },
    { id: 'spring-blossom', name: 'Spring Blossom', desc: 'Sakura & Lime', swatches: ['#FC97B7', '#D1F985', '#FFE1E7'] },
    { id: 'blueberry-berry', name: 'Mulberry', desc: 'Mulberry & Sage', swatches: ['#754A70', '#86A88E', '#291E24'] },
  ];

interface ExtractedPalette {
    surface: string;
    panel: string;
    band: string;
    brand: string;
    brandText: string;
    heading: string;
    body: string;
    muted: string;
    border: string;
    hover: string;
    hoverS: string;
    cardHover: string;
    lineStrong: string;
    logoA: string;
    logoB: string;
    grad: string;
    contentBg: string;
    chipsBg: string;
  }

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
    r /= 255; g /= 255; b /= 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h = 0, s = 0;
    const l = (max + min) / 2;

    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r: h = (g - b) / d + (g < b ? 6 : 0); break;
        case g: h = (b - r) / d + 2; break;
        case b: h = (r - g) / d + 4; break;
      }
      h = Math.round(h * 60);
    }
    return [h, s, l];
  }

function buildAdaptivePalette(h1: number, s1: number, _l1: number, h2: number, s2: number): ExtractedPalette {
    const satMain = Math.max(0.55, Math.min(s1, 0.95));
    const satBg = Math.min(s1, 0.35);

    const surface = `hsl(${h1}, ${Math.round(satBg * 100)}%, 9%)`;
    const panel = `hsl(${h1}, ${Math.round(satBg * 100)}%, 14%)`;
    const band = `hsl(${h1}, ${Math.round(satBg * 100)}%, 20%)`;
    const cardHover = `hsl(${h1}, ${Math.round(satBg * 100)}%, 18%)`;

    const brand = `hsl(${h1}, ${Math.round(satMain * 100)}%, 60%)`;
    const brandText = `hsl(${h1}, ${Math.round(satMain * 100)}%, 72%)`;
    const logoA = brand;
    const logoB = `hsl(${h2}, ${Math.round(Math.max(s2, 0.6) * 100)}%, 65%)`;

    const border = `hsla(${h1}, ${Math.round(satMain * 100)}%, 65%, 0.18)`;
    const hover = `hsla(${h1}, ${Math.round(satMain * 100)}%, 65%, 0.12)`;
    const hoverS = `hsla(${h1}, ${Math.round(satMain * 100)}%, 65%, 0.07)`;
    const lineStrong = `hsla(${h1}, ${Math.round(satMain * 100)}%, 65%, 0.35)`;

    const grad = `linear-gradient(135deg, hsl(${h1}, ${Math.round(satMain * 100)}%, 60%), hsl(${h2}, ${Math.round(Math.max(s2, 0.6) * 100)}%, 54%))`;
    const contentBg = `linear-gradient(hsl(${h1}, ${Math.round(satBg * 100)}%, 22%), var(--surface) 380px)`;
    const chipsBg = `linear-gradient(hsl(${h1}, ${Math.round(satBg * 100)}%, 22%) 75%, transparent)`;

    return {
      surface,
      panel,
      band,
      brand,
      brandText,
      heading: '#FFFFFF',
      body: `hsl(${h1}, 15%, 84%)`,
      muted: `hsl(${h1}, 12%, 62%)`,
      border,
      hover,
      hoverS,
      cardHover,
      lineStrong,
      logoA,
      logoB,
      grad,
      contentBg,
      chipsBg,
    };
  }

function getDefaultAdaptivePalette(): ExtractedPalette {
    return buildAdaptivePalette(6, 0.75, 0.55, 36, 0.85); // Warm Coral / Gold
  }

const adaptiveCache = new Map<string, ExtractedPalette>();

function extractColorsFromImage(imgUrl: string): Promise<ExtractedPalette> {
    return new Promise((resolve) => {
      const fallback = getDefaultAdaptivePalette();
      if (!imgUrl || imgUrl.includes('data:image/gif') || imgUrl === PLACEHOLDER) {
        resolve(fallback);
        return;
      }

      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.onload = () => {
        try {
          const canvas = document.createElement('canvas');
          const ctx = canvas.getContext('2d', { willReadFrequently: true });
          if (!ctx) { resolve(fallback); return; }

          const size = 48;
          canvas.width = size;
          canvas.height = size;
          ctx.drawImage(img, 0, 0, size, size);
          const data = ctx.getImageData(0, 0, size, size).data;
          canvas.width = 0;
          canvas.height = 0;

          const bins: Map<string, { r: number; g: number; b: number; count: number; sat: number; lum: number; score: number }> = new Map();

          for (let i = 0; i < data.length; i += 16) {
            const r = data[i];
            const g = data[i + 1];
            const b = data[i + 2];
            const a = data[i + 3];
            if (a < 128) continue;

            const max = Math.max(r, g, b);
            const min = Math.min(r, g, b);
            const lum = (max + min) / 510;
            const sat = max === min ? 0 : (max - min) / (lum > 0.5 ? (510 - max - min) : (max + min));

            // Ignore near-black or near-white for vibrant accent determination
            if (lum < 0.08 || lum > 0.94) continue;

            const qr = Math.round(r / 24) * 24;
            const qg = Math.round(g / 24) * 24;
            const qb = Math.round(b / 24) * 24;
            const key = `${qr},${qg},${qb}`;

            const score = (sat * 2.5 + (1 - Math.abs(lum - 0.5))) * 1.5;

            const existing = bins.get(key);
            if (existing) {
              existing.count += 1;
              existing.score += score;
            } else {
              bins.set(key, { r: qr, g: qg, b: qb, count: 1, sat, lum, score });
            }
          }

          const sorted = Array.from(bins.values()).sort((a, b) => b.score - a.score);
          if (sorted.length === 0) {
            resolve(fallback);
            return;
          }

          const best = sorted[0];
          const second = sorted.find((c) => {
            const dist = Math.hypot(c.r - best.r, c.g - best.g, c.b - best.b);
            return dist > 50;
          }) || sorted[1] || best;

          const [h1, s1, l1] = rgbToHsl(best.r, best.g, best.b);
          const [h2, s2] = rgbToHsl(second.r, second.g, second.b);

          resolve(buildAdaptivePalette(h1, s1, l1, h2, s2));
        } catch (err) {
          console.warn('Adaptive palette extraction error:', err);
          resolve(fallback);
        }
      };
      img.onerror = () => resolve(fallback);
      img.src = imgUrl;
    });
  }

function applyExtractedPalette(pal: ExtractedPalette) {
    root.style.setProperty('--surface', pal.surface);
    root.style.setProperty('--panel', pal.panel);
    root.style.setProperty('--band', pal.band);
    root.style.setProperty('--brand', pal.brand);
    root.style.setProperty('--brand-text', pal.brandText);
    root.style.setProperty('--heading', pal.heading);
    root.style.setProperty('--body', pal.body);
    root.style.setProperty('--muted', pal.muted);
    root.style.setProperty('--border', pal.border);
    root.style.setProperty('--hover', pal.hover);
    root.style.setProperty('--hover-s', pal.hoverS);
    root.style.setProperty('--card-hover', pal.cardHover);
    root.style.setProperty('--line-strong', pal.lineStrong);
    root.style.setProperty('--logo-a', pal.logoA);
    root.style.setProperty('--logo-b', pal.logoB);
    root.style.setProperty('--grad', pal.grad);
    root.style.setProperty('--content-bg', pal.contentBg);
    root.style.setProperty('--chips-bg', pal.chipsBg);

    const adaptiveCard = document.querySelector('[data-theme-id="adaptive"] .theme-preview');
    if (adaptiveCard) {
      adaptiveCard.innerHTML = `
        <div class="theme-preview-dot" style="background: ${pal.brand};"></div>
        <div class="theme-preview-dot" style="background: ${pal.band};"></div>
        <div class="theme-preview-dot" style="background: ${pal.surface};"></div>
      `;
    }
  }

function clearAdaptiveInlineStyles() {
    const props = [
      '--surface', '--panel', '--band', '--brand', '--brand-text',
      '--heading', '--body', '--muted', '--border', '--hover',
      '--hover-s', '--card-hover', '--line-strong', '--logo-a',
      '--logo-b', '--grad', '--content-bg', '--chips-bg',
    ];
    props.forEach((p) => root.style.removeProperty(p));
  }

async function updateAdaptiveThemeFromArtwork(url?: string) {
    if (root.getAttribute('data-theme') !== 'adaptive') return;
    const targetUrl = url || current?.artworkUrl;
    if (!targetUrl || targetUrl.includes('data:image/gif') || targetUrl === PLACEHOLDER) {
      applyExtractedPalette(getDefaultAdaptivePalette());
      return;
    }

    if (adaptiveCache.has(targetUrl)) {
      applyExtractedPalette(adaptiveCache.get(targetUrl)!);
      return;
    }

    try {
      const pal = await extractColorsFromImage(targetUrl);
      if (adaptiveCache.size > 50) {
        const firstKey = adaptiveCache.keys().next().value;
        if (firstKey) adaptiveCache.delete(firstKey);
      }
      adaptiveCache.set(targetUrl, pal);
      if (root.getAttribute('data-theme') === 'adaptive') {
        applyExtractedPalette(pal);
      }
    } catch {
      applyExtractedPalette(getDefaultAdaptivePalette());
    }
  }

  // Periodic frontend V8 garbage collection if --expose-gc is available
  setInterval(() => {
    if (typeof (window as any).gc === 'function') {
      try {
        (window as any).gc();
      } catch {}
    }
  }, 30000);

function renderThemeSelector() {
    const container = $('theme-selector-grid');
    if (!container) return;
    const currentTheme = root.getAttribute('data-theme') || 'light';
    container.innerHTML = THEMES.map((t) => `
    <div class="theme-card ${t.id === currentTheme ? 'active' : ''}" data-theme-id="${t.id}">
      <div class="theme-preview">
        ${t.swatches.map((c) => `<div class="theme-preview-dot" style="background: ${c};"></div>`).join('')}
      </div>
      <div class="theme-info">
        <div class="theme-name">${t.name}</div>
        <div class="theme-desc">${t.desc}</div>
      </div>
    </div>
  `).join('');

    container.querySelectorAll('.theme-card').forEach((card) => {
      (card as HTMLElement).onclick = () => {
        const tid = card.getAttribute('data-theme-id');
        if (tid) {
          applyTheme(tid);
          try { localStorage.setItem('theme', tid); } catch { }
        }
      };
    });
  }

const applyTheme = (v: string) => {
    root.setAttribute('data-theme', v);
    const container = $('theme-selector-grid');
    if (container) {
      container.querySelectorAll('.theme-card').forEach((c) => {
        c.classList.toggle('active', c.getAttribute('data-theme-id') === v);
      });
    }

    if (v === 'adaptive') {
      updateAdaptiveThemeFromArtwork(current?.artworkUrl);
    } else {
      clearAdaptiveInlineStyles();
    }
  };

let savedTheme: string | null = null;
  try { savedTheme = localStorage.getItem('theme'); } catch { }
  applyTheme(savedTheme || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'));

const themeBtn = $('theme-btn');
  if (themeBtn) {
    themeBtn.onclick = () => {
      const cur = root.getAttribute('data-theme') || 'light';
      const curIdx = THEMES.findIndex((t) => t.id === cur);
      const nextTheme = THEMES[(curIdx + 1) % THEMES.length].id;
      applyTheme(nextTheme);
      try { localStorage.setItem('theme', nextTheme); } catch { }
    };
  }

  init();