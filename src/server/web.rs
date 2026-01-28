// fileName: src/server/web.rs

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

// --- TIPOS WRAPPER PARA EVITAR CONFLITO DE INJEÇÃO ---
// O Actix diferencia por tipo. Criamos tipos distintos para cada Path.
#[derive(Clone)]
struct OutputDir(PathBuf);

#[derive(Clone)]
struct ConfigPath(PathBuf);

#[derive(Clone)]
struct StatsPath(Option<PathBuf>);

pub struct WebServer {
    output_dir: PathBuf,
    port: u16,
    version_notifier: Arc<VersionNotifier>,
    library_items: Arc<Vec<LibraryItem>>,
    library_path: PathBuf,
    statistics_db_path: Option<PathBuf>,
    config_file_path: PathBuf,
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

async fn index_handler(
    output_dir: web::Data<OutputDir>,
    config_path: web::Data<ConfigPath>
) -> impl Responder {
    // 1. Caminhos
    let index_path = output_dir.get_ref().0.join("index.html");
    let settings_path = &config_path.get_ref().0;

    // 2. Headers anti-cache rigorosos
    let cache_control = ("Cache-Control", "no-store, no-cache, must-revalidate, proxy-revalidate");

    // 3. Lógica de Decisão: A Verdade está no Disco (settings.json)
    // Se o arquivo de configuração existe, NUNCA mostre a tela de setup.
    let is_configured = settings_path.exists();

    if is_configured {
        // --- MODO APP ---
        if index_path.exists() {
            // Cenário Ideal: Config existe e Site gerado existe.
            match std::fs::read_to_string(index_path) {
                Ok(content) => return HttpResponse::Ok()
                    .insert_header(cache_control)
                    .content_type("text/html; charset=utf-8")
                    .body(content),
                Err(_) => return HttpResponse::InternalServerError().body("Failed to read application index."),
            }
        } else {
            // Cenário "Loading": Config existe, mas o site ainda está sendo gerado na pasta temp.
            // Retorna um HTML de "Loading" que se atualiza sozinho até o arquivo aparecer.
            return HttpResponse::Ok()
                .content_type("text/html")
                .body(r#"
                    <html>
                    <head><meta http-equiv="refresh" content="2"></head>
                    <body style="background:#111827;color:white;display:flex;justify-content:center;align-items:center;height:100vh;font-family:sans-serif;">
                        <div>
                            <h1>Loading Library...</h1>
                            <p>Generating static files. Please wait.</p>
                        </div>
                    </body>
                    </html>
                "#);
        }
    }

    // --- MODO SETUP ---
    // Só chega aqui se não tiver settings.json
    HttpResponse::Ok()
        .insert_header(cache_control)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../templates/setup.html"))
}

// Handler specifically for serving the settings page when requested via menu
async fn library_settings_page_handler() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../templates/setup.html"))
}

// Returns current config (JSON) to populate the form
async fn get_config_handler(config_path: web::Data<ConfigPath>) -> impl Responder {
    match AppSettings::load_from_path(&config_path.get_ref().0) {
        Some(settings) => HttpResponse::Ok()
            .insert_header(("Cache-Control", "no-cache"))
            .json(settings),
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

async fn setup_handler(
    payload: web::Json<SetupPayload>,
    config_path: web::Data<ConfigPath> // Usando o Wrapper correto
) -> impl Responder {
    let library_paths: Vec<PathBuf> = payload.library_paths.iter().map(PathBuf::from).collect();
    let stats_path = payload.statistics_db_path.as_ref().map(PathBuf::from);

    // Validation
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
        statistics_db_path: stats_path.clone(),
        language: payload.language.clone(),
        config_file_path: config_path.get_ref().0.clone(),
    };

    if let Err(e) = settings.save() {
        log::error!("Failed to save settings: {}", e);
        return HttpResponse::InternalServerError().body(format!("Error saving settings: {}", e));
    }

    log::info!("Configuration saved to {:?}. Restarting...", settings.config_file_path);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        log::info!("Restarting system now...");
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("koshelf"));
            let _ = std::process::Command::new(exe).exec();
        }
        #[cfg(not(unix))]
        {
            std::process::exit(0);
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
    output_dir: web::Data<OutputDir>,
    stats_path: web::Data<StatsPath>, 
    config_path: web::Data<ConfigPath>,
) -> impl Responder {
    let new_lang = payload.lang.clone();
    let config_file_path = &config_path.get_ref().0;
    
    // 1. Load from the correct path
    let mut settings = AppSettings::load_from_path(config_file_path).unwrap_or_default();
    
    // Update and Save
    settings.config_file_path = config_file_path.clone();
    settings.language = new_lang.clone();
    
    if settings.statistics_db_path.is_none() {
        settings.statistics_db_path = stats_path.get_ref().0.clone();
    }

    if let Err(e) = settings.save() {
        log::error!("Failed to persist language change to {:?}: {}", config_file_path, e);
    }

    // 2. Update memory state and regenerate
    set_global_locale(new_lang);

    let items = library.get_ref().clone();
    let out_path = output_dir.get_ref().0.clone();
    let db_path = stats_path.get_ref().0.clone();

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
        config_file_path: PathBuf,
    ) -> Self {
        Self {
            output_dir,
            port,
            version_notifier,
            library_items,
            library_path,
            statistics_db_path,
            config_file_path,
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
                .route("/config", web::get().to(get_config_handler))
                .route("/browse", web::get().to(browse_handler))
        );
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
        let library_items = self.library_items.clone();
        let version_notifier = self.version_notifier.clone();
        let library_path = self.library_path.clone();
        
        let output_dir_data = OutputDir(self.output_dir.clone());
        let stats_path_data = StatsPath(self.statistics_db_path.clone());
        let config_path_data = ConfigPath(self.config_file_path.clone());

        let output_dir_for_files = self.output_dir.clone();

        log::info!("Waiting for library items to be ready...");
        let mut retry_count = 0;
        while library_items.is_empty() && retry_count < 10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            retry_count += 1;
        }

        log::info!("Starting server on port {} (Items: {})", self.port, library_items.len());

        HttpServer::new(move || {
            // LOGGER MODO SILENCIOSO
            let logger = middleware::Logger::default()
                // 1. Arquivos de sistema e PWA
                .exclude("/service-worker.js")
                .exclude("/manifest.json")
                .exclude("/favicon.ico")
                .exclude("/version.txt")
                .exclude("/api/events/version")
                .exclude("/") // Remove o log de acesso à raiz
                
                // 2. Bloqueia Cache Manifest (com ou sem query string)
                .exclude_regex(".*cache-manifest.json.*")
                
                // 3. Bloqueia Conteúdo de Livros (Capas, JSONs, MDs)
                .exclude_regex("^/books/.*")
                
                // 4. Bloqueia Assets do Site (CSS, JS, Ícones, Capas cacheadas)
                .exclude_regex("^/assets/.*")
                
                // 5. Bloqueia Páginas Principais (para não poluir ao navegar)
                .exclude_regex("^/calendar/.*")
                .exclude_regex("^/statistics/.*")
                .exclude_regex("^/recap/.*");

            App::new()
                .wrap(logger)
                .wrap(Cors::permissive())
                .app_data(web::Data::new(library_items.clone()))
                .app_data(web::Data::new(version_notifier.clone()))
                .app_data(web::Data::new(output_dir_data.clone()))
                .app_data(web::Data::new(stats_path_data.clone()))
                .app_data(web::Data::new(config_path_data.clone()))
                .configure(Self::configure_api)
                .service(fs::Files::new("/raw", library_path.clone()).show_files_listing())
                .route("/", web::get().to(index_handler))
                .service(
                    fs::Files::new("/", output_dir_for_files.clone())
                        .index_file("index.html")
                )
        })
        .bind(("0.0.0.0", self.port))?
        .run()
        .await?;

        Ok(())
    }
}