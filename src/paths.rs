use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

pub const APP_QUALIFIER: &str = "com";
pub const APP_ORGANIZATION: &str = "RaydioSurfer";
pub const APP_NAME: &str = "RaydioSurfer";

pub const SETTINGS_FILENAME: &str = "settings.json";
pub const CACHE_FILENAME: &str = "stations_cache.json";

/// Returns project directories for the application if available on the current OS.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
}

/// Returns the path to the configuration directory if available.
pub fn config_dir() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.config_dir().to_path_buf())
}

/// Returns the path to the cache directory if available.
pub fn cache_dir() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.cache_dir().to_path_buf())
}

/// Returns the target path for `settings.json`.
/// If project directories are unavailable, falls back to the current working directory.
pub fn settings_path() -> PathBuf {
    if let Some(dir) = config_dir() {
        dir.join(SETTINGS_FILENAME)
    } else {
        PathBuf::from(SETTINGS_FILENAME)
    }
}

/// Returns the target path for `stations_cache.json`.
/// If project directories are unavailable, falls back to the current working directory.
pub fn cache_path() -> PathBuf {
    if let Some(dir) = cache_dir() {
        dir.join(CACHE_FILENAME)
    } else {
        PathBuf::from(CACHE_FILENAME)
    }
}

/// Helper to ensure the parent directory of a file path exists before writing.
pub fn ensure_parent_dir_exists(file_path: &Path) {
    if let Some(parent) = file_path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_not_empty() {
        let settings = settings_path();
        assert!(settings.to_string_lossy().ends_with(SETTINGS_FILENAME));

        let cache = cache_path();
        assert!(cache.to_string_lossy().ends_with(CACHE_FILENAME));
    }

    #[test]
    fn test_project_dirs_resolution() {
        if let Some(dirs) = project_dirs() {
            assert!(dirs.config_dir().to_string_lossy().contains(APP_NAME));
            assert!(dirs.cache_dir().to_string_lossy().contains(APP_NAME));
        }
    }

    #[test]
    fn test_ensure_parent_dir() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_test_dirs");
        let test_file = temp_dir.join("sub").join("test.json");
        ensure_parent_dir_exists(&test_file);
        assert!(test_file.parent().unwrap().exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
