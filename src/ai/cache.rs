use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CACHE_FILE: &str = ".autocommit-cache.json";
const CACHE_TTL_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    key: String,
    candidates: Vec<String>,
    created_at_seconds: u64,
}

pub(super) fn build_key(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub(super) fn load(key: &str) -> Option<Vec<String>> {
    let path = cache_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    let entry = serde_json::from_str::<CacheEntry>(&data).ok()?;
    let now = unix_timestamp().ok()?;

    if entry.key != key || !is_fresh(entry.created_at_seconds, now) {
        return None;
    }
    Some(entry.candidates)
}

pub(super) fn store(key: &str, candidates: &[String]) -> Result<()> {
    let Some(path) = cache_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory: {}", parent.display()))?;
    }

    let entry = CacheEntry {
        key: key.to_string(),
        candidates: candidates.to_vec(),
        created_at_seconds: unix_timestamp()?,
    };
    let data = serde_json::to_vec(&entry)?;
    std::fs::write(&path, data)
        .with_context(|| format!("Failed to write cache file: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn cache_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("AUTOCOMMIT_CACHE_PATH")
        && !path.is_empty()
    {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(CACHE_FILE))
}

fn unix_timestamp() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System clock is before the Unix epoch")?
        .as_secs())
}

fn is_fresh(created_at_seconds: u64, now_seconds: u64) -> bool {
    created_at_seconds <= now_seconds && now_seconds - created_at_seconds <= CACHE_TTL_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_unambiguous_and_deterministic() {
        assert_eq!(
            build_key(&["provider", "model"]),
            build_key(&["provider", "model"])
        );
        assert_ne!(build_key(&["ab", "c"]), build_key(&["a", "bc"]));
    }

    #[test]
    fn freshness_rejects_expired_and_future_entries() {
        assert!(is_fresh(100, 100 + CACHE_TTL_SECONDS));
        assert!(!is_fresh(100, 101 + CACHE_TTL_SECONDS));
        assert!(!is_fresh(101, 100));
    }
}
