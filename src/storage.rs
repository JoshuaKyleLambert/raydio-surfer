use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Helper to ensure the parent directory of a file path exists before writing.
pub fn ensure_parent_dir_exists(file_path: &Path) {
    if let Some(parent) = file_path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }
}

/// Checks whether a cache file at the specified path exists and has not exceeded `max_age`.
pub fn is_cache_valid(path: &Path, max_age: Duration) -> bool {
    if !path.exists() {
        return false;
    }

    if let Ok(metadata) = fs::metadata(path)
        && let Ok(modified) = metadata.modified()
        && let Ok(age) = SystemTime::now().duration_since(modified)
    {
        age <= max_age
    } else {
        false
    }
}

/// Loads and deserializes JSON content from a file path.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Loads and deserializes JSON content from a primary path, falling back to an optional fallback path if missing or failed.
pub fn load_json_with_fallback<T: DeserializeOwned>(
    primary_path: &Path,
    fallback_path: Option<&Path>,
) -> Option<T> {
    if let Some(data) = load_json::<T>(primary_path) {
        return Some(data);
    }

    if let Some(fallback) = fallback_path
        && let Some(data) = load_json::<T>(fallback)
    {
        return Some(data);
    }

    None
}

/// Serializes and writes a value as compact JSON to a destination path, ensuring parent directories exist.
pub fn save_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    ensure_parent_dir_exists(path);
    fs::write(path, json)
}

/// Serializes and writes a value as pretty-printed JSON to a destination path, ensuring parent directories exist.
pub fn save_json_pretty<T: Serialize + ?Sized>(path: &Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    ensure_parent_dir_exists(path);
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs::File;

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
    struct DummyConfig {
        name: String,
        count: u32,
    }

    #[test]
    fn test_save_and_load_json_pretty() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_storage_test_1");
        let file_path = temp_dir.join("config.json");

        let sample = DummyConfig {
            name: "Test Radio".into(),
            count: 42,
        };

        let save_res = save_json_pretty(&file_path, &sample);
        assert!(save_res.is_ok());
        assert!(file_path.exists());

        let loaded: Option<DummyConfig> = load_json(&file_path);
        assert_eq!(loaded, Some(sample));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_and_load_json_compact() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_storage_test_2");
        let file_path = temp_dir.join("cache.json");

        let sample = DummyConfig {
            name: "Cache Item".into(),
            count: 100,
        };

        let save_res = save_json(&file_path, &sample);
        assert!(save_res.is_ok());

        let loaded: Option<DummyConfig> = load_json(&file_path);
        assert_eq!(loaded, Some(sample));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_json_with_fallback() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_storage_test_3");
        let primary = temp_dir.join("missing.json");
        let fallback = temp_dir.join("fallback.json");

        let sample = DummyConfig {
            name: "Fallback Item".into(),
            count: 7,
        };

        save_json(&fallback, &sample).unwrap();

        let loaded: Option<DummyConfig> = load_json_with_fallback(&primary, Some(&fallback));
        assert_eq!(loaded, Some(sample));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_cache_valid() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_storage_test_4");
        let file_path = temp_dir.join("valid_cache.json");
        ensure_parent_dir_exists(&file_path);
        File::create(&file_path).unwrap();

        assert!(is_cache_valid(&file_path, Duration::from_secs(60)));
        assert!(!is_cache_valid(&file_path, Duration::from_secs(0)));

        let missing = temp_dir.join("missing_cache.json");
        assert!(!is_cache_valid(&missing, Duration::from_secs(60)));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
