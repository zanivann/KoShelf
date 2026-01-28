// fileName: src/config.rs

//! Site configuration module.

use crate::library::MetadataLocation;
use crate::time_config::TimeConfig;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use std::fs;

/// Persistent settings structure (saved to settings.json)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub library_paths: Vec<PathBuf>,
    pub statistics_db_path: Option<PathBuf>,
    pub language: String,
    
    // Campo interno para saber onde salvar. Não é salvo no JSON.
    #[serde(skip)]
    pub config_file_path: PathBuf,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            library_paths: Vec::new(),
            statistics_db_path: None,
            language: "en".to_string(),
            // PADRÃO SIMPLES: Sempre settings.json na raiz
            config_file_path: PathBuf::from("settings.json"),
        }
    }
}

impl AppSettings {
    /// Carrega configurações de um caminho específico.
    pub fn load_from_path(path: &Path) -> Option<Self> {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<AppSettings>(&content) {
                    Ok(mut settings) => {
                        log::info!("Loaded settings from {:?}", path);
                        // Garante que sabemos de onde carregamos para salvar no mesmo lugar
                        settings.config_file_path = path.to_path_buf();
                        Some(settings)
                    },
                    Err(e) => {
                        log::error!("Failed to parse settings from {:?}: {}", path, e);
                        None
                    }
                },
                Err(e) => {
                    log::error!("Failed to read settings from {:?}: {}", path, e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Carrega do local padrão (Raiz).
    pub fn load() -> Option<Self> {
        Self::load_from_path(Path::new("settings.json"))
    }

    /// Salva as configurações no local definido em config_file_path.
    pub fn save(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&self.config_file_path, content)?;
        log::info!("Settings saved to {:?}", self.config_file_path);
        Ok(())
    }
}

/// Configuration for site generation and file watching.
#[derive(Clone)]
pub struct SiteConfig {
    pub output_dir: PathBuf,
    pub site_title: String,
    pub include_unread: bool,
    pub library_paths: Vec<PathBuf>,
    pub metadata_location: MetadataLocation,
    pub statistics_db_path: Option<PathBuf>,
    pub heatmap_scale_max: Option<u32>,
    pub time_config: TimeConfig,
    pub min_pages_per_day: Option<u32>,
    pub min_time_per_day: Option<u32>,
    pub include_all_stats: bool,
    pub is_internal_server: bool,
    pub language: String,
}