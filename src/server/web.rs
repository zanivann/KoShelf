use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer, middleware, HttpResponse, Responder};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;

use crate::models::LibraryItem;
use crate::server::version::VersionNotifier;

pub struct WebServer {
    output_dir: PathBuf,
    port: u16,
    version_notifier: Arc<VersionNotifier>,
    library_items: Arc<Vec<LibraryItem>>,
    library_path: PathBuf,
}

impl WebServer {
    pub fn new(
        output_dir: PathBuf,
        port: u16,
        version_notifier: Arc<VersionNotifier>,
        library_items: Arc<Vec<LibraryItem>>,
        library_path: PathBuf,
    ) -> Self {
        Self {
            output_dir,
            port,
            version_notifier,
            library_items,
            library_path,
        }
    }

    pub fn configure_api(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api")
                .route("/stats", web::get().to(Self::get_stats_handler))
                .route("/events/version", web::get().to(Self::version_events_handler))
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
            // Pegamos o progresso do metadado
            let percent = item.koreader_metadata.as_ref()
                .and_then(|m| m.percent_finished)
                .unwrap_or(0.0);

            // Verificação de Status Abandoned/Pausado (Case-insensitive)
            let status_debug = format!("{:?}", item.koreader_metadata).to_lowercase();

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

        // Aguarda a sincronização inicial do Scanner
        log::info!("Aguardando sincronização da biblioteca...");
        let mut retry_count = 0;
        while library_items.is_empty() && retry_count < 10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            retry_count += 1;
        }

        log::info!("Iniciando servidor na porta {} (Itens: {})", self.port, library_items.len());

        HttpServer::new(move || {
            // Logger configurado para silenciar os ruídos do frontend e da versão
            let logger = middleware::Logger::default()
                .exclude("/service-worker.js")
                .exclude("/manifest.json")
                .exclude("/favicon.ico")
                .exclude("/api/events/version"); // Retirado conforme solicitado

            App::new()
                .wrap(logger)
                .wrap(Cors::permissive())
                .app_data(web::Data::new(library_items.clone()))
                .app_data(web::Data::new(version_notifier.clone()))
                .configure(Self::configure_api)
                .service(fs::Files::new("/books", library_path.clone()).show_files_listing())
                .service(fs::Files::new("/settings", library_path.clone()))
                .service(fs::Files::new("/", output_dir.clone()).index_file("index.html"))
        })
        .bind(("0.0.0.0", self.port))?
        .run()
        .await?;

        Ok(())
    }
}