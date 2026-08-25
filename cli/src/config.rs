use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Folder to write capture files
    pub folder: PathBuf,
    /// Polling interval in seconds
    pub interval_secs: u64,
    /// Minimum dwell time in seconds before a block is kept
    pub min_dwell_secs: i64,
    /// Jaccard similarity threshold for block segmentation
    pub similarity_threshold: f64,
    /// Path to the accessibility helper binary
    pub ax_helper_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        let folder = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Ambient Context");

        Config {
            folder,
            interval_secs: 5,
            min_dwell_secs: 10,
            similarity_threshold: 0.5,
            ax_helper_path: None,
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("ambient-context")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn pid_path() -> PathBuf {
    config_dir().join("daemon.pid")
}

pub fn status_path() -> PathBuf {
    config_dir().join("status.json")
}

pub fn log_path() -> PathBuf {
    config_dir().join("daemon.log")
}

pub fn load() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }

    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Config::default(),
    };

    toml::from_str(&contents).unwrap_or_default()
}

#[allow(dead_code)]
pub fn save(config: &Config) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, contents)
}

pub fn is_icloud_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("Mobile Documents")
        || s.contains("com~apple~CloudDocs")
        || s.contains("iCloud Drive")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_reasonable_values() {
        let config = Config::default();
        assert_eq!(config.interval_secs, 5);
        assert_eq!(config.min_dwell_secs, 10);
        assert_eq!(config.similarity_threshold, 0.5);
        assert!(config.folder.to_string_lossy().contains("Ambient Context"));
    }

    #[test]
    fn roundtrip_toml() {
        let config = Config {
            folder: PathBuf::from("/tmp/test"),
            interval_secs: 10,
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.interval_secs, 10);
    }

    #[test]
    fn detects_icloud_paths() {
        assert!(is_icloud_path(Path::new(
            "/Users/x/Library/Mobile Documents/com~apple~CloudDocs/Notes"
        )));
        assert!(!is_icloud_path(Path::new("/Users/x/Ambient Context")));
    }
}
