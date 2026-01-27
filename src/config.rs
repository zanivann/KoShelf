//! Site configuration module - bundles generator/watcher configuration.

use crate::library::MetadataLocation;
use crate::time_config::TimeConfig;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::fs;

/// Persistent settings structure (saved to settings.json)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppSettings {
    pub library_paths: Vec<PathBuf>,
    pub statistics_db_path: Option<PathBuf>,
    pub language: String,
}

impl AppSettings {
    /// Loads settings from the local JSON file
    pub fn load() -> Option<Self> {
        let path = std::path::Path::new("settings.json");
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(settings) => {
                        log::info!("Loaded settings from settings.json");
                        Some(settings)
                    },
                    Err(e) => {
                        log::error!("Failed to parse settings.json: {}", e);
                        None
                    }
                },
                Err(e) => {
                    log::error!("Failed to read settings.json: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Persists settings to the local JSON file
    pub fn save(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write("settings.json", content)?;
        log::info!("Settings saved to settings.json");
        Ok(())
    }
}

/// Configuration for site generation and file watching.
#[derive(Clone)]
pub struct SiteConfig {
    /// Output directory for the generated site
    pub output_dir: PathBuf,
    /// Title for the generated site
    pub site_title: String,
    /// Whether to include unread books
    pub include_unread: bool,
    /// Paths to library directories (books and/or comics)
    pub library_paths: Vec<PathBuf>,
    /// Where to look for KoReader metadata
    pub metadata_location: MetadataLocation,
    /// Path to the statistics database (optional)
    pub statistics_db_path: Option<PathBuf>,
    /// Maximum value for heatmap scale (optional)
    pub heatmap_scale_max: Option<u32>,
    /// Time zone configuration
    pub time_config: TimeConfig,
    /// Minimum pages per day for statistics filtering (optional)
    pub min_pages_per_day: Option<u32>,
    /// Minimum time per day in seconds for statistics filtering (optional)
    pub min_time_per_day: Option<u32>,
    /// Whether to include all stats or filter to library books only
    pub include_all_stats: bool,
    /// Whether running with internal web server (enables long-polling)
    pub is_internal_server: bool,
    /// Language for UI translations (e.g., "en_US", "de_DE")
    pub language: String,
}