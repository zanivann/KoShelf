use crate::cli::{Cli, parse_time_to_seconds};
use crate::config::SiteConfig;
use crate::library::{FileWatcher, MetadataLocation};
use crate::server::{WebServer, create_version_notifier};
use crate::site_generator::SiteGenerator;
use crate::time_config::TimeConfig;
use anyhow::{Context, Result};
use log::{info, error};
use tempfile::TempDir;
use std::sync::Arc;

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
    cli.validate()?;

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
        library_paths: cli.library_path.clone(),
        metadata_location: metadata_location(&cli),
        statistics_db_path: cli.statistics_db.clone(),
        heatmap_scale_max,
        time_config: time_config.clone(),
        min_pages_per_day: cli.min_pages_per_day,
        min_time_per_day,
        include_all_stats: cli.include_all_stats,
        is_internal_server: matches!(plan.mode, RunMode::Serve),
        language: cli.language.clone(),
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
            let library_path = cli.library_path.first().cloned().unwrap_or_default();

            // CHANGE: Pass cli.statistics_db (the 6th argument)
            let web_server = WebServer::new(
                plan.output_dir, 
                cli.port, 
                version_notifier, 
                library_items,
                library_path,
                cli.statistics_db // <--- PASS HERE
            );

            info!("Server mode active. Port: {}", cli.port);

            tokio::select! {
                res = file_watcher.run() => { if let Err(e) = res { error!("Watcher: {}", e); } }
                res = web_server.run() => { if let Err(e) = res { error!("Server: {}", e); } }
            }
            Ok(())
        }
    }
}