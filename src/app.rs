use crate::cli::{Cli, parse_time_to_seconds};
use crate::config::{SiteConfig, AppSettings};
use crate::library::{FileWatcher, MetadataLocation};
use crate::server::{WebServer, create_version_notifier};
use crate::site_generator::SiteGenerator;
use crate::time_config::TimeConfig;
use anyhow::{Context, Result};
use log::{info, error};
use tempfile::TempDir;
use std::sync::Arc;
use std::path::PathBuf;

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

    // 1. Load settings from JSON (if exists)
    let saved_settings = AppSettings::load();

    // 2. Check if we have CLI args that override settings
    // We consider it an override if library_path is provided OR statistics_db is set via CLI.
    let has_cli_args = !cli.library_path.is_empty() || cli.statistics_db.is_some();

    // 3. Resolve final configuration priority: CLI > Saved JSON > Setup Mode (Empty)
    let (final_library_paths, final_stats_path, final_language) = if has_cli_args {
        // Option A: Configuration provided via CLI. Use it and SAVE it.
        info!("Configuration provided via CLI.");
        
        let settings_to_save = AppSettings {
            library_paths: cli.library_path.clone(),
            statistics_db_path: cli.statistics_db.clone(),
            language: cli.language.clone(),
        };
        
        // Persist for next run
        if let Err(e) = settings_to_save.save() {
            error!("Failed to save settings: {}", e);
        }

        (cli.library_path.clone(), cli.statistics_db.clone(), cli.language.clone())
    } else if let Some(settings) = saved_settings {
        // Option B: No CLI args, but we have saved settings. Use them.
        info!("Configuration loaded from settings.json.");
        (settings.library_paths, settings.statistics_db_path, settings.language)
    } else {
        // Option C: No CLI args, no settings.json. Enter Setup Mode.
        info!("No configuration found. Entering Setup Mode.");
        (vec![], None, cli.language.clone())
    };

    // 4. Validate inputs.
    // FIX: We pass 'true' (allow_empty) if:
    // a) We are strictly in Setup Mode (no paths found anywhere)
    // b) We ALREADY have configuration (from JSON), so CLI args being empty is fine.
    let has_any_config = !final_library_paths.is_empty() || final_stats_path.is_some();
    let is_setup_mode = !has_any_config;

    // The validator should allow empty CLI args if we already have config from JSON OR if we want Setup Mode.
    cli.validate(has_any_config || is_setup_mode)?;

    let heatmap_scale_max = parse_time_to_seconds(&cli.heatmap_scale_max)?;
    let min_time_per_day = if let Some(ref t) = cli.min_time_per_day {
        parse_time_to_seconds(t)?
    } else { None };

    let time_config = TimeConfig::from_cli(&cli.timezone, &cli.day_start_time)?;
    let plan = plan_output(&cli)?;

    // Construct the Config object using the RESOLVED paths, not just CLI defaults
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
    
    // Scanner might return empty if no paths (Setup Mode), which is expected.
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
            
            // For the raw file server, use the first library path or fallback to current dir
            let library_path_root = final_library_paths.first().cloned().unwrap_or_else(|| PathBuf::from("."));

            let web_server = WebServer::new(
                plan.output_dir, 
                cli.port, 
                version_notifier, 
                library_items,
                library_path_root,
                final_stats_path
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