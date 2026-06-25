/// Update check — queries crates.io for latest version.
use std::time::{SystemTime, UNIX_EPOCH};
const CACHE_DURATION: u64 = 86400;

#[derive(serde::Deserialize)]
struct CrateInfo {
    max_version: String,
}
#[derive(serde::Deserialize)]
struct ApiResponse {
    #[serde(rename = "crate")]
    info: CrateInfo,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Cache {
    versions: std::collections::HashMap<String, Cached>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Cached {
    latest: String,
    checked_at: u64,
}

pub fn check_update(crate_name: &str, current_version: &str) -> Option<String> {
    let cache = load_cache();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(cached) = cache.versions.get(crate_name) {
        if now - cached.checked_at < CACHE_DURATION {
            return compare_versions(current_version, &cached.latest);
        }
    }
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    if let Ok(resp) = ureq::get(&url).call() {
        if let Ok(body) = resp.into_body().read_to_string() {
            if let Ok(data) = serde_json::from_str::<ApiResponse>(&body) {
                let latest = data.info.max_version;
                let mut cache = load_cache();
                cache.versions.insert(
                    crate_name.to_string(),
                    Cached {
                        latest: latest.clone(),
                        checked_at: now,
                    },
                );
                save_cache(&cache);
                return compare_versions(current_version, &latest);
            }
        }
    }
    None
}

fn compare_versions(current: &str, latest: &str) -> Option<String> {
    if current == latest {
        return None;
    }
    let c: Vec<u32> = current.split('.').filter_map(|p| p.parse().ok()).collect();
    let l: Vec<u32> = latest.split('.').filter_map(|p| p.parse().ok()).collect();
    for i in 0..3 {
        if l.get(i).unwrap_or(&0) > c.get(i).unwrap_or(&0) {
            return Some(format!(
                "⚠  v{} → v{} (update available)\n   Run: cargo install mneme-brain --force",
                current, latest
            ));
        }
        if c.get(i).unwrap_or(&0) > l.get(i).unwrap_or(&0) {
            return None;
        }
    }
    None
}

fn cache_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mneme")
        .join("update-cache.json")
}

fn load_cache() -> Cache {
    let p = cache_path();
    if p.exists() {
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Cache {
                versions: std::collections::HashMap::new(),
            })
    } else {
        Cache {
            versions: std::collections::HashMap::new(),
        }
    }
}

fn save_cache(c: &Cache) {
    if let Some(parent) = cache_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string(c) {
        let _ = std::fs::write(cache_path(), content);
    }
}

pub fn print_update_status(crate_name: &str, current_version: &str) {
    if let Some(msg) = check_update(crate_name, current_version) {
        println!("{}", msg);
    }
}
