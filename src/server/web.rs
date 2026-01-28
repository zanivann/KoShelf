use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer, middleware, HttpResponse, Responder};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use serde::Deserialize;

use crate::models::LibraryItem;
use crate::server::version::VersionNotifier;
use crate::i18n::{get_available_locales, set_global_locale};
use crate::config::AppSettings;

pub struct WebServer {
    output_dir: PathBuf,
    port: u16,
    version_notifier: Arc<VersionNotifier>,
    library_items: Arc<Vec<LibraryItem>>,
    library_path: PathBuf,
    statistics_db_path: Option<PathBuf>, 
}

#[derive(Deserialize)]
struct LanguagePayload {
    lang: String,
}

#[derive(Deserialize)]
struct SetupPayload {
    library_paths: Vec<String>,
    statistics_db_path: Option<String>,
    language: String,
}

#[derive(Deserialize)]
struct BrowseQuery {
    path: Option<String>,
}

// --- HANDLERS ---

async fn index_handler(output_dir: web::Data<PathBuf>) -> impl Responder {
    let index_path = output_dir.get_ref().join("index.html");
    if !index_path.exists() {
        // Setup Inicial
        return HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../../templates/setup.html"));
    }
    match std::fs::read_to_string(index_path) {
        Ok(content) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body(content),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

// Handler específico para servir a página de configuração quando solicitado via menu
async fn library_settings_page_handler() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../templates/setup.html"))
}

// Retorna a configuração atual (JSON) para preencher o formulário
// ALTERADO: Agora recebe o stats_path para saber onde buscar o settings.json correto
async fn get_config_handler(stats_path: web::Data<Option<PathBuf>>) -> impl Responder {
    let current_db_path = stats_path.get_ref().as_deref();
    let config_path = AppSettings::get_config_path(current_db_path);

    match AppSettings::load_from_path(&config_path) {
        Some(settings) => HttpResponse::Ok().json(settings),
        None => HttpResponse::NotFound().json(serde_json::json!({"error": "No settings found"}))
    }
}

async fn browse_handler(query: web::Query<BrowseQuery>) -> impl Responder {
    let root = query.path.clone().unwrap_or_else(|| "/".to_string());
    let path = PathBuf::from(&root);

    if !path.exists() {
         return HttpResponse::Ok().json(serde_json::json!({ 
             "error": "Path not found", 
             "current": root, 
             "parent": null,
             "entries": [] 
         }));
    }

    let mut directories = Vec::new();
    let parent = path.parent().map(|p| p.to_string_lossy().to_string());

    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') { continue; }

                let is_sqlite = name.ends_with(".sqlite3");
                
                if file_type.is_dir() || is_sqlite {
                    directories.push(serde_json::json!({
                        "name": name,
                        "path": entry.path().to_string_lossy(),
                        "type": if file_type.is_dir() { "dir" } else { "file" }
                    }));
                }
            }
        }
    }
    
    directories.sort_by(|a, b| {
        let type_a = a["type"].as_str().unwrap();
        let type_b = b["type"].as_str().unwrap();
        if type_a == type_b {
            a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap())
        } else {
            type_a.cmp(type_b) 
        }
    });
    
    HttpResponse::Ok().json(serde_json::json!({
        "current": path.to_string_lossy(),
        "parent": parent,
        "entries": directories
    }))
}

async fn get_languages_handler() -> impl Responder {
    let languages = get_available_locales();
    HttpResponse::Ok().json(languages)
}

async fn setup_handler(payload: web::Json<SetupPayload>) -> impl Responder {
    let library_paths: Vec<PathBuf> = payload.library_paths.iter().map(PathBuf::from).collect();
    let stats_path = payload.statistics_db_path.as_ref().map(PathBuf::from);

    // Validação simples
    for path in &library_paths {
        if !path.exists() || !path.is_dir() {
             return HttpResponse::BadRequest().body(format!("Invalid library path: {:?}", path));
        }
    }
    if let Some(ref p) = stats_path {
        if !p.exists() {
             return HttpResponse::BadRequest().body(format!("Invalid statistics DB path: {:?}", p));
        }
    }

    let settings = AppSettings {
        library_paths,
        statistics_db_path: stats_path.clone(), // Clone para uso no save()
        language: payload.language.clone(),
    };

    // O método .save() atualizado vai olhar para settings.statistics_db_path
    // e salvar o JSON na mesma pasta do SQLite.
    if let Err(e) = settings.save() {
        log::error!("Failed to save settings: {}", e);
        return HttpResponse::InternalServerError().body(format!("Error saving settings: {}", e));
    }

    log::info!("Configuration saved. Scheduling restart...");

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        log::info!("Restarting system now...");
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("koshelf"));
            let _ = std::process::Command::new(exe).exec();
        }
    });

    HttpResponse::Ok().json(serde_json::json!({ 
        "status": "success", 
        "message": "Configuration saved. Restarting..." 
    }))
}

async fn set_language_handler(
    payload: web::Json<LanguagePayload>,
    library: web::Data<Arc<Vec<LibraryItem>>>,
    output_dir: web::Data<PathBuf>,
    stats_path: web::Data<Option<PathBuf>>, // Recebe o caminho atual do DB (estado do servidor)
) -> impl Responder {
    let new_lang = payload.lang.clone();
    
    // 1. Persistência: Salva a nova configuração no settings.json CORRETO
    let current_db_path = stats_path.get_ref().as_deref();
    let config_path = AppSettings::get_config_path(current_db_path);
    
    // Tenta carregar config existente para preservar paths, ou cria default se não existir
    let mut settings = AppSettings::load_from_path(&config_path).unwrap_or_default();
    
    // Atualiza apenas o necessário
    settings.language = new_lang.clone();
    
    // Garante que o path do DB está na struct para que o .save() saiba onde salvar
    if settings.statistics_db_path.is_none() {
        settings.statistics_db_path = stats_path.get_ref().clone();
    }

    if let Err(e) = settings.save() {
        log::error!("Failed to persist language change to {:?}: {}", config_path, e);
    }

    // 2. Atualiza estado em memória e regenera
    set_global_locale(new_lang);

    let items = library.get_ref().clone();
    let out_path = output_dir.get_ref().clone();
    let db_path = stats_path.get_ref().clone();

    let result = web::block(move || {
        crate::site_generator::generate_site(&items, &out_path, db_path)
    }).await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().json(serde_json::json!({ "status": "regenerated" })),
        Ok(Err(e)) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

impl WebServer {
    pub fn new(
        output_dir: PathBuf,
        port: u16,
        version_notifier: Arc<VersionNotifier>,
        library_items: Arc<Vec<LibraryItem>>,
        library_path: PathBuf,
        statistics_db_path: Option<PathBuf>,
    ) -> Self {
        Self {
            output_dir,
            port,
            version_notifier,
            library_items,
            library_path,
            statistics_db_path,
        }
    }

    pub fn configure_api(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api")
                .route("/stats", web::get().to(Self::get_stats_handler))
                .route("/events/version", web::get().to(Self::version_events_handler))
                .route("/languages", web::get().to(get_languages_handler))
                .route("/settings/language", web::post().to(set_language_handler))
                .route("/setup", web::post().to(setup_handler))
                .route("/config", web::get().to(get_config_handler)) // Handler atualizado
                .route("/browse", web::get().to(browse_handler))
        );
        // Rota para a página HTML de configuração
        cfg.route("/library-settings", web::get().to(library_settings_page_handler));
    }

    async fn version_events_handler() -> impl Responder {
        HttpResponse::Ok()
            .insert_header(("Content-Type", "text/event-stream"))
            .insert_header(("Cache-Control", "no-cache"))
            .body("data: {\"version\": \"1.0.0\"}\n\n")
    }

    async fn get_stats_handler(library: web::Data<Arc<Vec<LibraryItem>>>) -> impl Responder {
        let mut read = 0;
        let mut reading = 0;
        let mut paused = 0;
        let mut unread = 0;

        for item in library.iter() {
            let percent = item.koreader_metadata.as_ref()
                .and_then(|m| m.percent_finished)
                .unwrap_or(0.0);
            
            let status_debug = match &item.koreader_metadata {
                Some(meta) => format!("{:?}", meta).to_lowercase(),
                None => String::new(),
            };

            if status_debug.contains("abandoned") {
                paused += 1;
            } else if percent >= 1.0 {
                read += 1;
            } else if percent > 0.0 {
                reading += 1;
            } else {
                unread += 1;
            }
        }

        HttpResponse::Ok().json(serde_json::json!({
            "total": library.len(),
            "read": read,
            "reading": reading,
            "unread": unread,
            "paused": paused
        }))
    }

    pub async fn run(self) -> Result<()> {
        let output_dir = self.output_dir.clone();
        let library_items = self.library_items.clone();
        let version_notifier = self.version_notifier.clone();
        let library_path = self.library_path.clone();
        let output_dir_data = self.output_dir.clone();
        let stats_path_data = self.statistics_db_path.clone();

        log::info!("Waiting for library items to be ready...");
        let mut retry_count = 0;
        while library_items.is_empty() && retry_count < 10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            retry_count += 1;
        }

        log::info!("Starting server on port {} (Items: {})", self.port, library_items.len());

        HttpServer::new(move || {
            let logger = middleware::Logger::default()
                .exclude("/service-worker.js")
                .exclude("/manifest.json")
                .exclude("/favicon.ico")
                .exclude("/api/events/version"); 

            App::new()
                .wrap(logger)
                .wrap(Cors::permissive())
                .app_data(web::Data::new(library_items.clone()))
                .app_data(web::Data::new(version_notifier.clone()))
                .app_data(web::Data::new(output_dir_data.clone()))
                .app_data(web::Data::new(stats_path_data.clone())) 
                .configure(Self::configure_api)
                // Serve arquivos brutos (livros)
                .service(fs::Files::new("/raw", library_path.clone()).show_files_listing())
                // Rota específica para a raiz (decide entre Setup ou Home)
                .route("/", web::get().to(index_handler))
                // Rota para arquivos estáticos (CSS, JS, e Subpáginas como /calendar/)
                .service(
                    fs::Files::new("/", output_dir.clone())
                        .index_file("index.html") // <--- ESSA LINHA CORRIGE O ERRO DE DIRETÓRIO
                )
        })
        .bind(("0.0.0.0", self.port))?
        .run()
        .await?;

        Ok(())
    }
}