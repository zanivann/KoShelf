// fileName: src/app.rs

use crate::cli::{Cli, parse_time_to_seconds};
use crate::config::{SiteConfig, AppSettings};
use crate::library::{FileWatcher, MetadataLocation};
use crate::server::{WebServer, create_version_notifier};
use crate::site_generator::SiteGenerator;
use crate::time_config::TimeConfig;
use anyhow::{Context, Result};
use log::{info, error, warn};
use tempfile::TempDir;
use std::sync::Arc;
use std::path::PathBuf;
use std::env;
use directories::ProjectDirs; // Nova dependência

enum RunMode {
    StaticExport,
    WatchStatic,
    Serve,
}

struct OutputPlan {
    output_dir: std::path::PathBuf,
    _temp_dir: Option<TempDir>,
    mode: RunMode,
}

// --- LÓGICA DE RESOLUÇÃO DE CONFIGURAÇÃO ---
fn resolve_config_path() -> PathBuf {
    // 1. Prioridade: Pasta atual (Modo Portátil ou Docker volume na raiz)
    if let Ok(current_dir) = env::current_dir() {
        let local_config = current_dir.join("settings.json");
        if local_config.exists() {
            info!("Config found in current directory: {:?}", local_config);
            return local_config;
        }
    }

    // 2. Prioridade: Pasta Padrão do Sistema (XDG no Linux, AppSupport no Mac)
    if let Some(proj_dirs) = ProjectDirs::from("com", "zanivann", "koshelf") {
        let config_dir = proj_dirs.config_dir();
        let sys_config = config_dir.join("settings.json");
        
        if sys_config.exists() {
            info!("Config found in system directory: {:?}", sys_config);
            return sys_config;
        }

        // Se nenhum existe, o padrão para SALVAR será o do sistema (mais organizado)
        // Mas se quisermos manter o comportamento "simples", podemos retornar o local.
        // Para garantir compatibilidade com Docker sem volumes complexos, vamos preferir
        // retornar o local se nada existir.
        
        // Descomente a linha abaixo se preferir salvar em ~/.config/koshelf por padrão
        // return sys_config;
    }

    // 3. Fallback: Salvar na pasta atual (padrão antigo e Docker-friendly)
    let fallback = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("settings.json");
        
    info!("No configuration found. Will use default path: {:?}", fallback);
    fallback
}

fn plan_output(cli: &Cli) -> Result<OutputPlan> {
    match (&cli.output, cli.watch) {
        (Some(dir), false) => Ok(OutputPlan {
            output_dir: dir.clone(),
            _temp_dir: None,
            mode: RunMode::StaticExport,
        }),
        (Some(dir), true) => Ok(OutputPlan {
            output_dir: dir.clone(),
            _temp_dir: None,
            mode: RunMode::WatchStatic,
        }),
        (None, _) => {
            let tmp = tempfile::tempdir().context("Failed to create temporary output directory")?;
            Ok(OutputPlan {
                output_dir: tmp.path().to_path_buf(),
                _temp_dir: Some(tmp),
                mode: RunMode::Serve,
            })
        }
    }
}

fn metadata_location(cli: &Cli) -> MetadataLocation {
    if let Some(ref docsettings_path) = cli.docsettings_path {
        MetadataLocation::DocSettings(docsettings_path.clone())
    } else if let Some(ref hashdocsettings_path) = cli.hashdocsettings_path {
        MetadataLocation::HashDocSettings(hashdocsettings_path.clone())
    } else {
        MetadataLocation::InBookFolder
    }
}

pub async fn run(cli: Cli) -> Result<()> {
    info!("Starting KOShelf...");

    // Usa a nova lógica para descobrir onde está (ou onde será criado) o arquivo
    let config_path = resolve_config_path();

    // Tenta carregar
    let saved_settings = AppSettings::load_from_path(&config_path);

    // Verifica CLI args
    let has_cli_args = !cli.library_path.is_empty() || cli.statistics_db.is_some();

    // Resolve a configuração final
    let (final_library_paths, final_stats_path, final_language) = if has_cli_args {
        info!("Configuration provided via CLI.");
        
        // Garante que o diretório pai existe antes de tentar salvar
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let settings_to_save = AppSettings {
            library_paths: cli.library_path.clone(),
            statistics_db_path: cli.statistics_db.clone(),
            language: cli.language.clone(),
            config_file_path: config_path.clone(),
        };
        
        if let Err(e) = settings_to_save.save() {
            error!("Failed to save settings: {}", e);
        }

        (cli.library_path.clone(), cli.statistics_db.clone(), cli.language.clone())
    } else if let Some(settings) = saved_settings {
        info!("Configuration loaded successfully.");
        (settings.library_paths, settings.statistics_db_path, settings.language)
    } else {
        info!("Entering Setup Mode.");
        (vec![], None, cli.language.clone())
    };

    // Validação
    let has_any_config = !final_library_paths.is_empty() || final_stats_path.is_some();
    let is_setup_mode = !has_any_config;

    cli.validate(has_any_config || is_setup_mode)?;

    let heatmap_scale_max = parse_time_to_seconds(&cli.heatmap_scale_max)?;
    let min_time_per_day = if let Some(ref t) = cli.min_time_per_day {
        parse_time_to_seconds(t)?
    } else { None };

    let time_config = TimeConfig::from_cli(&cli.timezone, &cli.day_start_time)?;
    let plan = plan_output(&cli)?;

    let config = SiteConfig {
        output_dir: plan.output_dir.clone(),
        site_title: cli.title.clone(),
        include_unread: cli.include_unread,
        library_paths: final_library_paths.clone(),
        metadata_location: metadata_location(&cli),
        statistics_db_path: final_stats_path.clone(),
        heatmap_scale_max,
        time_config: time_config.clone(),
        min_pages_per_day: cli.min_pages_per_day,
        min_time_per_day,
        include_all_stats: cli.include_all_stats,
        is_internal_server: matches!(plan.mode, RunMode::Serve),
        language: final_language.clone(),
    };

    let site_generator = SiteGenerator::new(config.clone());
    site_generator.generate().await?;
    
    let scanner = crate::library::scanner::Scanner::new(config.clone());
    let library_items = Arc::new(scanner.scan().await?);

    match plan.mode {
        RunMode::StaticExport => Ok(()),
        RunMode::WatchStatic => {
            let file_watcher = FileWatcher::new(config, None);
            file_watcher.run().await.map_err(|e| { error!("{}", e); e })?;
            Ok(())
        }
        RunMode::Serve => {
            let version_notifier = create_version_notifier();
            let file_watcher = FileWatcher::new(config.clone(), Some(version_notifier.clone()));
            
            let library_path_root = final_library_paths.first().cloned().unwrap_or_else(|| PathBuf::from("."));

            let web_server = WebServer::new(
                plan.output_dir, 
                cli.port, 
                version_notifier, 
                library_items,
                library_path_root,
                final_stats_path,
                config_path // Passa o caminho resolvido dinamicamente
            );

            info!("Server mode active. Port: {}", cli.port);
            if is_setup_mode {
                info!("SETUP REQUIRED: Visit http://localhost:{}/ to configure your library.", cli.port);
            }

            tokio::select! {
                res = file_watcher.run() => { if let Err(e) = res { error!("Watcher: {}", e); } }
                res = web_server.run() => { if let Err(e) = res { error!("Server: {}", e); } }
            }
            Ok(())
        }
    }
}