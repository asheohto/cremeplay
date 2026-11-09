use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub download_url: String,
    pub html_url: String,
    pub is_update_available: bool,
}

pub async fn check_for_updates(current_version: &str) -> Option<UpdateInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .user_agent("Mozilla/5.0 Cremeplay-Updater")
        .build()
        .ok()?;

    let resp = client
        .get("https://api.github.com/repos/asheohto/cremeplay/releases/latest")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let val: serde_json::Value = resp.json().await.ok()?;
    let raw_tag = val.get("tag_name")?.as_str()?;
    let latest_version = raw_tag.trim_start_matches('v').to_string();
    let release_notes = val.get("body").and_then(|b| b.as_str()).unwrap_or("").to_string();
    let html_url = val
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or("https://github.com/asheohto/cremeplay/releases/latest")
        .to_string();

    let mut download_url = html_url.clone();
    if let Some(assets) = val.get("assets").and_then(|a| a.as_array()) {
        for asset in assets {
            if let Some(name) = asset.get("name").and_then(|n| n.as_str()) {
                if name.ends_with(".exe") {
                    if let Some(url) = asset.get("browser_download_url").and_then(|u| u.as_str()) {
                        download_url = url.to_string();
                        break;
                    }
                }
            }
        }
    }

    let is_update_available = is_newer_version(current_version, &latest_version);

    Some(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version,
        release_notes,
        download_url,
        html_url,
        is_update_available,
    })
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse::<u32>().ok())
            .collect()
    };
    let cur_parts = parse(current);
    let lat_parts = parse(latest);

    let max_len = cur_parts.len().max(lat_parts.len());
    for i in 0..max_len {
        let cur = cur_parts.get(i).copied().unwrap_or(0);
        let lat = lat_parts.get(i).copied().unwrap_or(0);
        if lat > cur {
            return true;
        } else if lat < cur {
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.2.0", "1.2.1"));
        assert!(is_newer_version("1.2.0", "1.3.0"));
        assert!(is_newer_version("1.2.0", "2.0.0"));
        assert!(is_newer_version("v1.2.0", "v1.2.1"));

        assert!(!is_newer_version("1.2.0", "1.2.0"));
        assert!(!is_newer_version("1.2.1", "1.2.0"));
        assert!(!is_newer_version("2.0.0", "1.9.9"));
    }
}
