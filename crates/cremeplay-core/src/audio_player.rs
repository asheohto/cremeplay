/*!
 * Native Rust Audio Engine (rodio 0.22 + symphonia)
 * Zero-overhead audio playback directly to Windows WASAPI sound device.
 */

use crate::innertube::TrackItem;
use crate::sponsorblock::{SponsorBlockClient, SponsorSegment};
use log::info;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

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
    speed: Arc<Mutex<f32>>,
    segments: Arc<Mutex<Vec<SponsorSegment>>>,
    sponsorblock: Arc<SponsorBlockClient>,
    selected_device_name: Arc<Mutex<Option<String>>>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    pub fn list_output_devices() -> Vec<String> {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let host = rodio::cpal::default_host();
        match host.output_devices() {
            Ok(devices) => devices.filter_map(|d| d.description().ok().map(|desc| desc.name().to_string())).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn set_output_device(&self, device_name: Option<String>) {
        *self.selected_device_name.lock().unwrap() = device_name;
        *self.device_sink.lock().unwrap() = None;
    }

    pub fn get_output_device(&self) -> Option<String> {
        self.selected_device_name.lock().unwrap().clone()
    }

    pub fn set_speed(&self, spd: f32) {
        let clamped = spd.clamp(0.25, 4.0);
        let mut speed_guard = self.speed.lock().unwrap();
        if *speed_guard != clamped {
            if self.is_playing.load(Ordering::Relaxed) {
                let mut start_guard = self.playback_start_instant.lock().unwrap();
                if let Some(start) = start_guard.take() {
                    *self.elapsed_offset.lock().unwrap() += start.elapsed().as_secs_f64() * (*speed_guard as f64);
                    *start_guard = Some(Instant::now());
                }
            }
            *speed_guard = clamped;
            if let Some(player) = self.player.lock().unwrap().as_ref() {
                player.set_speed(clamped);
            }
        }
    }

    pub fn get_speed(&self) -> f32 {
        *self.speed.lock().unwrap()
    }

    pub fn new() -> Self {
        Self {
            player: Arc::new(Mutex::new(None)),
            device_sink: Arc::new(Mutex::new(None)),
            current_track: Arc::new(Mutex::new(None)),
            is_playing: Arc::new(AtomicBool::new(false)),
            playback_start_instant: Arc::new(Mutex::new(None)),
            elapsed_offset: Arc::new(Mutex::new(0.0)),
            duration: Arc::new(Mutex::new(0.0)),
            volume: Arc::new(Mutex::new(1.0)),
            speed: Arc::new(Mutex::new(1.0)),
            segments: Arc::new(Mutex::new(Vec::new())),
            sponsorblock: Arc::new(SponsorBlockClient::new()),
            selected_device_name: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn play_stream(&self, stream_url: &str, track: TrackItem) -> Result<(), String> {
        info!("[AudioPlayer] Streaming: {} - {}", track.title, track.artist);

        // Fetch SponsorBlock segments
        let segments = self.sponsorblock.get_segments(&track.video_id).await;
        *self.segments.lock().unwrap() = segments;

        // Fetch audio stream in bounded 1MB chunks to comply with YouTube googlevideo protocol
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| e.to_string())?;

        const CHUNK_SIZE: usize = 1024 * 1024;
        let mut buffer = Vec::new();
        let mut start: usize = 0;

        loop {
            let end = start + CHUNK_SIZE - 1;
            let range_val = format!("bytes={}-{}", start, end);
            let res = client
                .get(stream_url)
                .header("Range", range_val)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch audio stream chunk: {}", e))?;

            if !res.status().is_success() && res.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                if buffer.is_empty() {
                    return Err(format!("Audio stream returned HTTP {}", res.status()));
                } else {
                    break;
                }
            }

            let content_range = res.headers().get("content-range").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
            let total_size: Option<usize> = content_range.as_ref().and_then(|cr| {
                cr.split('/').next_back().and_then(|s| s.parse().ok())
            });

            let bytes = res.bytes().await.map_err(|e| format!("Failed to read chunk bytes: {}", e))?;
            let chunk_len = bytes.len();
            if chunk_len == 0 {
                break;
            }
            buffer.extend_from_slice(&bytes);
            start += chunk_len;

            if let Some(total) = total_size
                && start >= total {
                    break;
                }
        }

        let cursor = Cursor::new(buffer);
        let source = Decoder::try_from(cursor)
            .map_err(|e| format!("Audio decode error: {:?}", e))?;

        let mut sink_guard = self.device_sink.lock().unwrap();
        if sink_guard.is_none() {
            use rodio::cpal::traits::{DeviceTrait, HostTrait};
            let selected_name = self.selected_device_name.lock().unwrap().clone();
            let opened_sink = if let Some(target_name) = selected_name {
                let host = rodio::cpal::default_host();
                let found_device = host.output_devices().ok().and_then(|mut devs| {
                    devs.find(|d| d.description().ok().map(|desc| desc.name().to_string()) == Some(target_name.clone()))
                });
                if let Some(dev) = found_device {
                    DeviceSinkBuilder::from_device(dev).and_then(|b| b.open_stream())
                } else {
                    DeviceSinkBuilder::open_default_sink()
                }
            } else {
                DeviceSinkBuilder::open_default_sink()
            };

            match opened_sink {
                Ok(s) => *sink_guard = Some(s),
                Err(e) => return Err(format!("No audio output device available: {:?}", e)),
            }
        }
        let sink = match sink_guard.as_ref() {
            Some(s) => s,
            None => return Err("Audio sink failed to initialize".to_string()),
        };

        let new_player = Player::connect_new(sink.mixer());
        let vol = *self.volume.lock().unwrap();
        new_player.set_volume(vol);
        let spd = *self.speed.lock().unwrap();
        new_player.set_speed(spd);
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
                let mut start_guard = self.playback_start_instant.lock().unwrap();
                if let Some(start) = start_guard.take() {
                    let spd = *self.speed.lock().unwrap() as f64;
                    *self.elapsed_offset.lock().unwrap() += start.elapsed().as_secs_f64() * spd;
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

    pub fn pause(&self) {
        let mut player_guard = self.player.lock().unwrap();
        if let Some(player) = player_guard.as_mut()
            && self.is_playing.load(Ordering::Relaxed) {
                player.pause();
                self.is_playing.store(false, Ordering::Relaxed);
                let mut start_guard = self.playback_start_instant.lock().unwrap();
                if let Some(start) = start_guard.take() {
                    let spd = *self.speed.lock().unwrap() as f64;
                    *self.elapsed_offset.lock().unwrap() += start.elapsed().as_secs_f64() * spd;
                }
            }
    }

    pub fn play(&self) {
        let mut player_guard = self.player.lock().unwrap();
        if let Some(player) = player_guard.as_mut()
            && !self.is_playing.load(Ordering::Relaxed) {
                player.play();
                self.is_playing.store(true, Ordering::Relaxed);
                *self.playback_start_instant.lock().unwrap() = Some(Instant::now());
            }
    }

    pub fn stop(&self) {
        let mut player_guard = self.player.lock().unwrap();
        if let Some(player) = player_guard.take() {
            player.stop();
        }
        self.is_playing.store(false, Ordering::Relaxed);
        *self.playback_start_instant.lock().unwrap() = None;
        *self.elapsed_offset.lock().unwrap() = 0.0;
        *self.current_track.lock().unwrap() = None;
    }

    pub fn seek(&self, seconds: f64) {
        let dur = *self.duration.lock().unwrap();
        let target = seconds.clamp(0.0, dur.max(0.0));
        *self.elapsed_offset.lock().unwrap() = target;
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            let _ = player.try_seek(std::time::Duration::from_secs_f64(target));
        }
        if self.is_playing.load(Ordering::Relaxed) {
            *self.playback_start_instant.lock().unwrap() = Some(Instant::now());
        } else {
            *self.playback_start_instant.lock().unwrap() = None;
        }
    }

    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
        if let Some(player) = self.player.lock().unwrap().as_ref() {
            player.set_volume(clamped);
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn get_status(&self) -> PlayerStatus {
        let playing = self.is_playing.load(Ordering::Relaxed);
        let mut current_sec = *self.elapsed_offset.lock().unwrap();
        if playing {
            if let Some(start) = *self.playback_start_instant.lock().unwrap() {
                let spd = *self.speed.lock().unwrap() as f64;
                current_sec += start.elapsed().as_secs_f64() * spd;
            }

            // Check SponsorBlock segments
            let segments = self.segments.lock().unwrap().clone();
            for seg in segments {
                let (start, end) = seg.segment;
                if current_sec >= start && current_sec < end - 0.2 {
                    info!("[SponsorBlock] Auto-skipping segment: {:.1}s -> {:.1}s", start, end);
                    *self.elapsed_offset.lock().unwrap() = end;
                    *self.playback_start_instant.lock().unwrap() = Some(Instant::now());
                    current_sec = end;
                    break;
                }
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
