/**
 * Native Rust Audio Engine (rodio 0.22 + symphonia)
 * Zero-overhead audio playback directly to Windows WASAPI sound device.
 */

use crate::innertube::TrackItem;
use crate::sponsorblock::{SponsorBlockClient, SponsorSegment};
use log::{error, info};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatus {
    pub is_playing: bool,
    pub current_time: f64,
    pub duration: f64,
    pub volume: f32,
    pub track: Option<TrackItem>,
}

pub struct AudioPlayer {
    player: Arc<Mutex<Option<Player>>>,
    device_sink: Arc<Mutex<Option<MixerDeviceSink>>>,
    current_track: Arc<Mutex<Option<TrackItem>>>,
    is_playing: Arc<AtomicBool>,
    playback_start_instant: Arc<Mutex<Option<Instant>>>,
    elapsed_offset: Arc<Mutex<f64>>,
    duration: Arc<Mutex<f64>>,
    volume: Arc<Mutex<f32>>,
    segments: Arc<Mutex<Vec<SponsorSegment>>>,
    sponsorblock: Arc<SponsorBlockClient>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let sink = match DeviceSinkBuilder::open_default_sink() {
            Ok(s) => Some(s),
            Err(e) => {
                error!("[AudioPlayer] Failed to open default audio output: {:?}", e);
                None
            }
        };

        Self {
            player: Arc::new(Mutex::new(None)),
            device_sink: Arc::new(Mutex::new(sink)),
            current_track: Arc::new(Mutex::new(None)),
            is_playing: Arc::new(AtomicBool::new(false)),
            playback_start_instant: Arc::new(Mutex::new(None)),
            elapsed_offset: Arc::new(Mutex::new(0.0)),
            duration: Arc::new(Mutex::new(0.0)),
            volume: Arc::new(Mutex::new(1.0)),
            segments: Arc::new(Mutex::new(Vec::new())),
            sponsorblock: Arc::new(SponsorBlockClient::new()),
        }
    }

    pub fn start_progress_ticker(&self, app: AppHandle) {
        let is_playing = self.is_playing.clone();
        let elapsed_offset = self.elapsed_offset.clone();
        let playback_start_instant = self.playback_start_instant.clone();
        let duration_arc = self.duration.clone();
        let volume_arc = self.volume.clone();
        let current_track = self.current_track.clone();
        let segments_arc = self.segments.clone();

        tauri::async_runtime::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(250));
            loop {
                ticker.tick().await;

                let playing = is_playing.load(Ordering::Relaxed);
                if playing {
                    let mut current_sec = *elapsed_offset.lock().unwrap();
                    if let Some(start) = *playback_start_instant.lock().unwrap() {
                        current_sec += start.elapsed().as_secs_f64();
                    }

                    // Check SponsorBlock segments
                    let segments = segments_arc.lock().unwrap().clone();
                    for seg in segments {
                        let (start, end) = seg.segment;
                        if current_sec >= start && current_sec < end - 0.2 {
                            info!("[SponsorBlock] Auto-skipping segment: {:.1}s -> {:.1}s", start, end);
                            *elapsed_offset.lock().unwrap() = end;
                            *playback_start_instant.lock().unwrap() = Some(Instant::now());
                            current_sec = end;
                            break;
                        }
                    }

                    let dur = *duration_arc.lock().unwrap();
                    let vol = *volume_arc.lock().unwrap();
                    let track = current_track.lock().unwrap().clone();

                    let status = PlayerStatus {
                        is_playing: playing,
                        current_time: current_sec,
                        duration: dur,
                        volume: vol,
                        track,
                    };

                    let _ = app.emit("player_status", status);
                }
            }
        });
    }

    pub async fn play_stream(&self, stream_url: &str, track: TrackItem) -> Result<(), String> {
        info!("[AudioPlayer] Streaming: {} - {}", track.title, track.artist);

        // Fetch SponsorBlock segments
        let segments = self.sponsorblock.get_segments(&track.video_id).await;
        *self.segments.lock().unwrap() = segments;

        // Download audio stream into memory
        let res = reqwest::get(stream_url)
            .await
            .map_err(|e| format!("Failed to fetch audio stream: {}", e))?;

        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("Failed to read stream bytes: {}", e))?;

        let cursor = Cursor::new(bytes.to_vec());
        let source = Decoder::try_from(cursor)
            .map_err(|e| format!("Audio decode error: {:?}", e))?;

        let sink_guard = self.device_sink.lock().unwrap();
        let sink = sink_guard
            .as_ref()
            .ok_or_else(|| "No audio output device available".to_string())?;

        let new_player = Player::connect_new(&sink.mixer());
        let vol = *self.volume.lock().unwrap();
        new_player.set_volume(vol);
        new_player.append(source);
        new_player.play();

        // Update state
        *self.duration.lock().unwrap() = track.duration;
        *self.elapsed_offset.lock().unwrap() = 0.0;
        *self.playback_start_instant.lock().unwrap() = Some(Instant::now());
        *self.current_track.lock().unwrap() = Some(track);
        self.is_playing.store(true, Ordering::Relaxed);

        let mut player_guard = self.player.lock().unwrap();
        if let Some(old_player) = player_guard.take() {
            old_player.stop();
        }
        *player_guard = Some(new_player);

        Ok(())
    }

    pub fn toggle_playback(&self) -> bool {
        let mut player_guard = self.player.lock().unwrap();
        if let Some(player) = player_guard.as_mut() {
            let was_playing = self.is_playing.load(Ordering::Relaxed);
            if was_playing {
                player.pause();
                self.is_playing.store(false, Ordering::Relaxed);
                if let Some(start) = *self.playback_start_instant.lock().unwrap() {
                    *self.elapsed_offset.lock().unwrap() += start.elapsed().as_secs_f64();
                    *self.playback_start_instant.lock().unwrap() = None;
                }
                false
            } else {
                player.play();
                self.is_playing.store(true, Ordering::Relaxed);
                *self.playback_start_instant.lock().unwrap() = Some(Instant::now());
                true
            }
        } else {
            false
        }
    }

    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.set_volume(clamped);
        }
    }

    pub fn get_status(&self) -> PlayerStatus {
        let playing = self.is_playing.load(Ordering::Relaxed);
        let mut current_sec = *self.elapsed_offset.lock().unwrap();
        if playing {
            if let Some(start) = *self.playback_start_instant.lock().unwrap() {
                current_sec += start.elapsed().as_secs_f64();
            }
        }

        PlayerStatus {
            is_playing: playing,
            current_time: current_sec,
            duration: *self.duration.lock().unwrap(),
            volume: *self.volume.lock().unwrap(),
            track: self.current_track.lock().unwrap().clone(),
        }
    }
}

