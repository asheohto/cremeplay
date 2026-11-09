use rune::Icon;

pub static NAV_BACK: Icon = Icon {
    name: "cp-nav-back",
    view: 24.0,
    body: r#"<path d="M19 12H5M12 19l-7-7 7-7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static SEARCH: Icon = Icon {
    name: "cp-search",
    view: 24.0,
    body: r#"<circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M20 20l-3.5-3.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static SEARCH_CLEAR: Icon = Icon {
    name: "cp-search-clear",
    view: 24.0,
    body: r#"<line x1="18" y1="6" x2="6" y2="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><line x1="6" y1="6" x2="18" y2="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static THEME: Icon = Icon {
    name: "cp-theme",
    view: 24.0,
    body: r#"<path d="M12 21C6.5 21 2 16.5 2 11C2 5.5 6.5 2 12 2C17.5 2 22 6.5 22 12C22 14.5 20.5 16 18.5 16C17.5 16 16.5 17.5 16.5 17.5C16.5 19 14.5 21 12 21Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><circle cx="7" cy="8.5" r="1.5" fill="currentColor"/><circle cx="12" cy="6.5" r="1.5" fill="currentColor"/><circle cx="16.5" cy="8.5" r="1.5" fill="currentColor"/>"#,
};

pub static SETTINGS: Icon = Icon {
    name: "cp-settings",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static HOME: Icon = Icon {
    name: "cp-home",
    view: 24.0,
    body: r#"<path d="M3 10.5L12 3l9 7.5V20a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 20v-9.5z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M9 21v-7h6v7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static EXPLORE: Icon = Icon {
    name: "cp-explore",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76" fill="currentColor" fill-opacity="0.2"/>"#,
};

pub static LIBRARY: Icon = Icon {
    name: "cp-library",
    view: 24.0,
    body: r#"<path d="M9 18V5l12-2v13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><circle cx="6" cy="18" r="3" fill="none" stroke="currentColor" stroke-width="1.8"/><circle cx="18" cy="16" r="3" fill="none" stroke="currentColor" stroke-width="1.8"/>"#,
};

pub static NEW_PLAYLIST: Icon = Icon {
    name: "cp-new-playlist",
    view: 24.0,
    body: r#"<line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static HEART: Icon = Icon {
    name: "cp-heart",
    view: 24.0,
    body: r#"<path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static HEART_FILLED: Icon = Icon {
    name: "cp-heart-filled",
    view: 24.0,
    body: r#"<path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" fill="currentColor"/>"#,
};

pub static SHUFFLE: Icon = Icon {
    name: "cp-shuffle",
    view: 24.0,
    body: r#"<polyline points="16 3 21 3 21 8" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="20" x2="21" y2="3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><polyline points="21 16 21 21 16 21" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="15" y1="15" x2="21" y2="21" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="4" x2="9" y2="9" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static PREV: Icon = Icon {
    name: "cp-prev",
    view: 24.0,
    body: r#"<polygon points="19 20 9 12 19 4 19 20" fill="currentColor" fill-opacity="0.15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="5" y1="19" x2="5" y2="5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static PLAY: Icon = Icon {
    name: "cp-play",
    view: 24.0,
    body: r#"<polygon points="6 4 20 12 6 20 6 4" fill="currentColor"/>"#,
};

pub static PAUSE: Icon = Icon {
    name: "cp-pause",
    view: 24.0,
    body: r#"<rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor"/><rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor"/>"#,
};

pub static NEXT: Icon = Icon {
    name: "cp-next",
    view: 24.0,
    body: r#"<polygon points="5 4 15 12 5 20 5 4" fill="currentColor" fill-opacity="0.15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="19" y1="5" x2="19" y2="19" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static REPEAT: Icon = Icon {
    name: "cp-repeat",
    view: 24.0,
    body: r#"<polyline points="17 1 21 5 17 9" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M3 11V9a4 4 0 0 1 4-4h14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><polyline points="7 23 3 19 7 15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M21 13v2a4 4 0 0 1-4 4H3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static VOLUME_HIGH: Icon = Icon {
    name: "cp-vol-high",
    view: 24.0,
    body: r#"<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" fill="currentColor" fill-opacity="0.15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static VOLUME_MUTE: Icon = Icon {
    name: "cp-vol-mute",
    view: 24.0,
    body: r#"<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" fill="currentColor" fill-opacity="0.15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="23" y1="9" x2="17" y2="15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="17" y1="9" x2="23" y2="15" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static QUEUE: Icon = Icon {
    name: "cp-queue",
    view: 24.0,
    body: r#"<line x1="4" y1="6" x2="20" y2="6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="12" x2="14" y2="12" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="18" x2="11" y2="18" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><circle cx="18" cy="15" r="3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="21" y1="15" x2="21" y2="9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static MUSIC_NOTE: Icon = Icon {
    name: "cp-music-note",
    view: 24.0,
    body: r#"<path d="M9 18V5l12-2v13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><circle cx="6" cy="18" r="3" fill="none" stroke="currentColor" stroke-width="1.8"/><circle cx="18" cy="16" r="3" fill="none" stroke="currentColor" stroke-width="1.8"/>"#,
};

pub static CLOSE: Icon = Icon {
    name: "cp-close",
    view: 24.0,
    body: r#"<path d="M18 6L6 18M6 6l12 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static CHEVRON_DOWN: Icon = Icon {
    name: "cp-chevron-down",
    view: 24.0,
    body: r#"<polyline points="6 9 12 15 18 9" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static CHEVRON_UP: Icon = Icon {
    name: "cp-chevron-up",
    view: 24.0,
    body: r#"<polyline points="18 15 12 9 6 15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static CLOCK: Icon = Icon {
    name: "cp-clock",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><path d="M12 7v5l3 3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#,
};
