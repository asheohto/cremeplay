pub mod audio_player;
pub mod config;
pub mod discord;
pub mod innertube;
pub mod lrclib;
pub mod sponsorblock;
pub mod tuna;

pub use audio_player::{AudioPlayer, PlayerStatus};
pub use config::{AppConfig, ConfigManager};
pub use discord::DiscordManager;
pub use innertube::{InnertubeClient, TrackItem, HomeSection, SearchResultPayload, PlaylistDetail, SidebarPlaylist};
pub use lrclib::{LrclibClient, LyricsPayload};
pub use sponsorblock::SponsorBlockClient;
pub use tuna::TunaManager;
