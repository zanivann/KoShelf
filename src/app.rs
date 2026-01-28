// fileName: src/app.rs

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

// A função run precisa ser pública para ser acessada pelo main.rs (via lib.rs)
pub async fn run(cli: Cli) -> Result<()> {
    info!("Starting KOShelf...");

    // 1. Lógica Inteligente de Carga:
    // Se o CLI tiver --statistics-db, calculamos que o settings.json deve estar na mesma pasta.
    // Se não, assume o diretório atual (comportamento padrão).
    let config_path = AppSettings::get_config_path(cli.statistics_db.as_deref());
    
    // Tenta carregar as configurações desse caminho específico
    let saved_settings = AppSettings::load_from_path(&config_path);

    if saved_settings.is_some() {
        info!("Loaded configuration from {:?}", config_path);
    }

    // 2. Verifica se há argumentos de CLI que sobrescrevem as configurações salvas
    let has_cli_args = !cli.library_path.is_empty() || cli.statistics_db.is_some();

    // 3. Resolve a configuração final (Prioridade: CLI > Arquivo Salvo > Padrão/Setup)
    let (final_library_paths, final_stats_path, final_language) = if has_cli_args {
        // Opção A: Configuração via CLI. Usamos e SALVAMOS.
        info!("Configuration provided via CLI.");
        
        let settings_to_save = AppSettings {
            library_paths: cli.library_path.clone(),
            statistics_db_path: cli.statistics_db.clone(),
            language: cli.language.clone(),
        };
        
        // Persiste para a próxima execução.
        // O método .save() (que está no config.rs) sabe onde salvar baseado no statistics_db_path.
        if let Err(e) = settings_to_save.save() {
            error!("Failed to save settings: {}", e);
        }

        (cli.library_path.clone(), cli.statistics_db.clone(), cli.language.clone())
    } else if let Some(settings) = saved_settings {
        // Opção B: Sem argumentos de CLI, mas temos configurações salvas. Usamos elas.
        (settings.library_paths, settings.statistics_db_path, settings.language)
    } else {
        // Opção C: Nada encontrado. Entra em Modo Setup.
        info!("No configuration found at {:?}. Entering Setup Mode.", config_path);
        (vec![], None, cli.language.clone())
    };

    // 4. Validação
    // Permitimos caminhos vazios SE estivermos em modo setup
    let has_any_config = !final_library_paths.is_empty() || final_stats_path.is_some();
    let is_setup_mode = !has_any_config;

    cli.validate(has_any_config || is_setup_mode)?;

    let heatmap_scale_max = parse_time_to_seconds(&cli.heatmap_scale_max)?;
    let min_time_per_day = if let Some(ref t) = cli.min_time_per_day {
        parse_time_to_seconds(t)?
    } else { None };

    let time_config = TimeConfig::from_cli(&cli.timezone, &cli.day_start_time)?;
    let plan = plan_output(&cli)?;

    // Constrói o objeto Config final
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
    
    // O scanner pode retornar vazio se estivermos no modo Setup
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
            
            // Para o servidor de arquivos brutos, usa o primeiro caminho da biblioteca ou fallback para "."
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