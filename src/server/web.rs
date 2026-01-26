use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer, middleware, HttpResponse, Responder};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use serde::Deserialize; // Necessário para ler o JSON do frontend

use crate::models::LibraryItem;
use crate::server::version::VersionNotifier;
// Importamos a função de lista e a de definir o idioma global
use crate::i18n::{get_available_locales, set_global_locale};

pub struct WebServer {
    output_dir: PathBuf,
    port: u16,
    version_notifier: Arc<VersionNotifier>,
    library_items: Arc<Vec<LibraryItem>>,
    library_path: PathBuf,
    statistics_db_path: Option<PathBuf>, // <--- ADD FIELD
}

// Estrutura para receber o JSON { "lang": "pt" }
#[derive(Deserialize)]
struct LanguagePayload {
    lang: String,
}

// --- HANDLERS INDEPENDENTES ---

// Retorna a lista de idiomas para o Dropdown
async fn get_languages_handler() -> impl Responder {
    let languages = get_available_locales();
    HttpResponse::Ok().json(languages)
}

// CHANGE: Update handler signature and logic
async fn set_language_handler(
    payload: web::Json<LanguagePayload>,
    library: web::Data<Arc<Vec<LibraryItem>>>,
    output_dir: web::Data<PathBuf>,
    stats_path: web::Data<Option<PathBuf>>, // <--- INJECT DATA
) -> impl Responder {
    let new_lang = payload.lang.clone();
    
    log::info!("Mudando idioma para '{}' e regenerando site...", new_lang);
    
    set_global_locale(new_lang);

    let items = library.get_ref().clone();
    let out_path = output_dir.get_ref().clone();
    let db_path = stats_path.get_ref().clone(); // <--- CLONE PATH

    let result = web::block(move || {
        // CHANGE: Pass db_path to generate_site
        crate::site_generator::generate_site(&items, &out_path, db_path)
    }).await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().json(serde_json::json!({ "status": "regenerated" })),
        Ok(Err(e)) => {
            log::error!("Erro na regeneração: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Erro: {}", e))
        },
        Err(e) => {
            log::error!("Erro no blocking: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

impl WebServer {
    pub fn new(
        output_dir: PathBuf,
        port: u16,
        version_notifier: Arc<VersionNotifier>,
        library_items: Arc<Vec<LibraryItem>>,
        library_path: PathBuf,
        statistics_db_path: Option<PathBuf>, // <--- ADD ARGUMENT
    ) -> Self {
        Self {
            output_dir,
            port,
            version_notifier,
            library_items,
            library_path,
            statistics_db_path, // <--- STORE IT
        }
    }

    pub fn configure_api(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api")
                .route("/stats", web::get().to(Self::get_stats_handler))
                .route("/events/version", web::get().to(Self::version_events_handler))
                .route("/languages", web::get().to(get_languages_handler))
                .route("/settings/language", web::post().to(set_language_handler))
        );
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
        
        // CHANGE: Capture stats path for injection
        let stats_path_data = self.statistics_db_path.clone();

        log::info!("Aguardando sincronização da biblioteca...");
        let mut retry_count = 0;
        while library_items.is_empty() && retry_count < 10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            retry_count += 1;
        }

        log::info!("Iniciando servidor na porta {} (Itens: {})", self.port, library_items.len());

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
                // CHANGE: Inject statistics_db_path
                .app_data(web::Data::new(stats_path_data.clone())) 
                .configure(Self::configure_api)
                .service(fs::Files::new("/raw", library_path.clone()).show_files_listing())
                .service(fs::Files::new("/", output_dir.clone()).index_file("index.html"))
        })
        .bind(("0.0.0.0", self.port))?
        .run()
        .await?;

        Ok(())
    }
}