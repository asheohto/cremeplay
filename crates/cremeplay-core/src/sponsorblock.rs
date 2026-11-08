use log::{debug, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorSegment {
    pub category: String,
    pub segment: (f64, f64),
}

#[derive(Deserialize)]
struct RawSegment {
    category: String,
    segment: (f64, f64),
}

pub struct SponsorBlockClient {
    client: Client,
}

impl SponsorBlockClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    pub async fn get_segments(&self, video_id: &str) -> Vec<SponsorSegment> {
        let url = format!(
            "https://sponsor.ajay.app/api/skipSegments?videoID={}&categories=%5B%22sponsor%22%2C%22intro%22%2C%22outro%22%2C%22music_offtopic%22%2C%22preview%22%5D",
            video_id
        );

        match self.client.get(&url).send().await {
            Ok(res) if res.status().is_success() => {
                if let Ok(raw_list) = res.json::<Vec<RawSegment>>().await {
                    let segments: Vec<SponsorSegment> = raw_list
                        .into_iter()
                        .map(|r| SponsorSegment {
                            category: r.category,
                            segment: r.segment,
                        })
                        .collect();
                    info!(
                        "[SponsorBlock] Loaded {} segments for {}",
                        segments.len(),
                        video_id
                    );
                    return segments;
                }
            }
            _ => {
                debug!("[SponsorBlock] No segments for {}", video_id);
            }
        }

        Vec::new()
    }
}

