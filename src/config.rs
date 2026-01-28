//! Site configuration module - bundles generator/watcher configuration.

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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            library_paths: Vec::new(),
            statistics_db_path: None,
            language: "en".to_string(),
        }
    }
}

impl AppSettings {
    /// Helper para determinar o caminho do arquivo de configurações.
    /// Se tivermos um caminho de DB, o settings.json deve ficar na mesma pasta (pai).
    /// Caso contrário, usamos o diretório atual.
    pub fn get_config_path(base_path: Option<&Path>) -> PathBuf {
        match base_path {
            Some(p) => {
                // Se o caminho apontar para um arquivo (ex: /dados/stats.sqlite),
                // pegamos o diretório pai (/dados/) e juntamos com settings.json.
                if p.is_file() || p.extension().is_some() {
                    p.parent()
                        .map(|parent| if parent.as_os_str().is_empty() { Path::new(".") } else { parent })
                        .unwrap_or(Path::new("."))
                        .join("settings.json")
                } else {
                    // Se já for um diretório (raro para o DB, mas possível)
                    p.join("settings.json")
                }
            },
            None => PathBuf::from("settings.json")
        }
    }

    /// Carrega configurações de um caminho específico.
    pub fn load_from_path(path: &Path) -> Option<Self> {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(settings) => {
                        log::info!("Loaded settings from {:?}", path);
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

    /// Carrega configurações do local padrão (Raiz).
    pub fn load() -> Option<Self> {
        Self::load_from_path(Path::new("settings.json"))
    }

    /// Salva as configurações.
    /// A inteligência de ONDE salvar está aqui: usa statistics_db_path como âncora.
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_config_path(self.statistics_db_path.as_deref());
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        
        log::info!("Settings saved to {:?}", config_path);
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